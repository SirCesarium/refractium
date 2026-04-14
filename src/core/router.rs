use crate::core::balancer::LoadBalancer;
use crate::core::health::HealthMonitor;
use crate::protocols::{ProtocolRegistry, RefractiumProtocol};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Result of a routing attempt.
#[derive(Clone)]
pub enum RouteResult {
    /// A matching protocol was found and routed to a backend.
    Matched(String, String, Arc<dyn RefractiumProtocol>), // Protocol, Address, Implementation
    /// No matching protocol was found, but traffic was routed to the fallback.
    Fallback(String), // Address
    /// Protocol was identified but no route or healthy fallback is available.
    Discarded,
}

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

    /// Identifies the protocol and routes it.
    /// Returns `None` if more data is needed to identify the protocol.
    pub async fn route(&self, data: &[u8]) -> Option<RouteResult> {
        let balancer_guard = self.balancer.read().await;

        if let Some(m) = self.registry.probe(data) {
            let metadata = m.metadata.as_deref();
            if let Some(addr) = balancer_guard.next_available(&m.name, metadata).await {
                Some(RouteResult::Matched(m.name, addr.clone(), m.implementation))
            } else if let Some(addr) = balancer_guard.next_available("fallback", None).await {
                Some(RouteResult::Fallback(addr.clone()))
            } else {
                Some(RouteResult::Discarded)
            }
        } else {
            // Protocol not yet identified, need more bytes
            None
        }
    }

    /// Routes to fallback directly (used on timeout).
    pub async fn route_fallback(&self) -> RouteResult {
        let balancer_guard = self.balancer.read().await;
        balancer_guard
            .next_available("fallback", None)
            .await
            .map_or(RouteResult::Discarded, |addr| {
                RouteResult::Fallback(addr.clone())
            })
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

    /// Returns the health status of all configured backends.
    pub async fn get_health_status(&self) -> HashMap<String, Vec<(String, bool)>> {
        let balancer_guard = self.balancer.read().await;
        balancer_guard.get_status().await
    }
}
