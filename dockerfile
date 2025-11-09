# Stage 1: Builder
FROM rust:1.91-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

COPY . .

RUN cargo build --release

# Stage 2: Runtime
FROM debian:trixie-slim

WORKDIR /app

COPY --from=builder /app/target/release/cakung-barat-server ./cakung-barat-server

COPY assets ./assets

EXPOSE 8080
VOLUME /data
VOLUME /assets
RUN useradd -ms /bin/bash appuser
USER appuser

CMD ["./cakung-barat-server"]