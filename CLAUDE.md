# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
with code in this repository.

## Project Overview

This is a Rust-based Model Context Protocol (MCP) server that provides AI
assistants with programmatic access to Neovim instances. The server supports
both Unix socket/named pipe and TCP connections, implements eight core MCP
tools for Neovim interaction, and provides diagnostic resources through the
`nvim-diagnostics://` URI scheme. The project uses Rust 2024 edition and
focuses on async/concurrent operations with proper error handling throughout.

## Development Commands

### Building and Running

```bash
# Development build and run
cargo build
cargo run

# With custom logging options
cargo run -- --log-file ./nvim-mcp.log --log-level debug

# Production build and run
cargo build --release
nix run .

# Enter Nix development environment (skip if IN_NIX_SHELL is set)
nix develop .
```

**CLI Options:**

- `--log-file <PATH>`: Log file path (defaults to stderr)
- `--log-level <LEVEL>`: Log level (trace, debug, info, warn, error;
  defaults to info)

### Testing

```bash
# Run all tests
cargo test -- --show-output

# Run single specific module test
cargo test -- --show-output neovim::integration_tests

# Run single specific test
cargo test -- --show-output neovim::integration_tests::test_tcp_connection_lifecycle

# Skip integration tests (which require Neovim)
cargo test -- --skip=integration_tests --show-output 1

# Run tests in Nix environment (requires IN_NIX_SHELL not set)
nix develop . --command cargo test -- --show-output 1
```

**Note**: The `nix develop . --command` syntax only works when the
`IN_NIX_SHELL` environment variable is not set. If you're already in a Nix
shell, use the commands directly without the `nix develop . --command` prefix.

## Architecture Overview

The codebase follows a layered architecture:

### Core Components

- **`src/server/neovim.rs`**: Main MCP server implementation (`NeovimMcpServer`)
  - Manages multiple concurrent connections via
    `Arc<DashMap<String, Box<dyn NeovimClientTrait + Send>>>`
  - Implements eight MCP tools using the `#[tool]` attribute
  - Handles multi-connection lifecycle and tool routing with deterministic connection IDs

- **`src/neovim/client.rs`**: Neovim client abstraction layer
  - Implements `NeovimClientTrait` for unified client interface
  - Supports both TCP and Unix socket/named pipe connections
  - Provides high-level operations: buffer management, diagnostics, LSP integration
  - Handles Lua code execution and autocmd setup

- **`src/neovim/connection.rs`**: Connection management layer
  - Wraps `nvim-rs` client with lifecycle management
  - Tracks connection address and background I/O tasks

- **`src/server/neovim_handler.rs`**: MCP protocol handler
  - Implements `ServerHandler` trait for MCP capabilities
  - Provides server metadata, tool discovery, and resource handling
  - Supports `nvim-diagnostics://` URI scheme for diagnostic resources

### Data Flow

1. **MCP Communication**: stdio transport ↔ MCP client ↔ `NeovimMcpServer`
2. **Neovim Integration**: `NeovimMcpServer` → `NeovimClientTrait` → `nvim-rs` →
   TCP/Unix socket → Neovim instance
3. **Tool Execution**: MCP tool request → async Neovim API call → response
4. **Resource Access**: MCP resource request → diagnostic data retrieval →
   structured JSON response

### Connection Management

- **Multi-connection support**: Multiple concurrent Neovim instances managed simultaneously
- **Thread-safe access** using `Arc<DashMap<String, Box<dyn NeovimClientTrait + Send>>>`
- **Deterministic connection IDs** generated using BLAKE3 hash of target string
- **Connection isolation**: Each connection operates independently with proper session isolation
- **Proper cleanup** of TCP connections and background tasks on disconnect
- **Connection validation** before tool execution using connection ID lookup

### Multi-Connection Architecture Benefits

**Performance Advantages:**
- **Lock-free reads**: DashMap enables concurrent read access without blocking
- **Fine-grained locking**: Only write operations require locks, not entire connection map access
- **Fast hashing**: BLAKE3 provides extremely fast deterministic connection ID generation
- **Independent operations**: Each connection operates concurrently without affecting others

**Reliability Features:**
- **Deterministic IDs**: Same target always produces same connection ID for predictable behavior
- **Connection replacement**: Connecting to existing target gracefully replaces previous connection
- **Session isolation**: Connections don't interfere with each other's state
- **Graceful cleanup**: Proper resource deallocation on disconnect prevents memory leaks

