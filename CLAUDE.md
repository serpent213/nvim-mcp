# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
with code in this repository.

## Project Overview

This is a Rust-based Model Context Protocol (MCP) server that provides AI
assistants with programmatic access to Neovim instances via TCP connections.
The server implements four core tools: TCP connection management, buffer
listing, and Lua code execution.

## Development Commands

### Building and Running

```bash
# Development build and run
cargo build
cargo run

# Production build and run
cargo build --release
nix run .

# Enter Nix development environment (skip if IN_NIX_SHELL is set)
nix develop .
```

### Testing

```bash
# Run all tests (use single thread to prevent port conflicts)
cargo test -- --show-output --test-threads 1

# Skip integration tests (which require Neovim)
cargo test -- --skip=integration_tests --show-output --test-threads 1

# Run tests in Nix environment (requires IN_NIX_SHELL not set)
nix develop . --command cargo test -- --show-output --test-threads 1
```

**Note**: The `nix develop . --command` syntax only works when the
`IN_NIX_SHELL` environment variable is not set. If you're already in a Nix
shell, use the commands directly without the `nix develop . --command` prefix.

### Neovim Setup for Testing

Start Neovim with TCP listening for integration tests:

```bash
nvim --listen 127.0.0.1:6666
```

## Architecture Overview

The codebase follows a layered architecture:

### Core Components

- **`src/server/neovim.rs`**: Main MCP server implementation (`NeovimMcpServer`)
  - Manages TCP connections to Neovim via `Arc<Mutex<Option<NeovimConnection>>>`
  - Implements four MCP tools using the `#[tool]` attribute
  - Handles connection lifecycle and tool routing

- **`src/neovim/connection.rs`**: Connection management layer
  - Wraps `nvim-rs` client with lifecycle management
  - Tracks connection address and background I/O tasks

- **`src/server/neovim_handler.rs`**: MCP protocol handler
  - Implements `ServerHandler` trait for MCP capabilities
  - Provides server metadata and tool discovery

### Data Flow

1. **MCP Communication**: stdio transport ↔ MCP client ↔ `NeovimMcpServer`
2. **Neovim Integration**: `NeovimMcpServer` → `nvim-rs` → TCP → Neovim instance
3. **Tool Execution**: MCP tool request → async Neovim API call → response

### Connection Management

- Only one active Neovim connection allowed at a time
- Thread-safe access using `Arc<Mutex<>>`
- Proper cleanup of TCP connections and background tasks
- Connection validation before tool execution

## Key Dependencies

- **`rmcp`**: MCP protocol implementation with stdio transport
- **`nvim-rs`**: Neovim msgpack-rpc client (with tokio feature)
- **`tokio`**: Async runtime for concurrent operations
- **`tracing`**: Structured logging throughout the application

## Testing Architecture

- **Integration tests**: Located in `src/server/integration_tests.rs` and
  `src/neovim/integration_tests.rs`
- **Global mutex**: Prevents port conflicts during concurrent test execution
- **Automated setup**: Tests spawn and manage Neovim instances automatically
- **Full MCP flow**: Tests cover complete client-server communication

## Error Handling

- **Layered errors**: `ServerError` (top-level) and `NeovimError` (Neovim-specific)
- **MCP compliance**: Errors are properly formatted for MCP protocol responses
- **Comprehensive propagation**: I/O and nvim-rs errors are properly converted

## Adding New MCP Tools

To add a new tool to the server:

1. Add a new method to `NeovimMcpServer` in `src/server/neovim.rs`
2. Use the `#[tool(description = "...")]` attribute
3. Define request/response parameter structs with `serde` and `schemars`
   derives
4. Add appropriate error handling and connection validation
5. Update integration tests as needed

## Development Environment

This project uses Nix flakes for reproducible development environments.
The flake provides:

- Rust toolchain (stable) with clippy, rustfmt, and rust-analyzer
- Neovim 0.11.3 for integration testing
- Pre-commit hooks for code quality

Use `nix develop .` to enter the development shell (only if `IN_NIX_SHELL` is
not already set) or set up direnv with `echo 'use flake' > .envrc` for
automatic environment activation.
