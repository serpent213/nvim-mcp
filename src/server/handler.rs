use super::CounterServer;
use rmcp::{ServerHandler, model::*, tool_handler};
use tracing::{debug, instrument};

#[tool_handler]
impl ServerHandler for CounterServer {
    #[instrument(skip(self))]
    fn get_info(&self) -> ServerInfo {
        debug!("Providing server information");
        ServerInfo {
            instructions: Some("A simple counter server for demonstration purposes".to_string()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
