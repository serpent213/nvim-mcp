use std::process::Command;
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router,
};
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::neovim::{Diagnostic, NeovimClient, NeovimClientTrait, NeovimError, Position, Range};

impl From<NeovimError> for McpError {
    fn from(err: NeovimError) -> Self {
        match err {
            NeovimError::Connection(msg) => McpError::invalid_request(msg, None),
            NeovimError::Api(msg) => McpError::internal_error(msg, None),
        }
    }
}

pub struct NeovimMcpServer {
    nvim_client: Arc<Mutex<Option<Box<dyn NeovimClientTrait + Send>>>>,
    pub tool_router: ToolRouter<Self>,
}

/// Connect to Neovim instance via unix socket or TCP
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConnectNvimRequest {
    /// target can be a unix socket path or a TCP address
    pub target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecuteLuaRequest {
    /// Lua code to execute in Neovim
    pub code: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BufferId {
    /// Neovim Buffer ID
    pub id: u64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BufferLSPParams {
    /// Neovim Buffer ID
    pub id: u64,
    /// Lsp client name
    pub lsp_client_name: String,
    /// Cursor start position in the buffer, line number starts from 0
    pub line: u64,
    /// Cursor start position in the buffer, character number starts from 0
    pub character: u64,
    /// Cursor end position in the buffer, line number starts from 0
    pub end_line: u64,
    /// Cursor end position in the buffer, character number starts from 0
    pub end_character: u64,
}

#[tool_router]
impl NeovimMcpServer {
    #[tool(description = "Get available Neovim targets")]
    #[instrument(skip(self))]
    pub async fn get_targets(&self) -> Result<CallToolResult, McpError> {
        let targets = find_get_all_targets();
        if targets.is_empty() {
            return Err(McpError::invalid_request(
                "No Neovim targets found".to_string(),
                None,
            ));
        }

        Ok(CallToolResult::success(vec![Content::json(targets)?]))
    }

    #[tool(description = "Connect to Neovim instance via unix socket(pipe)")]
    #[instrument(skip(self))]
    pub async fn connect(
        &self,
        Parameters(ConnectNvimRequest { target: path }): Parameters<ConnectNvimRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut client_guard = self.nvim_client.lock().await;

        let mut client = NeovimClient::new();
        client.connect_path(&path).await?;
        client.setup_diagnostics_changed_autocmd().await?;

        *client_guard = Some(Box::new(client));

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Connected to Neovim at {path}"
        ))]))
    }

    #[tool(description = "Connect to Neovim instance via TCP")]
    #[instrument(skip(self))]
    pub async fn connect_tcp(
        &self,
        Parameters(ConnectNvimRequest { target: address }): Parameters<ConnectNvimRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut client_guard = self.nvim_client.lock().await;

        let mut client = NeovimClient::new();
        client.connect_tcp(&address).await?;
        client.setup_diagnostics_changed_autocmd().await?;

        *client_guard = Some(Box::new(client));

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Connected to Neovim at {address}"
        ))]))
    }

    #[tool(description = "Disconnect from Neovim instance")]
    #[instrument(skip(self))]
    pub async fn disconnect(&self) -> Result<CallToolResult, McpError> {
        let mut client_guard = self.nvim_client.lock().await;
        if let Some(client) = client_guard.as_mut() {
            let target = client.target().unwrap_or_else(|| "Unknown".to_string());
            if let Err(e) = client.disconnect().await {
                return Err(McpError::internal_error(
                    format!("Failed to disconnect: {e}"),
                    None,
                ));
            }
            *client_guard = None;
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Disconnected from Neovim at {target}"
            ))]))
        } else {
            Err(self.no_client_error())
        }
    }

    #[tool(description = "List all open buffers in Neovim")]
    #[instrument(skip(self))]
    pub async fn list_buffers(&self) -> Result<CallToolResult, McpError> {
        let client_guard = self.get_client_guard().await;
        let client = self.with_client_ref(&client_guard)?;
        let buffer_info = client.list_buffers_info().await?;

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
        let client_guard = self.get_client_guard().await;
        let client = self.with_client_ref(&client_guard)?;
        let result = client.execute_lua(&code).await?;

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
        let client_guard = self.get_client_guard().await;
        let client = self.with_client_ref(&client_guard)?;
        let diagnostics = client.get_buffer_diagnostics(id).await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Diagnostics for buffer ID {id}: {diagnostics:?}",
        ))]))
    }

    #[tool(description = "Get workspace's lsp clients")]
    #[instrument(skip(self))]
    pub async fn lsp_clients(&self) -> Result<CallToolResult, McpError> {
        let client_guard = self.get_client_guard().await;
        let client = self.with_client_ref(&client_guard)?;
        let clients = client.lsp_get_clients().await?;

        Ok(CallToolResult::success(vec![Content::json(clients)?]))
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
        let range = Range {
            start: Position { line, character },
            end: Position {
                line: end_line,
                character: end_character,
            },
        };

        let client_guard = self.get_client_guard().await;
        let client = self.with_client_ref(&client_guard)?;
        let actions = client
            .lsp_get_code_actions(&lsp_client_name, id, range)
            .await?;

        Ok(CallToolResult::success(vec![Content::json(actions)?]))
    }
}

