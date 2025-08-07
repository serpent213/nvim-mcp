# Neovim MCP Server

A Model Context Protocol (MCP) server that provides seamless integration with
Neovim instances, enabling AI assistants to interact with your editor through
connections and access diagnostic information via structured resources.

## Features

- **Connection Management**: Connect via TCP or Unix socket/named pipe
- **Buffer Operations**: List and inspect all open buffers with detailed information
- **Diagnostics Access**: Retrieve diagnostics for buffers with error/warning details
- **LSP Integration**: Access code actions and LSP client information
- **MCP Resources**: Structured diagnostic data via `nvim-diagnostics://` URI scheme
- **Lua Execution**: Execute arbitrary Lua code directly in Neovim
- **Plugin Integration**: Automatic setup through Neovim plugin
- **Modular Architecture**: Clean separation between core infrastructure, MCP tools, and resource handlers

## Installation

### From Source

```bash
git clone https://github.com/linw1995/nvim-mcp.git
cd nvim-mcp
cargo build --release
```

### Using Nix

```bash
nix run github:linw1995/nvim-mcp
```

## Quick Start

### 1. Install and Start the Server

```bash
# Run the MCP server
cargo run --release

# With custom logging
cargo run --release -- --log-file ./nvim-mcp.log --log-level debug

# Using Nix
nix run .
```

#### Command Line Options

- `--log-file <PATH>`: Path to log file (defaults to stderr)
- `--log-level <LEVEL>`: Log level (trace, debug, info, warn, error;
  defaults to info)

### 2. Setup Neovim Integration

#### Option A: Using Neovim Plugin (Recommended)

With a plugin manager like `lazy.nvim`:

```lua
return {
    "linw1995/nvim-mcp",
    opts = {},
}
```

This automatically creates a Unix socket/pipe for MCP connections.

#### Option B: Manual TCP Setup

Start Neovim with TCP listening:

```bash
nvim --listen 127.0.0.1:6666
```

Or add to your Neovim config:

```lua
vim.fn.serverstart("127.0.0.1:6666")
```

## Available Tools

The server provides these MCP tools for interacting with Neovim:

### Connection Management

- **`get_targets`**: Get available Neovim targets
  - Returns list of discoverable Neovim socket paths created by the plugin
- **`connect`**: Connect via Unix socket/named pipe
  - Parameters: `target` (string) - Socket path
- **`connect_tcp`**: Connect via TCP
  - Parameters: `target` (string) - TCP address (e.g., "127.0.0.1:6666")
- **`disconnect`**: Disconnect from current Neovim instance

### Buffer Operations

- **`list_buffers`**: List all open buffers with names and line counts
- **`buffer_diagnostics`**: Get diagnostics for a specific buffer
  - Parameters: `id` (number) - Buffer ID

### LSP Integration

- **`lsp_clients`**: Get workspace LSP clients
- **`buffer_code_actions`**: Get available code actions for buffer range
  - Parameters: `id` (number), `lsp_client_name` (string), `line` (number),
    `character` (number), `end_line` (number), `end_character` (number)
    (all positions are 0-indexed)

### Code Execution

- **`exec_lua`**: Execute Lua code in Neovim
  - Parameters: `code` (string) - Lua code to execute

## MCP Resources

Access diagnostic information through the `nvim-diagnostics://` URI scheme:

### Available Resources

- **`nvim-diagnostics://workspace`**: All diagnostic messages across the workspace
- **`nvim-diagnostics://buffer/{buffer_id}`**: Diagnostics for a specific buffer

### Usage Example

```json
{
  "method": "resources/read",
  "params": {
    "uri": "nvim-diagnostics://workspace"
  }
}
```

Returns structured JSON with diagnostic information including severity levels,
messages, file paths, and line/column positions.

## Development

This project uses Nix flakes for reproducible development environments.

### Setup

```bash
# Enter development shell
nix develop .

# Auto-activate with direnv (optional)
echo 'use flake' > .envrc
```

### Testing

```bash
# Run all tests (single-threaded to prevent port conflicts)
cargo test -- --show-output --test-threads 1

# Skip integration tests (which require Neovim)
cargo test -- --skip=integration_tests --show-output --test-threads 1

# In Nix environment
nix develop . --command cargo test -- --show-output --test-threads 1
```

**Note**: If already in a Nix shell, omit the `nix develop . --command` prefix.

### Building and Running

```bash
# Development
cargo build && cargo run

# Production
cargo build --release

# With custom logging
cargo run -- --log-file ./debug.log --log-level debug

# Using Nix
nix run .
```

### Plugin Development

For local development with `lazy.nvim`, create `.lazy.lua` in the project root:

```lua
return {
    {
        "linw1995/nvim-mcp",
        dir = ".",
        opts = {},
    },
}
```

## License

MIT
