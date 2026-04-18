//! Infrastructure for protocol identification and registration.
//!
//! This module provides the `ProtocolRegistry` for managing multiple protocol
//! identifiers and the `DynamicProtocol` structure for simple pattern-based matching.

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

/// A simple protocol identification implementation based on raw byte patterns.
///
/// `DynamicProtocol` searches for specific byte sequences anywhere within the
/// initial peeked data of a connection.
#[derive(Clone)]
pub struct DynamicProtocol {
    /// The display name of the protocol.
    pub name: String,
    /// The byte patterns to search for.
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
                name: self.name.to_snake_case(),
                metadata: None,
                implementation: self,
            });
        }
        None
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn transport(&self) -> Transport {
        Transport::Both
    }
}

/// A registry for storing and querying protocol identification logic.
///
/// The registry stores several [`RefractiumProtocol`] implementations and allows
/// probing raw byte slices against them to find a match.
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

    /// Registers a new protocol identification logic into the registry.
    ///
    /// Protocols are probed in the order they are registered.
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
    /// Returns the [`ProtocolMatch`] for the first protocol that successfully
    /// identifies the traffic.
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
