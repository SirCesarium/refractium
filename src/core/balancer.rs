//! Load balancing logic for distributing traffic across backends.

use crate::core::health::HealthMonitor;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A load balancer that distributes traffic based on protocol identification.
pub struct LoadBalancer {
    routes: HashMap<String, BackendPool>,
    health: Arc<HealthMonitor>,
}

/// A pool of backend addresses for a specific protocol.
pub struct BackendPool {
    /// The list of backend addresses in the pool.
    pub addresses: Vec<String>,
    counter: AtomicUsize,
}

impl LoadBalancer {
    /// Creates a new `LoadBalancer` with the given routes and health monitor.
    #[must_use]
    pub fn new(routes: HashMap<String, Vec<String>>, health: Arc<HealthMonitor>) -> Self {
        let mut pools = HashMap::new();
        for (name, addrs) in routes {
            pools.insert(
                name,
                BackendPool {
                    addresses: addrs,
                    counter: AtomicUsize::new(0),
                },
            );
        }
        Self {
            routes: pools,
            health,
        }
    }

    /// Selects the next available healthy backend for the given protocol.
    ///
    /// This implementation uses a round-robin strategy combined with health checks.
    pub async fn next_available(&self, protocol: &str) -> Option<&String> {
        let pool = self.routes.get(protocol)?;
        let len = pool.addresses.len();

        for _ in 0..len {
            let idx = pool.counter.fetch_add(1, Ordering::Relaxed) % len;
            let addr = &pool.addresses[idx];
            if self.health.is_healthy(addr).await {
                return Some(addr);
            }
        }
        None
    }
}
