# Refractium

Low-level reverse proxy for port multiplexing and protocol-based routing.

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
refractium = "3.0"
```

Basic TCP proxy implementation:

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