impl NeovimMcpServer {
    pub fn new() -> Self {
        debug!("Creating new NeovimMcpServer instance");
        Self {
            nvim_client: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    pub async fn get_buffer_diagnostics(
        &self,
        buffer_id: u64,
    ) -> Result<Vec<Diagnostic>, McpError> {
        let client_guard = self.get_client_guard().await;
        let client = self.with_client_ref(&client_guard)?;
        Ok(client.get_buffer_diagnostics(buffer_id).await?)
    }

    pub async fn get_workspace_diagnostics(&self) -> Result<Vec<Diagnostic>, McpError> {
        let client_guard = self.get_client_guard().await;
        let client = self.with_client_ref(&client_guard)?;
        Ok(client.get_workspace_diagnostics().await?)
    }

    pub fn router(&self) -> &ToolRouter<Self> {
        &self.tool_router
    }

    /// Helper method to get a locked reference to the client
    async fn get_client_guard(
        &self,
    ) -> tokio::sync::MutexGuard<'_, Option<Box<dyn NeovimClientTrait + Send>>> {
        self.nvim_client.lock().await
    }

    /// Helper method to safely access the client or return an error
    fn with_client_ref<'a>(
        &'a self,
        client_guard: &'a tokio::sync::MutexGuard<'_, Option<Box<dyn NeovimClientTrait + Send>>>,
    ) -> Result<&'a dyn NeovimClientTrait, McpError> {
        if let Some(client) = client_guard.as_ref() {
            Ok(client.as_ref())
        } else {
            Err(self.no_client_error())
        }
    }

    /// Helper method to create consistent "no client connected" error
    fn no_client_error(&self) -> McpError {
        McpError::invalid_request("No Neovim client connected".to_string(), None)
    }
}

impl Default for NeovimMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape path for use in filename by replacing problematic characters
#[allow(dead_code)]
fn escape_path(path: &str) -> String {
    // Remove leading/trailing whitespace and replace '/' with '%'
    path.trim().replace("/", "%")
}

/// Get git root directory
#[allow(dead_code)]
fn get_git_root() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if output.status.success() {
        let result = String::from_utf8(output.stdout).ok()?;
        Some(result.trim().to_string())
    } else {
        None
    }
}

/// Get platform-specific temp directory
fn get_temp_dir() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("TEMP").unwrap_or_else(|_| "C:\\temp".to_string())
    } else {
        "/tmp".to_string()
    }
}

/// Find all existing nvim-mcp socket targets in the filesystem
/// Returns a vector of socket paths that match the pattern generated by the Lua plugin
pub fn find_get_all_targets() -> Vec<String> {
    let temp_dir = get_temp_dir();
    let pattern = format!("{temp_dir}/nvim-mcp.*.sock");

    match glob::glob(&pattern) {
        Ok(paths) => paths
            .filter_map(|entry| entry.ok())
            .filter_map(|path| path.to_str().map(String::from))
            .collect(),
        Err(_) => Vec::new(),
    }
}
