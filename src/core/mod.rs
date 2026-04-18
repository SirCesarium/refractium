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
/// Types used across the core module.
pub mod types;
/// UDP server implementation.
pub mod udp;

use crate::core::types::ProtocolRoute;
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

/// The main engine for the Refractium proxy.
///
/// `Refractium` manages the lifecycle of TCP and UDP servers, protocol identification,
/// and backend health monitoring. It is designed to be highly concurrent and supports
/// atomic routing table updates via `reload_routes`.
///
/// # Examples
///
/// ```rust,no_run
/// use refractium::{Refractium, Http, types::{ProtocolRoute, ForwardTarget}};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let routes = vec![ProtocolRoute {
///         protocol: Arc::new(Http),
///         sni: None,
///         forward_to: ForwardTarget::Single("127.0.0.1:8080".to_string()),
///     }];
///
///     let refractium = Refractium::builder()
///         .routes(routes, Vec::new())
///         .build()?;
///
///     refractium.run_tcp("0.0.0.0:80".parse()?).await?;
///     Ok(())
/// }
/// ```
pub struct Refractium {
    router_tcp: Arc<Router>,
    router_udp: Arc<Router>,
    health: Arc<HealthMonitor>,
    peek_buffer_size: usize,
    peek_timeout_ms: u64,
    max_connections: usize,
    max_connections_per_ip: usize,
    cancel_token: CancellationToken,
}

impl Refractium {
    /// Returns a new [`RefractiumBuilder`] with default settings.
    #[must_use]
    pub const fn builder() -> RefractiumBuilder {
        RefractiumBuilder::new()
    }

    /// Atomically reloads the routing table for all active servers.
    ///
    /// This method updates the internal load balancers and starts monitoring any new
    /// backend addresses. Active connections are not dropped during the reload.
    pub async fn reload_routes(&self, tcp: Vec<ProtocolRoute>, udp: Vec<ProtocolRoute>) {
        let mut targets = tcp
            .iter()
            .flat_map(|r| r.forward_to.to_vec())
            .collect::<Vec<_>>();
        targets.extend(udp.iter().flat_map(|r| r.forward_to.to_vec()));

        self.router_tcp
            .update_balancer(tcp, Arc::clone(&self.health))
            .await;
        self.router_udp
            .update_balancer(udp, Arc::clone(&self.health))
            .await;

        self.health.start_monitoring(targets);
    }

    /// Returns the [`CancellationToken`] used by the engine.
    ///
    /// This can be used to trigger a graceful shutdown from external logic.
    #[must_use]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Starts the TCP server on the provided address.
    ///
    /// This method will block until the server is shut down or an unrecoverable error occurs.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::errors::RefractiumError::BindError`] if the address is already in use
    /// or other IO errors occur during startup.
    pub async fn run_tcp(&self, addr: SocketAddr) -> Result<()> {
        TcpServer::new(
            addr,
            Arc::clone(&self.router_tcp),
            Arc::clone(&self.health),
            self.peek_buffer_size,
            self.peek_timeout_ms,
            self.max_connections,
            self.max_connections_per_ip,
            self.cancel_token.clone(),
        )
        .start()
        .await
    }

    /// Starts the UDP server on the provided address.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::errors::RefractiumError::BindError`] if the address is already in use.
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

    /// Starts both TCP and UDP servers concurrently.
    ///
    /// # Errors
    ///
    /// Returns an error if either the TCP or UDP server fails to bind.
    pub async fn run_both(&self, addr: SocketAddr) -> Result<()> {
        tokio::try_join!(self.run_tcp(addr), self.run_udp(addr))?;
        Ok(())
    }

    /// Prints a visual health report of all configured backends to stdout.
    ///
    /// This is mainly used for debugging or CLI reporting.
    pub async fn report_health(&self) {
        let tcp_status = self.router_tcp.get_health_status().await;
        let udp_status = self.router_udp.get_health_status().await;
        if !tcp_status.is_empty() {
            println!("\n[TCP Backends]");
            Self::print_status_map(tcp_status);
        }
        if !udp_status.is_empty() {
            println!("\n[UDP Backends]");
            Self::print_status_map(udp_status);
        }
        println!();
    }

    fn print_status_map(status: HashMap<String, Vec<(String, bool)>>) {
        for (proto, backends) in status {
            print!("  {proto} -> ");
            for (idx, (addr, alive)) in backends.iter().enumerate() {
                if idx > 0 {
                    print!(", ");
                }
                let s = if *alive {
                    "\x1b[32mUP\x1b[0m"
                } else {
                    "\x1b[31mDOWN\x1b[0m"
                };
                print!("{addr} [{s}]");
            }
            println!();
        }
    }
}

