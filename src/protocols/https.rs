//! HTTPS protocol identification logic.

use crate::core::types::Transport;
use crate::protocols::{PrismaProtocol, ProtocolMatch};
use std::cmp;

/// HTTPS protocol identification implementation.
pub struct Https;

impl PrismaProtocol for Https {
    fn identify(&self, data: &[u8]) -> Option<ProtocolMatch> {
        if data.len() < 43 || data[0] != 0x16 || data[1] != 0x03 || data[5] != 0x01 {
            return None;
        }

        let mut pos = 43;
        let skip_len = |p: &mut usize, data: &[u8], len: usize| -> Option<usize> {
            if data.len() < *p + len {
                return None;
            }
            let val = match len {
                1 => data[*p] as usize,
                2 => u16::from_be_bytes([data[*p], data[*p + 1]]) as usize,
                _ => 0,
            };
            *p += len;
            Some(val)
        };

        let session_id_len = skip_len(&mut pos, data, 1)?;
        pos += session_id_len;

        let cipher_suites_len = skip_len(&mut pos, data, 2)?;
        pos += cipher_suites_len;

        let compression_methods_len = skip_len(&mut pos, data, 1)?;
        pos += compression_methods_len;

        let extensions_len = skip_len(&mut pos, data, 2)?;
        let extensions_end = cmp::min(pos + extensions_len, data.len());

        while pos + 4 <= extensions_end {
            let ext_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
            let ext_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
            pos += 4;

            if ext_type == 0x00
                && pos + ext_len <= extensions_end
                && let Some(sni) = Self::parse_sni(&data[pos..pos + ext_len])
            {
                return Some(ProtocolMatch {
                    name: "https".to_string(),
                    metadata: Some(sni),
                });
            }
            pos += ext_len;
        }

        Some(ProtocolMatch {
            name: "https".to_string(),
            metadata: None,
        })
    }

    fn name(&self) -> &'static str {
        "https"
    }

    fn transport(&self) -> Transport {
        Transport::Tcp
    }
}

impl Https {
    fn parse_sni(data: &[u8]) -> Option<String> {
        if data.len() < 5 {
            return None;
        }
        let list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + list_len || list_len < 3 {
            return None;
        }

        let mut pos = 2;
        while pos + 3 <= 2 + list_len {
            let name_type = data[pos];
            let name_len = u16::from_be_bytes([data[pos + 1], data[pos + 2]]) as usize;
            pos += 3;
            if name_type == 0x00 && pos + name_len <= data.len() {
                return Some(String::from_utf8_lossy(&data[pos..pos + name_len]).to_string());
            }
            pos += name_len;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_https_identify_invalid() {
        let proto = Https;
        assert!(proto.identify(b"GET / HTTP/1.1").is_none());
    }

    #[test]
    fn test_sni_parsing() {
        let sni_ext = vec![
            0x00, 0x0b, // list len
            0x00, // type host_name
            0x00, 0x08, // name len
            b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.',
        ];
        if let Some(sni) = Https::parse_sni(&sni_ext) {
            assert_eq!(sni, "example.");
        } else {
            panic!("SNI parsing failed");
        }
    }
}
