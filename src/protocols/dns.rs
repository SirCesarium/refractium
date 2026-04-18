//! DNS protocol identification logic.

use crate::core::types::Transport;
use crate::protocols::{ProtocolMatch, RefractiumProtocol};
use std::sync::Arc;

/// DNS protocol identifier for UDP traffic.
///
/// This implementation inspects the DNS header to identify standard queries.
/// It checks for:
/// - A minimum header length of 12 bytes.
/// - The "QR" flag (Query/Response) set to 0 (indicating a Query).
/// - The "Opcode" set to 0 (indicating a Standard Query).
#[derive(Clone)]
pub struct Dns;

impl RefractiumProtocol for Dns {
    fn identify(self: Arc<Self>, data: &[u8]) -> Option<ProtocolMatch> {
        if data.len() < 12 {
            return None;
        }

        let flags = u16::from_be_bytes([data[2], data[3]]);
        let is_query = (flags & 0x8000) == 0;
        let op_code = (flags >> 11) & 0x0F;

        if is_query && op_code == 0 {
            return Some(ProtocolMatch {
                name: "dns".to_string(),
                metadata: None,
                implementation: self,
            });
        }
        None
    }

    fn name(&self) -> String {
        "dns".to_string()
    }

    fn transport(&self) -> Transport {
        Transport::Udp
    }
}
