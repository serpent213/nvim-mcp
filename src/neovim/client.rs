use async_trait::async_trait;
use nvim_rs::{Handler, Neovim, compat::tokio::Compat, create::tokio as create};
use rmpv::Value;
use tokio::net::TcpStream;
use tracing::{debug, info, instrument};

use super::{connection::NeovimConnection, error::NeovimError};

#[derive(Clone)]
pub struct NeovimHandler;

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Compat<tokio::io::WriteHalf<TcpStream>>;

    async fn handle_notify(
        &self,
        name: String,
        args: Vec<Value>,
        _neovim: Neovim<Compat<tokio::io::WriteHalf<TcpStream>>>,
    ) {
        info!("handling notification: {name:?}, {args:?}");
    }

    async fn handle_request(
        &self,
        name: String,
        args: Vec<Value>,
        _neovim: Neovim<Compat<tokio::io::WriteHalf<TcpStream>>>,
    ) -> Result<Value, Value> {
        info!("handling request: {name:?}, {args:?}");
        match name.as_ref() {
            "ping" => Ok(Value::from("pong")),
            _ => Ok(Value::Nil),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Diagnostic {
    pub message: String,
    pub code: Option<String>,
    pub severity: u8,
    pub lnum: u64,
    pub col: u64,
    pub source: String,
    pub bufnr: u64,
    pub end_lnum: u64,
    pub end_col: u64,
    pub namespace: u64,
    pub user_data: serde_json::Value,
}

pub struct NeovimClient {
    connection: Option<NeovimConnection>,
}

impl NeovimClient {
    pub fn new() -> Self {
        Self { connection: None }
    }

    #[instrument(skip(self))]
    pub async fn connect(&mut self, address: &str) -> Result<(), NeovimError> {
        if self.connection.is_some() {
            return Err(NeovimError::Connection(format!(
                "Already connected to {}. Disconnect first.",
                self.connection.as_ref().unwrap().address()
            )));
        }

        debug!("Attempting to connect to Neovim at {}", address);
        let handler = NeovimHandler;
        match create::new_tcp(address, handler).await {
            Ok((nvim, io_handler)) => {
                let connection = NeovimConnection::new(
                    nvim,
                    tokio::spawn(async move {
                        let rv = io_handler.await;
                        info!("io_handler completed with result: {:?}", rv);
                        rv
                    }),
                    address.to_string(),
                );
                self.connection = Some(connection);
                debug!("Successfully connected to Neovim at {}", address);
                Ok(())
            }
            Err(e) => {
                debug!("Failed to connect to Neovim at {}: {}", address, e);
                Err(NeovimError::Connection(format!("Connection failed: {e}")))
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn disconnect(&mut self) -> Result<String, NeovimError> {
        debug!("Attempting to disconnect from Neovim");

        if let Some(connection) = self.connection.take() {
            let address = connection.address().to_string();
            connection.io_handler.abort();
            debug!("Successfully disconnected from Neovim at {}", address);
            Ok(address)
        } else {
            Err(NeovimError::Connection(
                "Not connected to any Neovim instance".to_string(),
            ))
        }
    }

    #[instrument(skip(self))]
    pub async fn list_buffers_info(&self) -> Result<Vec<String>, NeovimError> {
        debug!("Listing buffers");

        let conn = self.connection.as_ref().ok_or_else(|| {
            NeovimError::Connection("Not connected to any Neovim instance".to_string())
        })?;

        match conn.nvim.list_bufs().await {
            Ok(buffers) => {
                let buffer_info: Vec<String> =
                    futures::future::try_join_all(buffers.iter().map(|buf| async {
                        let number = buf
                            .get_number()
                            .await
                            .map_err(|e| format!("Number error: {e}"))?;
                        let name = buf
                            .get_name()
                            .await
                            .unwrap_or_else(|_| "[No Name]".to_string());
                        let lines = buf
                            .line_count()
                            .await
                            .map_err(|e| format!("Lines error: {e}"))?;
                        Ok::<String, String>(format!("Buffer {number}: {name} ({lines} lines)"))
                    }))
                    .await
                    .map_err(|e| NeovimError::Api(format!("Failed to get buffer info: {e}")))?;

                debug!("Found {} buffers", buffer_info.len());
                Ok(buffer_info)
            }
            Err(e) => {
                debug!("Failed to list buffers: {}", e);
                Err(NeovimError::Api(format!("Failed to list buffers: {e}")))
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn execute_lua(&self, code: &str) -> Result<Value, NeovimError> {
        debug!("Executing Lua code: {}", code);

        if code.trim().is_empty() {
            return Err(NeovimError::Api("Lua code cannot be empty".to_string()));
        }

        let conn = self.connection.as_ref().ok_or_else(|| {
            NeovimError::Connection("Not connected to any Neovim instance".to_string())
        })?;

        let lua_args = Vec::<Value>::new();
        match conn.nvim.exec_lua(code, lua_args).await {
            Ok(result) => {
                debug!("Lua execution successful, result: {:?}", result);
                Ok(result)
            }
            Err(e) => {
                debug!("Lua execution failed: {e}");
                Err(NeovimError::Api(format!("Lua execution failed: {e}")))
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn setup_diagnostics_changed_autocmd(&self) -> Result<(), NeovimError> {
        debug!("Setting up diagnostics changed autocmd");

        let conn = self.connection.as_ref().ok_or_else(|| {
            NeovimError::Connection("Not connected to any Neovim instance".to_string())
        })?;

        match conn
            .nvim
            .exec_lua(
                r#"
                    local group = vim.api.nvim_create_augroup("NVIM_MCP_DiagnosticsChanged", { clear = true })
                    vim.api.nvim_create_autocmd("DiagnosticChanged", {
                        group = group,
                        callback = function(args)
                            vim.rpcnotify(0, "NVIM_MCP_DiagnosticsChanged", args.data.diagnostics)
                        end
                    })
                    vim.api.nvim_create_autocmd("LspAttach", {
                        group = group,
                        callback = function(args)
                            vim.rpcnotify(0, "NVIM_MCP_LspAttach", args.data.diagnostics)
                        end
                    })
                    vim.rpcnotify(0, "NVIM_MCP", "setup diagnostics changed autocmd")
                "#,
                vec![],
            )
            .await
        {
            Ok(_) => {
                debug!("Autocmd for diagnostics changed set up successfully");
                Ok(())
            }
            Err(e) => {
                debug!("Failed to set up diagnostics changed autocmd: {}", e);
                Err(NeovimError::Api(format!(
                    "Failed to set up diagnostics changed autocmd: {e}"
                )))
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn get_buffer_diagnostics(
        &self,
        buffer_id: u64,
    ) -> Result<Vec<Diagnostic>, NeovimError> {
        debug!("Getting diagnostics for buffer ID: {}", buffer_id);

        let conn = self.connection.as_ref().ok_or_else(|| {
            NeovimError::Connection("Not connected to any Neovim instance".to_string())
        })?;

        match conn
            .nvim
            .execute_lua(
                format!("return vim.json.encode(vim.diagnostic.get({buffer_id}))").as_str(),
                vec![],
            )
            .await
        {
            Ok(diagnostics) => {
                let diagnostics: Vec<Diagnostic> =
                    match serde_json::from_str(diagnostics.as_str().unwrap()) {
                        Ok(d) => d,
                        Err(e) => {
                            debug!("Failed to parse diagnostics: {}", e);
                            return Err(NeovimError::Api(format!(
                                "Failed to parse diagnostics: {e}"
                            )));
                        }
                    };
                debug!(
                    "Found {} diagnostics for buffer ID {}",
                    diagnostics.len(),
                    buffer_id
                );
                Ok(diagnostics)
            }
            Err(e) => {
                debug!(
                    "Failed to get diagnostics for buffer ID {}: {}",
                    buffer_id, e
                );
                Err(NeovimError::Api(format!("Failed to get diagnostics: {e}")))
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn get_workspace_diagnostics(&self) -> Result<Vec<Diagnostic>, NeovimError> {
        debug!("Getting all workspace diagnostics");

        let conn = self.connection.as_ref().ok_or_else(|| {
            NeovimError::Connection("Not connected to any Neovim instance".to_string())
        })?;

        match conn
            .nvim
            .execute_lua("return vim.json.encode(vim.diagnostic.get())", vec![])
            .await
        {
            Ok(diagnostics) => {
                let diagnostics: Vec<Diagnostic> =
                    match serde_json::from_str(diagnostics.as_str().unwrap()) {
                        Ok(d) => d,
                        Err(e) => {
                            debug!("Failed to parse workspace diagnostics: {}", e);
                            return Err(NeovimError::Api(format!(
                                "Failed to parse workspace diagnostics: {e}"
                            )));
                        }
                    };
                debug!("Found {} workspace diagnostics", diagnostics.len());
                Ok(diagnostics)
            }
            Err(e) => {
                debug!("Failed to get workspace diagnostics: {}", e);
                Err(NeovimError::Api(format!(
                    "Failed to get workspace diagnostics: {e}"
                )))
            }
        }
    }
}
