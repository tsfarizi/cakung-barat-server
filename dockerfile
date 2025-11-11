# Stage 1: Builder
FROM rust:1.91-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y libclang-dev build-essential pkg-config libssl-dev

COPY Cargo.toml Cargo.lock ./

COPY . .

RUN cargo build --release

# Stage 2: Runtime
FROM debian:trixie

WORKDIR /app

ENV SSL_CERT_DIR=/etc/ssl/certs

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cakung-barat-server ./cakung-barat-server
COPY --from=builder /app/.env ./.env

EXPOSE 8080
VOLUME /app/data

RUN useradd -ms /bin/bash appuser
RUN mkdir -p /app/data && chown -R appuser:appuser /app/data
USER appuser

CMD ["./cakung-barat-server"]