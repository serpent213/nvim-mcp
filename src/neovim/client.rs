use async_trait::async_trait;
use nvim_rs::{Handler, Neovim, compat::tokio::Compat};
use rmpv::Value;
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct NeovimHandler;

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Compat<tokio::io::WriteHalf<TcpStream>>;

    async fn handle_request(
        &self,
        name: String,
        _args: Vec<Value>,
        _neovim: Neovim<Compat<tokio::io::WriteHalf<TcpStream>>>,
    ) -> Result<Value, Value> {
        match name.as_ref() {
            "ping" => Ok(Value::from("pong")),
            _ => Ok(Value::Nil),
        }
    }
}
