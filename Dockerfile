# PayTrust Payment Orchestration Platform - Docker Deployment

FROM rust:1.91 as builder

# Install required system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY rustfmt.toml ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY specs ./specs

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 paytrust

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/paytrust ./

# Copy migrations for runtime
COPY --from=builder /app/migrations ./migrations

# Copy specs for OpenAPI documentation
COPY --from=builder /app/specs ./specs

# Change ownership to paytrust user
RUN chown -R paytrust:paytrust /app

# Switch to non-root user
USER paytrust

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the binary
CMD ["./paytrust"]
