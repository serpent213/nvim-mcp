pub mod neovim;
pub mod neovim_handler;

#[cfg(test)]
mod integration_tests;

pub use neovim::NeovimMcpServer;
