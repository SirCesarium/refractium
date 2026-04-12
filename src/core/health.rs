//! Health monitoring for backend servers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time;

/// Monitors the health of backend servers by periodically checking their availability.
pub struct HealthMonitor {
    states: Arc<RwLock<HashMap<String, bool>>>,
}

impl HealthMonitor {
    /// Creates a new `HealthMonitor`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Checks if a backend target is currently healthy.
    ///
    /// Returns `false` if the target is unhealthy or its state is unknown.
    pub async fn is_healthy(&self, target: &str) -> bool {
        let guard = self.states.read().await;
        // Default to false to avoid routing to unchecked backends
        *guard.get(target).unwrap_or(&false)
    }

    /// Starts a background task to monitor the health of the given targets.
    pub fn start_monitoring(&self, targets: Vec<String>) {
        let states = Arc::clone(&self.states);
        tokio::spawn(async move {
            // Immediate check on startup
            for target in &targets {
                let alive = Self::check_target(target).await;
                states.write().await.insert(target.clone(), alive);
            }

            let mut interval = time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                for target in &targets {
                    let alive = Self::check_target(target).await;
                    states.write().await.insert(target.clone(), alive);
                }
            }
        });
    }

    async fn check_target(target: &str) -> bool {
        matches!(
            time::timeout(Duration::from_secs(1), TcpStream::connect(target)).await,
            Ok(Ok(_))
        )
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}
