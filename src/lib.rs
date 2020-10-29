#[deny(unconditional_recursion, missing_debug_implementations)]
mod config;
pub mod global_static;
mod server;

pub use server::{Error, Server};
