use rmcp::{
    ErrorData as McpError, handler::server::router::tool::ToolRouter, model::*, tool, tool_router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

#[derive(Clone)]
pub struct CounterServer {
    counter: Arc<Mutex<i32>>,
    pub tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CounterServer {
    pub fn new() -> Self {
        debug!("Creating new CounterServer instance");
        Self {
            counter: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Increment the counter by 1")]
    #[instrument(skip(self))]
    pub async fn increment(&self) -> Result<CallToolResult, McpError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        debug!("Counter incremented to: {}", *counter);
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }

    #[tool(description = "Get the current counter value")]
    #[instrument(skip(self))]
    pub async fn get(&self) -> Result<CallToolResult, McpError> {
        let counter = self.counter.lock().await;
        debug!("Counter value requested: {}", *counter);
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }

    pub fn router(&self) -> &ToolRouter<Self> {
        &self.tool_router
    }
}

impl Default for CounterServer {
    fn default() -> Self {
        Self::new()
    }
}
