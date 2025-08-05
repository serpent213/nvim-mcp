use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::neovim::{Diagnostic, NeovimClient, NeovimError, Position, Range};

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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BufferLSPParams {
    pub id: u64,
    pub lsp_client_name: String,
    pub line: u64,
    pub character: u64,
    pub end_line: u64,
    pub end_character: u64,
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

        client_guard.setup_diagnostics_changed_autocmd().await?;

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

        let diagnostics = client_guard.get_buffer_diagnostics(id).await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Diagnostics for buffer ID {id}: {diagnostics:?}",
        ))]))
    }

    #[tool(description = "Get buffer's code actions")]
    #[instrument(skip(self))]
    pub async fn buffer_code_actions(
        &self,
        Parameters(BufferLSPParams {
            id,
            lsp_client_name,
            line,
            character,
            end_line,
            end_character,
        }): Parameters<BufferLSPParams>,
    ) -> Result<CallToolResult, McpError> {
        let client_guard = self.nvim_client.lock().await;

        let actions = client_guard
            .lsp_get_code_actions(
                &lsp_client_name,
                id,
                Range {
                    start: Position { line, character },
                    end: Position {
                        line: end_line,
                        character: end_character,
                    },
                },
            )
            .await?;

        Ok(CallToolResult::success(vec![Content::json(actions)?]))
    }

    pub async fn get_buffer_diagnostics(
        &self,
        buffer_id: u64,
    ) -> Result<Vec<Diagnostic>, McpError> {
        let client_guard = self.nvim_client.lock().await;
        Ok(client_guard.get_buffer_diagnostics(buffer_id).await?)
    }

    pub async fn get_workspace_diagnostics(&self) -> Result<Vec<Diagnostic>, McpError> {
        let client_guard = self.nvim_client.lock().await;
        Ok(client_guard.get_workspace_diagnostics().await?)
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
