use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use nvim_mcp::NeovimMcpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("nvim_mcp=debug".parse()?))
        .init();

    info!("Starting nvim-mcp Neovim server");
    let server = NeovimMcpServer::new();
    let service = server.serve(stdio()).await.inspect_err(|e| {
        error!("Error starting Neovim server: {}", e);
    })?;
    info!("Neovim server started, waiting for connections...");
    service.waiting().await?;

    info!("Server shutdown complete");
    Ok(())
}
