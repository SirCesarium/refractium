//! FTP protocol identification logic.
//!
//! This implementation identifies FTP traffic by looking for common initial
//! commands or the standard server response banner.

use crate::core::types::Transport;

define_protocol!(
    /// FTP protocol identifier.
    ///
    /// Matches the following signatures:
    /// - `USER ` (Standard login start)
    /// - `AUTH TLS` (Secure FTP handshake start)
    /// - `220 ` (Server greeting banner)
    name: Ftp,
    transport: Transport::Tcp,
    identify: |data| {
        data.starts_with(b"USER ") || data.starts_with(b"AUTH TLS") || data.starts_with(b"220 ")
    }
);
