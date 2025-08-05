# Neovim-MCP

A Model Context Protocol (MCP) server that provides seamless integration
with Neovim instances, enabling AI assistants to interact with your editor
through TCP connections and access diagnostic information via structured
resources.

## Features

- **TCP Connection Management**: Connect to and disconnect from Neovim
  instances via TCP
- **Buffer Operations**: List and inspect all open buffers with detailed
  information
- **Diagnostics**: Get diagnostics for specific buffers with detailed
  error and warning information
- **LSP Code Actions**: Retrieve available code actions for specific buffer
  positions using LSP clients
- **MCP Resources**: Access diagnostic information through structured resources
  using the `nvim-diagnostics://` URI scheme
- **Lua Execution**: Execute arbitrary Lua code directly in Neovim

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

## Usage

### Starting the Server

```bash
# Run the MCP server (basic)
cargo run --release

# Run with custom log file and level
cargo run --release -- --log-file ./nvim-mcp.log --log-level debug

# Or with Nix
nix run .
```

### Command Line Options

The server supports the following command-line options:

- `--log-file <PATH>`: Path to the log file (optional, defaults to stderr)
- `--log-level <LEVEL>`: Log level - trace, debug, info, warn, error
  (defaults to info)

### Available Tools

The server provides the following MCP tools:

- **`connect_nvim_tcp`**: Connect to a Neovim instance via TCP

  - Parameters: `address` (string) - TCP address (e.g., "127.0.0.1:6666")

- **`disconnect_nvim_tcp`**: Disconnect from the current Neovim instance

- **`list_buffers`**: List all open buffers with their names and line counts

- **`buffer_diagnostics`**: Get diagnostics for a specific buffer

  - Parameters: `id` (number) - Buffer ID to get diagnostics for

- **`exec_lua`**: Execute Lua code in Neovim
  - Parameters: `code` (string) - Lua code to execute

- **`buffer_code_actions`**: Get available LSP code actions for a buffer
  position
  - Parameters:
    - `id` (number) - Buffer ID
    - `lsp_client_name` (string) - Name of the LSP client
    - `line` (number) - Line number (0-indexed)
    - `character` (number) - Character position (0-indexed)
    - `end_line` (number) - End line number (0-indexed)
    - `end_character` (number) - End character position (0-indexed)

### Setting up Neovim for TCP

To enable TCP connections in Neovim, start it with:

```bash
nvim --listen 127.0.0.1:6666
```

Or add this to your Neovim configuration:

```lua
vim.fn.serverstart('127.0.0.1:6666')
```

## Resources

The server provides MCP resources to access diagnostic information through
the `nvim-diagnostics://` URI scheme:

### Available Resources

- **`nvim-diagnostics://workspace`**: Get all diagnostic messages across the
  entire workspace
- **`nvim-diagnostics://buffer/{buffer_id}`**: Get diagnostics for a specific
  buffer by its ID

### Usage Example

```json
{
  "method": "resources/read",
  "params": {
    "uri": "nvim-diagnostics://workspace"
  }
}
```

This returns structured JSON containing all diagnostic information, including
severity levels, messages, file paths, and line/column positions.

## Development

This project uses Nix flakes for reproducible development environments.

### Setup

```bash
# Enter development shell
nix develop .

# Or auto-activate with direnv (if available)
echo 'use flake' > .envrc
```

### Testing

```bash
# Run all tests (use single thread to prevent port conflicts)
cargo test -- --show-output --test-threads 1

# Skip integration tests (which require Neovim)
cargo test -- --skip=integration_tests --show-output --test-threads 1

# Run tests in Nix environment
nix develop . --command cargo test -- --show-output --test-threads 1
```

**Note**: If you're already in a Nix shell,
use the commands directly without the `nix develop . --command` prefix.

### Running

```bash
# Run the server
nix run .

# Development build and run
cargo build
cargo run

# Production build and run
cargo build --release

# Run with custom logging in development
cargo run -- --log-file ./debug.log --log-level debug
```

## License

MIT
