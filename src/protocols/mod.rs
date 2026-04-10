//! Protocol identification and registry logic.
//!
//! This module provides the infrastructure for identifying different protocols
//! based on the initial data received (magic bytes, SNI, etc.).

use crate::core::types::Transport;
use memchr::memmem;

/// DNS protocol identification.
pub mod dns;
/// FTP protocol identification.
pub mod ftp;
/// HTTP protocol identification.
pub mod http;
/// HTTPS protocol identification.
pub mod https;
/// SSH protocol identification.
pub mod ssh;

/// A match result for a protocol identification attempt.
pub struct ProtocolMatch {
    /// The name of the identified protocol.
    pub name: String,
    /// Optional metadata extracted during identification (e.g., SNI hostname).
    pub metadata: Option<String>,
}

/// A trait for protocol identification logic.
pub trait PrismaProtocol: Send + Sync {
    /// Returns the name of the protocol.
    fn name(&self) -> &str;
    /// Identifies the protocol based on the provided data.
    fn identify(&self, data: &[u8]) -> Option<ProtocolMatch>;
    /// Returns the transport type of the protocol.
    fn transport(&self) -> Transport;
}

/// A simple protocol identification implementation based on string patterns.
pub struct DynamicProtocol {
    /// The name of the protocol.
    pub name: String,
    /// The byte patterns to search for in the initial data.
    pub patterns: Vec<String>,
}

impl PrismaProtocol for DynamicProtocol {
    fn identify(&self, data: &[u8]) -> Option<ProtocolMatch> {
        let matched = self
            .patterns
            .iter()
            .any(|p| memmem::find(data, p.as_bytes()).is_some());

        if matched {
            return Some(ProtocolMatch {
                name: self.name.to_lowercase(),
                metadata: None,
            });
        }
        None
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn transport(&self) -> Transport {
        Transport::Both
    }
}

/// A registry for storing and querying protocol identification logic.
pub struct ProtocolRegistry {
    protocols: Vec<Box<dyn PrismaProtocol>>,
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolRegistry {
    /// Creates a new, empty protocol registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            protocols: Vec::new(),
        }
    }

    /// Registers a new protocol identification logic.
    pub fn register(&mut self, proto: Box<dyn PrismaProtocol>) {
        self.protocols.push(proto);
    }

    /// Probes the provided data against all registered protocols.
    ///
    /// Returns the first protocol that matches the data.
    #[must_use]
    pub fn probe(&self, data: &[u8]) -> Option<ProtocolMatch> {
        for proto in &self.protocols {
            if let Some(m) = proto.identify(data) {
                return Some(m);
            }
        }
        None
    }
}
