use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Environment variable error: {0}")]
    Environment(String),
    #[error("Filesystem error: {0}")]
    Filesystem(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Socket operation mode determined by the provided socket-path
#[derive(Debug, Clone, PartialEq)]
pub enum SocketGlobMode {
    /// Path is an existing directory - search for nvim-mcp.*.sock files
    Directory,
    /// Path is an existing file - locked mode with single Neovim instance
    SingleFile,
    /// Path doesn't exist - treat as glob pattern to find files
    GlobPattern,
}

/// Configuration for the Neovim MCP server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub socket_path: PathBuf,
    pub socket_mode: SocketGlobMode,
    pub log_file: Option<PathBuf>,
    pub log_level: String,
}

impl ServerConfig {
    /// Create a new server configuration with resolved socket path
    pub fn new(
        socket_path: Option<String>,
        log_file: Option<PathBuf>,
        log_level: String,
    ) -> Result<Self, ConfigError> {
        let (socket_path, socket_mode) = Self::resolve_socket_path_and_mode(socket_path)?;

        Ok(Self {
            socket_path,
            socket_mode,
            log_file,
            log_level,
        })
    }

    /// Resolve socket path and determine mode from optional user input or platform defaults
    pub fn resolve_socket_path_and_mode(
        provided: Option<String>,
    ) -> Result<(PathBuf, SocketGlobMode), ConfigError> {
        match provided {
            Some(path) => {
                let path_buf = PathBuf::from(&path);

                if path_buf.exists() {
                    if path_buf.is_dir() {
                        // Existing directory - Directory mode
                        Ok((path_buf, SocketGlobMode::Directory))
                    } else if path_buf.is_file() {
                        // Existing file - SingleFile mode (locked)
                        Ok((path_buf, SocketGlobMode::SingleFile))
                    } else {
                        Err(ConfigError::InvalidPath(format!(
                            "Path exists but is neither file nor directory: {}",
                            path_buf.display()
                        )))
                    }
                } else {
                    // Path doesn't exist - treat as glob pattern
                    Ok((path_buf, SocketGlobMode::GlobPattern))
                }
            }
            None => {
                // Use default directory path
                let default_path = Self::default_socket_path()?;
                Ok((default_path, SocketGlobMode::Directory))
            }
        }
    }

    /// Resolve socket path from optional user input or platform defaults
    pub fn resolve_socket_path(provided: Option<String>) -> Result<PathBuf, ConfigError> {
        let (path, _mode) = Self::resolve_socket_path_and_mode(provided)?;
        Ok(path)
    }

    /// Get platform-specific default socket directory
    fn default_socket_path() -> Result<PathBuf, ConfigError> {
        let socket_dir = if cfg!(target_os = "windows") {
            PathBuf::from(
                std::env::var("TEMP").map_err(|e| {
                    ConfigError::Environment(format!("TEMP variable not set: {}", e))
                })?,
            )
        } else {
            let home = std::env::var("HOME")
                .map_err(|e| ConfigError::Environment(format!("HOME variable not set: {}", e)))?;
            PathBuf::from(home).join(".cache").join("nvim").join("rpc")
        };

        // Ensure directory exists
        std::fs::create_dir_all(&socket_dir).map_err(|e| {
            ConfigError::Filesystem(format!(
                "Cannot create socket directory {}: {}",
                socket_dir.display(),
                e
            ))
        })?;

        Ok(socket_dir)
    }
}
