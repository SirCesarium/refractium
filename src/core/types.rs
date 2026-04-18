use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "hooks")]
use crate::protocols::hooks::ProtocolHook;

/// Supported transport layer protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Transmission Control Protocol (Stream-based).
    Tcp,
    /// User Datagram Protocol (Packet-based).
    Udp,
    /// Indicates the protocol can operate over both TCP and UDP.
    Both,
}

/// The result of a successful protocol identification attempt.
pub struct ProtocolMatch {
    /// The canonical name of the identified protocol (e.g., "http").
    pub name: String,
    /// Optional metadata extracted from the initial bytes (e.g., a hostname from SNI).
    pub metadata: Option<String>,
    /// The specific protocol implementation that successfully identified the traffic.
    pub implementation: Arc<dyn RefractiumProtocol>,
}

/// Trait for implementing custom protocol identification logic.
///
/// Any type that implements this trait can be used by Refractium to inspect
/// incoming streams and decide how they should be routed.
///
/// # Example
///
/// ```rust
/// use refractium::types::{RefractiumProtocol, ProtocolMatch, Transport};
/// use std::sync::Arc;
///
/// #[derive(Clone)]
/// struct MyMagicProtocol;
///
/// impl RefractiumProtocol for MyMagicProtocol {
///     fn name(&self) -> String { "magic".to_string() }
///     fn transport(&self) -> Transport { Transport::Tcp }
///     fn identify(self: Arc<Self>, data: &[u8]) -> Option<ProtocolMatch> {
///         if data.starts_with(b"MAGIC") {
///             Some(ProtocolMatch {
///                 name: self.name(),
///                 metadata: None,
///                 implementation: self,
///             })
///         } else {
///             None
///         }
///     }
/// }
/// ```
pub trait RefractiumProtocol: Send + Sync + dyn_clone::DynClone {
    /// Returns the unique name of this protocol.
    fn name(&self) -> String;

    /// Inspects the provided data to determine if it matches this protocol.
    ///
    /// The `data` slice contains the initial bytes peeked from the connection.
    /// This method should return `Some(ProtocolMatch)` if the protocol is identified,
    /// or `None` otherwise.
    fn identify(self: Arc<Self>, data: &[u8]) -> Option<ProtocolMatch>;

    /// Returns the transport layer this protocol operates on.
    fn transport(&self) -> Transport;

    /// Returns a list of hooks associated with this protocol.
    ///
    /// Hooks allow for real-time traffic interception and modification.
    #[cfg(feature = "hooks")]
    fn hooks(&self) -> Vec<Arc<dyn ProtocolHook>> {
        Vec::new()
    }
}

dyn_clone::clone_trait_object!(RefractiumProtocol);

/// Defines the destination backend(s) for a protocol route.
#[derive(Debug, Clone)]
pub enum ForwardTarget {
    /// A single backend address (e.g., `"127.0.0.1:8080"`).
    Single(String),
    /// Multiple backend addresses for load balancing.
    Multiple(Vec<String>),
}

impl ForwardTarget {
    /// Converts the target into a vector of address strings.
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Multiple(v) => v.clone(),
        }
    }
}

/// A routing rule that associates a protocol with a specific destination.
///
/// When an incoming connection matches the `protocol` signature, Refractium
/// will forward the traffic to one of the addresses defined in `forward_to`.
#[derive(Clone)]
pub struct ProtocolRoute {
    /// The logic used to identify the protocol.
    pub protocol: Arc<dyn RefractiumProtocol>,
    /// Optional Server Name Indication (SNI) string.
    ///
    /// If provided, this route will only match if both the protocol signature
    /// and the SNI hostname match the incoming traffic.
    pub sni: Option<String>,
    /// Where to forward the traffic upon a successful match.
    pub forward_to: ForwardTarget,
}

/// Full configuration state used by the Refractium engine.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// The interface address to bind the server to (e.g., `"0.0.0.0"`).
    pub bind: String,
    /// The port number to listen on.
    pub port: u16,
    /// Maximum size of the peek buffer in bytes.
    pub peek_buffer_size: usize,
    /// Maximum time in milliseconds to wait for identification data.
    pub peek_timeout_ms: u64,
    /// Maximum number of concurrent connections the server will accept globally.
    pub max_connections: usize,
    /// Maximum number of concurrent connections allowed from a single IP.
    pub max_connections_per_ip: usize,
    /// Whether to watch the configuration file for changes.
    pub hot_reload: bool,
    /// List of defined protocol routes.
    pub protocols: Vec<TomlRouteData>,
    /// Default backend for unidentified TCP traffic.
    pub fallback_tcp: Option<String>,
    /// Default backend for unidentified UDP traffic.
    pub fallback_udp: Option<String>,
}

/// Intermediate structure for parsing route data from TOML/CLI.
#[derive(Debug, Clone)]
pub struct TomlRouteData {
    /// Name of the protocol to match.
    pub name: String,
    /// Optional SNI filter.
    pub sni: Option<String>,
    /// Optional list of byte patterns to use for identification.
    pub patterns: Option<Vec<String>>,
    /// Destination backend(s).
    pub forward_to: ForwardTarget,
    /// Transport type (TCP/UDP).
    pub transport: Transport,
}
