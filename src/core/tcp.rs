use crate::core::health::HealthMonitor;
use crate::core::proxy::proxy_tcp;
use crate::core::router::Router;
use crate::errors::{PrismaError, Result};
use crate::prisma_debug;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{io, time};
use tokio::net::{TcpListener, TcpStream};
use tokio::time as tokio_time;
use tokio_util::sync::CancellationToken;

/// A high-performance TCP server that performs protocol identification and routing.
pub struct TcpServer {
    addr: SocketAddr,
    router: Arc<Router>,
    _health: Arc<HealthMonitor>,
    peek_buffer_size: usize,
    peek_timeout_ms: u64,
    cancel_token: CancellationToken,
}

impl TcpServer {
    /// Creates a new `TcpServer` instance.
    #[must_use]
    pub const fn new(
        addr: SocketAddr,
        router: Arc<Router>,
        health: Arc<HealthMonitor>,
        peek_buffer_size: usize,
        peek_timeout_ms: u64,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            addr,
            router,
            _health: health,
            peek_buffer_size,
            peek_timeout_ms,
            cancel_token,
        }
    }

    /// Starts the TCP server and begins accepting connections.
    ///
    /// # Errors
    ///
    /// Returns a `PrismaError` if:
    /// - Binding to the address fails.
    /// - Accepting a connection fails.
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| PrismaError::BindError(self.addr.to_string(), e))?;

        loop {
            tokio::select! {
                () = self.cancel_token.cancelled() => {
                    prisma_debug!("TCP Server shutting down...");
                    break;
                }
                accept_result = listener.accept() => {
                    self.accept_connection(accept_result)?;
                }
            }
        }
        Ok(())
    }

    fn accept_connection(&self, res: io::Result<(TcpStream, SocketAddr)>) -> Result<()> {
        let (socket, peer) = res?;
        let router = Arc::clone(&self.router);
        let peek_size = self.peek_buffer_size;
        let peek_timeout = self.peek_timeout_ms;

        tokio::spawn(async move {
            if let Err(e) = Self::handle_connection(socket, router, peek_size, peek_timeout).await {
                prisma_debug!("TCP Error at {}: {}", peer, e);
            }
        });
        Ok(())
    }

    async fn handle_connection(
        socket: TcpStream,
        router: Arc<Router>,
        peek_size: usize,
        peek_timeout: u64,
    ) -> Result<()> {
        let mut buf = vec![0u8; peek_size];
        let start = time::Instant::now();
        let timeout_duration = Duration::from_millis(peek_timeout);

        loop {
            socket.readable().await.map_err(PrismaError::Io)?;

            let n = socket.peek(&mut buf).await.map_err(PrismaError::Io)?;

            if n > 0
                && let Some(target_addr) = router.route(&buf[..n]).await
            {
                let backend = TcpStream::connect(&target_addr).await?;
                return proxy_tcp(socket, backend).await.map_err(PrismaError::Io);
            }

            if start.elapsed() >= timeout_duration {
                return Err(PrismaError::Generic(
                    "Protocol identification timeout".to_string(),
                ));
            }
        }
    }
}
