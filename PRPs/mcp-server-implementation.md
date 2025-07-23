# MCP Server Implementation

## Goal

Build a stdio MCP (Model Context Protocol) server using the `rmcp` crate that
provides a foundation for MCP implementation without built-in tools, allowing
for custom tool integration as needed. The server should demonstrate a simple
counter tool implementation and be ready for extension with additional tools.

## Why

- **Protocol Standardization**: Implement MCP to enable seamless integration
  between AI applications and external services following the official
  specification
- **Neovim Integration**: Create a foundation for building MCP-powered
  Neovim tools and extensions
- **Extensibility**: Provide a clean, documented codebase that can be
  extended with domain-specific tools
- **Educational Value**: Serve as a reference implementation for Rust-based MCP servers

## What

A complete MCP server implementation that:

- Implements the MCP protocol using stdio transport
- Provides a simple counter tool for demonstration
- Includes proper error handling and logging
- Is testable and follows Rust best practices
- Can be easily extended with additional tools

### Success Criteria

- [ ] Server successfully implements MCP protocol via stdio
- [ ] Counter tool with increment/get operations works correctly
- [ ] Server can be tested with MCP client implementations
- [ ] Code passes all linting and type checking
- [ ] Integration test demonstrates end-to-end functionality
- [ ] Documentation explains how to extend with new tools

## All Needed Context

### Documentation & References

```yaml
# MUST READ - Include these in your context window
- url: https://docs.rs/rmcp/latest/rmcp/
  why: Complete API reference for rmcp crate usage, traits, and patterns
  critical: Understanding tool_router macro and ServerHandler trait implementation

- url: https://modelcontextprotocol.io/specification
  why: Official MCP protocol specification for proper implementation
  critical: JSON-RPC 2.0 requirements, server capabilities, tool definitions

- file: PRPs/mcp-server-implementation.STARTER.md
  why: Contains comprehensive implementation examples and patterns to follow
  critical: Counter tool implementation, async patterns, error handling

- url: https://docs.rs/tracing/latest/tracing/
  why: Structured logging patterns for production-ready server
  section: Instrumentation and span creation for async functions
```

### Current Codebase Tree

```bash
nvim-mcp/
├── Cargo.toml              # Basic Rust project configuration
├── src/
│   └── main.rs            # Simple "Hello, world!" placeholder
├── PRPs/
│   └── mcp-server-implementation.STARTER.md  # Implementation examples
└── README.md              # Minimal project description
```

### Desired Codebase Tree

```bash
nvim-mcp/
├── Cargo.toml              # Updated with rmcp, tracing dependencies
├── src/
│   ├── main.rs            # Server entry point with stdio transport
│   ├── server/
│   │   ├── mod.rs         # Server module declaration
│   │   ├── counter.rs     # Counter tool implementation
│   │   └── handler.rs     # Server handler implementation
│   └── lib.rs             # Library module for reusable components
├── examples/
│   └── client.rs          # Example MCP client for testing
├── tests/
│   └── integration.rs     # Integration tests
└── PRPs/                  # Project requirements and documentation
```

### Known Gotchas & Library Quirks

```rust
// CRITICAL: rmcp requires specific async patterns
// - All tool methods must be async and return Result<CallToolResult, McpError>
// - #[tool_router] macro must be applied to impl blocks
// - ServerHandler trait must be implemented for server info

// GOTCHA: stdio transport requires specific setup
// - Server communicates via stdin/stdout only
// - No direct debugging output to console during operation
// - Use structured logging to files for debugging

// PATTERN: Tool definitions require specific attributes
// - #[tool(description = "...")] for tool documentation
// - CallToolResult::success(vec![Content::text(result)]) for responses
// - Proper error propagation using McpError type

// CRITICAL: Testing MCP servers requires child process spawning
// - Servers run as separate processes communicating via stdio
// - Client tests must use TokioChildProcess transport
// - Integration tests need Command::new setup
```

## Implementation Blueprint

### Data Models and Structure

Core data structures follow rmcp patterns:

```rust
// Server state with Arc<Mutex<T>> for async safety
#[derive(Clone)]
pub struct CounterServer {
    counter: Arc<Mutex<i32>>,
    tool_router: ToolRouter<Self>,
}

// Tool responses use rmcp types
// - CallToolResult for successful responses
// - McpError for error cases
// - Content::text() for string responses
```

### List of Tasks to be Completed

