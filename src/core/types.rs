use serde::{Deserialize, Serialize};

/// Supported transport protocols for proxying.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Use Transmission Control Protocol.
    Tcp,
    /// Use User Datagram Protocol.
    Udp,
    /// Support both TCP and UDP protocols.
    Both,
}

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

/// Routing rule that associates a protocol name with its targets.
#[derive(Debug, Clone)]
pub struct ProtocolRoute {
    /// Unique identifier for the protocol.
    pub name: String,
    /// Optional byte patterns to match against the stream.
    pub patterns: Option<Vec<String>>,
    /// The target destination for the traffic.
    pub forward_to: ForwardTarget,
    /// The transport layer protocol to use.
    pub transport: Transport,
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
    /// List of defined protocol routing rules.
    pub protocols: Vec<ProtocolRoute>,
    /// Optional fallback address for unmatched TCP traffic.
    pub fallback_tcp: Option<String>,
    /// Optional fallback address for unmatched UDP traffic.
    pub fallback_udp: Option<String>,
}
