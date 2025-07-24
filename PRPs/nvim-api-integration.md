+++
title: Neovim API Integration PRP - TCP Client with MCP Server Tools

description: 
  ## Purpose

  Implement a comprehensive Neovim API integration through a TCP client that
  provides MCP server tools for seamless editor interaction. This PRP provides
  complete context for one-pass implementation success.

  ## Core Principles

  1. **Context is King**: Include ALL nvim-rs patterns, MCP server architecture,
     and error handling approaches
  2. **Validation Loops**: Executable tests with real Neovim instances
  3. **Information Dense**: Leverage existing codebase patterns and conventions
  4. **Progressive Success**: TCP connection → API tools → comprehensive testing
  5. **Global rules**: Follow all rules in CLAUDE.md (no co-sign, markdownlint)

+++

## Goal

Build a production-ready Neovim API integration that enables seamless
communication with Neovim instances through TCP connections, exposing key
functionality through MCP server tools for buffer management and Lua execution.

## Why

- **Editor Integration**: Enable MCP clients to interact directly with Neovim
  instances for enhanced development workflows
- **API Expansion**: Provide foundation for future Neovim API feature additions
- **Developer Productivity**: Allow programmatic access to Neovim's powerful
  buffer and Lua execution capabilities
- **Ecosystem Bridge**: Connect MCP protocol with Neovim's rich plugin ecosystem

## What

A Neovim TCP client implementation integrated as MCP server tools that provides:

### Core Functionality

- **Single Connection Management**: Connect to one Neovim instance at a time
  via TCP
- **Buffer Operations**: List and inspect open buffers using `nvim_list_bufs`
- **Lua Execution**: Execute custom Lua scripts with `nvim_exec_lua`
- **Connection Lifecycle**: Robust connect/disconnect operations with cleanup

### MCP Server Tools

- `connect_nvim_tcp` - Establish TCP connection to Neovim instance
- `disconnect_nvim_tcp` - Cleanly disconnect from active Neovim instance
- `list_buffers` - Retrieve information about all open buffers
- `exec_lua` - Execute Lua code within Neovim context with argument support

### Success Criteria

- [ ] Successfully connect to Neovim instance via TCP address
- [ ] Retrieve buffer list with accurate buffer information
- [ ] Execute Lua code and return results through MCP protocol
- [ ] Handle connection failures and network issues gracefully
- [ ] Maintain single connection constraint (disconnect before new connection)
- [ ] Pass all integration tests with real Neovim instances
- [ ] Achieve zero compilation warnings (clippy clean)
- [ ] Complete test coverage for all tool functions

## All Needed Context

### Documentation & References

```yaml
# MUST READ - Include these in your context window
- url: https://docs.rs/nvim-rs/latest/nvim_rs/create/tokio/fn.new_tcp.html
  why: TCP connection creation with nvim-rs, Handler trait requirements
  
- url: https://docs.rs/nvim-rs/latest/nvim_rs/neovim/struct.Neovim.html
  why: Core API methods for buffer operations and Lua execution
  
- file: src/server/counter.rs
  why: MCP server implementation pattern with tool_router macro
  
- file: src/server/handler.rs
  why: ServerHandler trait implementation and capabilities setup
  
- file: tests/integration.rs
  why: Async testing patterns, concurrent operation testing
  
- file: examples/client.rs
  why: Child process client pattern for testing MCP servers

- doc: https://neovim.io/doc/user/api.html#nvim_exec_lua()
  section: API function specifications and parameter requirements
  critical: Lua code validation and security considerations

- file: PRPs/nvim-api-integration.STARTER.md
  why: Example code patterns, integration test setup, dependency specs
```

### Current Codebase Tree

```bash
.
├── tests/
│   └── integration.rs          # Async testing patterns
├── examples/
│   └── client.rs              # MCP client example
├── PRPs/
│   ├── nvim-api-integration.STARTER.md  # Reference implementation
│   └── nvim-api-integration.md          # This PRP
├── src/
│   ├── main.rs               # Entry point with STDIO transport
│   └── server/
│       ├── counter.rs        # MCP server implementation pattern
│       └── handler.rs        # ServerHandler trait implementation
└── Cargo.toml               # Dependencies and features
```

### Desired Codebase Tree

Files to be added and responsibility:

