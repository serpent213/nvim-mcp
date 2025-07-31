# Neovim Diagnostics MCP Resources

## Overview

Implement MCP Resources capability for the nvim-mcp server with
`nvim-diagnostics://` URI scheme that exposes Neovim diagnostic data as
structured resources. This enables AI assistants to access diagnostic
information through the standardized MCP Resources API instead of relying
solely on tool calls.

## Context and Research Findings

### Current Architecture Analysis

The codebase follows a clean layered architecture:

- **MCP Server**: `NeovimMcpServer` in `src/server/neovim.rs` with existing
  tools
- **Handler**: `src/server/neovim_handler.rs` implements `ServerHandler` trait
  with `#[tool_handler]` macro
- **Client Layer**: `src/neovim/client.rs` with `NeovimClient` managing TCP
  connections
- **Existing Diagnostic Support**: `buffer_diagnostics` tool and
  `setup_diagnostics_changed_autocmd`

### Key Dependencies and Patterns

- **rmcp v0.3.0**: Uses `ServerCapabilities::builder().enable_resources()` to
  enable resources
- **Tool Pattern**: `#[tool]` attribute with `Parameters<RequestStruct>` for
  tools
- **Handler Pattern**: `#[tool_handler]` impl block with `ServerHandler` trait
- **Async Architecture**: All methods return `impl Future` with proper error
  handling

### Existing Diagnostic Infrastructure

- **Diagnostic Struct**: Complete with all fields (message, code, severity,
  lnum, col, source, etc.)
- **Buffer Diagnostics**: `get_buffer_diagnostics(buffer_id)` using
  `vim.diagnostic.get({buffer_id})`
- **Event Handling**: `setup_diagnostics_changed_autocmd` with
  `DiagnosticChanged` autocmd
- **Test Pattern**: LSP integration tests in
  `src/neovim/integration_tests.rs`

### MCP Resources API Requirements

From rmcp documentation analysis:

- **list_resources**: Returns `ListResourcesResult` with `Vec<Resource>` and
  pagination
- **read_resource**: Takes `ReadResourceRequestParam` with URI, returns
  `ReadResourceResult`
- **Resource Structure**: `uri`, `name`, `description`, `mime_type`, `size`
  fields
- **Content Types**: `ResourceContents::text()` for JSON diagnostic data

## Implementation Blueprint

### Phase 1: Enable Resources Capability

**File**: `src/server/neovim_handler.rs`

Modify the existing `ServerHandler` implementation to enable resources:

```rust
#[tool_handler]
impl ServerHandler for NeovimMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Neovim API integration server providing TCP connection \
                management, buffer operations, Lua execution capabilities, and \
                diagnostic resources through the nvim-diagnostics:// URI scheme."
                    .to_string()
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }

    // Add resource methods here
}
```

**Key Pattern**: Follow existing `get_info()` structure but extend
capabilities.

### Phase 2: Implement Workspace Diagnostic Support

**File**: `src/neovim/client.rs`

Add missing workspace diagnostics functionality:

```rust
impl NeovimClient {
    #[instrument(skip(self))]
    pub async fn get_workspace_diagnostics(
        &self,
    ) -> Result<Vec<Diagnostic>, NeovimError> {
        debug!("Getting all workspace diagnostics");

        let conn = self.connection.as_ref().ok_or_else(|| {
            NeovimError::Connection("Not connected to any Neovim instance".to_string())
        })?;

        match conn
            .nvim
            .execute_lua(
                "return vim.json.encode(vim.diagnostic.get())",
                vec![]
            )
            .await
        {
            Ok(diagnostics) => {
                let diagnostics: Vec<Diagnostic> =
                    match serde_json::from_str(diagnostics.as_str().unwrap()) {
                        Ok(d) => d,
                        Err(e) => {
                            debug!("Failed to parse workspace diagnostics: {}", e);
                            return Err(NeovimError::Api(format!(
                                "Failed to parse workspace diagnostics: {e}"
                            )));
                        }
                    };
                debug!("Found {} workspace diagnostics", diagnostics.len());
                Ok(diagnostics)
            }
            Err(e) => {
                debug!("Failed to get workspace diagnostics: {}", e);
                Err(NeovimError::Api(format!(
                    "Failed to get workspace diagnostics: {e}"
                )))
            }
        }
    }
}
```

**Pattern Reference**: Mirror `get_buffer_diagnostics` but call
`vim.diagnostic.get()` without buffer ID.

