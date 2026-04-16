# Stage 1: Cargo Chef Preparation
FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
# Use SQLX offline mode
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin backend

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl\
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN groupadd -r appgroup && useradd -r -g appgroup appuser
# Copy the binary from builder
COPY --from=builder /app/target/release/backend /usr/local/bin/backend

# Use the non-root user
USER appuser

# Set environment variables
ENV RUST_LOG=info

# Expose port
EXPOSE 8000

# Command to run
CMD ["backend"]
