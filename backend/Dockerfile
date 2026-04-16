# Stage 1: Cargo Chef Preparation
FROM lukemathwalker/cargo-chef:latest-rust-1.85-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-json recipe.json

# Stage 2: Builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer
RUN cargo chef cook --release --recipe-json recipe.json
# Build application
COPY . .
# Use SQLX offline mode
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin backend

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/backend /usr/local/bin/backend

# Set environment variables
ENV RUST_LOG=info

# Expose port
EXPOSE 8080

# Command to run
ENTRYPOINT ["/usr/local/bin/backend"]
