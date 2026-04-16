# Custom Protocol Implementation

Refractium is designed to be extensible. You can define new protocol identification logic by implementing the `RefractiumProtocol` trait or by using the `define_protocol!` macro.

## Option 1: Using the `define_protocol!` Macro

This is the recommended approach for simple signature-based matching. It generates the necessary boilerplate and ensures the protocol is compatible with Refractium's internal registry.

```rust
use refractium::{define_protocol, types::{ProtocolMatch, Transport}};
use std::sync::Arc;

define_protocol!(
    name: MyProto,
    transport: Transport::Tcp,
    identify: |data| {
        // Matches if the first 4 bytes are "MYPR"
        data.starts_with(b"MYPR")
    }
);
```

The macro automatically handles:
- Implementing the `Clone` trait.
- Implementing the `RefractiumProtocol` trait.
- Wrapping the result in a `ProtocolMatch` with snake_case naming.

## Option 2: Implementing the Trait Manually

For more complex identification logic (e.g., extracting SNI from a TLS handshake), implement the trait directly.

```rust
use refractium::types::{RefractiumProtocol, ProtocolMatch, Transport};
use std::sync::Arc;

#[derive(Clone)]
pub struct TlsInspector;

impl RefractiumProtocol for TlsInspector {
    fn name(&self) -> &str {
        "tls"
    }

    fn identify(self: Arc<Self>, data: &[u8]) -> Option<ProtocolMatch> {
        if data.len() < 5 || data[0] != 0x16 {
            return None;
        }

        // Complex parsing logic to extract the SNI hostname
        let hostname = parse_sni(data);

        Some(ProtocolMatch {
            name: "tls".to_string(),
            metadata: hostname,
            implementation: self,
        })
    }

    fn transport(&self) -> Transport {
        Transport::Tcp
    }
}
```

## Best Practices

1.  **Non-Destructive Identification**: Your `identify` method must only inspect the bytes. Do not attempt to consume them from the stream.
2.  **Performance**: The identification logic is executed for every new connection. Keep it efficient and avoid heavy allocations.
3.  **Ambiguity**: If multiple protocols match the same signature, Refractium will use the first one registered in the `ProtocolRoute` list.
4.  **Buffering**: Remember that the `data` slice passed to `identify` may be smaller than your required signature if the client is slow. Handle partial matches gracefully.
