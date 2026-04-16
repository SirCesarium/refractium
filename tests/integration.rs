use refractium::core::Refractium;
use refractium::protocols::http::Http;
use refractium::types::{ForwardTarget, ProtocolRoute};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn test_tcp_proxy_flow() {
    let backend_addr: SocketAddr = "127.0.0.1:9090".parse().expect("Invalid addr");
    let proxy_addr: SocketAddr = "127.0.0.1:8081".parse().expect("Invalid addr");

    tokio::spawn(async move {
        if let Ok(listener) = TcpListener::bind(backend_addr).await
            && let Ok((mut stream, _)) = listener.accept().await
        {
            let mut buf = [0u8; 12];
            if stream.read_exact(&mut buf).await.is_ok() {
                let _ = stream.write_all(b"HELLO BACKEND").await;
            }
        }
    });

    let routes = vec![ProtocolRoute {
        protocol: Arc::new(Http),
        sni: None,
        forward_to: ForwardTarget::Single(backend_addr.to_string()),
    }];

    let refractium = Refractium::builder()
        .routes(routes, Vec::new())
        .peek_config(1024, 3000)
        .build()
        .expect("Failed to build Refractium");

    let token = refractium.cancel_token();
    let r_clone = Arc::new(refractium);
    let r_task = Arc::clone(&r_clone);

    let refractium_task = tokio::spawn(async move {
        let _ = r_task.run_tcp(proxy_addr).await;
    });

    sleep(Duration::from_millis(200)).await;

    if let Ok(mut client) = TcpStream::connect(proxy_addr).await {
        let _ = client.write_all(b"GET / HTTP/1").await;

        let mut resp = [0u8; 13];
        if client.read_exact(&mut resp).await.is_ok() {
            assert_eq!(&resp, b"HELLO BACKEND");
        }
    }

    token.cancel();
    let _ = refractium_task.await;
}
