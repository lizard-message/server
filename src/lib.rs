#[deny(unconditional_recursion)]
mod config;
pub mod global_static;
mod server;

pub use server::{Error, Server};
