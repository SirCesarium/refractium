//! Protocol identification and registry logic.
//!
//! This module provides the infrastructure for identifying different protocols
//! based on the initial data received (magic bytes, SNI, etc.).

use crate::core::types::Transport;
use memchr::memmem;

/// DNS protocol identification.
#[cfg(feature = "proto-dns")]
pub mod dns;
/// FTP protocol identification.
#[cfg(feature = "proto-ftp")]
pub mod ftp;
/// HTTP protocol identification.
#[cfg(feature = "proto-http")]
pub mod http;
/// HTTPS protocol identification.
#[cfg(feature = "proto-https")]
pub mod https;
/// SSH protocol identification.
#[cfg(feature = "proto-ssh")]
pub mod ssh;

/// Hook system for protocol interception.
#[cfg(feature = "hooks")]
pub mod hooks;

#[cfg(feature = "hooks")]
use std::sync::Arc;
#[cfg(not(feature = "hooks"))]
use std::sync::Arc;

#[cfg(feature = "hooks")]
use crate::protocols::hooks::ProtocolHook;

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

/// A simple protocol identification implementation based on string patterns.
#[derive(Clone)]
pub struct DynamicProtocol {
    /// The name of the protocol.
    pub name: String,
    /// The byte patterns to search for in the initial data.
    pub patterns: Vec<String>,
}

impl RefractiumProtocol for DynamicProtocol {
    fn identify(self: Arc<Self>, data: &[u8]) -> Option<ProtocolMatch> {
        let matched = self
            .patterns
            .iter()
            .any(|p| memmem::find(data, p.as_bytes()).is_some());

        if matched {
            return Some(ProtocolMatch {
                name: self.name.to_lowercase(),
                metadata: None,
                implementation: self,
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
    protocols: Vec<Arc<dyn RefractiumProtocol>>,
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
    pub fn register(&mut self, proto: Arc<dyn RefractiumProtocol>) {
        self.protocols.push(proto);
    }

    /// Probes the provided data against all registered protocols.
    ///
    /// Returns the first protocol that matches the data.
    #[must_use]
    pub fn probe(&self, data: &[u8]) -> Option<ProtocolMatch> {
        for proto in &self.protocols {
            if let Some(m) = Arc::clone(proto).identify(data) {
                return Some(m);
            }
        }
        None
    }
}
