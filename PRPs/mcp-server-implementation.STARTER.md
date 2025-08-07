# MCP Server Blueprint

## Feature

Build a stdio MCP (Model Context Protocol) server using the `rmcp` crate. This
server provides a foundation for MCP implementation without built-in tools,
allowing for custom tool integration as needed.

## Examples

### Basic MCP Server Implementation

```toml
[dependencies]
rmcp = "0.3.0"
```

```rust
use rmcp::{
  ErrorData as McpError,
  ServiceExt,
  model::*,
  tool,
  tool_router,
  transport::stdio,
  handler::server::tool::ToolCallContext,
  handler::server::router::tool::ToolRouter,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Counter {
    counter: Arc<Mutex<i32>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Counter {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Increment the counter by 1")]
    async fn increment(&self) -> Result<CallToolResult, McpError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }

    #[tool(description = "Get the current counter value")]
    async fn get(&self) -> Result<CallToolResult, McpError> {
        let counter = self.counter.lock().await;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }
}

// Implement the server handler
#[tool_handler]
impl rmcp::ServerHandler for Counter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// Run the server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and run the server with STDIO transport
    let service = Counter::new().serve(stdio()).await.inspect_err(|e| {
        println!("Error starting server: {}", e);
    })?;
    service.waiting().await?;

    Ok(())
}
```

### Basic MCP Client Implementation

```rust
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::{TokioChildProcess, ConfigureCommandExt}
};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a server running as a child process
    let service = ()
    .serve(TokioChildProcess::new(Command::new("uvx").configure(
        |cmd| {
            cmd.arg("mcp-server-git");
        },
    ))?)
    .await?;

    // Get server information
    let server_info = service.peer_info();
    println!("Connected to server: {server_info:#?}");

    // List available tools
    let tools = service.list_tools(Default::default()).await?;
    println!("Available tools: {tools:#?}");

    // Call a tool
    let result = service
        .call_tool(CallToolRequestParam {
            name: "increment".into(),
            arguments: None,
        })
        .await?;
    println!("Result: {result:#?}");

    // Gracefully close the connection
    service.cancel().await?;

    Ok(())
}
```

### Adding Tracing and Logging

```toml
[dependencies]
tracing = "0.1.41"
tracing-subscriber = { version = "0.3.19", features = ["env-filter"] }
```

```rust
use std::{error::Error, io};
use tracing::{debug, error, info, span, warn, Level};

// the `#[tracing::instrument]` attribute creates and enters a span
// every time the instrumented function is called. The span is named after the
// the function or method. Parameters passed to the function are recorded as fields.
#[tracing::instrument]
pub fn shave(yak: usize) -> Result<(), Box<dyn Error + 'static>> {
    // this creates an event at the DEBUG level with two fields:
    // - `excitement`, with the key "excitement" and the value "yay!"
    // - `message`, with the key "message" and the value
    //   "hello! I'm gonna shave a yak."
    //
    // unlike other fields, `message`'s shorthand initialization is just the
    // string itself.
    debug!(excitement = "yay!", "hello! I'm gonna shave a yak.");
    if yak == 3 {
        warn!("could not locate yak!");
        // note that this is intended to demonstrate `tracing`'s features, not idiomatic
        // error handling! in a library or application, you should consider returning
        // a dedicated `YakError`. libraries like snafu or thiserror make this easy.
        return Err(io::Error::new(io::ErrorKind::Other, "shaving yak failed!").into());
    } else {
        debug!("yak shaved successfully");
    }
    Ok(())
}

pub fn shave_all(yaks: usize) -> usize {
    // Constructs a new span named "shaving_yaks" at the TRACE level,
    // and a field whose key is "yaks". This is equivalent to writing:
    //
    // let span = span!(Level::TRACE, "shaving_yaks", yaks = yaks);
    //
    // local variables (`yaks`) can be used as field values
    // without an assignment, similar to struct initializers.
    let _span = span!(Level::TRACE, "shaving_yaks", yaks).entered();

    info!("shaving yaks");

    let mut yaks_shaved = 0;
    for yak in 1..=yaks {
        let res = shave(yak);
        debug!(yak, shaved = res.is_ok());

        if let Err(ref error) = res {
            // Like spans, events can also use the field initialization shorthand.
            // In this instance, `yak` is the field being initalized.
            error!(yak, error = error.as_ref(), "failed to shave yak!");
        } else {
            yaks_shaved += 1;
        }
        debug!(yaks_shaved);
    }

    yaks_shaved
}

fn main() {
  use tracing_subscriber::{EnvFilter, fmt, prelude::*};

  tracing_subscriber::registry()
      .with(fmt::layer())
      .with(EnvFilter::from_default_env())
      .init();
}
```

## Documentation

- [rmcp Crate Documentation](https://docs.rs/rmcp/latest/rmcp/) - Complete API
  reference and usage guides
- [Model Context Protocol Specification](https://spec.modelcontextprotocol.io/) -
  Official MCP protocol documentation

## Other Considerations

- Implement comprehensive error handling for stdio communication failures
- Add structured logging with appropriate log levels for production debugging
- Design your tool interface to be extensible for future feature additions
- Consider rate limiting and resource management for production deployments
- Use MCP Client implementation of `rmcp` to interact with the server for testing
