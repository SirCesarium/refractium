FROM rust:1.95-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    mkdir benches && touch benches/protocol_bench.rs && \
    cargo build --release --features cli,logging --bin refractium && \
    rm -rf src/ benches/

COPY . .

RUN cargo build --release --features cli,logging

FROM gcr.io/distroless/cc-debian13:nonroot

WORKDIR /app

COPY --from=builder /app/target/release/refractium ./refractium

ENTRYPOINT ["./refractium"]
