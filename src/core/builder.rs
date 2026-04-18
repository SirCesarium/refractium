//! Builder for the Refractium engine.

use crate::core::Refractium;
use crate::core::balancer::LoadBalancer;
use crate::core::health::HealthMonitor;
use crate::core::router::Router;
use crate::core::types::ProtocolRoute;
use crate::errors::Result;
use crate::protocols::ProtocolRegistry;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// A fluent builder for configuring and initializing the [`Refractium`] engine.
pub struct RefractiumBuilder {
    pub(crate) routes_tcp: Vec<ProtocolRoute>,
    pub(crate) routes_udp: Vec<ProtocolRoute>,
    pub(crate) peek_size: usize,
    pub(crate) peek_timeout: u64,
    pub(crate) max_connections: usize,
    pub(crate) max_connections_per_ip: usize,
    pub(crate) cancel_token: Option<CancellationToken>,
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
    /// Useful for basic `DoS` mitigation.
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
