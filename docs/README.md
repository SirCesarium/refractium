# Refractium Documentation

## Documentation Sections

-   [**Architecture and Design Overview**](./ARCHITECTURE.md)
    An explanation of the "Peeking" mechanism and the internals of the proxy engine.

-   [**API Reference**](./API_REFERENCE.md)
    A detailed technical specification of the Refractium types, traits, and the `RefractiumBuilder`.

-   [**Custom Protocol Implementation**](./CUSTOM_PROTOCOLS.md)
    A guide on how to extend Refractium by implementing your own protocol identification logic.

-   [**Load Balancing and Health Monitoring**](./LOAD_BALANCING.md)
    Technical details on how Refractium manages backend reliability and traffic distribution.

## Key Design Principles

1.  **Library-First Design**: While Refractium provides a powerful CLI, it is fundamentally a library. All CLI features are built on top of the same public API documented here.
2.  **Zero-Overhead Abstractions**: The engine is built using asynchronous Rust (Tokio), ensuring that protocol identification does not block the data path.
3.  **Extensible Routing**: Every aspect of the routing table can be updated programmatically without restarting the server.
4.  **Fail-Safe Operations**: Built-in health monitoring and fallback routes prevent downtime when individual backends fail.

## Getting Started Example

To use Refractium as a dependency, add it to your `Cargo.toml`:

```toml
[dependencies]
refractium = "3.0"
```

Then, initialize and run the engine programmatically:

```rust
use refractium::{Refractium, Http, types::{ProtocolRoute, ForwardTarget}};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let routes = vec![ProtocolRoute {
        protocol: Arc::new(Http),
        sni: None,
        forward_to: ForwardTarget::Single("127.0.0.1:8080".to_string()),
    }];

    let refractium = Refractium::builder()
        .routes(routes, Vec::new())
        .build()?;

    refractium.run_tcp("0.0.0.0:8080".parse()?).await?;
    Ok(())
}
```
