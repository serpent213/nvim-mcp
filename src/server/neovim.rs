use nvim_rs::create::tokio as create;
use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router,
};
use rmpv::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::neovim::{NeovimConnection, NeovimHandler};

#[derive(Clone)]
pub struct NeovimMcpServer {
    connection: Arc<Mutex<Option<NeovimConnection>>>,
    pub tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConnectNvimTCPRequest {
    pub address: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecuteLuaRequest {
    pub code: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BufferId {
    pub id: u64,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub struct Diagnostic {
    pub message: String,
    pub code: String,
    pub severity: u8,
    pub lnum: u64,
    pub col: u64,
    pub source: String,
    pub bufnr: u64,
    pub end_lnum: u64,
    pub end_col: u64,
    pub namespace: u64,
    pub user_data: serde_json::Value,
}

#[tool_router]
impl NeovimMcpServer {
    pub fn new() -> Self {
        debug!("Creating new NeovimMcpServer instance");
        Self {
            connection: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Connect to Neovim instance via TCP")]
    #[instrument(skip(self))]
    pub async fn connect_nvim_tcp(
        &self,
        Parameters(ConnectNvimTCPRequest { address }): Parameters<ConnectNvimTCPRequest>,
    ) -> Result<CallToolResult, McpError> {
        debug!("Attempting to connect to Neovim at {}", address);

        let mut conn_guard = self.connection.lock().await;

        if let Some(ref existing_conn) = *conn_guard {
            return Err(McpError::invalid_request(
                format!(
                    "Already connected to {}. Disconnect first.",
                    existing_conn.address()
                ),
                None,
            ));
        }

        let handler = NeovimHandler;
        match create::new_tcp(&address, handler).await {
            Ok((nvim, io_handler)) => {
                let connection =
                    NeovimConnection::new(nvim, tokio::spawn(io_handler), address.to_string());
                *conn_guard = Some(connection);
                debug!("Successfully connected to Neovim at {}", address);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Connected to Neovim at {address}"
                ))]))
            }
            Err(e) => {
                debug!("Failed to connect to Neovim at {}: {}", address, e);
                Err(McpError::internal_error(
                    format!("Connection failed: {e}"),
                    None,
                ))
            }
        }
    }

    #[tool(description = "Disconnect from Neovim instance")]
    #[instrument(skip(self))]
    pub async fn disconnect_nvim_tcp(&self) -> Result<CallToolResult, McpError> {
        debug!("Attempting to disconnect from Neovim");

        let mut conn_guard = self.connection.lock().await;

        if let Some(connection) = conn_guard.take() {
            connection.io_handler.abort();
            debug!(
                "Successfully disconnected from Neovim at {}",
                connection.address()
            );
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Disconnected from Neovim at {}",
                connection.address()
            ))]))
        } else {
            Err(McpError::invalid_request(
                "Not connected to any Neovim instance",
                None,
            ))
        }
    }

    #[tool(description = "List all open buffers in Neovim")]
    #[instrument(skip(self))]
    pub async fn list_buffers(&self) -> Result<CallToolResult, McpError> {
        debug!("Listing buffers");

        let conn_guard = self.connection.lock().await;
        let conn = conn_guard.as_ref().ok_or_else(|| {
            McpError::invalid_request("Not connected to any Neovim instance", None)
        })?;

        match conn.nvim.list_bufs().await {
            Ok(buffers) => {
                let buffer_info: Vec<String> =
                    futures::future::try_join_all(buffers.iter().map(|buf| async {
                        let number = buf
                            .get_number()
                            .await
                            .map_err(|e| format!("Number error: {e}"))?;
                        let name = buf
                            .get_name()
                            .await
                            .unwrap_or_else(|_| "[No Name]".to_string());
                        let lines = buf
                            .line_count()
                            .await
                            .map_err(|e| format!("Lines error: {e}"))?;
                        Ok::<String, String>(format!("Buffer {number}: {name} ({lines} lines)"))
                    }))
                    .await
                    .map_err(|e| {
                        McpError::internal_error(format!("Failed to get buffer info: {e}"), None)
                    })?;

                debug!("Found {} buffers", buffer_info.len());
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Buffers ({}): {}",
                    buffer_info.len(),
                    buffer_info.join(", ")
                ))]))
            }
            Err(e) => {
                debug!("Failed to list buffers: {}", e);
                Err(McpError::internal_error(
                    format!("Failed to list buffers: {e}"),
                    None,
                ))
            }
        }
    }

    #[tool(description = "Execute Lua code in Neovim")]
    #[instrument(skip(self))]
    pub async fn exec_lua(
        &self,
        Parameters(ExecuteLuaRequest { code }): Parameters<ExecuteLuaRequest>,
    ) -> Result<CallToolResult, McpError> {
        debug!("Executing Lua code: {}", code);

        if code.trim().is_empty() {
            return Err(McpError::invalid_request("Lua code cannot be empty", None));
        }

        let conn_guard = self.connection.lock().await;
        let conn = conn_guard.as_ref().ok_or_else(|| {
            McpError::invalid_request("Not connected to any Neovim instance", None)
        })?;

        let lua_args = Vec::<Value>::new();
        match conn.nvim.exec_lua(&code, lua_args).await {
            Ok(result) => {
                debug!("Lua execution successful, result: {:?}", result);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Lua result: {result:?}",
                ))]))
            }
            Err(e) => {
                debug!("Lua execution failed: {e}");
                Err(McpError::internal_error(
                    format!("Lua execution failed: {e}"),
                    None,
                ))
            }
        }
    }

    #[tool(description = "Get buffer's diagnostics")]
    #[instrument(skip(self))]
    pub async fn buffer_diagnostics(
        &self,
        Parameters(BufferId { id }): Parameters<BufferId>,
    ) -> Result<CallToolResult, McpError> {
        debug!("Getting diagnostics for buffer ID: {}", id);

        let conn_guard = self.connection.lock().await;
        let conn = conn_guard.as_ref().ok_or_else(|| {
            McpError::invalid_request("Not connected to any Neovim instance", None)
        })?;

        match conn
            .nvim
            .execute_lua(
                format!("return vim.json.encode(vim.diagnostic.get({id}))").as_str(),
                vec![],
            )
            .await
        {
            Ok(diagnostics) => {
                let diagnostics: Vec<Diagnostic> =
                    match serde_json::from_str(diagnostics.as_str().unwrap()) {
                        Ok(d) => d,
                        Err(e) => {
                            debug!("Failed to parse diagnostics: {}", e);
                            return Err(McpError::internal_error(
                                format!("Failed to parse diagnostics: {e}"),
                                None,
                            ));
                        }
                    };
                debug!("Found {diagnostics:?} diagnostics for buffer ID {id}");
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Diagnostics for buffer ID {id}: {diagnostics:?}",
                ))]))
            }
            Err(e) => {
                debug!("Failed to get diagnostics for buffer ID {}: {}", id, e);
                Err(McpError::internal_error(
                    format!("Failed to get diagnostics: {e}"),
                    None,
                ))
            }
        }
    }

    pub fn router(&self) -> &ToolRouter<Self> {
        &self.tool_router
    }
}

impl Default for NeovimMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
