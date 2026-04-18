# API Reference

This document provides a technical specification for the Refractium library.

## Feature Flags

Refractium uses cargo features to keep the core binary lean. Some APIs documented here require specific features:

- `full`: Enables all features (cli, logging, protocols, watch, hooks).
- `protocols`: Includes all built-in protocol identifiers (HTTP, HTTPS, SSH, DNS, FTP).
- `hooks`: Enables the protocol interception system.
- `logging`: Enables `tracing` integration.

## Core Engine

### `Refractium`

The main runtime for protocol-based proxying.

- **`builder() -> RefractiumBuilder`**: Returns a new builder with default settings.
- **`run_tcp(addr: SocketAddr) -> Result<()>`**: Starts the TCP server.
- **`run_udp(addr: SocketAddr) -> Result<()>`**: Starts the UDP server.
- **`run_both(addr: SocketAddr) -> Result<()>`**: Runs both servers concurrently in separate tasks.
- **`reload_routes(tcp: Vec<ProtocolRoute>, udp: Vec<ProtocolRoute>)`**: Atomically updates the routing table for all active servers.
- **`cancel_token() -> CancellationToken`**: Retrieves the cancellation token used to trigger graceful shutdown.
- **`report_health() -> impl Future`**: Prints a summary of backend statuses to stdout.

### `RefractiumBuilder`

Fluent API for configuring the proxy engine.

- **`new()`**: Default configuration:
  - Max connections: 10,000
  - Max connections per IP: 50
  - Peek buffer size: 1024 bytes
  - Peek timeout: 3000ms
- **`routes(tcp: Vec<ProtocolRoute>, udp: Vec<ProtocolRoute>)`**: Sets the routing table.
- **`fallback_tcp(addr: String)`**: Sets the default backend for unidentified TCP traffic.
- **`fallback_udp(addr: String)`**: Sets the default backend for unidentified UDP traffic.
- **`peek_config(size: usize, timeout_ms: u64)`**: Tunes the identification phase.
- **`max_connections(max: usize)`**: Sets global connection limits.
- **`max_connections_per_ip(max: usize)`**: Sets per-IP connection limits (DoS protection).
- **`cancel_token(token: CancellationToken)`**: Attaches an external shutdown signal.
- **`build() -> Result<Refractium>`**: Finalizes configuration and initializes the engine.

---

## Protocols and Routing

### `RefractiumProtocol` (Trait)

Implemented by any logic that can identify a protocol from a byte stream.

```rust
pub trait RefractiumProtocol: Send + Sync {
    fn name(&self) -> &str;
    fn identify(self: Arc<Self>, data: &[u8]) -> Option<ProtocolMatch>;
    fn transport(&self) -> Transport;
    #[cfg(feature = "hooks")]
    fn hooks(&self) -> Vec<Arc<dyn ProtocolHook>>;
}
```

### `ProtocolRoute`

Maps a protocol to its destination.

- `protocol: Arc<dyn RefractiumProtocol>`: The protocol identification object.
- `sni: Option<String>`: Optional hostname for domain-based routing (used by HTTPS).
- `forward_to: ForwardTarget`: The backend address(es).

### `ForwardTarget` (Enum)

- `Single(String)`: A single backend (e.g., `"127.0.0.1:8080"`).
- `Multiple(Vec<String>)`: A pool of backends for Round-Robin load balancing.

---

## Built-in Protocols

Available when the `protocols` feature is enabled:

- **`Http`**: Matches standard HTTP methods (GET, POST, etc.).
- **`Https`**: Extracts SNI from TLS Client Hello for routing.
- **`Ssh`**: Matches the SSH version string handshake.
- **`Dns`**: Identifies DNS queries (UDP).
- **`Ftp`**: Identifies FTP control channel signatures.

---

## Interception (Hooks)

Requires the `hooks` feature.

### `ProtocolHook` (Trait)

```rust
pub trait ProtocolHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn on_packet(&self, context: &HookContext, direction: Direction, packet: Bytes);
}
```

- **`Direction`**: Enum (`Inbound`, `Outbound`).
- **`HookContext`**: Struct containing `client_addr`, `protocol` name, and `session_id`.

---

## Error Handling

### `RefractiumError` (Enum)

- `BindError(String, io::Error)`: Failed to listen on the requested address.
- `ConfigError(String)`: Invalid configuration or routing logic.
- `Io(io::Error)`: Standard network or filesystem errors.
- `AddrResolution(String)`: Failed to resolve a backend hostname.
- `Generic(String)`: Custom error message.

---

## Public Macros

- **`define_protocol!`**: Quickly generate a `RefractiumProtocol` implementation.
- **`define_hook!`**: Generate a `ProtocolHook` from a closure.
- **`hook_protocol!`**: Wrap an existing protocol with one or more hooks.
