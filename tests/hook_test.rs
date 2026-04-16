// @swt-disable max-repetition

use bytes::Bytes;
use refractium::core::Refractium;
use refractium::hook_protocol;
use refractium::protocols::ftp::Ftp;
use refractium::protocols::hooks::{Direction, HookContext, ProtocolHook};
use refractium::protocols::http::Http;
use refractium::types::{ForwardTarget, ProtocolRoute};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, sleep};

struct Tracker {
    http_calls: Arc<Mutex<usize>>,
    ftp_calls: Arc<Mutex<usize>>,
}

#[derive(Clone)]
struct HttpHook {
    counter: Arc<Mutex<usize>>,
}

impl ProtocolHook for HttpHook {
    fn name(&self) -> &'static str {
        "http_hook"
    }
    fn on_packet(&self, _context: &HookContext, _direction: Direction, _packet: Bytes) {
        let mut n = self.counter.lock().unwrap();
        *n += 1;
    }
}

#[derive(Clone)]
struct FtpHook {
    counter: Arc<Mutex<usize>>,
}

impl ProtocolHook for FtpHook {
    fn name(&self) -> &'static str {
        "ftp_hook"
    }
    fn on_packet(&self, _context: &HookContext, _direction: Direction, _packet: Bytes) {
        let mut n = self.counter.lock().unwrap();
        *n += 1;
    }
}

#[cfg(feature = "hooks")]
#[tokio::test]
async fn test_hook_isolation() {
    let backend_addr: SocketAddr = "127.0.0.1:9095".parse().unwrap();
    let proxy_addr: SocketAddr = "127.0.0.1:8085".parse().unwrap();

    let tracker = Tracker {
        http_calls: Arc::new(Mutex::new(0)),
        ftp_calls: Arc::new(Mutex::new(0)),
    };

    tokio::spawn(async move {
        if let Ok(listener) = TcpListener::bind(backend_addr).await {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf).await;
                    let _ = stream.write_all(b"OK").await;
                }
            }
        }
    });

    hook_protocol!(
        wrapper: HookedHttp,
        proto: Http,
        hooks: []
    );

    hook_protocol!(
        wrapper: HookedFtp,
        proto: Ftp,
        hooks: []
    );

    let http_hooks: Vec<Arc<dyn ProtocolHook>> = vec![Arc::new(HttpHook {
        counter: Arc::clone(&tracker.http_calls),
    })];
    let hooked_http = Arc::new(HookedHttp::with_hooks(http_hooks));

    let ftp_hooks: Vec<Arc<dyn ProtocolHook>> = vec![Arc::new(FtpHook {
        counter: Arc::clone(&tracker.ftp_calls),
    })];
    let hooked_ftp = Arc::new(HookedFtp::with_hooks(ftp_hooks));

    let routes = vec![
        ProtocolRoute {
            protocol: hooked_http,
            sni: None,
            forward_to: ForwardTarget::Single(backend_addr.to_string()),
        },
        ProtocolRoute {
            protocol: hooked_ftp,
            sni: None,
            forward_to: ForwardTarget::Single(backend_addr.to_string()),
        },
    ];

    let refractium = Refractium::builder()
        .routes(routes, Vec::new())
        .build()
        .expect("Failed to build Refractium");

    let token = refractium.cancel_token();
    let r_clone = Arc::new(refractium);
    let r_task = Arc::clone(&r_clone);

    tokio::spawn(async move {
        let _ = r_task.run_tcp(proxy_addr).await;
    });

    sleep(Duration::from_millis(300)).await;

    if let Ok(mut http_client) = TcpStream::connect(proxy_addr).await {
        let _ = http_client.write_all(b"GET / HTTP/1.1\r\n\r\n").await;
        let mut buf = Vec::new();
        let _ = http_client.read_to_end(&mut buf).await;
    }

    sleep(Duration::from_millis(200)).await;

    {
        assert!(
            *tracker.http_calls.lock().unwrap() > 0,
            "HTTP hook should have been called"
        );
        assert_eq!(
            *tracker.ftp_calls.lock().unwrap(),
            0,
            "FTP hook should NOT have been called"
        );
    }

    if let Ok(mut ftp_client) = TcpStream::connect(proxy_addr).await {
        let _ = ftp_client.write_all(b"USER anonymous\r\n").await;
        let mut buf = Vec::new();
        let _ = ftp_client.read_to_end(&mut buf).await;
    }

    sleep(Duration::from_millis(200)).await;

    {
        assert!(
            *tracker.http_calls.lock().unwrap() > 0,
            "HTTP hook should still have been called"
        );
        assert!(
            *tracker.ftp_calls.lock().unwrap() > 0,
            "FTP hook should have been called"
        );
    }

    token.cancel();
}
