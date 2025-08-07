# Multi-Connection Neovim Management PRP

**Feature**: Enable the MCP server to manage multiple Neovim instances
concurrently, providing seamless interaction across different Neovim sessions.

**Confidence Level**: 8/10 - Comprehensive research and clear patterns
identified from existing codebase.

## Context & Research Findings

### Current Architecture Analysis

The current `NeovimMcpServer` in `src/server/neovim.rs` uses a
single-connection design:

- **Storage**: `Arc<Mutex<Option<Box<dyn NeovimClientTrait + Send>>>>`
- **Connection Management**: Centralized through `get_client_guard()`,
  `with_client_ref()`, `no_client_error()`
- **Tools**: 8 tools that all require a connected client (except `get_targets`)
- **Resources**: Static `nvim-diagnostics://workspace` and
  `nvim-diagnostics://buffer/{buffer_id}` patterns

### Multi-Connection Patterns from Industry Research

Based on research of Rust async patterns and MCP server implementations:

1. **Connection Storage**: Use `DashMap<String, Box<dyn NeovimClientTrait +
   Send>>` for better performance than `Arc<Mutex<HashMap>>`
   - Source: [Why You Shouldn't Arc Mutex a HashMap in Rust][1]
   - Avoids coarse-grained locking and contention issues

2. **Connection ID Strategy**: Use deterministic checksum of target string
   - Pattern: `blake3::hash(target.as_bytes()).to_hex().to_string()` for
     consistency
   - Same target produces same connection ID (one connection per target)
   - Eliminates timestamp complexity and connection confusion

3. **Resource URI Patterns**: Connection-scoped resource hierarchy
   - `nvim-connections://` - list active connections
   - `nvim-diagnostics://{connection_id}/workspace` - connection-specific
     workspace diagnostics
   - `nvim-diagnostics://{connection_id}/buffer/{buffer_id}` -
     connection-specific buffer diagnostics

4. **Async Concurrency**: Each connection operates independently with proper
   session isolation
   - Source: [MCP Server Multiple Connections Guide][2]

## Implementation Blueprint

### Phase 1: Core Connection Management Refactoring

#### 1.1 Update `NeovimMcpServer` Structure

`src/server/neovim.rs:24-31`

```rust
// Replace single connection with connection map
pub struct NeovimMcpServer {
    nvim_clients: Arc<DashMap<String, Box<dyn NeovimClientTrait + Send>>>,
    pub tool_router: ToolRouter<Self>,
}
```

#### 1.2 Add Connection Management Methods

```rust
impl NeovimMcpServer {
    // Generate deterministic connection ID from target checksum
    fn generate_connection_id(&self, target: &str) -> String {
        blake3::hash(target.as_bytes()).to_hex().to_string()
    }

    // Get connection by ID with proper error handling
    fn get_connection(&self, connection_id: &str) -> Result<
        dashmap::mapref::one::Ref<String, Box<dyn NeovimClientTrait + Send>>,
        McpError
    > {
        self.nvim_clients.get(connection_id)
            .ok_or_else(|| McpError::invalid_request(
                format!("No Neovim connection found for ID: {}", connection_id),
                None
            ))
    }
}
```

### Phase 2: Tool Parameter Extension

#### 2.1 Add Connection ID Parameter Structs

```rust
// New parameter struct for connection-aware requests
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConnectionRequest {
    /// Unique identifier for the target Neovim instance
    pub connection_id: String,
}

// Update existing parameter structs
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BufferConnectionRequest {
    /// Unique identifier for the target Neovim instance
    pub connection_id: String,
    /// Neovim Buffer ID
    pub id: u64,
}
```

#### 2.2 Update Connection Tools

`src/server/neovim.rs:80-116`

```rust
#[tool(description = "Connect to Neovim instance via unix socket(pipe)")]
pub async fn connect(
    &self,
    Parameters(ConnectNvimRequest { target: path }):
        Parameters<ConnectNvimRequest>
) -> Result<CallToolResult, McpError> {
    let connection_id = self.generate_connection_id(&path);

    let mut client = NeovimClient::new();
    client.connect_path(&path).await?;
    client.setup_diagnostics_changed_autocmd().await?;

    self.nvim_clients.insert(connection_id.clone(), Box::new(client));

    Ok(CallToolResult::success(vec![Content::json(serde_json::json!({
        "connection_id": connection_id,
        "target": path,
        "message": format!("Connected to Neovim at {path}")
    }))?]))
}
```

#### 2.3 Update All Existing Tools

Follow pattern in `src/server/neovim.rs:139-221` - each tool needs
`connection_id` parameter and connection lookup:

```rust
#[tool(description = "List buffers for a specific connection")]
pub async fn list_buffers(
    &self,
    Parameters(ConnectionRequest { connection_id }): Parameters<ConnectionRequest>
) -> Result<CallToolResult, McpError> {
    let client = self.get_connection(&connection_id)?;
    let buffer_info = client.list_buffers_info().await?;
    // ... rest of implementation
}
```

### Phase 3: Resource System Enhancement

#### 3.1 Update Resource Handler

`src/server/neovim_handler.rs:24-44`

```rust
async fn list_resources(
    &self,
    _request: Option<PaginatedRequestParam>,
    _: RequestContext<RoleServer>
) -> Result<ListResourcesResult, McpError> {
    let mut resources = vec![
        Resource {
            raw: RawResource {
                uri: "nvim-connections://".to_string(),
                name: "Active Neovim Connections".to_string(),
                description: Some("List of active Neovim connections".to_string()),
                mime_type: Some("application/json".to_string()),
                size: None,
            },
            annotations: None,
        }
    ];

    // Add connection-specific workspace resources
    for connection_id in self.nvim_clients.iter()
        .map(|entry| entry.key().clone()) {
        resources.push(Resource {
            raw: RawResource {
                uri: format!("nvim-diagnostics://{}/workspace", connection_id),
                name: format!("Workspace Diagnostics ({})", connection_id),
                description: Some(format!(
                    "Diagnostic messages for connection {}", connection_id
                )),
                mime_type: Some("application/json".to_string()),
                size: None,
            },
            annotations: None,
        });
    }

    Ok(ListResourcesResult { resources, next_cursor: None })
}
```

#### 3.2 Update Resource Reading

`src/server/neovim_handler.rs:46-93`

```rust
async fn read_resource(
    &self,
    ReadResourceRequestParam { uri }: ReadResourceRequestParam,
    _: RequestContext<RoleServer>
) -> Result<ReadResourceResult, McpError> {
    match uri.as_str() {
        "nvim-connections://" => {
            let connections: Vec<_> = self.nvim_clients.iter()
                .map(|entry| serde_json::json!({
                    "id": entry.key(),
                    "target": entry.value().target()
                        .unwrap_or_else(|| "Unknown".to_string())
                }))
                .collect();

            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    serde_json::to_string_pretty(&connections).unwrap(), uri
                )],
            })
        },
        uri if uri.starts_with("nvim-diagnostics://") => {
            // Parse connection_id from URI pattern
            if let Some(captures) =
                regex::Regex::new(r"nvim-diagnostics://([^/]+)/(.+)")
                    .unwrap().captures(uri) {
                let connection_id = captures.get(1).unwrap().as_str();
                let resource_type = captures.get(2).unwrap().as_str();

                let client = self.get_connection(connection_id)?;

                match resource_type {
                    "workspace" => {
                        let diagnostics = client.get_workspace_diagnostics()
                            .await?;
                        Ok(ReadResourceResult {
                            contents: vec![ResourceContents::text(
                                serde_json::to_string_pretty(&diagnostics)
                                    .unwrap(), uri
                            )],
                        })
                    },
                    path if path.starts_with("buffer/") => {
                        let buffer_id = path.strip_prefix("buffer/")
                            .and_then(|s| s.parse::<u64>().ok())
                            .ok_or_else(|| McpError::invalid_params(
                                "Invalid buffer ID", None
                            ))?;

                        let diagnostics = client.get_buffer_diagnostics(
                            buffer_id
                        ).await?;
                        Ok(ReadResourceResult {
                            contents: vec![ResourceContents::text(
                                serde_json::to_string_pretty(&diagnostics)
                                    .unwrap(), uri
                            )],
                        })
                    },
                    _ => Err(McpError::resource_not_found(
                        "resource_not_found",
                        Some(serde_json::json!({"uri": uri}))
                    ))
                }
            } else {
                Err(McpError::resource_not_found(
                    "resource_not_found",
                    Some(serde_json::json!({"uri": uri}))
                ))
            }
        },
        _ => Err(McpError::resource_not_found(
            "resource_not_found",
            Some(serde_json::json!({"uri": uri}))
        ))
    }
}
```

### Phase 4: Dependencies and Integration

#### 4.1 Add Dependencies

`Cargo.toml`

```toml
[dependencies]
dashmap = "6.1"
regex = "1.11"
blake3 = "1.5"  # Fast, deterministic hashing for connection IDs
```

#### 4.2 Update Imports

`src/server/neovim.rs:1-13`

```rust
use dashmap::DashMap;
use std::sync::Arc;
use blake3;
// ... existing imports
```

## Error Handling Strategy

**Connection-Specific Errors**: Each tool validates connection existence
before operation:

```rust
fn get_connection(&self, connection_id: &str) -> Result<...> {
    self.nvim_clients.get(connection_id)
        .ok_or_else(|| McpError::invalid_request(
            format!("No Neovim connection found for ID: {}", connection_id),
            None
        ))
}
```

**Connection Replacement**: Connecting to same target replaces existing
connection (deterministic IDs).

**Graceful Degradation**: Tools continue working with valid connections even
if others fail.

**Cleanup Strategy**: Implement connection cleanup on disconnect with proper
resource deallocation.

## Testing Strategy

### 4.3 Update Integration Tests

`src/server/integration_tests.rs`

Following existing patterns, create tests for:

1. **Multi-connection lifecycle**: Connect, operate, disconnect multiple
   instances
2. **Connection isolation**: Ensure operations on one connection don't affect
   others
3. **Resource scoping**: Verify connection-specific diagnostic resources
4. **Concurrent operations**: Test multiple tool calls across different
   connections

## Performance Considerations

**DashMap Benefits**:

- Lock-free for reads with multiple concurrent connections
- Fine-grained locking only on write operations
- Better than `Arc<Mutex<HashMap>>` which blocks entire map access

**BLAKE3 Checksums**:

- Extremely fast hash computation (faster than SHA-256)
- Deterministic connection IDs eliminate timestamp complexity
- Same target always produces same ID for predictable behavior

**Memory Management**:

- Connection cleanup on disconnect prevents memory leaks
- Resource deallocation properly handled through Drop traits
- Deterministic IDs simplify connection lifecycle management

**Concurrency**:

- Each connection operates independently
- Async operations don't block other connections
- Proper session isolation maintained

## Implementation Tasks (Ordered)

1. **Update `NeovimMcpServer` structure** with DashMap-based connection storage
2. **Add connection management methods** (generate_connection_id with BLAKE3,
   get_connection)
3. **Create new parameter structs** for connection-aware requests
4. **Update connection tools** (connect, connect_tcp, disconnect) to return
   deterministic connection IDs
5. **Update all existing tools** to accept connection_id parameter
6. **Update resource handler** to support connection-scoped resources
7. **Add dependencies** (dashmap, regex, blake3) to Cargo.toml
8. **Create comprehensive tests** for multi-connection scenarios with
   deterministic IDs
9. **Update tool registration** in main server setup
10. **Verify connection replacement behavior** for same-target connections

## Files to Modify

- `src/server/neovim.rs` - Main server implementation
- `src/server/neovim_handler.rs` - Resource handling
- `Cargo.toml` - Dependencies
- `src/server/integration_tests.rs` - Test coverage
- `CLAUDE.md` - Architecture documentation updates

## Validation Gates

```bash
# Syntax and Type Checking
cargo build
cargo clippy -- -D warnings

# Test Suite (must pass)
cargo test -- --show-output

# Integration Tests (with Neovim instances)
cargo test -- --show-output neovim::integration_tests
cargo test -- --show-output server::integration_tests
```

**Success Criteria**: All existing functionality preserved while supporting
multiple concurrent connections with proper resource isolation and connection
lifecycle management.

[1]: https://packetandpine.com/blog/arc-mutex-hashmap-rust/
[2]: https://mcpcat.io/guides/configuring-mcp-servers-multiple-simultaneous-connections/
