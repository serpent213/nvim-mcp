use nvim_mcp::NeovimMcpServer;
use rmcp::ServerHandler;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing_test::traced_test;

const HOST: &str = "127.0.0.1";
const PORT_BASE: u16 = 6666;

fn nvim_path() -> &'static str {
    "nvim"
}

async fn setup_neovim_instance(port: u16) -> std::process::Child {
    let listen = format!("{}:{}", HOST, port);

    let mut child = Command::new(nvim_path())
        .args(&["-u", "NONE", "--headless", "--listen", &listen])
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

        if start.elapsed() >= Duration::from_secs(3) {
            child.kill().expect("Failed to kill Neovim");
            panic!("Neovim failed to start within 3 seconds at {}", listen);
        }
    }

    child
}

async fn setup_connected_server(port: u16) -> (NeovimMcpServer, std::process::Child) {
    let mut child = setup_neovim_instance(port).await;
    let server = NeovimMcpServer::new();

    // Note: Current implementation connects to hardcoded 127.0.0.1:6666
    // For tests to work properly, we need to use port 6666
    if port != 6666 {
        child.kill().expect("Failed to kill Neovim");
        panic!("Current implementation only supports connecting to 127.0.0.1:6666");
    }

    let result = server.connect_nvim_tcp().await;
    if result.is_err() {
        child.kill().expect("Failed to kill Neovim");
        panic!("Failed to connect to Neovim: {:?}", result);
    }

    (server, child)
}

#[tokio::test]
#[traced_test]
async fn test_connection_lifecycle() {
    let port = PORT_BASE;
    let mut child = setup_neovim_instance(port).await;
    let server = NeovimMcpServer::new();

    // Test connection
    let result = server.connect_nvim_tcp().await;
    assert!(result.is_ok(), "Failed to connect: {:?}", result);

    // Test that we can't connect again while already connected
    let result = server.connect_nvim_tcp().await;
    assert!(result.is_err(), "Should not be able to connect twice");

    // Test disconnect
    let result = server.disconnect_nvim_tcp().await;
    assert!(result.is_ok(), "Failed to disconnect: {:?}", result);

    // Test that disconnect fails when not connected
    let result = server.disconnect_nvim_tcp().await;
    assert!(
        result.is_err(),
        "Should not be able to disconnect when not connected"
    );

    child.kill().expect("Failed to kill Neovim");
}

#[tokio::test]
#[traced_test]
async fn test_buffer_operations() {
    let port = PORT_BASE + 1;
    let (server, mut child) = setup_connected_server(port).await;

    // Test buffer listing
    let result = server.list_buffers().await;
    assert!(result.is_ok(), "Failed to list buffers: {:?}", result);

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
        "Buffer list should contain buffer info: {}",
        content_text
    );

    child.kill().expect("Failed to kill Neovim");
}

// NOTE: exec_lua is currently commented out in implementation
// #[tokio::test]
// #[traced_test]
// async fn test_lua_execution() {
//     // Placeholder for when exec_lua is implemented
// }

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

    // NOTE: exec_lua is currently commented out in implementation
    // let result = server.exec_lua("return 1".to_string(), None).await;
    // assert!(result.is_err(), "exec_lua should fail when not connected");

    let result = server.disconnect_nvim_tcp().await;
    assert!(result.is_err(), "disconnect should fail when not connected");

    // NOTE: Current implementation doesn't take address parameter
    // Test that connection works when Neovim is available (since it connects to hardcoded address)
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
    // NOTE: Current implementation hardcodes connection to 127.0.0.1:6666
    // We can only test the single connection constraint with one instance
    let port = PORT_BASE;
    let mut child = setup_neovim_instance(port).await;
    let server = NeovimMcpServer::new();

    // Connect to instance
    let result = server.connect_nvim_tcp().await;
    assert!(result.is_ok(), "Failed to connect to instance");

    // Try to connect again (should fail)
    let result = server.connect_nvim_tcp().await;
    assert!(
        result.is_err(),
        "Should not be able to connect twice"
    );

    // Disconnect and then connect again (should work)
    let result = server.disconnect_nvim_tcp().await;
    assert!(result.is_ok(), "Failed to disconnect from instance");

    let result = server.connect_nvim_tcp().await;
    assert!(
        result.is_ok(),
        "Failed to reconnect after disconnect"
    );

    child.kill().expect("Failed to kill Neovim");
}
