# API Reference

This document provides a technical specification of the Refractium library's public API.

## Core Engine

### Refractium

The main entry point for running the proxy servers.

- `builder() -> RefractiumBuilder`: Returns a new builder instance.
- `run_tcp(addr: SocketAddr) -> Result<()>`: Starts the TCP server.
- `run_udp(addr: SocketAddr) -> Result<()>`: Starts the UDP server.
- `run_both(addr: SocketAddr) -> Result<()>`: Concurrently runs both TCP and UDP servers.
- `reload_routes(tcp: Vec<ProtocolRoute>, udp: Vec<ProtocolRoute>)`: Atomic update of the routing table.
- `cancel_token() -> CancellationToken`: Retrieves the engine's shutdown signal.

### RefractiumBuilder

- `new()`: Default configuration (10k connections, 50 per IP, 1k peek buffer).
- `routes(tcp: Vec<ProtocolRoute>, udp: Vec<ProtocolRoute>)`: Defines the routing table.
- `peek_config(size: usize, timeout_ms: u64)`: Configures the identification phase.
- `max_connections(max: usize)`: Global connection limit.
- `max_connections_per_ip(max: usize)`: Per-IP rate limiting.
- `cancel_token(token: CancellationToken)`: Attaches a termination signal.
- `build() -> Result<Refractium>`: Finalizes and initializes the engine.

## Routing and Protocols

### RefractiumProtocol (Trait)

Any custom protocol identification logic must implement this trait.

```rust
pub trait RefractiumProtocol: Send + Sync + dyn_clone::DynClone {
    fn name(&self) -> &str;
    fn identify(self: Arc<Self>, data: &[u8]) -> Option<ProtocolMatch>;
    fn transport(&self) -> Transport;
}
```

- `name()`: Internal identifier (must be unique if using the CLI/Config).
- `identify()`: Analyzes the peeked bytes. Returns `Some(ProtocolMatch)` on success.
- `transport()`: Specifies whether it applies to `Tcp`, `Udp`, or `Both`.

### ProtocolRoute

Associates a protocol with its forwarding destination.

- `protocol: Arc<dyn RefractiumProtocol>`: The identification logic.
- `sni: Option<String>`: Optional hostname for TLS/HTTPS routing.
- `forward_to: ForwardTarget`: One or more backend addresses.

### ForwardTarget

- `Single(String)`: A single backend address (e.g., "127.0.0.1:8080").
- `Multiple(Vec<String>)`: A pool of backends for round-robin load balancing.

## Common Types

### Transport (Enum)
- `Tcp`
- `Udp`
- `Both`

### ProtocolMatch
A structure returned by protocols upon successful identification.
- `name: String`: Matched protocol name.
- `metadata: Option<String>`: Extracted data (e.g., SNI hostname).
- `implementation: Arc<dyn RefractiumProtocol>`: The protocol object that matched.
