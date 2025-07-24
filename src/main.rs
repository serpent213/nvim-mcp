use nvim_mcp::{CounterServer, NeovimMcpServer};
use rmcp::{ServiceExt, transport::stdio};
use std::env;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("nvim_mcp=debug".parse()?))
        .init();

    // Check command line arguments to determine server type
    let args: Vec<String> = env::args().collect();
    let server_type = args.get(1).map(|s| s.as_str()).unwrap_or("counter");

    match server_type {
        "neovim" | "nvim" => {
            info!("Starting nvim-mcp Neovim server");
            let server = NeovimMcpServer::new();
            let service = server.serve(stdio()).await.inspect_err(|e| {
                error!("Error starting Neovim server: {}", e);
            })?;
            info!("Neovim server started, waiting for connections...");
            service.waiting().await?;
        }
        _ => {
            info!("Starting nvim-mcp counter server");
            let server = CounterServer::new();
            let service = server.serve(stdio()).await.inspect_err(|e| {
                error!("Error starting counter server: {}", e);
            })?;
            info!("Counter server started, waiting for connections...");
            service.waiting().await?;
        }
    }

    info!("Server shutdown complete");
    Ok(())
}
