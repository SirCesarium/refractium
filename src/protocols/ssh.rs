//! SSH protocol identification logic.

use crate::define_protocol;

define_protocol!(
    /// SSH protocol identification implementation.
    name: Ssh,
    identify: |data| {
        data.starts_with(b"SSH-2.0-") || data.starts_with(b"SSH-1.99-")
    }
);
