# Neovim API integration Blueprint

## Feature

Integrate Neovim's powerful API through a TCP client that enables seamless
communication with Neovim instances. This implementation provides a robust
foundation for building editor-aware tools that can interact with Neovim's
rich feature set.

**Core API Features:**

- **Buffer Management**: List and manipulate buffers using `nvim_list_bufs`
- **Lua Execution**: Execute Lua code directly in Neovim with `nvim_exec_lua`
- **Extensible Architecture**: Designed to easily support additional Neovim API features

**MCP Server Tools:**

Transform these API capabilities into actionable MCP server tools:

- `list_buffers` - Retrieve and inspect open buffers
- `exec_lua` - Execute custom Lua scripts within Neovim context

## Examples

### Basic Neovim TCP Client Implementation

```toml
[dependencies]
nvim-rs = { version = "0.9.2", feature = ["use_tokio"]}
```

```rust
//! A basic example. Mainly for use in a test, but also shows off some basic
//! functionality.
use std::{env, error::Error, fs};


use rmpv::Value;

use tokio::fs::File as TokioFile;

use nvim_rs::{
  compat::tokio::Compat, create::tokio as create, rpc::IntoVal, Handler, Neovim,
};

#[derive(Clone)]
struct NeovimHandler {}

impl Handler for NeovimHandler {
  type Writer = Compat<TokioFile>;

  async fn handle_request(
    &self,
    name: String,
    _args: Vec<Value>,
    _neovim: Neovim<Compat<TokioFile>>,
  ) -> Result<Value, Value> {
    match name.as_ref() {
      "ping" => Ok(Value::from("pong")),
      _ => unimplemented!(),
    }
  }
}

#[tokio::main]
async fn main() {
  let handler: NeovimHandler = NeovimHandler {};
  let (nvim, io_handler) = create::new_tcp("127.0.0.1:6666", handler).await.unwrap();
  let curbuf = nvim.get_current_buf().await.unwrap();

  let mut envargs = env::args();
  let _ = envargs.next();
  let testfile = envargs.next().unwrap();

  fs::write(testfile, &format!("{:?}", curbuf.into_val())).unwrap();

  // Any error should probably be logged, as stderr is not visible to users.
  match io_handler.await {
    Err(joinerr) => eprintln!("Error joining IO loop: '{}'", joinerr),
    Ok(Err(err)) => {
      if !err.is_reader_error() {
        // One last try, since there wasn't an error with writing to the
        // stream
        nvim
          .err_writeln(&format!("Error: '{}'", err))
          .await
          .unwrap_or_else(|e| {
            // We could inspect this error to see what was happening, and
            // maybe retry, but at this point it's probably best
            // to assume the worst and print a friendly and
            // supportive message to our users
            eprintln!("Well, dang... '{}'", e);
          });
      }

      if !err.is_channel_closed() {
        // Closed channel usually means neovim quit itself, or this plugin was
        // told to quit by closing the channel, so it's not always an error
        // condition.
        eprintln!("Error: '{}'", err);

        let mut source = err.source();

        while let Some(e) = source {
          eprintln!("Caused by: '{}'", e);
          source = e.source();
        }
      }
    }
    Ok(Ok(())) => {}
  }
}
```

### Simple integration test Example

```rust
const HOST: &str = "127.0.0.1";
const PORT: u16 = 6666;

#[tokio::test]
async fn can_connect_via_tcp() {
  let listen = HOST.to_string() + ":" + &PORT.to_string();

  let mut child = Command::new(nvim_path())
    .args(&["-u", "NONE", "--headless", "--listen", &listen])
    .spawn()
    .expect("Cannot start neovim");

  // wait at most 1 second for neovim to start and create the tcp socket
  let start = Instant::now();

  let (nvim, _io_handle) = loop {
    sleep(Duration::from_millis(100));

    let handler = DummyHandler::new();
    if let Ok(r) = create::new_tcp(&listen, handler).await {
      break r;
    } else {
      if Duration::from_secs(1) <= start.elapsed() {
        panic!("Unable to connect to neovim via tcp at {}", listen);
      }
    }
  };

  let servername = nvim
    .get_vvar("servername")
    .await
    .expect("Error retrieving servername from neovim");

  child.kill().expect("Could not kill neovim");

  assert_eq!(&listen, servername.as_str().unwrap());
}
```

## Documentation

- [nvim-rs TCP Connection Guide](https://docs.rs/nvim-rs/latest/nvim_rs/create/tokio/fn.new_tcp.html)
  \- Comprehensive guide for establishing TCP connections to Neovim
- [nvim-rs Lua Execution Reference](https://docs.rs/nvim-rs/latest/nvim_rs/neovim/struct.Neovim.html#method.exec_lua)
  \- Detailed documentation for executing Lua code through the API
- [Neovim API Reference](<https://neovim.io/doc/user/api.html#nvim_exec_lua()>) -
  Official Neovim API documentation with complete function specifications

## Other Considerations

- **Connection Management**: Implement robust connection handling with automatic
  reconnection and graceful degradation
- **Error Handling**: Provide comprehensive error handling for network failures,
  API errors, and protocol-level issues
- **Security**: Validate and sanitize Lua code execution to prevent potential
  security risks
- **Testing**: Use integration tests with actual Neovim instances to ensure API
  compatibility across versions
- **Monitoring**: Add logging and metrics to track API usage and connection
  health