```yaml
Task 1:
MODIFY Cargo.toml:
  - ADD dependencies: rmcp = "0.3.0", tokio, tracing, tracing-subscriber
  - SET edition = "2024" and update package metadata
  - ENSURE async runtime support

Task 2:
CREATE src/lib.rs:
  - DEFINE public modules for reusable components
  - EXPORT server types and traits for testing
  - ESTABLISH error handling patterns

Task 3:
CREATE src/server/mod.rs:
  - DECLARE counter and handler submodules
  - EXPORT CounterServer and related types
  - FOLLOW standard Rust module patterns

Task 4:
CREATE src/server/counter.rs:
  - IMPLEMENT CounterServer with Arc<Mutex<i32>> state
  - ADD #[tool_router] implementation with increment/get tools
  - MIRROR pattern from STARTER.md examples
  - INCLUDE proper async error handling

Task 5:
CREATE src/server/handler.rs:
  - IMPLEMENT ServerHandler trait for CounterServer
  - DEFINE server capabilities and information
  - SET up tool routing integration

Task 6:
MODIFY src/main.rs:
  - REPLACE placeholder with full server implementation
  - SET up tracing/logging initialization
  - IMPLEMENT stdio transport setup
  - ADD graceful shutdown handling

Task 7:
CREATE examples/client.rs:
  - IMPLEMENT MCP client for testing server
  - MIRROR client patterns from STARTER.md
  - DEMONSTRATE tool calling and server interaction

Task 8:
CREATE tests/integration.rs:
  - SET up child process testing for server
  - TEST tool calling via MCP client
  - VERIFY error handling and edge cases
```

### Per Task Pseudocode

```rust
// Task 4: Counter Server Implementation
#[derive(Clone)]
pub struct CounterServer {
    counter: Arc<Mutex<i32>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CounterServer {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Increment the counter by 1")]
    async fn increment(&self) -> Result<CallToolResult, McpError> {
        // PATTERN: Lock mutex, modify state, return text result
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(CallToolResult::success(vec![Content::text(counter.to_string())]))
    }

    #[tool(description = "Get the current counter value")]
    async fn get(&self) -> Result<CallToolResult, McpError> {
        // PATTERN: Read-only access, return current state
        let counter = self.counter.lock().await;
        Ok(CallToolResult::success(vec![Content::text(counter.to_string())]))
    }
}
```

### Integration Points

```yaml
DEPENDENCIES:
  - add to: Cargo.toml
  - pattern: |
      rmcp = "0.3.0"
      tokio = { version = "1.0", features = ["full"] }
      tracing = "0.1.41"
      tracing-subscriber = { version = "0.3.19", features = ["env-filter"] }

TRANSPORT:
  - protocol: stdio (standard input/output)
  - pattern: "service.serve(stdio()).await"
  - critical: No console output during operation

LOGGING:
  - setup: tracing_subscriber with env filter
  - pattern: Use #[tracing::instrument] for async functions
  - output: Structured logs to stderr or files
```

## Validation Loop

### Level 1: Syntax & Style

```bash
# Run these FIRST - fix any errors before proceeding
cargo check                    # Basic compilation check
cargo clippy -- -D warnings   # Linting with error promotion
cargo fmt                     # Code formatting

# Expected: No errors or warnings
```

### Level 2: Unit Tests

```rust
// CREATE tests/integration.rs with comprehensive test cases:
#[tokio::test]
async fn test_counter_increment() {
    let server = CounterServer::new();
    let result = server.increment().await.unwrap();
    // Verify result contains "1"
}

#[tokio::test]
async fn test_counter_get() {
    let server = CounterServer::new();
    let result = server.get().await.unwrap();
    // Verify result contains "0"
}

#[tokio::test]
async fn test_server_info() {
    let server = CounterServer::new();
    let info = server.get_info();
    // Verify capabilities include tools
}
```

```bash
# Run and iterate until passing:
cargo test
# If failing: Read error, understand root cause, fix code, re-run
```

### Level 3: Integration Test

```bash
# Build the server
cargo build --bin nvim-mcp

# Test via example client
cargo run --example client

# Expected output showing:
# - Server connection successful
# - Available tools listed (increment, get)
# - Tool calls returning expected results
```

## Final Validation Checklist

- [ ] All tests pass: `cargo test`
- [ ] No linting errors: `cargo clippy -- -D warnings`
- [ ] Code is formatted: `cargo fmt --check`
- [ ] Server builds successfully: `cargo build`
- [ ] Example client can connect and call tools
- [ ] Server responds correctly to MCP protocol messages
- [ ] Error cases handled gracefully with proper MCP error responses
- [ ] Structured logging works without interfering with stdio transport

---

## Anti-Patterns to Avoid

- ❌ Don't use println! or eprintln! in server code (interferes with stdio)
- ❌ Don't skip async/await in tool implementations
- ❌ Don't ignore mutex lock errors - handle them properly
- ❌ Don't hardcode server capabilities - use builder pattern
- ❌ Don't forget to export types from lib.rs for testing
- ❌ Don't use blocking operations in async contexts

**PRP Confidence Score: 9/10** - Comprehensive context, clear
implementation path, executable validation gates, and thorough documentation
references for one-pass implementation success.
