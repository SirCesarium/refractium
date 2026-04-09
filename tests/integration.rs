use prisma_rs::core::Prisma;
use prisma_rs::protocols::ProtocolRegistry;
use prisma_rs::protocols::http::Http;
use std::collections::HashMap;
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

    let mut registry = ProtocolRegistry::new();
    registry.register(Box::new(Http));
    let mut routes = HashMap::new();
    routes.insert("Http".to_string(), vec![backend_addr.to_string()]);

    let prisma = Prisma::builder()
        .registries(Arc::new(registry), Arc::new(ProtocolRegistry::new()))
        .routes(routes, HashMap::new())
        .peek_config(1024, 3000)
        .build();

    let token = prisma.cancel_token();
    let prisma_task = tokio::spawn(async move {
        let _ = prisma.run_tcp(proxy_addr).await;
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
    let _ = prisma_task.await;
}