/// A fluent builder for configuring and initializing the [`Refractium`] engine.
pub struct RefractiumBuilder {
    routes_tcp: Vec<ProtocolRoute>,
    routes_udp: Vec<ProtocolRoute>,
    peek_size: usize,
    peek_timeout: u64,
    max_connections: usize,
    max_connections_per_ip: usize,
    cancel_token: Option<CancellationToken>,
}

impl RefractiumBuilder {
    /// Creates a new `RefractiumBuilder` with the following defaults:
    ///
    /// - `peek_size`: 1024 bytes
    /// - `peek_timeout`: 3000 ms
    /// - `max_connections`: 10000
    /// - `max_connections_per_ip`: 50
    #[must_use]
    pub const fn new() -> Self {
        Self {
            routes_tcp: Vec::new(),
            routes_udp: Vec::new(),
            peek_size: 1024,
            peek_timeout: 3000,
            max_connections: 10000,
            max_connections_per_ip: 50,
            cancel_token: None,
        }
    }

    /// Sets the routing table for the engine.
    #[must_use]
    pub fn routes(mut self, tcp: Vec<ProtocolRoute>, udp: Vec<ProtocolRoute>) -> Self {
        self.routes_tcp = tcp;
        self.routes_udp = udp;
        self
    }

    /// Configures the peeking phase behavior.
    ///
    /// - `size`: The maximum number of bytes to inspect for protocol identification.
    /// - `timeout_ms`: Maximum time to wait for sufficient data before falling back.
    #[must_use]
    pub const fn peek_config(mut self, size: usize, timeout_ms: u64) -> Self {
        self.peek_size = size;
        self.peek_timeout = timeout_ms;
        self
    }

    /// Sets the global maximum number of concurrent connections across all IPs.
    #[must_use]
    pub const fn max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Sets the maximum number of concurrent connections allowed from a single IP address.
    ///
    /// Useful for basic DoS mitigation.
    #[must_use]
    pub const fn max_connections_per_ip(mut self, max: usize) -> Self {
        self.max_connections_per_ip = max;
        self
    }

    /// Attaches an external [`CancellationToken`] to the engine.
    #[must_use]
    pub fn cancel_token(mut self, token: CancellationToken) -> Self {
        self.cancel_token = Some(token);
        self
    }

    /// Finalizes the configuration and initializes the [`Refractium`] engine.
    ///
    /// This also starts the background [`HealthMonitor`] task.
    ///
    /// # Errors
    ///
    /// Returns [`crate::errors::RefractiumError::ConfigError`] if the provided configuration
    /// is invalid or incomplete.
    pub fn build(self) -> Result<Refractium> {
        let health = Arc::new(HealthMonitor::new());
        self.init_health(&health);

        let (reg_tcp, reg_udp) = self.build_registries();

        let router_tcp = Self::do_build_router(self.routes_tcp, Arc::new(reg_tcp), &health);
        let router_udp = Self::do_build_router(self.routes_udp, Arc::new(reg_udp), &health);

        Ok(Refractium {
            router_tcp,
            router_udp,
            health,
            peek_buffer_size: self.peek_size,
            peek_timeout_ms: self.peek_timeout,
            max_connections: self.max_connections,
            max_connections_per_ip: self.max_connections_per_ip,
            cancel_token: self.cancel_token.unwrap_or_default(),
        })
    }

    fn build_registries(&self) -> (ProtocolRegistry, ProtocolRegistry) {
        let mut reg_tcp = ProtocolRegistry::new();
        let mut reg_udp = ProtocolRegistry::new();

        for route in &self.routes_tcp {
            reg_tcp.register(Arc::clone(&route.protocol));
        }
        for route in &self.routes_udp {
            reg_udp.register(Arc::clone(&route.protocol));
        }

        (reg_tcp, reg_udp)
    }

    fn init_health(&self, health: &Arc<HealthMonitor>) {
        let mut targets = self
            .routes_tcp
            .iter()
            .flat_map(|r| r.forward_to.to_vec())
            .collect::<Vec<_>>();
        targets.extend(self.routes_udp.iter().flat_map(|r| r.forward_to.to_vec()));
        health.start_monitoring(targets);
    }

    fn do_build_router(
        routes: Vec<ProtocolRoute>,
        registry: Arc<ProtocolRegistry>,
        health: &Arc<HealthMonitor>,
    ) -> Arc<Router> {
        let balancer = Arc::new(RwLock::new(LoadBalancer::new(routes, Arc::clone(health))));
        Arc::new(Router::new(registry, balancer))
    }
}

impl Default for RefractiumBuilder {
    fn default() -> Self {
        Self::new()
    }
}
