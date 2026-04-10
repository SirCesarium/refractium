//! FTP protocol identification logic.

use crate::core::types::Transport;
use crate::define_protocol;

define_protocol!(
    /// FTP protocol identification implementation.
    name: Ftp,
    transport: Transport::Tcp,
    identify: |data| {
        data.starts_with(b"USER ") || data.starts_with(b"AUTH TLS") || data.starts_with(b"220 ")
    }
);
