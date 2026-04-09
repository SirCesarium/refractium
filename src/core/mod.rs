/// Balancer module for load balancing logic.
pub mod balancer;
/// Health module for backend monitoring.
pub mod health;
/// Proxy module for TCP tunneling.
pub mod proxy;
/// Router module for protocol-based routing.
pub mod router;
/// TCP server implementation.
pub mod tcp;
/// UDP server implementation.
pub mod udp;

use crate::errors::Result;
use crate::protocols::ProtocolRegistry;
use balancer::LoadBalancer;
use health::HealthMonitor;
use router::Router;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tcp::TcpServer;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use udp::UdpServer;

/// Main Prisma engine that manages TCP and UDP servers.
pub struct Prisma {
    router_tcp: Arc<Router>,
    router_udp: Arc<Router>,
    health: Arc<HealthMonitor>,
    peek_buffer_size: usize,
    peek_timeout_ms: u64,
    cancel_token: CancellationToken,
}

impl Prisma {
    /// Returns a new `PrismaBuilder` instance.
    #[must_use]
    pub fn builder() -> PrismaBuilder {
        PrismaBuilder::new()
    }

    /// Reloads the routing table for both TCP and UDP.
    pub async fn reload_routes(
        &self,
        tcp: HashMap<String, Vec<String>>,
        udp: HashMap<String, Vec<String>>,
    ) {
        let mut targets = tcp.values().flatten().cloned().collect::<Vec<_>>();
        targets.extend(udp.values().flatten().cloned());

        self.router_tcp
            .update_balancer(tcp, Arc::clone(&self.health))
            .await;
        self.router_udp
            .update_balancer(udp, Arc::clone(&self.health))
            .await;

        self.health.start_monitoring(targets);
    }

    /// Returns a clone of the cancellation token.
    #[must_use]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Runs the TCP server on the specified address.
    ///
    /// # Errors
    ///
    /// Returns a `PrismaError` if the server fails to start or encounters a fatal error.
    pub async fn run_tcp(&self, addr: SocketAddr) -> Result<()> {
        TcpServer::new(
            addr,
            Arc::clone(&self.router_tcp),
            Arc::clone(&self.health),
            self.peek_buffer_size,
            self.peek_timeout_ms,
            self.cancel_token.clone(),
        )
        .start()
        .await
    }

    /// Runs the UDP server on the specified address.
    ///
    /// # Errors
    ///
    /// Returns a `PrismaError` if the server fails to start or encounters a fatal error.
    pub async fn run_udp(&self, addr: SocketAddr) -> Result<()> {
        UdpServer::new(
            addr,
            Arc::clone(&self.router_udp),
            Arc::clone(&self.health),
            self.cancel_token.clone(),
        )
        .start()
        .await
    }

    /// Runs both TCP and UDP servers on the specified address.
    ///
    /// # Errors
    ///
    /// Returns a `PrismaError` if either server fails to start.
    pub async fn run_both(&self, addr: SocketAddr) -> Result<()> {
        tokio::try_join!(self.run_tcp(addr), self.run_udp(addr))?;
        Ok(())
    }
}

/// Builder for the `Prisma` engine.
pub struct PrismaBuilder {
    registry_tcp: Option<Arc<ProtocolRegistry>>,
    registry_udp: Option<Arc<ProtocolRegistry>>,
    routes_tcp: HashMap<String, Vec<String>>,
    routes_udp: HashMap<String, Vec<String>>,
    peek_size: usize,
    peek_timeout: u64,
    cancel_token: Option<CancellationToken>,
}

impl PrismaBuilder {
    /// Creates a new `PrismaBuilder` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry_tcp: None,
            registry_udp: None,
            routes_tcp: HashMap::new(),
            routes_udp: HashMap::new(),
            peek_size: 1024,
            peek_timeout: 3000,
            cancel_token: None,
        }
    }

    /// Sets the protocol registries for TCP and UDP.
    #[must_use]
    pub fn registries(mut self, tcp: Arc<ProtocolRegistry>, udp: Arc<ProtocolRegistry>) -> Self {
        self.registry_tcp = Some(tcp);
        self.registry_udp = Some(udp);
        self
    }

    /// Sets the routing tables for TCP and UDP.
    #[must_use]
    pub fn routes(
        mut self,
        tcp: HashMap<String, Vec<String>>,
        udp: HashMap<String, Vec<String>>,
    ) -> Self {
        self.routes_tcp = tcp;
        self.routes_udp = udp;
        self
    }

    /// Sets the peek configuration for protocol identification.
    #[must_use]
    pub const fn peek_config(mut self, size: usize, timeout_ms: u64) -> Self {
        self.peek_size = size;
        self.peek_timeout = timeout_ms;
        self
    }

    /// Sets the cancellation token for the engine.
    #[must_use]
    pub fn cancel_token(mut self, token: CancellationToken) -> Self {
        self.cancel_token = Some(token);
        self
    }

    /// Builds the `Prisma` engine.
    #[must_use]
    pub fn build(self) -> Prisma {
        let health = Arc::new(HealthMonitor::new());
        self.init_health(&health);

        let router_tcp = Self::do_build_router(self.routes_tcp, self.registry_tcp, &health);
        let router_udp = Self::do_build_router(self.routes_udp, self.registry_udp, &health);

        Prisma {
            router_tcp,
            router_udp,
            health,
            peek_buffer_size: self.peek_size,
            peek_timeout_ms: self.peek_timeout,
            cancel_token: self.cancel_token.unwrap_or_default(),
        }
    }

    fn init_health(&self, health: &Arc<HealthMonitor>) {
        let mut targets = self
            .routes_tcp
            .values()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        targets.extend(self.routes_udp.values().flatten().cloned());
        health.start_monitoring(targets);
    }

    fn do_build_router(
        routes: HashMap<String, Vec<String>>,
        registry: Option<Arc<ProtocolRegistry>>,
        health: &Arc<HealthMonitor>,
    ) -> Arc<Router> {
        let balancer = Arc::new(RwLock::new(LoadBalancer::new(routes, Arc::clone(health))));
        let registry = registry.unwrap_or_else(|| Arc::new(ProtocolRegistry::new()));
        Arc::new(Router::new(registry, balancer))
    }
}

impl Default for PrismaBuilder {
    fn default() -> Self {
        Self::new()
    }
}
