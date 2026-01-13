# Stage 1: Builder
FROM rust:1.91-trixie AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y libclang-dev build-essential pkg-config libssl-dev

COPY Cargo.toml Cargo.lock ./

COPY . .

RUN cargo build --release

# Stage 2: Runtime
FROM debian:trixie

WORKDIR /app

ENV SSL_CERT_DIR=/etc/ssl/certs

# Install dependencies including xz-utils for tar.xz extraction
RUN apt-get update && apt-get install -y ca-certificates curl xz-utils && \
    rm -rf /var/lib/apt/lists/*

# Download and install Typst CLI
RUN curl -fsSL -o /tmp/typst.tar.xz https://github.com/typst/typst/releases/download/v0.13.1/typst-x86_64-unknown-linux-musl.tar.xz && \
    tar -xJf /tmp/typst.tar.xz -C /tmp && \
    mv /tmp/typst-x86_64-unknown-linux-musl/typst /usr/local/bin/typst && \
    rm -rf /tmp/typst* && \
    chmod +x /usr/local/bin/typst

COPY --from=builder /app/target/release/cakung-barat-server ./cakung-barat-server
COPY --from=builder /app/.env ./.env
COPY --from=builder /app/static ./static

EXPOSE 8080
VOLUME /app/data

RUN useradd -ms /bin/bash appuser
RUN mkdir -p /app/data && chown -R appuser:appuser /app/data && chown -R appuser:appuser /app/static
USER appuser

CMD ["./cakung-barat-server"]