**Developer Experience:**
- **Predictable workflow**: Connection IDs are consistent across sessions
- **Clear separation**: Connection-scoped resources eliminate ambiguity
- **Concurrent debugging**: Multiple development environments can run simultaneously

### Available MCP Tools

The server provides these tools (implemented with `#[tool]` attribute):

**Connection Management:**
1. **`connect`**: Connect via Unix socket/named pipe, returns deterministic `connection_id`
2. **`connect_tcp`**: Connect via TCP address, returns deterministic `connection_id`
3. **`disconnect`**: Disconnect from specific Neovim instance by `connection_id`

**Connection-Aware Tools** (require `connection_id` parameter):
4. **`list_buffers`**: List all open buffers for specific connection
5. **`exec_lua`**: Execute arbitrary Lua code in specific Neovim instance
6. **`buffer_diagnostics`**: Get diagnostics for specific buffer on specific connection
7. **`lsp_clients`**: Get workspace LSP clients for specific connection
8. **`buffer_code_actions`**: Get LSP code actions for buffer range on specific connection

### MCP Resources

The server provides connection-aware resources via multiple URI schemes:

**Connection Management:**
- **`nvim-connections://`**: Lists all active Neovim connections with their IDs and targets

**Connection-Scoped Diagnostics** via `nvim-diagnostics://` URI scheme:
- **`nvim-diagnostics://{connection_id}/workspace`**: All diagnostic messages across workspace for specific connection
- **`nvim-diagnostics://{connection_id}/buffer/{buffer_id}`**: Diagnostics for specific buffer on specific connection

Resources return structured JSON with diagnostic information including severity,
messages, file paths, and line/column positions. Connection IDs are deterministic
BLAKE3 hashes of the target string for consistent identification.

## Key Dependencies

- **`rmcp`**: MCP protocol implementation with stdio transport and client features
- **`nvim-rs`**: Neovim msgpack-rpc client (with tokio feature)
- **`tokio`**: Async runtime for concurrent operations (full feature set)
- **`tracing`**: Structured logging with subscriber and appender support
- **`clap`**: CLI argument parsing with derive features
- **`thiserror`**: Ergonomic error handling and error type derivation

**Multi-Connection Support Dependencies:**
- **`dashmap`**: Lock-free concurrent HashMap for connection storage
- **`regex`**: Pattern matching for connection-scoped resource URI parsing
- **`blake3`**: Fast, deterministic hashing for connection ID generation

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

To add a new connection-aware tool to the server:

1. Add a new method to `NeovimMcpServer` in `src/server/neovim.rs`
2. Use the `#[tool(description = "...")]` attribute with `#[instrument(skip(self))]`
3. Define request parameter structs with `serde::Deserialize` and
   `schemars::JsonSchema` derives
   - **For connection-aware tools**: Include `connection_id: String` parameter
   - **For connection management**: Use existing parameter types or create new ones
4. Return `Result<CallToolResult, McpError>` and use `NeovimError::from()`
   for error conversion
5. **Connection validation**: Use `self.get_connection(&connection_id)?` to validate
   and retrieve the specific Neovim connection
6. **Tool implementation**: Use the retrieved client reference for Neovim operations
7. Update integration tests in `src/server/integration_tests.rs`
8. Register the tool by adding it to the `tool_router!` macro in server initialization

**Example connection-aware tool pattern:**
```rust
#[tool(description = "Your tool description")]
pub async fn your_tool(
    &self,
    Parameters(YourConnectionRequest { connection_id, other_params }): Parameters<YourConnectionRequest>,
) -> Result<CallToolResult, McpError> {
    let client = self.get_connection(&connection_id)?;
    // Use client for Neovim operations...
}
```

## Development Environment

This project uses Nix flakes for reproducible development environments.
The flake provides:

- Rust toolchain (stable) with clippy, rustfmt, and rust-analyzer
- Neovim 0.11.3+ for integration testing
- Pre-commit hooks for code quality

Use `nix develop .` to enter the development shell (only if `IN_NIX_SHELL` is
not already set) or set up direnv with `echo 'use flake' > .envrc` for
automatic environment activation.

### Code Formatting

The project uses `stylua.toml` for Lua code formatting. Rust code follows
standard rustfmt conventions.

## Neovim Lua Plugin

The project includes a Neovim Lua plugin at `lua/nvim-mcp/init.lua` that:

- Automatically starts a Neovim RPC server on a Unix socket/named pipe
- Generates unique pipe paths based on git root and process ID
- Provides a `setup()` function for initialization
- Enables seamless MCP server connection without manual TCP setup

This eliminates the need to manually start Neovim with `--listen` for MCP
server connections.
