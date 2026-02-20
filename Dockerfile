FROM rust:1.81 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p openclaw-cli

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/openclaw /usr/local/bin/openclaw
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*
CMD ["openclaw", "gateway"]
