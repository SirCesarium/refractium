# Prisma-RS

![CI](https://github.com/SirCesarium/prisma-rs/actions/workflows/ci.yml/badge.svg)
![Release](https://github.com/SirCesarium/prisma-rs/actions/workflows/release.yml/badge.svg)

Prisma-RS is a lightweight L4 protocol multiplexer. It listens on a single TCP port and routes incoming traffic to different backends based on the initial bytes of the stream.

It's designed to solve a specific problem: running an HTTP service and a binary/raw TCP service on the same external port without the overhead of a full Layer 7 proxy (like Nginx).

## How it works

Prisma performs a non-destructive peek on the first few bytes of every new connection:

- HTTP Traffic: If the stream starts with a standard method (GET, POST, PUT, etc.), it's routed to your web target.

- Binary Traffic: If the signature doesn't match HTTP, the proxy assumes it's a raw binary stream and routes it to your secondary target.

Once the destination is identified, Prisma bridges the two TCP sockets using tokio::io::copy_bidirectional. It stays out of the way, letting the data flow with near-zero latency.

## Why use it?

- Single Port: Bypass firewall restrictions or save public IP resources.

- Near Zero-Copy (ish): It doesn't store or modify your data; it just pipes it.

- Async-First: Built on top of Tokio to handle thousands of concurrent connections.

- Transparent: Your backends don't need to know Prisma exists.

## How to use

### Standard execution

Check the [Releases](https://github.com/SirCesarium/prisma-rs/releases) page for optimized, standalone binaries.

```
./prisma --listen 0.0.0.0:80 --web 127.0.0.1:8080 --bin 127.0.0.1:9000
```

### Docker (Official Image)

You don't need to build it yourself. Pull it from GitHub Container Registry:

```
docker run -p 80:80 ghcr.io/sircesarium/prisma-rs:latest \
  --listen 0.0.0.0:80 --web 1.2.3.4:8080 --bin 1.2.3.4:9000
```

## How to compile

- Make sure to have `rust` and `cargo` installed and updated.

- Run `cargo build --release`

### Docker (local build)

- Build the image: `docker build -t prisma-rs .`

- Run it: `docker run -p 80:80 prisma-rs --listen 0.0.0.0:80 --web 1.2.3.4:8080 --bin 1.2.3.4:9000`

## As a Library

Add Prisma-RS to your `Cargo.toml` without the CLI dependencies:

```bash
cargo add prisma-rs --no-default-features
```

Basic usage

```rust
use prisma_rs::{identify, tunnel, Protocol};

async fn handle(socket: TcpStream) -> io::Result<()> {
    let mut buf = [0u8; 8];
    let n = socket.peek(&mut buf).await?;

    match identify(&buf[..n]) {
        Protocol::Http => tunnel(socket, "127.0.0.1:8080".into()).await,
        Protocol::Binary => tunnel(socket, "127.0.0.1:9000".into()).await,
    }
}
```
