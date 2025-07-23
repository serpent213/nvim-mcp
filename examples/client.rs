use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging for the client
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    info!("Starting MCP client to test counter server");

    // Connect to the server running as a child process
    let service = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.args(["run", "--bin", "nvim-mcp"]);
            },
        ))?)
        .await
        .map_err(|e| {
            error!("Failed to connect to server: {}", e);
            e
        })?;

    // Get server information
    let server_info = service.peer_info();
    info!("Connected to server: {:#?}", server_info);

    // List available tools
    let tools = service.list_tools(Default::default()).await?;
    info!("Available tools: {:#?}", tools);

    // Test the 'get' tool - should return 0 initially
    info!("Testing 'get' tool - should return 0");
    let result = service
        .call_tool(CallToolRequestParam {
            name: "get".into(),
            arguments: None,
        })
        .await?;
    info!("Get result: {:#?}", result);

    // Test the 'increment' tool - should increment and return 1
    info!("Testing 'increment' tool - should increment and return 1");
    let result = service
        .call_tool(CallToolRequestParam {
            name: "increment".into(),
            arguments: None,
        })
        .await?;
    info!("Increment result: {:#?}", result);

    // Test 'get' again - should now return 1
    info!("Testing 'get' tool again - should now return 1");
    let result = service
        .call_tool(CallToolRequestParam {
            name: "get".into(),
            arguments: None,
        })
        .await?;
    info!("Get result after increment: {:#?}", result);

    // Test multiple increments
    for i in 2..=5 {
        info!("Incrementing to {}", i);
        let result = service
            .call_tool(CallToolRequestParam {
                name: "increment".into(),
                arguments: None,
            })
            .await?;
        info!("Increment result: {:#?}", result);
    }

    // Final get to confirm final value
    info!("Final counter value:");
    let result = service
        .call_tool(CallToolRequestParam {
            name: "get".into(),
            arguments: None,
        })
        .await?;
    info!("Final get result: {:#?}", result);

    // Gracefully close the connection
    service.cancel().await?;
    info!("Client session completed successfully");

    Ok(())
}
