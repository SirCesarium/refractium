//! DNS protocol identification logic.

use crate::protocols::{PrismaProtocol, ProtocolMatch};

/// DNS protocol identification implementation.
pub struct Dns;

impl PrismaProtocol for Dns {
    fn identify(&self, data: &[u8]) -> Option<ProtocolMatch> {
        if data.len() < 12 {
            return None;
        }

        let flags = u16::from_be_bytes([data[2], data[3]]);
        let is_query = (flags & 0x8000) == 0;
        let op_code = (flags >> 11) & 0x0F;

        if is_query && op_code == 0 {
            return Some(ProtocolMatch {
                name: "Dns".to_string(),
                metadata: None,
            });
        }
        None
    }

    fn name(&self) -> &'static str {
        "Dns"
    }
}
