use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use nvim_mcp::NeovimMcpServer;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the log file. If not specified, logs to stderr
    #[arg(long)]
    log_file: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize tracing/logging
    let env_filter = EnvFilter::from_default_env()
        .add_directive("nvim_mcp=debug".parse()?)
        .add_directive(cli.log_level.parse()?);

    if let Some(log_file) = cli.log_file {
        // Log to file
        let file_appender = tracing_appender::rolling::daily(
            log_file
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
            log_file
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("nvim-mcp.log")),
        );
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
            .with(env_filter)
            .init();
    } else {
        // Log to stderr (default behavior)
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(std::io::stderr))
            .with(env_filter)
            .init();
    }

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
