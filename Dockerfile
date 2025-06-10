# Use the official Rust image as the base
FROM rust:1.82 AS builder

# Set the working directory inside the container
WORKDIR /app

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src/ ./src/

# Build the application in release mode
RUN cargo build --release

# Use a minimal runtime image
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/monitor .

# Expose the default port
EXPOSE 9999

# Set default environment variables
ENV PORT=9999

# Run the application
CMD ["./monitor"]