### Phase 3: Add Server Methods for Resource Access

**File**: `src/server/neovim.rs`

Add methods to expose diagnostics:

```rust
impl NeovimMcpServer {
    pub async fn get_buffer_diagnostics(
        &self,
        buffer_id: u64,
    ) -> Result<Vec<Diagnostic>, McpError> {
        let client_guard = self.nvim_client.lock().await;
        Ok(client_guard.get_buffer_diagnostics(buffer_id).await?)
    }

    pub async fn get_workspace_diagnostics(
        &self,
    ) -> Result<Vec<Diagnostic>, McpError> {
        let client_guard = self.nvim_client.lock().await;
        Ok(client_guard.get_workspace_diagnostics().await?)
    }
}
```

**Pattern Reference**: Mirror existing tool methods but without `#[tool]`
attribute.

### Phase 4: Implement MCP Resources Methods

**File**: `src/server/neovim_handler.rs`

Add resource methods to the `ServerHandler` impl:

```rust
async fn list_resources(
    &self,
    _request: PaginatedRequestParam,
    _: RequestContext<RoleServer>,
) -> Result<ListResourcesResult, McpError> {
    debug!("Listing available diagnostic resources");

    Ok(ListResourcesResult {
        resources: vec![
            Resource {
                uri: "nvim-diagnostics://workspace".to_string(),
                name: "Workspace Diagnostics".to_string(),
                description: Some("All diagnostic messages across the workspace".to_string()),
                mime_type: Some("application/json".to_string()),
                ..Default::default()
            },
        ],
        next_cursor: None,
    })
}

async fn read_resource(
    &self,
    ReadResourceRequestParam { uri }: ReadResourceRequestParam,
    _: RequestContext<RoleServer>,
) -> Result<ReadResourceResult, McpError> {
    debug!("Reading resource: {}", uri);

    match uri.as_str() {
        uri if uri.starts_with("nvim-diagnostics://buffer/") => {
            let buffer_id = uri
                .strip_prefix("nvim-diagnostics://buffer/")
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| McpError::invalid_params("Invalid buffer ID"))?;

            let diagnostics = self.get_buffer_diagnostics(buffer_id).await?;
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    serde_json::to_string_pretty(&diagnostics)
                        .map_err(|e| McpError::internal_error(
                            "Failed to serialize diagnostics",
                            Some(json!({"error": e.to_string()}))
                        ))?,
                    uri
                )],
            })
        }
        "nvim-diagnostics://workspace" => {
            let diagnostics = self.get_workspace_diagnostics().await?;
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    serde_json::to_string_pretty(&diagnostics)
                        .map_err(|e| McpError::internal_error(
                            "Failed to serialize workspace diagnostics",
                            Some(json!({"error": e.to_string()}))
                        ))?,
                    uri
                )],
            })
        }
        _ => Err(McpError::resource_not_found(
            "resource_not_found",
            Some(json!({"uri": uri})),
        )),
    }
}
```

**Critical Patterns**:

- Use `debug!` logging consistently with existing codebase
- Error handling with proper `McpError` types and context
- JSON serialization with pretty printing for readability
- URI pattern matching with validation

### Phase 5: Add Integration Tests

**File**: `src/server/integration_tests.rs` (create if needed)

Add comprehensive integration tests:

```rust
#[tokio::test]
#[traced_test]
async fn test_list_diagnostic_resources() {
    let port = PORT_BASE + 10;
    let (server, _child) = setup_connected_server(port).await;

    let result = server.list_resources(
        PaginatedRequestParam::default(),
        RequestContext::default()
    ).await;

    assert!(result.is_ok());
    let resources = result.unwrap().resources;
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "nvim-diagnostics://workspace");
}

#[tokio::test]
#[traced_test]
async fn test_read_workspace_diagnostics() {
    let port = PORT_BASE + 11;
    let (server, _child) = setup_connected_server_with_lsp(port).await;

    let result = server.read_resource(
        ReadResourceRequestParam {
            uri: "nvim-diagnostics://workspace".to_string()
        },
        RequestContext::default()
    ).await;

    assert!(result.is_ok());
    let content = result.unwrap().contents;
    assert_eq!(content.len(), 1);
    // Verify JSON structure
}
```

**Pattern Reference**: Mirror `test_get_vim_diagnostics` but for resources
API.

## Documentation References

### Critical URLs and Context

