mod client;
mod connection;
mod error;

#[cfg(test)]
mod integration_tests;

pub use client::{Diagnostic, NeovimClient, Position, Range};
pub use error::NeovimError;
