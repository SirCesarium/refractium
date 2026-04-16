//! Protocol identification and registry logic.
//!
//! This module provides the infrastructure for identifying different protocols
//! based on the initial data received (magic bytes, SNI, etc.).

use crate::core::types::{ProtocolMatch, RefractiumProtocol, Transport};
use memchr::memmem;
use std::sync::Arc;

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
        use heck::ToSnakeCase;
        let matched = self
            .patterns
            .iter()
            .any(|p| memmem::find(data, p.as_bytes()).is_some());

        if matched {
            return Some(ProtocolMatch {
                name: self.name.to_string().to_snake_case(),
                metadata: None,
                implementation: self,
            });
        }
        None
    }

    fn name(&self) -> String {
        self.name.to_string()
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

    /// Returns a list of all registered protocol names, normalized to `snake_case`.
    #[must_use]
    pub fn get_registered_names(&self) -> Vec<String> {
        use heck::ToSnakeCase;
        self.protocols
            .iter()
            .map(|p| p.name().to_snake_case())
            .collect()
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
