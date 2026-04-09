//! FTP protocol identification logic.

use crate::define_protocol;

define_protocol!(
    /// FTP protocol identification implementation.
    name: Ftp,
    identify: |data| {
        data.starts_with(b"USER ") || data.starts_with(b"AUTH TLS") || data.starts_with(b"220 ")
    }
);