```bash
src/
├── server/
│   ├── neovim.rs            # NeovimMcpServer implementation with tools
│   └── mod.rs               # Module exports for neovim server
└── neovim/
    ├── client.rs            # TCP client wrapper and connection management
    ├── connection.rs        # Connection state and lifecycle management
    ├── error.rs             # Neovim-specific error types
    └── mod.rs               # Module exports for neovim client

tests/
└── neovim_integration.rs    # Integration tests with real Neovim instances
```

### Known Gotchas of Codebase & Library Quirks

```rust
// CRITICAL: nvim-rs requires specific features in Cargo.toml
// Add: nvim-rs = { version = "0.9.2", features = ["use_tokio"] }

// CRITICAL: TCP connection returns tuple (Neovim, JoinHandle)
// JoinHandle MUST be stored and awaited for proper cleanup
let (nvim, io_handler) = create::new_tcp(addr, handler).await?;

// CRITICAL: Handler trait requires specific Writer type
// Use Compat<TokioTcpStream> for TCP connections
impl Handler for NeovimHandler {
    type Writer = Compat<TokioTcpStream>;
    // ...
}

// CRITICAL: MCP server state must be Arc<Mutex<T>> for thread safety
// Follow existing pattern from counter.rs
connection: Arc<Mutex<Option<NeovimConnection>>>,

// CRITICAL: Only one connection allowed at a time per PRP requirements
// Must disconnect existing connection before new connection

// GOTCHA: Neovim instance spawning for tests requires --headless --listen
// Command::new("nvim").args(&["-u", "NONE", "--headless", "--listen", addr])

// GOTCHA: TCP connection may take time to establish
// Use retry loop with timeout for connection establishment in tests

// CRITICAL: Lua code execution requires validation for security
// Sanitize input and handle execution errors gracefully

// PATTERN: All MCP tools return Result<CallToolResult, McpError>
// Use CallToolResult::success() for successful responses
```

## Implementation Blueprint

### Data Models and Structure

Create core data models for type safety and connection management:

```rust
// Connection state management
#[derive(Clone)]
pub struct NeovimMcpServer {
    connection: Arc<Mutex<Option<NeovimConnection>>>,
    pub tool_router: ToolRouter<Self>,
}

struct NeovimConnection {
    nvim: Neovim<Compat<TokioTcpStream>>,
    _io_handler: JoinHandle<Result<Result<(), Box<LoopError>>, JoinError>>,
    address: String,
}

// Error types for comprehensive error handling
#[derive(Debug, thiserror::Error)]
pub enum NeovimError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("No active connection")]
    NotConnected,
    #[error("Already connected to {0}")]
    AlreadyConnected(String),
}

// Handler for nvim-rs TCP connection
#[derive(Clone)]
struct NeovimHandler;

impl Handler for NeovimHandler {
    type Writer = Compat<TokioTcpStream>;
    
    async fn handle_request(
        &self,
        name: String,
        _args: Vec<Value>,
        _neovim: Neovim<Compat<TokioTcpStream>>,
    ) -> Result<Value, Value> {
        match name.as_ref() {
            "ping" => Ok(Value::from("pong")),
            _ => Ok(Value::Nil),
        }
    }
}
```

### List of Tasks to be Completed

Tasks to fulfill the PRP in the order they should be completed:

```yaml
Task 1:
CREATE src/neovim/mod.rs:
  - Export client, connection, and error modules
  - Follow existing module pattern from src/server/mod.rs

CREATE src/neovim/error.rs:
  - MIRROR pattern from: existing error handling with thiserror
  - Define NeovimError enum with Connection, Api, NotConnected,
    AlreadyConnected variants
  - Implement From traits for nvim-rs errors and std::io::Error

Task 2:
CREATE src/neovim/connection.rs:
  - Define NeovimConnection struct with nvim instance and io_handler
  - Implement connection lifecycle methods (connect, disconnect, is_connected)
  - CRITICAL: Store JoinHandle for proper cleanup

CREATE src/neovim/client.rs:
  - Define NeovimHandler implementing Handler trait
  - PATTERN: Use Compat<TokioTcpStream> as Writer type
  - Implement basic request handling (ping/pong pattern)

Task 3:
MODIFY Cargo.toml:
  - ADD dependency: nvim-rs = { version = "0.9.2", features = ["use_tokio"] }
  - ADD dependency: rmpv = "1.0" for MessagePack Value handling
  - PRESERVE existing dependencies and structure

Task 4:
CREATE src/server/neovim.rs:
  - MIRROR pattern from: src/server/counter.rs
  - Define NeovimMcpServer with Arc<Mutex<Option<NeovimConnection>>>
  - Implement #[tool_router] macro for MCP tool registration
  - PRESERVE existing error handling patterns

Task 5:
IMPLEMENT MCP tools in src/server/neovim.rs:
  - connect_nvim_tcp: Establish TCP connection with address validation
  - disconnect_nvim_tcp: Clean disconnection with io_handler cleanup
  - list_buffers: Use nvim.list_bufs() API call
  - exec_lua: Use nvim.exec_lua() with code and args parameters
  - PATTERN: All tools return Result<CallToolResult, McpError>

Task 6:
IMPLEMENT ServerHandler trait for NeovimMcpServer:
  - MIRROR pattern from: src/server/handler.rs
  - Set server info with appropriate instructions
  - Enable tools capability in ServerCapabilities
  - Use #[tool_handler] macro

Task 7:
MODIFY src/server/mod.rs:
  - ADD export: pub mod neovim;
  - PRESERVE existing exports

Task 8:
CREATE tests/neovim_integration.rs:
  - MIRROR pattern from: tests/integration.rs
  - Implement tests with real Neovim instance spawning
  - Test connection lifecycle and all MCP tools
  - PATTERN: Use #[tokio::test] for async tests

Task 9:
MODIFY src/main.rs:
  - ADD command-line option for neovim server mode
  - PRESERVE existing counter server functionality
  - Follow existing STDIO transport pattern
```

### Per Task Pseudocode

```rust
// Task 4-5: NeovimMcpServer implementation
#[derive(Clone)]
pub struct NeovimMcpServer {
    connection: Arc<Mutex<Option<NeovimConnection>>>,
    pub tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NeovimMcpServer {
    pub fn new() -> Self {
        Self {
            connection: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Connect to Neovim instance via TCP")]
    pub async fn connect_nvim_tcp(
        &self,
        address: String,
    ) -> Result<CallToolResult, McpError> {
        let mut conn_guard = self.connection.lock().await;
        
        // CRITICAL: Check existing connection
        if conn_guard.is_some() {
            return Err(McpError::InvalidRequest(
                "Already connected".to_string()));
        }
        
        // PATTERN: Create connection with error handling
        let handler = NeovimHandler;
        match create::new_tcp(&address, handler).await {
            Ok((nvim, io_handler)) => {
                *conn_guard = Some(NeovimConnection {
                    nvim,
                    _io_handler: tokio::spawn(io_handler),
                    address: address.clone(),
                });
                Ok(CallToolResult::success(vec![
                    Content::text(
                        format!("Connected to Neovim at {}", address))
                ]))
            }
            Err(e) => Err(McpError::InternalError(
                format!("Connection failed: {}", e)))
        }
    }

    #[tool(description = "Execute Lua code in Neovim")]
    pub async fn exec_lua(
        &self,
        code: String,
        args: Option<Vec<Value>>,
    ) -> Result<CallToolResult, McpError> {
        let conn_guard = self.connection.lock().await;
        let conn = conn_guard.as_ref()
            .ok_or_else(|| McpError::InvalidRequest(
                "Not connected".to_string()))?;
        
        // CRITICAL: Validate Lua code for security
        if code.trim().is_empty() {
            return Err(McpError::InvalidRequest("Empty Lua code".to_string()));
        }
        
        // PATTERN: Execute with proper error handling
        let lua_args = args.unwrap_or_default();
        match conn.nvim.exec_lua(&code, lua_args).await {
            Ok(result) => Ok(CallToolResult::success(vec![
                Content::text(format!("Lua result: {:?}", result))
            ])),
            Err(e) => Err(McpError::InternalError(
                format!("Lua execution failed: {}", e)))
        }
    }
}
```

### Integration Points

```yaml
DEPENDENCIES:
  - add to: Cargo.toml
  - pattern: 'nvim-rs = { version = "0.9.2", features = ["use_tokio"] }'
  - pattern: 'rmpv = "1.0"'
  
MODULES:
  - add to: src/server/mod.rs
  - pattern: "pub mod neovim;"
  
MAIN:
  - modify: src/main.rs
  - pattern: "Add CLI option for neovim server selection"
  
TESTS:
  - create: tests/neovim_integration.rs
  - pattern: "Spawn real Neovim instance for testing"
```

