use crate::core::health::HealthMonitor;
use crate::core::router::Router;
use crate::errors::{PrismaError, Result};
use crate::prisma_debug;
use bytes::Bytes;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use tokio_util::sync::CancellationToken;

/// A high-performance UDP server that performs protocol identification and session-based routing.
pub struct UdpServer {
    addr: SocketAddr,
    router: Arc<Router>,
    _health: Arc<HealthMonitor>,
    sessions: Arc<RwLock<HashMap<SocketAddr, Arc<UdpSocket>>>>,
    cancel_token: CancellationToken,
}

impl UdpServer {
    /// Creates a new `UdpServer` instance.
    #[must_use]
    pub fn new(
        addr: SocketAddr,
        router: Arc<Router>,
        health: Arc<HealthMonitor>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            addr,
            router,
            _health: health,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            cancel_token,
        }
    }

    /// Starts the UDP server and begins processing packets.
    ///
    /// This method blocks until the cancellation token is triggered or a fatal
    /// error occurs.
    ///
    /// # Errors
    ///
    /// Returns a `PrismaError` if:
    /// - Binding to the address fails.
    /// - Receiving a packet from the socket fails.
    pub async fn start(&self) -> Result<()> {
        let socket = Arc::new(
            UdpSocket::bind(self.addr)
                .await
                .map_err(|e| PrismaError::BindError(self.addr.to_string(), e))?,
        );
        let mut buf = [0u8; 2048];

        loop {
            tokio::select! {
                () = self.cancel_token.cancelled() => {
                    prisma_debug!("UDP Server shutting down...");
                    break;
                }
                recv_result = socket.recv_from(&mut buf) => {
                    let (n, peer) = recv_result?;
                    let data = Bytes::copy_from_slice(&buf[..n]);
                    self.handle_packet(Arc::clone(&socket), data, peer).await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_packet(
        &self,
        socket: Arc<UdpSocket>,
        data: Bytes,
        peer: SocketAddr,
    ) -> Result<()> {
        {
            let sessions_guard = self.sessions.read().await;
            if let Some(proxy_socket) = sessions_guard.get(&peer) {
                proxy_socket.send_to(&data, peer).await?;
                return Ok(());
            }
        }

        let target_addr = self.router.route(&data).await;
        let Some(target) = target_addr else {
            return Ok(());
        };

        let proxy_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
        let mut sessions_guard = self.sessions.write().await;
        sessions_guard.insert(peer, Arc::clone(&proxy_socket));
        drop(sessions_guard);

        let sessions_task = Arc::clone(&self.sessions);
        let token = self.cancel_token.clone();

        tokio::spawn(async move {
            tokio::select! {
                () = token.cancelled() => {
                    prisma_debug!("Closing UDP session for {} due to shutdown", peer);
                }
                res = Self::handle_session(data, target, peer, socket, proxy_socket, sessions_task) => {
                    if let Err(e) = res {
                        prisma_debug!("UDP Session Error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_session(
        initial_data: Bytes,
        target: String,
        peer: SocketAddr,
        main_sock: Arc<UdpSocket>,
        proxy_sock: Arc<UdpSocket>,
        sessions: Arc<RwLock<HashMap<SocketAddr, Arc<UdpSocket>>>>,
    ) -> Result<()> {
        proxy_sock.send_to(&initial_data, &target).await?;
        let mut resp_buf = [0u8; 2048];
        let timeout_duration = Duration::from_secs(30);

        loop {
            tokio::select! {
                result = proxy_sock.recv_from(&mut resp_buf) => {
                    let (n, _) = result.map_err(PrismaError::Io)?;
                    main_sock.send_to(&resp_buf[..n], &peer).await?;
                }
                () = time::sleep(timeout_duration) => {
                    break;
                }
            }
        }

        sessions.write().await.remove(&peer);
        Ok(())
    }
}
