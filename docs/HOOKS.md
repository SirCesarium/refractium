# Protocol Hooks

Refractium provides a hook system that allows for real-time traffic interception and inspection. Hooks can be used for logging, auditing, or mirroring traffic without breaking the bidirectional proxy flow.

## The `ProtocolHook` Trait

To create a hook, you must implement the `ProtocolHook` trait.

```rust
pub trait ProtocolHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn on_packet(&self, context: &HookContext, direction: Direction, packet: Bytes);
}
```

- **`name()`**: A unique identifier for the hook.
- **`on_packet()`**: This method is called whenever a packet is received (Inbound) or sent (Outbound).

## Hook Context and Direction

Each hook receives a `HookContext` containing session-specific metadata:

- **`client_addr`**: The IP and port of the remote client.
- **`protocol`**: The name of the protocol identified by Refractium.
- **`session_id`**: A unique 64-bit identifier for the connection.

The `Direction` enum indicates the flow of the packet:
- **`Inbound`**: Data coming from the client to the backend.
- **`Outbound`**: Data coming from the backend to the client.

## Practical Example: Traffic Logger

```rust
use refractium::protocols::hooks::{ProtocolHook, HookContext, Direction};
use bytes::Bytes;

#[derive(Clone)]
pub struct TrafficLogger;

impl ProtocolHook for TrafficLogger {
    fn name(&self) -> &'static str {
        "logger"
    }

    fn on_packet(&self, ctx: &HookContext, dir: Direction, pkt: Bytes) {
        println!(
            "[{}] {:?} packet: {} bytes from {}",
            ctx.session_id, dir, pkt.len(), ctx.client_addr
        );
    }
}
```

## Using Macros for Quick Hooks

Refractium provides the `define_hook!` and `hook_protocol!` macros to simplify the process.

### `define_hook!`
Creates a hook implementation from a closure.

```rust
use refractium::define_hook;

define_hook!(MyHook, |ctx, dir, pkt| {
    // Your logic here
});
```

### `hook_protocol!`
Wraps an existing protocol with one or more hooks.

```rust
use refractium::{hook_protocol, Http};

hook_protocol!(
    wrapper: MyHookedHttp,
    proto: Http,
    hooks: [MyHook]
);
```

## Internal Execution Model

Hooks are executed in a separate, dedicated task for each connection. This design ensures that:

1.  **Isolation**: Slow hook logic does not block the main proxy loop or other connections.
2.  **Order Preservation**: Packets are processed by hooks in the same order they arrived at the proxy.
3.  **Non-Blocking**: The data continues to flow between the client and backend while the hooks are being executed.

> **Note**: While hooks are executed asynchronously, they still consume CPU and memory. For high-throughput environments, ensure your `on_packet` logic is as efficient as possible.
