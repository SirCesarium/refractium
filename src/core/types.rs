use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "hooks")]
use crate::protocols::hooks::ProtocolHook;

/// Supported transport protocols for proxying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Use Transmission Control Protocol.
    Tcp,
    /// Use User Datagram Protocol.
    Udp,
    /// Support both TCP and UDP protocols.
    Both,
}

/// A match result for a protocol identification attempt.
pub struct ProtocolMatch {
    /// The name of the identified protocol.
    pub name: String,
    /// Optional metadata extracted during identification (e.g., SNI hostname).
    pub metadata: Option<String>,
    /// The protocol implementation that matched.
    pub implementation: Arc<dyn RefractiumProtocol>,
}

/// A trait for protocol identification logic.
pub trait RefractiumProtocol: Send + Sync + dyn_clone::DynClone {
    /// Returns the name of the protocol.
    fn name(&self) -> &str;
    /// Identifies the protocol based on the provided data.
    fn identify(self: Arc<Self>, data: &[u8]) -> Option<ProtocolMatch>;
    /// Returns the transport type of the protocol.
    fn transport(&self) -> Transport;

    /// Returns the registered hooks for this protocol.
    #[cfg(feature = "hooks")]
    fn hooks(&self) -> Vec<Arc<dyn ProtocolHook>> {
        Vec::new()
    }
}

dyn_clone::clone_trait_object!(RefractiumProtocol);

/// Defines the destination address(es) for a protocol route.
#[derive(Debug, Clone)]
pub enum ForwardTarget {
    /// A single destination address.
    Single(String),
    /// Multiple destination addresses for load balancing or broadcasting.
    Multiple(Vec<String>),
}

impl ForwardTarget {
    /// Returns all target addresses as a vector.
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Multiple(v) => v.clone(),
        }
    }
}

/// Routing rule that associates a protocol with its targets.
#[derive(Clone)]
pub struct ProtocolRoute {
    /// The protocol identification logic.
    pub protocol: Arc<dyn RefractiumProtocol>,
    /// Optional SNI (Server Name Indication) for HTTPS routing.
    pub sni: Option<String>,
    /// The target destination for the traffic.
    pub forward_to: ForwardTarget,
}

/// Final application state after merging configuration sources.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Interface address to bind the server to.
    pub bind: String,
    /// Port number to listen on.
    pub port: u16,
    /// Size of the initial buffer used to peek into connections.
    pub peek_buffer_size: usize,
    /// Timeout in milliseconds for the peeking phase.
    pub peek_timeout_ms: u64,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
    /// Maximum number of concurrent connections per IP.
    pub max_connections_per_ip: usize,
    /// Whether hot reload is enabled.
    pub hot_reload: bool,
    /// List of defined protocol routing rules (internal representation).
    pub protocols: Vec<TomlRouteData>,
    /// Optional fallback address for unmatched TCP traffic.
    pub fallback_tcp: Option<String>,
    /// Optional fallback address for unmatched UDP traffic.
    pub fallback_udp: Option<String>,
}

/// Intermediate structure for configuration.
#[derive(Debug, Clone)]
pub struct TomlRouteData {
    /// Protocol name.
    pub name: String,
    /// Optional SNI.
    pub sni: Option<String>,
    /// Identification patterns.
    pub patterns: Option<Vec<String>>,
    /// Forwarding target.
    pub forward_to: ForwardTarget,
    /// Transport type.
    pub transport: Transport,
}
