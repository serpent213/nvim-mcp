# Neovim MCP Server

A Model Context Protocol (MCP) server that provides seamless integration with
Neovim instances, enabling AI assistants to interact with your editor through
connections and access diagnostic information via structured resources.

## Features

- **Multi-Connection Support**: Manage multiple concurrent Neovim instances with
  deterministic connection IDs
- **Connection Management**: Connect via TCP or Unix socket/named pipe with
  automatic discovery
- **Buffer Operations**: List and inspect all open buffers with detailed information
- **Diagnostics Access**: Retrieve diagnostics for buffers with error/warning details
- **LSP Integration**: Access code actions and LSP client information
- **MCP Resources**: Structured diagnostic data via connection-aware URI schemes
- **Lua Execution**: Execute arbitrary Lua code directly in Neovim
- **Plugin Integration**: Automatic setup through Neovim plugin
- **Modular Architecture**: Clean separation between core infrastructure,
  MCP tools, and resource handlers

## Installation

### Use Cargo install from crates.io

```bash
cargo install nvim-mcp
```

### Using Nix

```bash
nix profile install github:linw1995/nvim-mcp#nvim-mcp
```

### From Source

```bash
git clone https://github.com/linw1995/nvim-mcp.git && cd nvim-mcp
cargo install --path .
```

## Quick Start

### 1. Start the Server

```bash
# Start as stdio MCP server (default)
nvim-mcp
# With custom logging
nvim-mcp --log-file ./nvim-mcp.log --log-level debug
```

#### Command Line Options

- `--log-file <PATH>`: Path to log file (defaults to stderr)
- `--log-level <LEVEL>`: Log level (trace, debug, info, warn, error;
  defaults to info)
- `--socket-path <PATH>`: Directory for socket files (defaults to
  `$HOME/.cache/nvim/rpc` on Unix-like systems, `%TEMP%` on Windows)

### 2. Setup Neovim Integration

#### Option A: Using Neovim Plugin (Recommended)

With a plugin manager like `lazy.nvim`:

```lua
return {
    "linw1995/nvim-mcp",
    -- install the mcp server binary automatically
    -- build = "cargo install --path .",
    build = [[
      nix build .#nvim-mcp
      nix profile remove nvim-mcp
      nix profile install .#nvim-mcp
    ]],
    opts = {},
}
```

This plugin automatically creates a Unix socket/pipe for MCP connections.

#### Option B: Manual TCP Setup

Start Neovim with TCP listening:

```bash
nvim --listen 127.0.0.1:6666
```

Or add to your Neovim config:

```lua
vim.fn.serverstart("127.0.0.1:6666")
```

### 3. Basic Usage Workflow

Once both the MCP server and Neovim are running, here's a typical workflow:

#### Using Unix Socket (Recommended)

1. **Discover available Neovim instances**:
   - Use `get_targets` tool to list available socket paths
2. **Connect to Neovim**:
   - Use `connect` tool with a socket path from step 1
   - Save the returned `connection_id` for subsequent operations
3. **Perform operations**:
   - Use tools like `list_buffers`, `buffer_diagnostics`, etc. with your
     `connection_id`
   - Access resources like `nvim-connections://` or
     `nvim-diagnostics://{connection_id}/workspace`
4. **Optional cleanup**:
   - Use `disconnect` tool when completely done

#### Using TCP Connection

1. **Connect to TCP endpoint**:
   - Use `connect_tcp` tool with address like "127.0.0.1:6666"
   - Save the returned `connection_id`
2. **Follow steps 3-4 above** with your connection ID

## Available Tools

The server provides 23 MCP tools for interacting with Neovim:

### Connection Management

- **`get_targets`**: Discover available Neovim targets
  - Returns list of discoverable Neovim socket paths created by the plugin
  - No parameters required

- **`connect`**: Connect via Unix socket/named pipe
  - Parameters: `target` (string) - Socket path from get_targets
  - Returns: `connection_id` (string) - Deterministic connection identifier

- **`connect_tcp`**: Connect via TCP
  - Parameters: `target` (string) - TCP address (e.g., "127.0.0.1:6666")
  - Returns: `connection_id` (string) - Deterministic connection identifier

- **`disconnect`**: Disconnect from specific Neovim instance
  - Parameters: `connection_id` (string) - Connection identifier to disconnect

### Connection-Aware Tools

All tools below require a `connection_id` parameter from the connection
establishment phase:

#### Buffer Operations

- **`list_buffers`**: List all open buffers with names and line counts
  - Parameters: `connection_id` (string) - Target Neovim connection

