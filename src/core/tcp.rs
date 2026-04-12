use crate::core::health::HealthMonitor;
use crate::core::proxy::proxy_tcp;
use crate::core::router::{RouteResult, Router};
use crate::errors::{RefractiumError, Result};
use crate::refractium_debug;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{io, time};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

/// A high-performance TCP server that performs protocol identification and routing.
pub struct TcpServer {
    addr: SocketAddr,
    router: Arc<Router>,
    _health: Arc<HealthMonitor>,
    peek_buffer_size: usize,
    peek_timeout_ms: u64,
    limit: Arc<Semaphore>,
    cancel_token: CancellationToken,
}

impl TcpServer {
    /// Creates a new `TcpServer` instance.
    #[must_use]
    pub fn new(
        addr: SocketAddr,
        router: Arc<Router>,
        health: Arc<HealthMonitor>,
        peek_buffer_size: usize,
        peek_timeout_ms: u64,
        max_connections: usize,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            addr,
            router,
            _health: health,
            peek_buffer_size,
            peek_timeout_ms,
            limit: Arc::new(Semaphore::new(max_connections)),
            cancel_token,
        }
    }

    /// Starts the TCP server and begins accepting connections.
    ///
    /// # Errors
    ///
    /// Returns a `RefractiumError` if binding to the address fails.
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| RefractiumError::BindError(self.addr.to_string(), e))?;

        loop {
            tokio::select! {
                () = self.cancel_token.cancelled() => break,
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
        let limit = Arc::clone(&self.limit);
        let peek_size = self.peek_buffer_size;
        let peek_timeout = self.peek_timeout_ms;

        tokio::spawn(async move {
            let Ok(_permit) = limit.try_acquire() else {
                refractium_debug!("Connection limit reached, rejecting {}", peer);
                return;
            };

            if let Err(e) = Self::handle_connection(socket, router, peek_size, peek_timeout).await {
                refractium_debug!("TCP Error at {}: {}", peer, e);
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
        let route = Self::identify_protocol(&socket, &router, peek_size, peek_timeout).await?;

        match route {
            RouteResult::Matched(proto, addr) => {
                refractium_debug!("Route matched: {} -> {}", proto, addr);
                let backend = TcpStream::connect(&addr).await?;
                proxy_tcp(socket, backend)
                    .await
                    .map_err(RefractiumError::Io)
            }
            RouteResult::Fallback(addr) => {
                refractium_debug!("No protocol match, routing to fallback -> {}", addr);
                let backend = TcpStream::connect(&addr).await?;
                proxy_tcp(socket, backend)
                    .await
                    .map_err(RefractiumError::Io)
            }
            RouteResult::Discarded => {
                refractium_debug!("No route found and no healthy fallback. Dropping connection.");
                Ok(())
            }
        }
    }

    async fn identify_protocol(
        socket: &TcpStream,
        router: &Router,
        peek_size: usize,
        peek_timeout: u64,
    ) -> Result<RouteResult> {
        let mut buf = vec![0u8; peek_size];
        let start = time::Instant::now();
        let timeout = Duration::from_millis(peek_timeout);

        loop {
            socket.readable().await.map_err(RefractiumError::Io)?;
            let n = socket.peek(&mut buf).await.map_err(RefractiumError::Io)?;

            if n > 0 {
                // If router returns Some, it means identification is DONE (match, fallback or discard)
                if let Some(result) = router.route(&buf[..n]).await {
                    return Ok(result);
                }
            }

            if start.elapsed() >= timeout {
                // Time's up. Force fallback if available.
                return Ok(router.route_fallback().await);
            }
        }
    }
}
