use rmcp::{ServerHandler, model::*, tool_handler};
use tracing::{debug, instrument};

use super::neovim::NeovimMcpServer;

#[tool_handler]
impl ServerHandler for NeovimMcpServer {
    #[instrument(skip(self))]
    fn get_info(&self) -> ServerInfo {
        debug!("Providing server information");
        ServerInfo {
            instructions: Some("Neovim API integration server providing TCP connection management, buffer operations, and Lua execution capabilities".to_string()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
