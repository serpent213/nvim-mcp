# Neovim MCP

## Features

### Tools

The server provides 8 MCP tools for interacting with Neovim instances:

#### Connection Management

- **`get_targets`**: Discover available Neovim socket paths
  - **Parameters**: None
  - **Returns**: Array of socket paths created by the nvim-mcp plugin
  - **Usage**: Call first to find available Neovim instances

- **`connect`**: Connect via Unix socket/named pipe
  - **Parameters**:
    - `target` (string): Socket path from get_targets
  - **Returns**: Object with `connection_id`, `target`, and `message`
  - **Usage**: Establishes connection and returns deterministic connection ID

- **`connect_tcp`**: Connect via TCP address
  - **Parameters**:
    - `target` (string): TCP address (e.g., "127.0.0.1:6666")
  - **Returns**: Object with `connection_id`, `target`, and `message`
  - **Usage**: For manual TCP connections to Neovim with --listen

- **`disconnect`**: Disconnect from Neovim instance
  - **Parameters**:
    - `connection_id` (string): ID from connect/connect_tcp response
  - **Returns**: Confirmation message with target info
  - **Usage**: Clean up specific connection when done

#### Connection-Aware Tools

All tools below require a `connection_id` parameter from connection establishment:

- **`list_buffers`**: List all open buffers
  - **Parameters**:
    - `connection_id` (string): Target Neovim instance ID
  - **Returns**: Array of buffer objects with ID, name, and line count
  - **Usage**: Get overview of available buffers for file operations

- **`exec_lua`**: Execute Lua code in Neovim
  - **Parameters**:
    - `connection_id` (string): Target Neovim instance ID
    - `code` (string): Lua code to execute
  - **Returns**: Object with execution result
  - **Usage**: Run Neovim commands, get editor state, or modify configuration

- **`buffer_diagnostics`**: Get diagnostics for specific buffer
  - **Parameters**:
    - `connection_id` (string): Target Neovim instance ID
    - `id` (number): Buffer ID from list_buffers
  - **Returns**: Array of diagnostic objects with severity, message, and position
  - **Usage**: Analyze errors/warnings in specific file

- **`lsp_clients`**: Get workspace LSP clients
  - **Parameters**:
    - `connection_id` (string): Target Neovim instance ID
  - **Returns**: Array of active LSP client objects
  - **Usage**: Check available language servers before requesting code actions

- **`buffer_code_actions`**: Get LSP code actions for buffer range
  - **Parameters**:
    - `connection_id` (string): Target Neovim instance ID
    - `id` (number): Buffer ID
    - `lsp_client_name` (string): LSP client name from lsp_clients
    - `line` (number): Start line (0-indexed)
    - `character` (number): Start character (0-indexed)
    - `end_line` (number): End line (0-indexed)
    - `end_character` (number): End character (0-indexed)
  - **Returns**: Array of available code action objects
  - **Usage**: Get refactoring options, quick fixes, and code suggestions

### Resources

The server provides connection-aware MCP resources via URI schemes:

#### Connection Management Resource

- **`nvim-connections://`**: Lists active Neovim connections
  - **Content**: JSON array of connection objects with `id` and `target`
  - **Usage**: Monitor active connections across multiple Neovim instances

#### Diagnostic Resources

Connection-scoped diagnostic resources using `nvim-diagnostics://` scheme:

- **`nvim-diagnostics://{connection_id}/workspace`**: All workspace diagnostics
  - **Content**: JSON array of diagnostic messages across entire workspace
  - **Usage**: Get comprehensive error/warning overview for project

- **`nvim-diagnostics://{connection_id}/buffer/{buffer_id}`**: Buffer-specific diagnostics
  - **Content**: JSON array of diagnostic messages for single buffer
  - **Usage**: Focus on errors/warnings in specific file

**Diagnostic Object Structure**:

```json
{
  "severity": 1,
  "message": "Error description",
  "source": "lsp_client_name",
  "range": {
    "start": { "line": 0, "character": 0 },
    "end": { "line": 0, "character": 10 }
  },
  "filename": "/path/to/file.ext"
}
```

## Guide

### Connection Workflow for LLMs

1. **Discovery Phase**: Use `get_targets` to find available Neovim instances
2. **Connection Phase**: Use `connect` with a target from the discovery results
3. **Caching Phase**: Store the `connection_id` for reuse across multiple operations
4. **Work Phase**: Use connection-aware tools with the cached `connection_id`
5. **Optional Cleanup**: Call `disconnect` only when you're completely done
   with a session

### Connection Caching and Management

- **Cache connections**: Store `connection_id` values and reuse them across operations
- **Connection IDs are deterministic**: Same target always produces same ID
- **Persistent connections**: Connections remain active until explicitly disconnected
- **Parallel operations**: Each connection operates independently
- **Connection replacement**: Connecting to existing target replaces previous connection
- **Resource isolation**: Each connection has separate diagnostic resources
- **Automatic cleanup**: Server handles connection cleanup on process termination

### Tool Usage Patterns

#### File Analysis Workflow

1. get_targets → connect → list_buffers (cache connection_id)
2. buffer_diagnostics (for each relevant buffer, reuse connection_id)
3. Read nvim-diagnostics://{connection_id}/workspace resource
4. Keep connection active for future operations

#### Code Action Workflow

1. get_targets → connect → list_buffers (cache connection_id)
2. lsp_clients (to find available language servers, reuse connection_id)
3. buffer_code_actions (with specific range and LSP client, reuse connection_id)
4. exec_lua (to apply selected actions if needed, reuse connection_id)
5. Keep connection active for additional operations

### Error Handling Guidelines

- **Connection errors**: Retry with different target from get_targets
- **Invalid connection_id**: Re-establish connection using connect/connect_tcp
- **Buffer not found**: Use list_buffers to get current buffer list
- **LSP errors**: Check lsp_clients for available language servers

### Resource Reading Strategy

- **Use workspace diagnostics**: For project-wide error analysis
- **Use buffer diagnostics**: For file-specific issue investigation
- **Monitor connections**: Use nvim-connections:// to track active instances
- **Parse diagnostic severity**: 1=Error, 2=Warning, 3=Information, 4=Hint

### Safe Code Execution

- **Read-only operations**: Prefer `vim.inspect()`, `vim.fn.getline()`, `vim.api.nvim_buf_get_lines()`
- **State queries**: Use `vim.fn.getcwd()`, `vim.bo.filetype`, `vim.api.nvim_get_current_buf()`
- **Avoid modifications**: Don't use `vim.api.nvim_buf_set_lines()` or similar
  write operations
- **Error handling**: Wrap Lua code in `pcall()` for safe execution

### Integration Workflows

#### Diagnostic Analysis

1. Connect to Neovim instance (cache connection_id)
2. Read workspace diagnostics resource
3. Group diagnostics by severity and file
4. Use buffer_diagnostics for detailed file analysis (reuse connection_id)
5. Provide structured error report
6. Keep connection active for follow-up analysis

#### Code Understanding

1. Connect and list buffers (cache connection_id)
2. Use exec_lua to get buffer content and metadata (reuse connection_id)
3. Check LSP clients for language-specific information (reuse connection_id)
4. Request code actions for interesting ranges (reuse connection_id)
5. Combine information for comprehensive analysis
6. Maintain connection for iterative code exploration

#### Multi-Instance Management

1. Use get_targets to find all available instances
2. Connect to each target (generates separate connection_ids, cache all IDs)
3. Work with each connection independently using cached IDs
4. Use nvim-connections:// resource to monitor all connections
5. Maintain connections for cross-instance operations
6. Optionally disconnect when completely finished with all instances
