use std::process::Command;
use std::sync::Arc;

use dashmap::DashMap;
use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router,
};
use tracing::{debug, instrument};

use crate::neovim::{NeovimClient, NeovimClientTrait, NeovimError, Position, Range};

impl From<NeovimError> for McpError {
    fn from(err: NeovimError) -> Self {
        match err {
            NeovimError::Connection(msg) => McpError::invalid_request(msg, None),
            NeovimError::Api(msg) => McpError::internal_error(msg, None),
        }
    }
}

pub struct NeovimMcpServer {
    pub nvim_clients: Arc<DashMap<String, Box<dyn NeovimClientTrait + Send>>>,
    pub tool_router: ToolRouter<Self>,
}

/// Connect to Neovim instance via unix socket or TCP
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConnectNvimRequest {
    /// target can be a unix socket path or a TCP address
    pub target: String,
}

/// New parameter struct for connection-aware requests
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConnectionRequest {
    /// Unique identifier for the target Neovim instance
    pub connection_id: String,
}

/// Updated parameter struct for buffer operations with connection context
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BufferConnectionRequest {
    /// Unique identifier for the target Neovim instance
    pub connection_id: String,
    /// Neovim Buffer ID
    pub id: u64,
}

/// Lua execution request with connection context
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecuteLuaConnectionRequest {
    /// Unique identifier for the target Neovim instance
    pub connection_id: String,
    /// Lua code to execute in Neovim
    pub code: String,
}

/// LSP parameters with connection context
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BufferLSPConnectionParams {
    /// Unique identifier for the target Neovim instance
    pub connection_id: String,
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
        let connection_id = self.generate_shorter_connection_id(&path);

        let mut client = NeovimClient::new();
        client.connect_path(&path).await?;
        client.setup_diagnostics_changed_autocmd().await?;

        self.nvim_clients
            .insert(connection_id.clone(), Box::new(client));

        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({
                "connection_id": connection_id,
                "target": path,
                "message": format!("Connected to Neovim at {path}")
            }),
        )?]))
    }

    #[tool(description = "Connect to Neovim instance via TCP")]
    #[instrument(skip(self))]
    pub async fn connect_tcp(
        &self,
        Parameters(ConnectNvimRequest { target: address }): Parameters<ConnectNvimRequest>,
    ) -> Result<CallToolResult, McpError> {
        let connection_id = self.generate_shorter_connection_id(&address);

        let mut client = NeovimClient::new();
        client.connect_tcp(&address).await?;
        client.setup_diagnostics_changed_autocmd().await?;

        self.nvim_clients
            .insert(connection_id.clone(), Box::new(client));

        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({
                "connection_id": connection_id,
                "target": address,
                "message": format!("Connected to Neovim at {address}")
            }),
        )?]))
    }

    #[tool(description = "Disconnect from Neovim instance")]
    #[instrument(skip(self))]
    pub async fn disconnect(
        &self,
        Parameters(ConnectionRequest { connection_id }): Parameters<ConnectionRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Verify connection exists first
        let target = {
            let client = self.get_connection(&connection_id)?;
            client.target().unwrap_or_else(|| "Unknown".to_string())
        };

        // Remove the connection from the map
        if let Some((_, mut client)) = self.nvim_clients.remove(&connection_id) {
            if let Err(e) = client.disconnect().await {
                return Err(McpError::internal_error(
                    format!("Failed to disconnect: {e}"),
                    None,
                ));
            }
            Ok(CallToolResult::success(vec![Content::json(
                serde_json::json!({
                    "connection_id": connection_id,
                    "target": target,
                    "message": format!("Disconnected from Neovim at {target}")
                }),
            )?]))
        } else {
            Err(McpError::invalid_request(
                format!("No Neovim connection found for ID: {connection_id}"),
                None,
            ))
        }
    }

    #[tool(description = "List all open buffers in Neovim")]
    #[instrument(skip(self))]
    pub async fn list_buffers(
        &self,
        Parameters(ConnectionRequest { connection_id }): Parameters<ConnectionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.get_connection(&connection_id)?;
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
        Parameters(ExecuteLuaConnectionRequest {
            connection_id,
            code,
        }): Parameters<ExecuteLuaConnectionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.get_connection(&connection_id)?;
        let result = client.execute_lua(&code).await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Lua result: {result:?}",
        ))]))
    }

    #[tool(description = "Get buffer's diagnostics")]
    #[instrument(skip(self))]
    pub async fn buffer_diagnostics(
        &self,
        Parameters(BufferConnectionRequest { connection_id, id }): Parameters<
            BufferConnectionRequest,
        >,
    ) -> Result<CallToolResult, McpError> {
        let client = self.get_connection(&connection_id)?;
        let diagnostics = client.get_buffer_diagnostics(id).await?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Diagnostics for buffer ID {id}: {diagnostics:?}",
        ))]))
    }

    #[tool(description = "Get workspace's lsp clients")]
    #[instrument(skip(self))]
    pub async fn lsp_clients(
        &self,
        Parameters(ConnectionRequest { connection_id }): Parameters<ConnectionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.get_connection(&connection_id)?;
        let clients = client.lsp_get_clients().await?;

        Ok(CallToolResult::success(vec![Content::json(clients)?]))
    }

    #[tool(description = "Get buffer's code actions")]
    #[instrument(skip(self))]
    pub async fn buffer_code_actions(
        &self,
        Parameters(BufferLSPConnectionParams {
            connection_id,
            id,
            lsp_client_name,
            line,
            character,
            end_line,
            end_character,
        }): Parameters<BufferLSPConnectionParams>,
    ) -> Result<CallToolResult, McpError> {
        let range = Range {
            start: Position { line, character },
            end: Position {
                line: end_line,
                character: end_character,
            },
        };

        let client = self.get_connection(&connection_id)?;
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
            nvim_clients: Arc::new(DashMap::new()),
            tool_router: Self::tool_router(),
        }
    }

    pub fn router(&self) -> &ToolRouter<Self> {
        &self.tool_router
    }

    /// Generate shorter connection ID with collision detection
    fn generate_shorter_connection_id(&self, target: &str) -> String {
        let full_hash = b3sum(target);
        let id_length = 7;

        // Try different starting positions in the hash for 7-char IDs
        for start in 0..=(full_hash.len().saturating_sub(id_length)) {
            let candidate = &full_hash[start..start + id_length];

            if let Some(existing_client) = self.nvim_clients.get(candidate) {
                // Check if the existing connection has the same target
                if let Some(existing_target) = existing_client.target() {
                    if existing_target == target {
                        // Same target, return existing connection ID (connection replacement)
                        return candidate.to_string();
                    }
                }
                // Different target, continue looking for another ID
                continue;
            }

            // No existing connection with this ID, safe to use
            return candidate.to_string();
        }

        // Fallback to full hash if somehow all combinations are taken
        full_hash
    }

    /// Get connection by ID with proper error handling
    pub fn get_connection(
        &self,
        connection_id: &str,
    ) -> Result<dashmap::mapref::one::Ref<String, Box<dyn NeovimClientTrait + Send>>, McpError>
    {
        self.nvim_clients.get(connection_id).ok_or_else(|| {
            McpError::invalid_request(
                format!("No Neovim connection found for ID: {connection_id}"),
                None,
            )
        })
    }
}

impl Default for NeovimMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate BLAKE3 hash from input string
fn b3sum(input: &str) -> String {
    blake3::hash(input.as_bytes()).to_hex().to_string()
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
