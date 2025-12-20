# Build stage
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --package bridge-cli

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/codebridge /usr/local/bin/

# Create data directory
RUN mkdir -p /data

ENV RUST_LOG=info

EXPOSE 8080

CMD ["codebridge", "daemon", "--foreground"]
