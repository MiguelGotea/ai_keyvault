//! Módulos de comandos Tauri expuestos al frontend React.
//! Cada sub-módulo agrupa comandos relacionados.
//!
//! REGLA: Todo HTTP al VPS sale de commands/api.rs — nunca fetch() en React.

pub mod api;
pub mod health;
pub mod poller;
pub mod switch_cmd;
pub mod vault;
