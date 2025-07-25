pub(crate) mod neovim;
mod neovim_handler;

#[cfg(test)]
mod integration_tests;

pub use neovim::NeovimMcpServer;
