use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::time::sleep;
use tracing_test::traced_test;

use crate::neovim::NeovimClient;

const HOST: &str = "127.0.0.1";
const PORT_BASE: u16 = 7777;

// Global mutex to prevent concurrent Neovim instances from using the same port
static NEOVIM_TEST_MUTEX: Mutex<()> = Mutex::new(());

fn nvim_path() -> &'static str {
    "nvim"
}

async fn setup_neovim_instance_advance(
    port: u16,
    cfg_path: &str,
    open_file: &str,
) -> std::process::Child {
    let listen = format!("{HOST}:{port}");

    let mut child = Command::new(nvim_path())
        .args(["-u", cfg_path, "--headless", "--listen", &listen])
        .args(
            (!open_file.is_empty())
                .then_some(vec![open_file])
                .unwrap_or_default(),
        )
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

async fn setup_neovim_instance(port: u16) -> std::process::Child {
    setup_neovim_instance_advance(port, "NONE", "").await
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

async fn setup_connected_client(port: u16) -> (NeovimClient, std::process::Child) {
    let child = setup_neovim_instance(port).await;
    let mut client = NeovimClient::new();
    let address = format!("{HOST}:{port}");

    let result = client.connect(&address).await;
    if result.is_err() {
        cleanup_nvim_process(child);
        panic!("Failed to connect to Neovim: {result:?}");
    }

    (client, child)
}

fn get_testdata_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src/neovim/testdata");
    path.push(filename);
    path
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
    let mut client = NeovimClient::new();

    // Test connection
    let result = client.connect(&address).await;
    assert!(result.is_ok(), "Failed to connect: {result:?}");

    // Test that we can't connect again while already connected
    let result = client.connect(&address).await;
    assert!(result.is_err(), "Should not be able to connect twice");

    // Test disconnect
    let result = client.disconnect().await;
    assert!(result.is_ok(), "Failed to disconnect: {result:?}");

    // Test that disconnect fails when not connected
    let result = client.disconnect().await;
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

    let (client, child) = {
        let _guard = NEOVIM_TEST_MUTEX.lock().unwrap();
        drop(_guard);
        setup_connected_client(port).await
    };

    // Test buffer listing
    let result = client.list_buffers_info().await;
    assert!(result.is_ok(), "Failed to list buffers: {result:?}");

    let buffer_info = result.unwrap();
    assert!(!buffer_info.is_empty());

    // Should have at least one buffer (the initial empty buffer)
    let buffer_info_text = buffer_info.join(", ");
    assert!(
        buffer_info_text.contains("Buffer"),
        "Buffer list should contain buffer info: {buffer_info_text:?}"
    );

    cleanup_nvim_process(child);
}

#[tokio::test]
#[traced_test]
async fn test_lua_execution() {
    let port = PORT_BASE + 3;

    let (client, child) = {
        let _guard = NEOVIM_TEST_MUTEX.lock().unwrap();
        drop(_guard);
        setup_connected_client(port).await
    };

    // Test successful Lua execution
    let result = client.execute_lua("return 42").await;
    assert!(result.is_ok(), "Failed to execute Lua: {result:?}");

    let lua_result = result.unwrap();
    assert!(
        format!("{lua_result:?}").contains("42"),
        "Lua result should contain 42: {lua_result:?}"
    );

    // Test Lua execution with string result
    let result = client.execute_lua("return 'hello world'").await;
    assert!(result.is_ok(), "Failed to execute Lua: {result:?}");

    // Test error handling for invalid Lua
    let result = client.execute_lua("invalid lua syntax !!!").await;
    assert!(result.is_err(), "Should fail for invalid Lua syntax");

    // Test error handling for empty code
    let result = client.execute_lua("").await;
    assert!(result.is_err(), "Should fail for empty Lua code");

    cleanup_nvim_process(child);
}

#[tokio::test]
#[traced_test]
async fn test_error_handling() {
    let client = NeovimClient::new();

    // Test operations without connection
    let result = client.list_buffers_info().await;
    assert!(
        result.is_err(),
        "list_buffers_info should fail when not connected"
    );

    let result = client.execute_lua("return 1").await;
    assert!(
        result.is_err(),
        "execute_lua should fail when not connected"
    );

    let mut client_mut = client;
    let result = client_mut.disconnect().await;
    assert!(result.is_err(), "disconnect should fail when not connected");
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
    let mut client = NeovimClient::new();
    let address = format!("{HOST}:{port}");

    // Connect to instance
    let result = client.connect(&address).await;
    assert!(result.is_ok(), "Failed to connect to instance");

    // Try to connect again (should fail)
    let result = client.connect(&address).await;
    assert!(result.is_err(), "Should not be able to connect twice");

    // Disconnect and then connect again (should work)
    let result = client.disconnect().await;
    assert!(result.is_ok(), "Failed to disconnect from instance");

    let result = client.connect(&address).await;
    assert!(result.is_ok(), "Failed to reconnect after disconnect");

    cleanup_nvim_process(child);
}

#[tokio::test]
#[traced_test]
async fn test_get_vim_diagnostics() {
    let port = PORT_BASE;

    let child = {
        let _guard = NEOVIM_TEST_MUTEX.lock().unwrap();
        drop(_guard);
        setup_neovim_instance_advance(
            port,
            get_testdata_path("cfg_lsp.lua").to_str().unwrap(),
            get_testdata_path("diagnostic_problems.lua")
                .to_str()
                .unwrap(),
        )
        .await
    };
    let mut client = NeovimClient::new();
    let address = format!("{HOST}:{port}");

    // Connect to instance
    let result = client.connect(&address).await;
    assert!(result.is_ok(), "Failed to connect to instance");

    // Set up diagnostics and get diagnostics for buffer 0
    let result = client.setup_diagnostics_changed_autocmd().await;
    assert!(
        result.is_ok(),
        "Failed to setup diagnostics autocmd: {result:?}"
    );

    sleep(Duration::from_secs(20)).await; // Allow time for LSP to initialize

    let result = client.get_buffer_diagnostics(0).await;
    assert!(result.is_ok(), "Failed to get diagnostics: {result:?}");

    cleanup_nvim_process(child);
}