- **rmcp ServerHandler**:
  <https://docs.rs/rmcp/latest/rmcp/handler/server/trait.ServerHandler.html>
- **rmcp Resources**: Context7 library `/context7/rs-rmcp` with comprehensive
  resource examples
- **Neovim Diagnostic API**: <https://neovim.io/doc/user/diagnostic.html>
- **nvim-rs Client**: <https://docs.rs/nvim-rs/latest/nvim_rs/>

### Key rmcp Resource Types

```rust
// From Context7 research
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub size: Option<u32>,
}

pub struct ReadResourceRequestParam {
    pub uri: String,
}

pub struct ReadResourceResult {
    pub contents: Vec<ResourceContents>,
}
```

## Implementation Tasks (Sequential Order)

1. **MODIFY** `src/server/neovim_handler.rs:get_info()` - Enable resources
   capability
2. **ADD** `src/neovim/client.rs:get_workspace_diagnostics()` - Workspace
   diagnostic support
3. **ADD** `src/server/neovim.rs` - Public methods for diagnostic access (2
   methods)
4. **ADD** `src/server/neovim_handler.rs` - Resource methods
   (`list_resources`, `read_resource`)
5. **ADD** integration tests for resources API
6. **UPDATE** existing diagnostic tests to validate resource URIs

## Error Handling Strategy

### Connection Validation

- Ensure Neovim connection exists before resource access
- Use existing `NeovimError` → `McpError` conversion pattern from
  `src/server/neovim.rs:13-20`

### URI Validation

- Validate buffer ID parsing for `nvim-diagnostics://buffer/{id}` URIs
- Return `McpError::invalid_params()` for malformed URIs
- Return `McpError::resource_not_found()` for unknown URI patterns

### JSON Serialization

- Handle `serde_json` errors with `McpError::internal_error()`
- Include error context in error data for debugging

## Performance Considerations

### Caching Strategy

- **Initial Implementation**: Direct API calls for simplicity
- **Future Enhancement**: Cache diagnostic data and use `DiagnosticsChanged`
  event for invalidation
- **Event Integration**: Leverage existing
  `setup_diagnostics_changed_autocmd` for reactive updates

### Resource Pagination

- **Current Scope**: Single workspace resource, no pagination needed
- **Future Enhancement**: Dynamic buffer resource discovery with pagination
  support

## Validation Gates

```bash
# Build and basic functionality
cargo build
cargo run --help

# Unit tests with single thread (prevent port conflicts)
cargo test -- --show-output --test-threads 1

# Integration tests requiring Neovim
cargo test integration_tests -- --show-output --test-threads 1

# Specific diagnostic tests
cargo test test_get_vim_diagnostics -- --show-output --test-threads 1
cargo test test_list_diagnostic_resources -- --show-output --test-threads 1
cargo test test_read_workspace_diagnostics -- --show-output --test-threads 1
```

## Success Criteria

1. **Resources Capability**: Server advertises resources capability in `get_info()`
2. **Resource Discovery**: `list_resources()` returns workspace diagnostic resource
3. **Buffer Resources**: `read_resource("nvim-diagnostics://buffer/0")`
   returns buffer diagnostics
4. **Workspace Resources**: `read_resource("nvim-diagnostics://workspace")`
   returns all diagnostics
5. **Error Handling**: Invalid URIs return proper `resource_not_found`
   errors
6. **JSON Format**: Resource contents are valid, pretty-printed JSON
7. **Integration**: All existing tests continue to pass
8. **Performance**: No significant performance regression

## Risk Mitigation

### Breaking Changes

- **Risk**: Modifying `ServerHandler` might break existing clients
- **Mitigation**: Only additive changes; tools capability remains enabled

### Connection Dependencies

- **Risk**: Resource calls might fail if Neovim not connected
- **Mitigation**: Use existing connection validation patterns; return proper
  errors

### LSP Integration

- **Risk**: Workspace diagnostics might not work without LSP
- **Mitigation**: Return empty array gracefully; test both with/without LSP

## Confidence Score: 9/10

**Rationale**:

- Complete understanding of existing architecture and patterns
- Comprehensive rmcp API research with working examples
- Clear implementation path with existing diagnostic infrastructure
- Well-defined validation strategy and error handling
- Conservative approach with additive changes only

**Risk Areas**:

- Workspace diagnostic API behavior without LSP (minor - testable)
- Resource URI scheme standardization (minor - documented patterns exist)
