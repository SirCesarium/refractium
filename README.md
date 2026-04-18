# Refractium

**Extensible low-level reverse proxy for port multiplexing and protocol-based routing.**

---

`refractium` is a high-performance multiplexer that identifies incoming traffic by protocol signature and routes it to the correct backend. It allows Web, SSH, and custom binary services to coexist on the same external port with high throughput.

## Features

- **Expose one single port for multiple services**
  - You can run an HTTP server and an SSH daemon on port 80 simultaneously.
  - Automatically detects protocols by inspecting the initial bytes of the stream.

- **Load Balancing & Health Checks**
  - Distributes traffic across multiple backend instances (Round Robin).
  - Active monitoring: automatically skips dead backends to ensure zero-downtime routing.

- **TCP & UDP Support**
  - Handles both stream-based (TCP) and packet-based (UDP) traffic within the same engine.
  - Optimized for low-latency infrastructure and high-throughput applications.

- **Extensible**
  - Define new protocols in the `refractium.toml` file or as a library.

- **Fail-Fast Configuration**
  - Strict validation: the engine ensures all routes point to registered protocols before starting.

- **Hot Reload**
  - Update your routing table without dropping active connections or restarting the server.

- **Packet Interception (Hooks)**
  - Intercept raw packets in real-time without blocking the main proxy loop.
  - Pluggable architecture: implement the `ProtocolHook` trait to audit, log, or mirror traffic.

- **Graceful Shutdown**
  - Built-in support for cancellation tokens to ensure clean termination.

### How it works

refractium operates at the edge of the transport layer. Unlike Layer 7 proxies (Nginx, HAProxy) that often parse the entire request, refractium:

1. **Peeks:** It briefly inspects the very first bytes of a connection without consuming them.
2. **Identifies:** Matches the signature against its protocol registry in nanoseconds.
3. **Welds:** Once the destination is known, it "welds" the incoming stream directly to the backend.

By using **high-performance asynchronous streams**, data flows through the proxy with high throughput, as if the client were directly connected to the target service.

## Why Refractium?

Because managing multiple open ports on a firewall is _tedious_ and **restricts your architectural flexibility**.

`refractium` lets you:

- **Bypass Firewall Restrictions:** Expose services that would normally be blocked by using standard ports (80, 443).
- **Consolidate IP Resources:** Multiplex diverse protocols on a single public IP address.
- **Scale Transparently:** Your backends receive traffic as if they were directly connected; they don't need to be "Refractium-aware."

## Installation

### Direct Download

