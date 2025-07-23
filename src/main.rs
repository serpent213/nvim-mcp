use nvim_mcp::CounterServer;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("nvim_mcp=debug".parse()?))
        .init();

    info!("Starting nvim-mcp counter server");

    // Create the counter server instance
    let server = CounterServer::new();

    // Create and run the server with STDIO transport
    let service = server.serve(stdio()).await.inspect_err(|e| {
        error!("Error starting server: {}", e);
    })?;

    info!("Server started, waiting for connections...");

    // Wait for the service to complete
    service.waiting().await?;

    info!("Server shutdown complete");
    Ok(())
}
