//! SSH protocol identification logic.
//!
//! This implementation identifies SSH traffic by inspecting the initial
//! version string exchange (handshake) that the client sends upon connection.

use crate::core::types::Transport;

define_protocol!(
    /// SSH protocol identifier.
    ///
    /// Matches the standard SSH identification strings:
    /// - `SSH-2.0-` (Modern SSH)
    /// - `SSH-1.99-` (Compatibility mode)
    ///
    /// This allows Refractium to route SSH traffic even if it is running
    /// on a non-standard port like 80 or 443.
    name: Ssh,
    transport: Transport::Tcp,
    identify: |data| {
        data.starts_with(b"SSH-2.0-") || data.starts_with(b"SSH-1.99-")
    }
);
