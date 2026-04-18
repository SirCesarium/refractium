# Getting Started

This guide will help you get Refractium up and running in minutes, whether you're using it as a standalone tool or integrating it into your Rust project.

## Installation

### For CLI Users
Download the latest binary for your OS from the [Releases](https://github.com/SirCesarium/refractium/releases) page, or install via Cargo:

```bash
cargo install refractium
```

### For Library Users
Add Refractium to your `Cargo.toml`:

```toml
[dependencies]
refractium = "3.0"
```

---

## 1. Quick Start with the CLI (Zero Config)

The fastest way to use Refractium is by defining routes directly in the command line.

**Example: Multiplexing HTTP and SSH on port 80**
```bash
refractium -p 80 -f "http=127.0.0.1:8080" -f "ssh=127.0.0.1:22"
```

Refractium will:
1. Listen on port 80.
2. Identify if the incoming traffic is HTTP or SSH.
3. Forward it to the correct local service.

---

## 2. Using a Configuration File

For more complex setups (multiple backends, custom protocols, health checks), use a `refractium.toml` file.

**Generate a template:**
```bash
refractium init
```

**Example `refractium.toml`:**
```toml
[server]
port = 443
fallback_tcp = "127.0.0.1:8080"

[[protocols]]
name = "https"
forward_to = ["10.0.0.1:443", "10.0.0.2:443"] # Round-robin load balancing
```

**Run it:**
```bash
refractium --config refractium.toml
```

---

## 3. Basic Library Integration

If you are building your own tools on top of Refractium, here is the minimal boilerplate:

```rust
use refractium::{Refractium, Http, types::{ProtocolRoute, ForwardTarget}};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Define your routes
    let routes = vec![ProtocolRoute {
        protocol: Arc::new(Http),
        sni: None,
        forward_to: ForwardTarget::Single("127.0.0.1:8080".to_string()),
    }];

    // 2. Build the engine
    let refractium = Refractium::builder()
        .routes(routes, Vec::new()) // TCP and UDP routes
        .build()?;

    // 3. Run it
    refractium.run_tcp("0.0.0.0:80".parse()?).await?;
    Ok(())
}
```
