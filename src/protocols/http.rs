//! HTTP protocol identification logic.
//!
//! This implementation identifies HTTP traffic by looking for standard HTTP
//! methods (verbs) at the beginning of the stream, as well as the HTTP/2
//! connection preface.

use crate::core::types::Transport;

define_protocol!(
    /// HTTP protocol identifier.
    ///
    /// Matches standard methods: `GET`, `POST`, `PUT`, `DELETE`, `HEAD`,
    /// `OPTIONS`, `CONNECT`, `TRACE`, `PATCH`.
    ///
    /// It also identifies HTTP/2 traffic by checking for the `PRI * HTTP/2.0`
    /// connection preface, making it compatible with gRPC and modern web traffic.
    name: Http,
    transport: Transport::Tcp,
    identify: |data| {
        let verbs: &[&[u8]] = &[
            b"GET ", b"POST ", b"PUT ", b"DELETE ", b"HEAD ", b"OPTIONS ", b"CONNECT ", b"TRACE ", b"PATCH ",
        ];
        verbs.iter().any(|v| data.starts_with(v)) || data.starts_with(b"PRI * HTTP/2.0")
    }
);
