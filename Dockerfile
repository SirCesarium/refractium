FROM rust:1.94-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src/

COPY . .

RUN cargo build --release

FROM gcr.io/distroless/cc-debian13:nonroot

WORKDIR /app

COPY --from=builder /app/target/release/prisma-rs ./prisma

ENTRYPOINT ["./prisma"]
