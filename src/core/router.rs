use crate::core::balancer::LoadBalancer;
use crate::core::health::HealthMonitor;
use crate::protocols::ProtocolRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// High-level router that combines a protocol registry with a load balancer.
pub struct Router {
    registry: Arc<ProtocolRegistry>,
    balancer: Arc<RwLock<LoadBalancer>>,
}

impl Router {
    /// Creates a new `Router` with the given registry and balancer.
    #[must_use]
    pub const fn new(registry: Arc<ProtocolRegistry>, balancer: Arc<RwLock<LoadBalancer>>) -> Self {
        Self { registry, balancer }
    }

    /// Identifies the protocol of the given data and routes it to an available backend.
    ///
    /// # Returns
    ///
    /// Returns an `Option<String>` containing the target address if a match is found
    /// and a healthy backend is available.
    pub async fn route(&self, data: &[u8]) -> Option<String> {
        let balancer_guard = self.balancer.read().await;

        match self.registry.probe(data) {
            Some(m) => {
                let mut addr = balancer_guard.next_available(&m.name).await;
                if addr.is_none() {
                    addr = balancer_guard.next_available("fallback").await;
                }
                addr.cloned()
            }
            None => balancer_guard.next_available("fallback").await.cloned(),
        }
    }

    /// Replaces the current routes and reinitializes the load balancer.
    pub async fn update_balancer(
        &self,
        routes: HashMap<String, Vec<String>>,
        health: Arc<HealthMonitor>,
    ) {
        let mut balancer_guard = self.balancer.write().await;
        *balancer_guard = LoadBalancer::new(routes, health);
    }
}
