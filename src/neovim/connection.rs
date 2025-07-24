use nvim_rs::{Neovim, compat::tokio::Compat, error::LoopError};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

pub struct NeovimConnection {
    pub nvim: Neovim<Compat<tokio::io::WriteHalf<TcpStream>>>,
    pub io_handler: JoinHandle<Result<Result<(), Box<LoopError>>, tokio::task::JoinError>>,
    pub address: String,
}

impl NeovimConnection {
    pub fn new(
        nvim: Neovim<Compat<tokio::io::WriteHalf<TcpStream>>>,
        io_handler: JoinHandle<Result<Result<(), Box<LoopError>>, tokio::task::JoinError>>,
        address: String,
    ) -> Self {
        Self {
            nvim,
            io_handler,
            address,
        }
    }

    pub fn is_connected(&self) -> bool {
        !self.io_handler.is_finished()
    }

    pub fn address(&self) -> &str {
        &self.address
    }
}
