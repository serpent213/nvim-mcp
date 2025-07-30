use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::neovim::{NeovimClient, NeovimError};

impl From<NeovimError> for McpError {
    fn from(err: NeovimError) -> Self {
        match err {
            NeovimError::Connection(msg) => McpError::invalid_request(msg, None),
            NeovimError::Api(msg) => McpError::internal_error(msg, None),
        }
    }
}

#[derive(Clone)]
pub struct NeovimMcpServer {
    nvim_client: Arc<Mutex<NeovimClient>>,
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

#[tool_router]
impl NeovimMcpServer {
    pub fn new() -> Self {
        debug!("Creating new NeovimMcpServer instance");
        Self {
            nvim_client: Arc::new(Mutex::new(NeovimClient::new())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Connect to Neovim instance via TCP")]
    #[instrument(skip(self))]
    pub async fn connect_nvim_tcp(
        &self,
        Parameters(ConnectNvimTCPRequest { address }): Parameters<ConnectNvimTCPRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut client_guard = self.nvim_client.lock().await;

        client_guard.connect(&address).await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Connected to Neovim at {address}"
        ))]))
    }

    #[tool(description = "Disconnect from Neovim instance")]
    #[instrument(skip(self))]
    pub async fn disconnect_nvim_tcp(&self) -> Result<CallToolResult, McpError> {
        let mut client_guard = self.nvim_client.lock().await;

        let address = client_guard.disconnect().await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Disconnected from Neovim at {address}"
        ))]))
    }

    #[tool(description = "List all open buffers in Neovim")]
    #[instrument(skip(self))]
    pub async fn list_buffers(&self) -> Result<CallToolResult, McpError> {
        let client_guard = self.nvim_client.lock().await;

        let buffer_info = client_guard.list_buffers_info().await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Buffers ({}): {}",
            buffer_info.len(),
            buffer_info.join(", ")
        ))]))
    }

    #[tool(description = "Execute Lua code in Neovim")]
    #[instrument(skip(self))]
    pub async fn exec_lua(
        &self,
        Parameters(ExecuteLuaRequest { code }): Parameters<ExecuteLuaRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client_guard = self.nvim_client.lock().await;

        let result = client_guard.execute_lua(&code).await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Lua result: {result:?}",
        ))]))
    }

    #[tool(description = "Get buffer's diagnostics")]
    #[instrument(skip(self))]
    pub async fn buffer_diagnostics(
        &self,
        Parameters(BufferId { id }): Parameters<BufferId>,
    ) -> Result<CallToolResult, McpError> {
        let client_guard = self.nvim_client.lock().await;

        client_guard.setup_diagnostics_changed_autocmd().await?;
        let diagnostics = client_guard.get_buffer_diagnostics(id).await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Diagnostics for buffer ID {id}: {diagnostics:?}",
        ))]))
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
