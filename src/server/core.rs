use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use dashmap::DashMap;
use rmcp::{ErrorData as McpError, handler::server::router::tool::ToolRouter};
use tracing::debug;

use crate::config::SocketGlobMode;
use crate::neovim::{NeovimClientTrait, NeovimError};

impl From<NeovimError> for McpError {
    fn from(err: NeovimError) -> Self {
        match err {
            NeovimError::Connection(msg) => McpError::invalid_request(msg, None),
            NeovimError::Lsp { code, message } => {
                McpError::invalid_request(format!("LSP Error: {code}, {message}"), None)
            }
            NeovimError::Api(msg) => McpError::internal_error(msg, None),
        }
    }
}

pub struct NeovimMcpServer {
    pub nvim_clients: Arc<DashMap<String, Box<dyn NeovimClientTrait + Send>>>,
    pub tool_router: ToolRouter<Self>,
    pub socket_path: PathBuf,
    pub socket_mode: SocketGlobMode,
}

impl NeovimMcpServer {
    pub fn new(socket_path: PathBuf, socket_mode: SocketGlobMode) -> Self {
        debug!(
            "Creating new NeovimMcpServer instance with socket_path: {}, mode: {:?}",
            socket_path.display(),
            socket_mode
        );
        let server = Self {
            nvim_clients: Arc::new(DashMap::new()),
            tool_router: crate::server::tools::build_tool_router(),
            socket_path,
            socket_mode,
        };

        // Auto-connect for SingleFile mode
        if let Some(target) = server.get_auto_connection_target() {
            debug!("Auto-connecting to single file target: {}", target);
            // We'll handle the actual connection asynchronously in the initialization
        }

        server
    }

    pub fn router(&self) -> &ToolRouter<Self> {
        &self.tool_router
    }

    /// Check if the server is in locked mode (single file connection)
    pub fn is_locked_mode(&self) -> bool {
        matches!(self.socket_mode, SocketGlobMode::SingleFile)
    }

    /// Get the auto-connection target for locked mode
    pub fn get_auto_connection_target(&self) -> Option<String> {
        if self.is_locked_mode() {
            Some(self.socket_path.to_string_lossy().to_string())
        } else {
            None
        }
    }

    /// Initialize auto-connection for locked mode (must be called after server creation)
    pub async fn initialize_auto_connection(&self) -> Result<Option<String>, McpError> {
        if let Some(target) = self.get_auto_connection_target() {
            debug!("Initializing auto-connection to: {}", target);

            // Use the existing connect logic from tools
            use crate::neovim::NeovimClient;

            let connection_id = self.generate_shorter_connection_id(&target);

            // Remove any existing connection with this ID
            if let Some(mut old_client) = self.nvim_clients.get_mut(&connection_id) {
                let _ = old_client.disconnect().await;
            }

            let mut client = NeovimClient::new();
            match client.connect_path(&target).await {
                Ok(()) => {
                    // Setup diagnostics autocmd
                    if let Err(e) = client.setup_diagnostics_changed_autocmd().await {
                        debug!("Failed to setup diagnostics autocmd: {}", e);
                    }

                    self.nvim_clients
                        .insert(connection_id.clone(), Box::new(client));
                    debug!("Auto-connected to {} with ID: {}", target, connection_id);
                    Ok(Some(connection_id))
                }
                Err(e) => {
                    debug!("Failed to auto-connect to {}: {}", target, e);
                    Err(e.into())
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Generate shorter connection ID with collision detection
    pub fn generate_shorter_connection_id(&self, target: &str) -> String {
        let full_hash = b3sum(target);
        let id_length = 7;

        // Try different starting positions in the hash for 7-char IDs
        for start in 0..=(full_hash.len().saturating_sub(id_length)) {
            let candidate = &full_hash[start..start + id_length];

            if let Some(existing_client) = self.nvim_clients.get(candidate) {
                // Check if the existing connection has the same target
                if let Some(existing_target) = existing_client.target()
                    && existing_target == target
                {
                    // Same target, return existing connection ID (connection replacement)
                    return candidate.to_string();
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
        &'_ self,
        connection_id: &str,
    ) -> Result<dashmap::mapref::one::Ref<'_, String, Box<dyn NeovimClientTrait + Send>>, McpError>
    {
        self.nvim_clients.get(connection_id).ok_or_else(|| {
            McpError::invalid_request(
                format!("No Neovim connection found for ID: {connection_id}"),
                None,
            )
        })
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

/// Find all existing nvim-mcp socket targets in the filesystem
/// Returns a vector of socket paths based on the socket mode
pub fn find_get_all_targets(socket_path: &Path, socket_mode: &SocketGlobMode) -> Vec<String> {
    match socket_mode {
        SocketGlobMode::Directory => {
            // Original behavior: search for nvim-mcp.*.sock files in directory
            let pattern = format!("{}/nvim-mcp.*.sock", socket_path.display());
            match glob::glob(&pattern) {
                Ok(paths) => paths
                    .filter_map(|entry| entry.ok())
                    .map(|path| path.to_string_lossy().to_string())
                    .collect(),
                Err(_) => Vec::new(),
            }
        }
        SocketGlobMode::SingleFile => {
            // Single file mode: return the single file if it exists
            if socket_path.exists() {
                vec![socket_path.to_string_lossy().to_string()]
            } else {
                Vec::new()
            }
        }
        SocketGlobMode::GlobPattern => {
            // Glob pattern mode: use the path as a glob pattern
            let pattern = socket_path.to_string_lossy();
            match glob::glob(&pattern) {
                Ok(paths) => paths
                    .filter_map(|entry| entry.ok())
                    .map(|path| path.to_string_lossy().to_string())
                    .collect(),
                Err(_) => Vec::new(),
            }
        }
    }
}
