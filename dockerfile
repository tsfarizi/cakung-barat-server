# =========================
# Stage 1: Builder
# =========================
FROM rust:1.91-slim AS builder

# Set working directory
WORKDIR /app

# Copy Cargo manifest terlebih dahulu untuk cache dependency
COPY Cargo.toml Cargo.lock ./

COPY . .

# Build ulang dengan source yang benar
RUN cargo build --release

# Stage 2: Runtime
# Gunakan image debian slim yang cocok dengan versi builder (trixie)
FROM debian:trixie-slim

WORKDIR /app

# Copy binari yang sudah dicompile dari stage builder
COPY --from=builder /app/target/release/cakung-barat-server ./cakung-barat-server

# Copy assets
COPY assets ./assets

EXPOSE 8080
VOLUME /data

# Set user non-root untuk keamanan (Opsional tapi sangat disarankan)
RUN useradd -ms /bin/bash appuser
USER appuser

CMD ["./cakung-barat-server"]