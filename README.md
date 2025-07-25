# Neovim-MCP

A Model Context Protocol (MCP) server that provides seamless integration
with Neovim instances, enabling AI assistants to interact with your editor
through TCP connections.

## Features

- **TCP Connection Management**: Connect to and disconnect from Neovim
  instances via TCP
- **Buffer Operations**: List and inspect all open buffers with detailed information
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
# Run the MCP server
cargo run --release
# or with Nix
nix run .
```

### Available Tools

The server provides the following MCP tools:

- **`connect_nvim_tcp`**: Connect to a Neovim instance via TCP
  - Parameters: `address` (string) - TCP address (e.g., "127.0.0.1:6666")

- **`disconnect_nvim_tcp`**: Disconnect from the current Neovim instance

- **`list_buffers`**: List all open buffers with their names and line counts

- **`exec_lua`**: Execute Lua code in Neovim
  - Parameters: `code` (string) - Lua code to execute

### Setting up Neovim for TCP

To enable TCP connections in Neovim, start it with:

```bash
nvim --listen 127.0.0.1:6666
```

Or add this to your Neovim configuration:

```lua
vim.fn.serverstart('127.0.0.1:6666')
```

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
# Run all tests
nix develop . --command cargo test -- --show-output --test-threads 1

# Skip integration tests
nix develop . --command cargo test -- \
  --skip=integration_tests --show-output --test-threads 1
```

### Running

```bash
# Run the server
nix run .

# Or in development mode
nix develop . --command cargo run
```

## License

MIT
