use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rmcp::ServerHandler;
use rmcp::handler::server::tool::Parameters;
use tokio::time::sleep;
use tracing_test::traced_test;

use crate::NeovimMcpServer;
use crate::server::neovim::{ConnectNvimTCPRequest, ExecuteLuaRequest};

const HOST: &str = "127.0.0.1";
const PORT_BASE: u16 = 7777;

// Global mutex to prevent concurrent Neovim instances from using the same port
static NEOVIM_TEST_MUTEX: Mutex<()> = Mutex::new(());

fn nvim_path() -> &'static str {
    "nvim"
}

async fn setup_neovim_instance(port: u16) -> std::process::Child {
    let listen = format!("{HOST}:{port}");

    let mut child = Command::new(nvim_path())
        .args(["-u", "NONE", "--headless", "--listen", &listen])
        .spawn()
        .expect("Failed to start Neovim - ensure nvim is installed and in PATH");

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
            panic!("Neovim failed to start within 10 seconds at {listen}");
        }
    }

    child
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

async fn setup_connected_server(port: u16) -> (NeovimMcpServer, std::process::Child) {
    let child = setup_neovim_instance(port).await;
    let server = NeovimMcpServer::new();
    let address = format!("{HOST}:{port}");

    let result = server
        .connect_nvim_tcp(Parameters(ConnectNvimTCPRequest { address }))
        .await;
    if result.is_err() {
        cleanup_nvim_process(child);
        panic!("Failed to connect to Neovim: {result:?}");
    }

    (server, child)
}

#[tokio::test]
#[traced_test]
async fn test_connection_lifecycle() {
    let port = PORT_BASE;
    let address = format!("{HOST}:{port}");

    let child = {
        let _guard = NEOVIM_TEST_MUTEX.lock().unwrap();
        drop(_guard);
        setup_neovim_instance(port).await
    };
    let server = NeovimMcpServer::new();

    // Test connection
    let result = server
        .connect_nvim_tcp(Parameters(ConnectNvimTCPRequest {
            address: address.clone(),
        }))
        .await;
    assert!(result.is_ok(), "Failed to connect: {result:?}");

    // Test that we can't connect again while already connected
    let result = server
        .connect_nvim_tcp(Parameters(ConnectNvimTCPRequest {
            address: address.clone(),
        }))
        .await;
    assert!(result.is_err(), "Should not be able to connect twice");

    // Test disconnect
    let result = server.disconnect_nvim_tcp().await;
    assert!(result.is_ok(), "Failed to disconnect: {result:?}");

    // Test that disconnect fails when not connected
    let result = server.disconnect_nvim_tcp().await;
    assert!(
        result.is_err(),
        "Should not be able to disconnect when not connected"
    );

    cleanup_nvim_process(child);
}

#[tokio::test]
#[traced_test]
async fn test_buffer_operations() {
    let port = PORT_BASE + 1;

    let (server, child) = {
        let _guard = NEOVIM_TEST_MUTEX.lock().unwrap();
        drop(_guard);
        setup_connected_server(port).await
    };

    // Test buffer listing
    let result = server.list_buffers().await;
    assert!(result.is_ok(), "Failed to list buffers: {result:?}");

    let result = result.unwrap();
    assert!(!result.content.is_empty());

    let content_text = if let Some(content) = result.content.first() {
        if let Some(text_content) = content.as_text() {
            &text_content.text
        } else {
            panic!("Expected text content")
        }
    } else {
        panic!("No content in result");
    };

    // Should have at least one buffer (the initial empty buffer)
    assert!(
        content_text.contains("Buffer"),
        "Buffer list should contain buffer info: {content_text:?}"
    );

    cleanup_nvim_process(child);
}

#[tokio::test]
#[traced_test]
async fn test_lua_execution() {
    let port = PORT_BASE + 3;

    let (server, child) = {
        let _guard = NEOVIM_TEST_MUTEX.lock().unwrap();
        drop(_guard);
        setup_connected_server(port).await
    };

    // Test successful Lua execution
    let result = server
        .exec_lua(Parameters(ExecuteLuaRequest {
            code: "return 42".to_string(),
        }))
        .await;
    assert!(result.is_ok(), "Failed to execute Lua: {result:?}");

    let result = result.unwrap();
    assert!(!result.content.is_empty());

    let content_text = if let Some(content) = result.content.first() {
        if let Some(text_content) = content.as_text() {
            &text_content.text
        } else {
            panic!("Expected text content")
        }
    } else {
        panic!("No content in result");
    };

    assert!(
        content_text.contains("42"),
        "Lua result should contain 42: {content_text:?}"
    );

    // Test Lua execution with string result
    let result = server
        .exec_lua(Parameters(ExecuteLuaRequest {
            code: "return 'hello world'".to_string(),
        }))
        .await;
    assert!(result.is_ok(), "Failed to execute Lua: {result:?}");

    // Test error handling for invalid Lua
    let result = server
        .exec_lua(Parameters(ExecuteLuaRequest {
            code: "invalid lua syntax !!!".to_string(),
        }))
        .await;
    assert!(result.is_err(), "Should fail for invalid Lua syntax");

    // Test error handling for empty code
    let result = server
        .exec_lua(Parameters(ExecuteLuaRequest {
            code: "".to_string(),
        }))
        .await;
    assert!(result.is_err(), "Should fail for empty Lua code");

    cleanup_nvim_process(child);
}

#[tokio::test]
#[traced_test]
async fn test_error_handling() {
    let server = NeovimMcpServer::new();

    // Test operations without connection
    let result = server.list_buffers().await;
    assert!(
        result.is_err(),
        "list_buffers should fail when not connected"
    );

    let result = server
        .exec_lua(Parameters(ExecuteLuaRequest {
            code: "return 1".to_string(),
        }))
        .await;
    assert!(result.is_err(), "exec_lua should fail when not connected");

    let result = server.disconnect_nvim_tcp().await;
    assert!(result.is_err(), "disconnect should fail when not connected");
}

#[tokio::test]
#[traced_test]
async fn test_server_info() {
    let server = NeovimMcpServer::new();
    let info = server.get_info();

    // Verify server information
    assert!(info.instructions.is_some());
    assert!(info.capabilities.tools.is_some());

    let instructions = info.instructions.unwrap();
    assert!(instructions.contains("Neovim"));
    assert!(instructions.contains("TCP"));
}

#[tokio::test]
#[traced_test]
async fn test_connection_constraint() {
    let port = PORT_BASE + 2;

    let child = {
        let _guard = NEOVIM_TEST_MUTEX.lock().unwrap();
        drop(_guard);
        setup_neovim_instance(port).await
    };
    let server = NeovimMcpServer::new();
    let address = format!("{HOST}:{port}");

    // Connect to instance
    let result = server
        .connect_nvim_tcp(Parameters(ConnectNvimTCPRequest {
            address: address.clone(),
        }))
        .await;
    assert!(result.is_ok(), "Failed to connect to instance");

    // Try to connect again (should fail)
    let result = server
        .connect_nvim_tcp(Parameters(ConnectNvimTCPRequest {
            address: address.clone(),
        }))
        .await;
    assert!(result.is_err(), "Should not be able to connect twice");

    // Disconnect and then connect again (should work)
    let result = server.disconnect_nvim_tcp().await;
    assert!(result.is_ok(), "Failed to disconnect from instance");

    let result = server
        .connect_nvim_tcp(Parameters(ConnectNvimTCPRequest {
            address: address.clone(),
        }))
        .await;
    assert!(result.is_ok(), "Failed to reconnect after disconnect");

    cleanup_nvim_process(child);
}
