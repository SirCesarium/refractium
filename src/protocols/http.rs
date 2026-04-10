//! HTTP protocol identification logic.

use crate::core::types::Transport;
use crate::define_protocol;

define_protocol!(
    /// HTTP protocol identification implementation.
    name: Http,
    transport: Transport::Tcp,
    identify: |data| {
        let verbs: &[&[u8]] = &[
            b"GET ", b"POST ", b"PUT ", b"DELETE ", b"HEAD ", b"OPTIONS ", b"CONNECT ", b"TRACE ", b"PATCH ",
        ];
        verbs.iter().any(|v| data.starts_with(v)) || data.starts_with(b"PRI * HTTP/2.0")
    }
);