Grab the pre-built binary for your operating system from the [Latest Releases](https://github.com/SirCesarium/refractium/releases/latest).

### From Source (Cargo)

```bash
cargo install refractium
```

#### Cargo Features:

- `default`/`full` (contains: `cli`, `logging`, `protocols`, `watch`, `hooks` features)
- `protocols` (contains: `proto-http`, `proto-https`, `proto-ssh`, `proto-dns`, `proto-ftp` features)
- `cli`: Includes the command line interface modules.
- `logging`: Includes `tracing` and `tracing-subscriber` libraries.
- `watch`: Includes hot-reload functions.
- `hooks`: Enables the protocol interception system.

## Command Reference

| Command | Description             | Details                                               |
| :------ | :---------------------- | :---------------------------------------------------- |
| `init`  | Generate default config | Creates a `refractium.toml` in the current directory. |
| `tcp`   | Run TCP server only     | Ignores UDP routes defined in the config.             |
| `udp`   | Run UDP server only     | Ignores TCP routes defined in the config.             |
| (none)  | Run both TCP & UDP      | Default behavior. Listen on both protocols.           |

**Global Options:**

- `-b`, `--bind`: Address to bind to (Default: `0.0.0.0`).
- `-p`, `--port`: Port to listen on (Default: `8080`).
- `-c`, `--config`: Path to configuration file (Default: `refractium.toml`).
- `-f`, `--forward`: Inline routing rules (e.g., `ssh=127.0.0.1:22`).
- `--debug`: Enable verbose logging.

## Usage Examples

### Zero-Config (CLI only)

You don't need a `refractium.toml` file to start using Refractium. Use the `-f` flag to define routes on the fly:

```bash
# Route HTTP and SSH on port 80
refractium -p 80 -f "http=127.0.0.1:8080" -f "ssh=127.0.0.1:22"

# Load balance HTTP across multiple backends
refractium -f "http=10.0.0.1:80" -f "http=10.0.0.2:80"
```

### Using a Configuration File

Create a `refractium.toml` to define more complex routing, including custom byte patterns and transport settings.

**1. Initialize the config:**

```bash
refractium init
```

**2. Edit `refractium.toml`:**

```toml
[server]
bind = "0.0.0.0"
port = 8080

# Optional server settings:
max_connections = 10000
max_connections_per_ip = 50    # DoS protection
peek_timeout_ms = 3000   # How long to wait for the first bytes
peek_buffer_size = 1024  # Max bytes to inspect for identification
hot_reload = true        # Watch for config changes
# Traffic that doesn't match any protocol can be sent to a default backend
fallback_tcp = "127.0.0.1:8080"
fallback_udp = "127.0.0.1:53"

[[protocols]]
name = "http"
# Round-robin load balancing across multiple backends
forward_to = ["10.0.0.1:80", "10.0.0.2:80", "10.0.0.3:80"]

[[protocols]]
name = "ssh"
forward_to = "127.0.0.1:22"

[[protocols]]
name = "dns"
forward_to = "1.1.1.1:53"

# Custom protocol via magic bytes
[[protocols]]
name = "my_app"
patterns = ["\x01\x02\x03", "MY_PROTO_V1"]
forward_to = "127.0.0.1:9000"
transport = "udp"
```

**3. Run with the config:**

```bash
refractium
# Or if you want to define the route to the config file:
refractium --config refractium.toml
```

## Built-in Protocols

| Protocol  | Transport | Support | Features                                    |
| :-------- | :-------- | :-----: | :------------------------------------------ |
| **HTTP**  | TCP       |   ✅    | Standard method detection (GET, POST, etc.) |
| **HTTPS** | TCP       |   ✅    | **SNI Extraction** for domain-based routing |
| **SSH**   | TCP       |   ✅    | Handshake version string identification     |
| **DNS**   | UDP       |   ✅    | Packet-level classification                 |
| **FTP**   | TCP       |   ✅    | Protocol signature matching                 |

## Performance

Refractium is designed for extreme speed. Protocol identification happens in nanoseconds, adding negligible overhead to your connections.

**Identification Benchmarks:**

- **HTTP:** ~16 ns
- **HTTPS (SNI):** ~47 ns
- **SSH:** ~24 ns
- **DNS (UDP):** ~25 ns
- **Mixed Traffic:** ~56 ns

_Environment: Intel Core i7-7700 @ 3.60GHz. Run `cargo bench` to verify on your own hardware._

## Library Usage (Extensibility)

To use `refractium` as a dependency without the CLI overhead, disable default features in your `Cargo.toml`:

```toml
[dependencies]
# Minimal installation for library usage
refractium = { version = "3.0", default-features = false, features = ["logging", "proto-http"] }
```

Refractium is also a modular engine. You can define complex identification logic in Rust by implementing the `RefractiumProtocol` trait or using the provided macro:

```rust
use refractium::{Refractium, define_protocol, types::{ProtocolRoute, ForwardTarget, Transport}};
use std::sync::Arc;

// Define a new protocol outside the project
define_protocol!(
    name: MyProto,
    transport: Transport::Tcp,
    identify: |data| data.starts_with(b"MY_SECRET_MAGIC")
);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Routes directly associate a protocol with its destination
    let routes = vec![ProtocolRoute {
        protocol: Arc::new(MyProto),
        sni: None,
        forward_to: ForwardTarget::Single("127.0.0.1:8080".to_string()),
    }];

    let refractium = Refractium::builder()
        .routes(routes, Vec::new()) // TCP and UDP routes
        .build()?;

    refractium.run_tcp("0.0.0.0:8081".parse()?).await?;
    Ok(())
}
```

### Extending with Hooks (Packet Interception)

Refractium allows you to intercept raw traffic in real-time. Hooks are executed in separate tasks, ensuring that your logic doesn't slow down the main proxy.

You can quickly create and attach hooks using the provided macros:

```rust
use refractium::{Refractium, define_hook, hook_protocol, Http, types::ProtocolRoute, ForwardTarget};
use std::sync::Arc;

// 1. Define a hook using a closure
define_hook!(MyPacketLogger, |ctx, direction, packet| {
    println!(
        "[{}] Intercepted {:?} packet from {}: {} bytes",
        ctx.session_id,
        direction,
        ctx.client_addr,
        packet.len()
    );
});

// 2. Wrap an existing protocol (like Http) with one or more hooks
hook_protocol!(
    wrapper: HookedHttp,
    proto: Http,
    hooks: [MyPacketLogger]
);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let routes = vec![ProtocolRoute {
        protocol: Arc::new(HookedHttp::new()),
        sni: None,
        forward_to: ForwardTarget::Single("127.0.0.1:8080".to_string()),
    }];

    let refractium = Refractium::builder()
        .routes(routes, Vec::new())
        .build()?;

    // ... run Refractium
    Ok(())
}
```

This approach is **non-blocking**, **thread-safe**, and works with any built-in or custom protocol.

## Reliability

- **Memory Safety:** Built 100% in Rust with zero `unsafe` blocks.
- **Panic-Free:** Rigorous use of `clippy` denials to ensure the proxy never crashes in production.
- **Resource Protection:** Configurable peek buffers and timeouts to mitigate slow-loris style attacks.
