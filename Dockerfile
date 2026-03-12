# GreedyClaw — multi-stage Rust build
FROM rust:1.83-bookworm AS builder

# Install protoc
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY proto/ proto/
COPY src/ src/

RUN cargo build --release

# Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/greedyclaw /usr/local/bin/greedyclaw

EXPOSE 7878
ENTRYPOINT ["greedyclaw"]
CMD ["serve"]
