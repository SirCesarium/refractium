//! SSH protocol identification logic.

use crate::core::types::Transport;

define_protocol!(
    /// SSH protocol identification implementation.
    name: Ssh,
    transport: Transport::Tcp,
    identify: |data| {
        data.starts_with(b"SSH-2.0-") || data.starts_with(b"SSH-1.99-")
    }
);