- **`buffer_diagnostics`**: Get diagnostics for a specific buffer
  - Parameters: `connection_id` (string), `id` (number) - Buffer ID

#### LSP Integration

- **`lsp_clients`**: Get workspace LSP clients
  - Parameters: `connection_id` (string) - Target Neovim connection

- **`lsp_workspace_symbols`**: Search workspace symbols by query
  - Parameters: `connection_id` (string), `lsp_client_name` (string), `query`
    (string) - Search query for filtering symbols

- **`lsp_code_actions`**: Get LSP code actions with universal document identification
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `start_line` (number), `start_character` (number),
    `end_line` (number), `end_character` (number) (all positions are 0-indexed)

- **`lsp_hover`**: Get LSP hover information with universal document identification
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `line` (number), `character` (number)
    (all positions are 0-indexed)

- **`lsp_document_symbols`**: Get document symbols with universal document identification
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string)

- **`lsp_references`**: Get LSP references with universal document identification
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `line` (number), `character` (number),
    `include_declaration` (boolean)

- **`lsp_resolve_code_action`**: Resolve code actions with incomplete data
  - Parameters: `connection_id` (string), `lsp_client_name` (string),
    `code_action` (CodeAction object) - Code action to resolve

- **`lsp_apply_edit`**: Apply workspace edits using Neovim's LSP utility functions
  - Parameters: `connection_id` (string), `lsp_client_name` (string),
    `workspace_edit` (WorkspaceEdit object) - Workspace edit to apply

- **`lsp_definition`**: Get LSP definition with universal document identification
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `line` (number), `character` (number)
    (all positions are 0-indexed)
  - Returns: Definition result supporting Location arrays, LocationLink arrays,
    or null responses

- **`lsp_type_definition`**: Get LSP type definition with universal document identification
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `line` (number), `character` (number)
    (all positions are 0-indexed)
  - Returns: Type definition result supporting Location arrays, LocationLink arrays,
    or null responses

- **`lsp_implementations`**: Get LSP implementations with universal document identification
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `line` (number), `character` (number)
    (all positions are 0-indexed)
  - Returns: Implementation result supporting Location arrays, LocationLink arrays,
    or null responses

- **`lsp_declaration`**: Get LSP declaration with universal document identification
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `line` (number), `character` (number)
    (all positions are 0-indexed)
  - Returns: Declaration result supporting Location arrays, LocationLink arrays,
    or null responses

- **`lsp_rename`**: Rename symbol across workspace using LSP
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `line` (number), `character` (number),
    `new_name` (string), `prepare_first` (boolean, optional)
    (all positions are 0-indexed)
  - Returns: WorkspaceEdit with file changes or validation errors

- **`lsp_formatting`**: Format document using LSP
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `options` (FormattingOptions),
    `apply_edits` (boolean, optional) (all positions are 0-indexed)
  - Returns: Array of TextEdit objects or success confirmation if auto-applied
  - Notes: Supports LSP 3.15.0+ formatting preferences including tab size,
    insert final newline, trim trailing whitespace, etc.

- **`lsp_range_formatting`**: Format a specific range in a document using LSP
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `start_line` (number), `start_character` (number),
    `end_line` (number), `end_character` (number), `options` (FormattingOptions),
    `apply_edits` (boolean, optional) (all positions are 0-indexed)
  - Returns: Array of TextEdit objects or success confirmation if auto-applied
  - Notes: Formats only the specified range with LSP 3.15.0+ formatting preferences

- **`lsp_organize_imports`**: Sort and organize imports using LSP
  - Parameters: `connection_id` (string), `document` (DocumentIdentifier),
    `lsp_client_name` (string), `apply_edits` (boolean, optional)
  - Returns: Array of TextEdit objects or success confirmation if auto-applied
  - Notes: Organizes and sorts imports with auto-apply enabled by default

### Universal Document Identifier

The `document` parameter in the universal LSP tools accepts a `DocumentIdentifier`
which can reference documents in three ways:

**DocumentIdentifier Enum**:

- **BufferId(u64)**: Reference by Neovim buffer ID (for currently open files)
  - JSON format: `{"buffer_id": 123}`
- **ProjectRelativePath(PathBuf)**: Reference by project-relative path
  - JSON format: `{"project_relative_path": "src/main.rs"}`
- **AbsolutePath(PathBuf)**: Reference by absolute file path
  - JSON format: `{"absolute_path": "/home/user/project/src/main.rs"}`

