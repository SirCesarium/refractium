// @swt-disable max-depth

use crate::core::health::HealthMonitor;
use crate::core::proxy::proxy_tcp;
use crate::core::router::{RouteResult, Router};
use crate::errors::{RefractiumError, Result};
use crate::{refractium_debug, refractium_error, refractium_trace, refractium_warn};
use dashmap::DashMap;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// A high-performance TCP server that performs protocol identification and routing.
pub struct TcpServer {
    addr: SocketAddr,
    router: Arc<Router>,
    _health: Arc<HealthMonitor>,
    peek_buffer_size: usize,
    peek_timeout_ms: u64,
    limit: Arc<Semaphore>,
    max_connections_per_ip: usize,
    cancel_token: CancellationToken,
    conns_per_ip: Arc<DashMap<IpAddr, usize>>,
}

struct ConnGuard {
    ip: IpAddr,
    map: Arc<DashMap<IpAddr, usize>>,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        if let Some(mut entry) = self.map.get_mut(&self.ip) {
            *entry = entry.saturating_sub(1);
            if *entry == 0 {
                drop(entry);
                self.map.remove(&self.ip);
            }
        }
    }
}

impl TcpServer {
    /// Creates a new `TcpServer` instance.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        addr: SocketAddr,
        router: Arc<Router>,
        health: Arc<HealthMonitor>,
        peek_buffer_size: usize,
        peek_timeout_ms: u64,
        max_connections: usize,
        max_connections_per_ip: usize,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            addr,
            router,
            _health: health,
            peek_buffer_size,
            peek_timeout_ms,
            limit: Arc::new(Semaphore::new(max_connections)),
            max_connections_per_ip,
            conns_per_ip: Arc::new(DashMap::new()),
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
        let ip = peer.ip();

        let conns_map = Arc::clone(&self.conns_per_ip);
        let router = Arc::clone(&self.router);
        let limit = Arc::clone(&self.limit);
        let peek_size = self.peek_buffer_size;
        let peek_timeout = self.peek_timeout_ms;
        let max_per_ip = self.max_connections_per_ip;

        {
            let mut entry = conns_map.entry(ip).or_insert(0);
            if *entry >= max_per_ip {
                refractium_warn!("IP {} reached limit, rejecting {}", ip, peer);
                return Ok(());
            }
            *entry += 1;
        }

        tokio::spawn(async move {
            let _guard = ConnGuard {
                ip,
                map: Arc::clone(&conns_map),
            };

            let Ok(_permit) = limit.try_acquire() else {
                refractium_warn!("Global connection limit reached, rejecting {}", peer);
                return;
            };

            if let Err(e) = Self::handle_connection(socket, router, peek_size, peek_timeout).await {
                match e {
                    RefractiumError::Io(ref io_err)
                        if io_err.kind() == io::ErrorKind::ConnectionReset
                            || io_err.kind() == io::ErrorKind::BrokenPipe =>
                    {
                        refractium_trace!("Client {} disconnected abruptly", peer);
                    }
                    _ => refractium_error!("TCP Error at {}: {}", peer, e),
                }
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
            RouteResult::Matched(proto, addr, implementation) => {
                refractium_debug!("Route matched: {} -> {}", proto, addr);
                let backend = TcpStream::connect(&addr).await?;

                #[cfg(feature = "hooks")]
                let hooks = implementation.hooks();

                #[cfg(not(feature = "hooks"))]
                let _ = implementation;

                proxy_tcp(
                    socket,
                    backend,
                    #[cfg(feature = "hooks")]
                    hooks,
                    #[cfg(feature = "hooks")]
                    proto,
                )
                .await
                .map_err(RefractiumError::Io)
            }
            RouteResult::Fallback(addr) => {
                refractium_debug!("No protocol match, routing to fallback -> {}", addr);
                let backend = TcpStream::connect(&addr).await?;
                proxy_tcp(
                    socket,
                    backend,
                    #[cfg(feature = "hooks")]
                    Vec::new(),
                    #[cfg(feature = "hooks")]
                    "fallback".to_string(),
                )
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
        let duration = Duration::from_millis(peek_timeout);

        let identify_fut = async {
            loop {
                socket.readable().await?;
                let n = socket.peek(&mut buf).await?;
                if n > 0
                    && let Some(result) = router.route(&buf[..n]).await
                {
                    return Ok::<RouteResult, io::Error>(result);
                }
            }
        };

        match timeout(duration, identify_fut).await {
            Ok(Ok(res)) => Ok(res),
            Ok(Err(e)) => Err(RefractiumError::Io(e)),
            Err(_) => Ok(router.route_fallback().await),
        }
    }
}
