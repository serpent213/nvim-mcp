use rmcp::{
    ErrorData as McpError, ServerHandler,
    model::*,
    service::{RequestContext, RoleServer},
    tool_handler,
};
use serde_json::json;
use tracing::{debug, instrument};

use super::neovim::NeovimMcpServer;

#[tool_handler]
impl ServerHandler for NeovimMcpServer {
    #[instrument(skip(self))]
    fn get_info(&self) -> ServerInfo {
        debug!("Providing server information");
        ServerInfo {
            instructions: Some("Neovim API integration server providing TCP connection management, buffer operations, Lua execution capabilities, and diagnostic resources through the nvim-diagnostics:// URI scheme.".to_string()),
            capabilities: ServerCapabilities::builder().enable_tools().enable_resources().build(),
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        debug!("Listing available diagnostic resources");

        Ok(ListResourcesResult {
            resources: vec![Resource {
                raw: RawResource {
                    uri: "nvim-diagnostics://workspace".to_string(),
                    name: "Workspace Diagnostics".to_string(),
                    description: Some("All diagnostic messages across the workspace".to_string()),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                },
                annotations: None,
            }],
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
                    .ok_or_else(|| McpError::invalid_params("Invalid buffer ID", None))?;

                let diagnostics = self.get_buffer_diagnostics(buffer_id).await?;
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&diagnostics).map_err(|e| {
                            McpError::internal_error(
                                "Failed to serialize diagnostics",
                                Some(json!({"error": e.to_string()})),
                            )
                        })?,
                        uri,
                    )],
                })
            }
            "nvim-diagnostics://workspace" => {
                let diagnostics = self.get_workspace_diagnostics().await?;
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&diagnostics).map_err(|e| {
                            McpError::internal_error(
                                "Failed to serialize workspace diagnostics",
                                Some(json!({"error": e.to_string()})),
                            )
                        })?,
                        uri,
                    )],
                })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({"uri": uri})),
            )),
        }
    }
}
