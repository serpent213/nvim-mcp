use std::process::Command as StdCommand;
use std::time::{Duration, Instant};

use rmcp::{
    model::CallToolRequestParam,
    serde_json::{Map, Value},
    service::ServiceExt,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{debug, error, info};
use tracing_test::traced_test;

/// Helper function to setup a Neovim instance for testing
async fn setup_test_neovim_instance(
    port: u16,
) -> Result<std::process::Child, Box<dyn std::error::Error>> {
    let listen = format!("127.0.0.1:{port}");

    let mut child = StdCommand::new("nvim")
        .args(["-u", "NONE", "--headless", "--listen", &listen])
        .spawn()
        .map_err(|e| {
            format!("Failed to start Neovim - ensure nvim is installed and in PATH: {e}")
        })?;

    // Wait for Neovim to start and create the TCP socket
    let start = Instant::now();
    loop {
        sleep(Duration::from_millis(100)).await;

        // Try to connect to see if Neovim is ready
        if tokio::net::TcpStream::connect(&listen).await.is_ok() {
            break;
        }

        if start.elapsed() >= Duration::from_secs(10) {
            let _ = child.kill();
            return Err(format!("Neovim failed to start within 10 seconds at {listen}").into());
        }
    }

    debug!("Neovim instance started at {}", listen);
    Ok(child)
}

/// Helper to cleanup Neovim process safely
fn cleanup_nvim_process(mut child: std::process::Child) {
    if let Err(e) = child.kill() {
        tracing::warn!("Failed to kill Neovim process: {}", e);
    }
    if let Err(e) = child.wait() {
        tracing::warn!("Failed to wait for Neovim process: {}", e);
    }
}

#[tokio::test]
#[traced_test]
async fn test_mcp_server_connection() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

    // Connect to the server running as a child process (exact copy from original)
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

    // Verify server info contains expected information
    if let Some(info) = server_info {
        assert!(info.instructions.is_some());
        if let Some(ref instructions) = info.instructions {
            assert!(instructions.contains("Neovim"));
        }

        // Verify server capabilities
        assert!(info.capabilities.tools.is_some());
    } else {
        panic!("Expected server info to be present");
    }

    // Gracefully close the connection
    service.cancel().await?;
    info!("MCP server connection test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_list_tools() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

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

    // List available tools
    let tools = service.list_tools(Default::default()).await?;
    info!("Available tools: {:#?}", tools);

    // Verify we have the expected tools
    let tool_names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(tool_names.contains(&"connect_nvim_tcp"));
    assert!(tool_names.contains(&"disconnect_nvim_tcp"));
    assert!(tool_names.contains(&"list_buffers"));

    // Verify tool descriptions are present
    for tool in &tools.tools {
        assert!(tool.description.is_some());
        assert!(!tool.description.as_ref().unwrap().is_empty());
    }

    service.cancel().await?;
    info!("List tools test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_connect_nvim_tcp_tool() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

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

    // Start a test Neovim instance
    let port = 6667; // Use different port to avoid conflicts
    let nvim_child = setup_test_neovim_instance(port).await?;

    let address = format!("127.0.0.1:{port}");

    // Create arguments as Map (based on rmcp expectations)
    let mut arguments = Map::new();
    arguments.insert("address".to_string(), Value::String(address.clone()));

    // Test successful connection
    let result = service
        .call_tool(CallToolRequestParam {
            name: "connect_nvim_tcp".into(),
            arguments: Some(arguments),
        })
        .await?;

    info!("Connect result: {:#?}", result);
    assert!(!result.content.is_empty());

    // Verify the response contains success message
    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            assert!(text.text.contains("Connected to Neovim"));
            assert!(text.text.contains(&address));
        } else {
            panic!("Expected text content in connect result");
        }
    } else {
        panic!("No content in connect result");
    }

    // Test that connecting again fails (already connected)
    let mut arguments2 = Map::new();
    arguments2.insert("address".to_string(), Value::String(address));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "connect_nvim_tcp".into(),
            arguments: Some(arguments2),
        })
        .await;

    assert!(result.is_err(), "Should not be able to connect twice");

    // Cleanup
    cleanup_nvim_process(nvim_child);
    service.cancel().await?;
    info!("Connect nvim TCP tool test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_disconnect_nvim_tcp_tool() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

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

    // Test disconnect without connection (should fail)
    let result = service
        .call_tool(CallToolRequestParam {
            name: "disconnect_nvim_tcp".into(),
            arguments: None,
        })
        .await;

    assert!(result.is_err(), "Disconnect should fail when not connected");

    // Now connect first, then test disconnect
    let port = 6668;
    let nvim_child = setup_test_neovim_instance(port).await?;

    let address = format!("127.0.0.1:{port}");

    // Connect first
    let mut connect_args = Map::new();
    connect_args.insert("address".to_string(), Value::String(address.clone()));

    let _connect_result = service
        .call_tool(CallToolRequestParam {
            name: "connect_nvim_tcp".into(),
            arguments: Some(connect_args),
        })
        .await?;

    // Now test successful disconnect
    let result = service
        .call_tool(CallToolRequestParam {
            name: "disconnect_nvim_tcp".into(),
            arguments: None,
        })
        .await?;

    info!("Disconnect result: {:#?}", result);
    assert!(!result.content.is_empty());

    // Verify the response contains success message
    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            assert!(text.text.contains("Disconnected from Neovim"));
            assert!(text.text.contains(&address));
        } else {
            panic!("Expected text content in disconnect result");
        }
    } else {
        panic!("No content in disconnect result");
    }

    // Test that disconnecting again fails (not connected)
    let result = service
        .call_tool(CallToolRequestParam {
            name: "disconnect_nvim_tcp".into(),
            arguments: None,
        })
        .await;

    assert!(
        result.is_err(),
        "Should not be able to disconnect when not connected"
    );

    // Cleanup
    cleanup_nvim_process(nvim_child);
    service.cancel().await?;
    info!("Disconnect nvim TCP tool test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_list_buffers_tool() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

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

    // Test list buffers without connection (should fail)
    let result = service
        .call_tool(CallToolRequestParam {
            name: "list_buffers".into(),
            arguments: None,
        })
        .await;

    assert!(
        result.is_err(),
        "List buffers should fail when not connected"
    );

    // Now connect first, then test list buffers
    let port = 6669;
    let nvim_child = setup_test_neovim_instance(port).await?;

    let address = format!("127.0.0.1:{port}");

    // Connect first
    let mut connect_args = Map::new();
    connect_args.insert("address".to_string(), Value::String(address));

    let _connect_result = service
        .call_tool(CallToolRequestParam {
            name: "connect_nvim_tcp".into(),
            arguments: Some(connect_args),
        })
        .await?;

    // Now test list buffers
    let result = service
        .call_tool(CallToolRequestParam {
            name: "list_buffers".into(),
            arguments: None,
        })
        .await?;

    info!("List buffers result: {:#?}", result);
    assert!(!result.content.is_empty());

    // Verify the response contains buffer information
    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            assert!(text.text.contains("Buffer"));
            // Should have at least the initial empty buffer
            assert!(text.text.contains("1"));
        } else {
            panic!("Expected text content in list buffers result");
        }
    } else {
        panic!("No content in list buffers result");
    }

    // Cleanup
    cleanup_nvim_process(nvim_child);
    service.cancel().await?;
    info!("List buffers tool test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_complete_workflow() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

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

    // Start Neovim instance
    let port = 6670;
    let nvim_child = setup_test_neovim_instance(port).await?;

    let address = format!("127.0.0.1:{port}");

    // Step 1: Connect to Neovim
    info!("Step 1: Connecting to Neovim");
    let mut connect_args = Map::new();
    connect_args.insert("address".to_string(), Value::String(address.clone()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "connect_nvim_tcp".into(),
            arguments: Some(connect_args),
        })
        .await?;

    assert!(!result.content.is_empty());
    info!("✓ Connected successfully");

    // Step 2: List buffers
    info!("Step 2: Listing buffers");
    let result = service
        .call_tool(CallToolRequestParam {
            name: "list_buffers".into(),
            arguments: None,
        })
        .await?;

    assert!(!result.content.is_empty());
    info!("✓ Listed buffers successfully");

    // Step 3: Disconnect
    info!("Step 3: Disconnecting from Neovim");
    let result = service
        .call_tool(CallToolRequestParam {
            name: "disconnect_nvim_tcp".into(),
            arguments: None,
        })
        .await?;

    assert!(!result.content.is_empty());
    info!("✓ Disconnected successfully");

    // Step 4: Verify we can't list buffers after disconnect
    info!("Step 4: Verifying disconnect");
    let result = service
        .call_tool(CallToolRequestParam {
            name: "list_buffers".into(),
            arguments: None,
        })
        .await;

    assert!(
        result.is_err(),
        "Should not be able to list buffers after disconnect"
    );
    info!("✓ Verified disconnect state");

    // Cleanup
    cleanup_nvim_process(nvim_child);
    service.cancel().await?;
    info!("Complete workflow test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

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

    // Test connecting to invalid address
    let mut invalid_args = Map::new();
    invalid_args.insert(
        "address".to_string(),
        Value::String("invalid:99999".to_string()),
    );

    let result = service
        .call_tool(CallToolRequestParam {
            name: "connect_nvim_tcp".into(),
            arguments: Some(invalid_args),
        })
        .await;

    assert!(result.is_err(), "Should fail to connect to invalid address");

    // Test calling tools with missing arguments
    let result = service
        .call_tool(CallToolRequestParam {
            name: "connect_nvim_tcp".into(),
            arguments: None,
        })
        .await;

    assert!(result.is_err(), "Should fail when arguments are missing");

    // Test calling non-existent tool
    let result = service
        .call_tool(CallToolRequestParam {
            name: "non_existent_tool".into(),
            arguments: None,
        })
        .await;

    assert!(
        result.is_err(),
        "Should fail when calling non-existent tool"
    );

    service.cancel().await?;
    info!("Error handling test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_exec_lua_tool() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

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

    // Test exec_lua without connection (should fail)
    let mut lua_args = Map::new();
    lua_args.insert("code".to_string(), Value::String("return 42".to_string()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "exec_lua".into(),
            arguments: Some(lua_args),
        })
        .await;

    assert!(result.is_err(), "exec_lua should fail when not connected");

    // Now connect first, then test exec_lua
    let port = 6671;
    let nvim_child = setup_test_neovim_instance(port).await?;

    let address = format!("127.0.0.1:{port}");

    // Connect first
    let mut connect_args = Map::new();
    connect_args.insert("address".to_string(), Value::String(address));

    let _connect_result = service
        .call_tool(CallToolRequestParam {
            name: "connect_nvim_tcp".into(),
            arguments: Some(connect_args),
        })
        .await?;

    // Test successful Lua execution
    let mut lua_args = Map::new();
    lua_args.insert("code".to_string(), Value::String("return 42".to_string()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "exec_lua".into(),
            arguments: Some(lua_args),
        })
        .await?;

    info!("Exec Lua result: {:#?}", result);
    assert!(!result.content.is_empty());

    // Verify the response contains Lua result
    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            assert!(text.text.contains("42"));
        } else {
            panic!("Expected text content in exec_lua result");
        }
    } else {
        panic!("No content in exec_lua result");
    }

    // Test Lua execution with string result
    let mut lua_args = Map::new();
    lua_args.insert(
        "code".to_string(),
        Value::String("return 'hello world'".to_string()),
    );

    let result = service
        .call_tool(CallToolRequestParam {
            name: "exec_lua".into(),
            arguments: Some(lua_args),
        })
        .await?;

    assert!(!result.content.is_empty());

    // Test error handling for invalid Lua
    let mut invalid_lua_args = Map::new();
    invalid_lua_args.insert(
        "code".to_string(),
        Value::String("invalid lua syntax !!!".to_string()),
    );

    let result = service
        .call_tool(CallToolRequestParam {
            name: "exec_lua".into(),
            arguments: Some(invalid_lua_args),
        })
        .await;

    assert!(result.is_err(), "Should fail for invalid Lua syntax");

    // Test error handling for empty code
    let mut empty_lua_args = Map::new();
    empty_lua_args.insert("code".to_string(), Value::String("".to_string()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "exec_lua".into(),
            arguments: Some(empty_lua_args),
        })
        .await;

    assert!(result.is_err(), "Should fail for empty Lua code");

    // Test missing arguments
    let result = service
        .call_tool(CallToolRequestParam {
            name: "exec_lua".into(),
            arguments: None,
        })
        .await;

    assert!(result.is_err(), "Should fail when code argument is missing");

    // Cleanup
    cleanup_nvim_process(nvim_child);
    service.cancel().await?;
    info!("Exec Lua tool test completed successfully");

    Ok(())
}