This system enables LSP operations on files that may not be open in Neovim buffers,
providing enhanced flexibility for code analysis and navigation.

#### Code Execution

- **`exec_lua`**: Execute Lua code in Neovim
  - Parameters: `connection_id` (string), `code` (string) - Lua code to execute

### Complete LSP Code Action Workflow

The server now supports the full LSP code action lifecycle:

1. **Get Available Actions**: Use `lsp_code_actions` to retrieve available
   code actions for a specific range
2. **Resolve Action**: Use `lsp_resolve_code_action` to resolve any code
   action that may have incomplete data
3. **Apply Changes**: Use `lsp_apply_edit` to apply the workspace edit from
   the resolved code action

**Example Workflow**:

```text
1. lsp_code_actions → Get available actions
2. lsp_resolve_code_action → Resolve incomplete action data
3. lsp_apply_edit → Apply the workspace edit to files
```

This enables AI assistants to perform complete code refactoring, quick fixes,
and other LSP-powered transformations. The implementation uses Neovim's native
`vim.lsp.util.apply_workspace_edit()` function with proper position encoding
handling, ensuring reliable and accurate file modifications.

## MCP Resources

Access diagnostic and connection information through structured URI schemes:

### Available Resources

#### Connection Monitoring

- **`nvim-connections://`**: List all active Neovim connections
  - Returns array of connection objects with `id` and `target` information
  - Useful for monitoring multiple concurrent Neovim instances

#### Connection-Scoped Diagnostics

Diagnostic resources use connection-specific URIs via the
`nvim-diagnostics://` scheme:

- **`nvim-diagnostics://{connection_id}/workspace`**: All diagnostic messages
  across workspace for specific connection
- **`nvim-diagnostics://{connection_id}/buffer/{buffer_id}`**: Diagnostics for
  specific buffer on specific connection

### Usage Examples

#### List Active Connections

```json
{
  "method": "resources/read",
  "params": {
    "uri": "nvim-connections://"
  }
}
```

#### Get Connection-Specific Workspace Diagnostics

```json
{
  "method": "resources/read",
  "params": {
    "uri": "nvim-diagnostics://abc123def456/workspace"
  }
}
```

#### Get Buffer Diagnostics for Specific Connection

```json
{
  "method": "resources/read",
  "params": {
    "uri": "nvim-diagnostics://abc123def456/buffer/1"
  }
}
```

All diagnostic resources return structured JSON with diagnostic information
including severity levels, messages, file paths, and line/column positions.
Connection IDs are deterministic BLAKE3 hashes of the target string for
consistent identification across sessions.

## Multi-Connection Architecture

The server supports managing multiple concurrent Neovim instances through a
multi-connection architecture with several key benefits:

### Architecture Features

- **Deterministic Connection IDs**: Each connection gets a consistent ID based
  on BLAKE3 hashing of the target string
- **Independent Sessions**: Each Neovim instance operates independently without
  interfering with others
- **Thread-Safe Operations**: Concurrent access to multiple connections using
  lock-free data structures
- **Connection Isolation**: Diagnostics and resources are scoped to specific
  connections

### Typical Workflow

1. **Discovery**: Use `get_targets` to find available Neovim socket paths
2. **Connection**: Use `connect` or `connect_tcp` to establish connection and
   get `connection_id`
3. **Operations**: Use connection-aware tools with the `connection_id` parameter
4. **Resource Access**: Read connection-scoped resources using the
   `connection_id` in URI patterns
5. **Cleanup**: Optionally use `disconnect` when done (connections persist
   until explicitly closed)

### Benefits

- **Concurrent Development**: Work with multiple Neovim instances simultaneously
- **Session Persistence**: Connection IDs remain consistent across MCP server
  restarts
- **Resource Efficiency**: Each connection operates independently without
  blocking others
- **Clear Separation**: Connection-scoped resources eliminate ambiguity about
  which Neovim instance data belongs to

## Development

This project uses Nix flakes for reproducible development environments.

### Setup

```bash
# Enter development shell
nix develop .

# Auto-activate with direnv (optional)
echo 'use flake' >.envrc
```

### Testing

```bash
# Run all tests
cargo test -- --show-output

# Skip integration tests (which require Neovim)
cargo test -- --skip=integration_tests --show-output

# In Nix environment
nix develop . --command cargo test -- --show-output
```

**Note**: If already in a Nix shell, omit the `nix develop . --command` prefix.

### Building and Running

```bash
# Build debug version
cargo build

# Build and run debug version
cargo run

# Build and run release version
cargo run --release

# Build and run with custom logging
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
