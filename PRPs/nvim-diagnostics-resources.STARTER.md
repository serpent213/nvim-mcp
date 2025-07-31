# Neovim Diagnostics MCP Resources

## Features

Implement MCP Resources capability for the nvim-mcp server with
`nvim-diagnostics://` URI scheme that exposes Neovim diagnostic data as
structured resources. This enables AI assistants to access diagnostic
information through the standardized MCP Resources API instead of relying
solely on tool calls.

### URI Scheme Support

The `nvim-diagnostics://` scheme supports multiple path patterns:

- `nvim-diagnostics://buffer/{buffer_id}` - Diagnostics for a specific buffer
- `nvim-diagnostics://workspace` - All workspace diagnostics

### Implementation Notes

- The `setup_diagnostics_changed_autocmd` function in `src/neovim/client.rs`
  already subscribes to the `DiagnosticsChanged` event, which can trigger
  resource updates via `NeovimHandler.handle_notify`.
- Reference the integration test `test_get_vim_diagnostics` in
  `src/neovim/integration_tests.rs` for implementation guidance

## Examples

### Enabling Resources Capabilities

The following example demonstrates how to implement MCP Resources support
in a server handler:

```rust
#[tool_handler]
impl ServerHandler for NeovimMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides Neovim diagnostic resources through the \
                nvim-diagnostics:// URI scheme. Access buffer-specific or \
                workspace-wide diagnostic information.".to_string()
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                Resource {
                    uri: "nvim-diagnostics://workspace".to_string(),
                    name: "Workspace Diagnostics".to_string(),
                    description: Some("All diagnostic messages across the workspace".to_string()),
                    mime_type: Some("application/json".to_string()),
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
        match uri.as_str() {
            uri if uri.starts_with("nvim-diagnostics://buffer/") => {
                let buffer_id = uri.strip_prefix("nvim-diagnostics://buffer/")
                    .and_then(|s| s.parse::<u64>().ok())
                    .ok_or_else(|| McpError::invalid_params("Invalid buffer ID"))?;

                let diagnostics = self.get_buffer_diagnostics(buffer_id).await?;
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&diagnostics)?,
                        uri
                    )],
                })
            }
            "nvim-diagnostics://workspace" => {
                let diagnostics = self.get_workspace_diagnostics().await?;
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&diagnostics)?,
                        uri
                    )],
                })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }
}
```

## Documentation

### MCP Resources API

- [rmcp.server.ServerHandler trait Documentation](https://docs.rs/rmcp/latest/rmcp/handler/server/trait.ServerHandler.html)
- [MCP Resources Specification](https://spec.modelcontextprotocol.io/specification/server/resources/)
- use mcp `context7` to retrieve more information
  - rmcp 's repo name is `modelcontextprotocol/rust-sdk`

### Neovim Diagnostics

- [Neovim Diagnostic API](https://neovim.io/doc/user/diagnostic.html)
- [nvim-rs Client Documentation](https://docs.rs/nvim-rs/latest/nvim_rs/)

## Other Considerations

### Performance

- Cache diagnostic data to avoid frequent Neovim API calls
- Implement incremental updates using the `DiagnosticsChanged` event
- Consider pagination for large diagnostic datasets

### Error Handling

- Handle connection failures gracefully
- Provide meaningful error messages for invalid buffer IDs
- Validate URI formats before processing

### Future Enhancements

- Support for diagnostic filtering by severity level
- Real-time diagnostic updates via MCP notifications
- Integration with LSP diagnostic sources
