//! Load balancing logic for distributing traffic across backends.

use crate::core::health::HealthMonitor;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::core::types::ProtocolRoute;

/// A pool of backend addresses for a specific protocol.
pub struct BackendPool {
    /// The list of backend addresses in the pool.
    pub addresses: Vec<String>,
    counter: AtomicUsize,
}

/// A load balancer that distributes traffic based on protocol identification and metadata.
pub struct LoadBalancer {
    /// Maps "protocol" or "protocol:metadata" to a backend pool.
    routes: HashMap<String, BackendPool>,
    health: Arc<HealthMonitor>,
    fallback: Option<String>,
}

impl LoadBalancer {
    /// Creates a new `LoadBalancer` with the given routes and health monitor.
    #[must_use]
    pub fn new(routes: Vec<ProtocolRoute>, health: Arc<HealthMonitor>) -> Self {
        let mut pools = HashMap::new();
        let mut fallback = None;

        for route in routes {
            let addrs = route.forward_to.to_vec();
            let proto_name = route.protocol.name();
            if proto_name == "fallback" {
                fallback = addrs.first().cloned();
                continue;
            }

            let key = route.sni.as_ref().map_or_else(
                || proto_name.to_string(),
                |sni| format!("{proto_name}:{sni}"),
            );

            pools.insert(
                key,
                BackendPool {
                    addresses: addrs,
                    counter: AtomicUsize::new(0),
                },
            );
        }

        Self {
            routes: pools,
            health,
            fallback,
        }
    }

    /// Selects the next available healthy backend for the given protocol and optional metadata.
    pub async fn next_available(&self, protocol: &str, metadata: Option<&str>) -> Option<&String> {
        // 1. Try more specific match (protocol:metadata)
        if let Some(m) = metadata {
            let specific_key = format!("{protocol}:{m}");
            if let Some(pool) = self.routes.get(&specific_key)
                && let Some(res) = self.pick_from_pool(pool).await
            {
                return Some(res);
            }
        }

        // 2. Try generic protocol match
        if let Some(pool) = self.routes.get(protocol) {
            return self.pick_from_pool(pool).await;
        }

        // 3. Fallback
        if protocol == "fallback"
            && let Some(ref fb) = self.fallback
            && self.health.is_healthy(fb).await
        {
            return Some(fb);
        }

        None
    }
    async fn pick_from_pool<'a>(&self, pool: &'a BackendPool) -> Option<&'a String> {
        let len = pool.addresses.len();
        if len == 0 {
            return None;
        }

        for _ in 0..len {
            let idx = pool.counter.fetch_add(1, Ordering::Relaxed) % len;
            let addr = &pool.addresses[idx];
            if self.health.is_healthy(addr).await {
                return Some(addr);
            }
        }
        None
    }

    /// Returns the health status of all configured backends.
    pub async fn get_status(&self) -> HashMap<String, Vec<(String, bool)>> {
        let mut status = HashMap::new();
        for (key, pool) in &self.routes {
            let mut backends = Vec::new();
            for addr in &pool.addresses {
                backends.push((addr.clone(), self.health.is_healthy(addr).await));
            }
            status.insert(key.clone(), backends);
        }
        status
    }
}
