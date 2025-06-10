# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a blockchain monitoring service that provides HTTP endpoints to check the synchronization status and health of Ethereum-compatible nodes. The project contains dual implementations:

- **Primary implementation**: Rust (src/main.rs) - Production-ready async web service using warp
- **Legacy implementation**: Go (main.go) - Simple HTTP server for basic sync checking

## Core Architecture

The Rust service provides three main monitoring endpoints:
- `GET /` - Checks node sync status by comparing block numbers over 30-second interval
- `GET /finalized_latest_diff` - Compares finalized vs latest block difference 
- `GET /check_balance` - Monitors account balance with configurable alert thresholds

All endpoints accept `rpc` query parameter to specify the Ethereum RPC URL (defaults to http://127.0.0.1:8545).

## Development Commands

### Build and Run
```bash
# Build Rust application
cargo build --release

# Run Rust service (default port 9999)
cargo run

# Run with custom port
PORT=8080 cargo run

# Run Go version (legacy)
go run main.go
```

### Testing
```bash
# Run all Rust tests
cargo test

# Run specific test
cargo test test_get_block_number_success

# Run tests with output
cargo test -- --nocapture
```

### Docker
```bash
# Build Docker image
docker build -t blockchain-monitor .

# Run container
docker run -p 9999:9999 blockchain-monitor
```

### Kubernetes Deployment
```bash
# Deploy to Kubernetes
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
```

## Key Dependencies

- **tokio**: Async runtime for Rust implementation
- **warp**: Web framework for HTTP endpoints
- **reqwest**: HTTP client for RPC calls
- **serde/serde_json**: JSON serialization
- **mockito**: HTTP mocking for tests (dev dependency)