## Validation Loop

### Level 1: Syntax & Style

```bash
# Run these FIRST - fix any errors before proceeding
cargo check                           # Compilation validation
cargo clippy -- -D warnings         # Linting with no warnings allowed
cargo fmt --check                    # Code formatting validation

# Expected: No errors or warnings. If errors, READ and fix immediately.
```

### Level 2: Unit Tests

Each new module should follow existing test patterns:

```rust
// CREATE tests/neovim_integration.rs with these test cases:
#[tokio::test]
async fn test_connection_lifecycle() {
    let server = NeovimMcpServer::new();
    
    // Test connection
    let result = server.connect_nvim_tcp("127.0.0.1:6666".to_string()).await;
    assert!(result.is_ok());
    
    // Test disconnect
    let result = server.disconnect_nvim_tcp().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_buffer_operations() {
    // Spawn Neovim instance
    let mut child = Command::new("nvim")
        .args(&["-u", "NONE", "--headless", "--listen", "127.0.0.1:6667"])
        .spawn()
        .expect("Failed to start Neovim");
    
    // Wait for startup
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let server = NeovimMcpServer::new();
    server.connect_nvim_tcp("127.0.0.1:6667".to_string()).await.unwrap();
    
    // Test buffer listing
    let result = server.list_buffers().await.unwrap();
    assert!(!result.content.is_empty());
    
    child.kill().expect("Failed to kill Neovim");
}

#[tokio::test]
async fn test_lua_execution() {
    // Test basic Lua execution
    let server = setup_connected_server().await;
    
    let result = server.exec_lua("return 2 + 2".to_string(), None).await.unwrap();
    assert!(result.content[0].text.contains("4"));
}

#[tokio::test]
async fn test_error_handling() {
    let server = NeovimMcpServer::new();
    
    // Test operations without connection
    let result = server.list_buffers().await;
    assert!(result.is_err());
    
    // Test invalid connection
    let result = server.connect_nvim_tcp("invalid:address".to_string()).await;
    assert!(result.is_err());
}
```

```bash
# Run and iterate until passing:
cargo test neovim_integration -v
# If failing: Read error, understand root cause, fix code, re-run
```

### Level 3: Integration Test

```bash
# Test with real Neovim instance
nvim -u NONE --headless --listen 127.0.0.1:6666 &
NVIM_PID=$!

# Run the server
cargo run --bin nvim-mcp &
SERVER_PID=$!

# Test MCP tools via client
cargo run --example client

# Cleanup
kill $NVIM_PID $SERVER_PID

# Expected: All MCP tools work correctly with real Neovim instance
```

## Final Validation Checklist

- [ ] All tests pass: `cargo test -v`
- [ ] No linting errors: `cargo clippy -- -D warnings`
- [ ] No formatting issues: `cargo fmt --check`
- [ ] Manual test with real Neovim: Connection and tool execution works
- [ ] Error cases handled gracefully: Invalid addresses, missing connections
- [ ] Connection cleanup verified: No resource leaks
- [ ] Security validation: Lua code execution is safe
- [ ] Single connection constraint enforced

---

## Anti-Patterns to Avoid

- ❌ Don't create multiple concurrent connections (violates PRP requirement)
- ❌ Don't skip JoinHandle cleanup (causes resource leaks)
- ❌ Don't ignore nvim-rs connection errors (leads to undefined behavior)
- ❌ Don't execute untrusted Lua code without validation
- ❌ Don't use sync operations in async context
- ❌ Don't hardcode timeouts or addresses in production code
- ❌ Don't mock Neovim in integration tests (use real instances)

## Confidence Score: 9/10

This PRP provides comprehensive context including:
✅ Complete nvim-rs API patterns and gotchas
✅ Existing MCP server architecture to follow
✅ Detailed error handling and validation approaches  
✅ Real-world testing with Neovim instances
✅ Security considerations for Lua execution
✅ Progressive implementation with validation loops

The only uncertainty (−1 point) is potential version compatibility issues
between nvim-rs and the target Neovim version, but comprehensive testing will
catch and resolve these.
