use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcError {
    NotFound(String),
    Forbidden(String),
    BadRequest(String),
    Timeout,
    Runtime(String),
    MaxDepthReached,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::NotFound(m) => write!(f, "RPC Method Not Found: {}", m),
            RpcError::Forbidden(m) => write!(f, "RPC Forbidden: {}", m),
            RpcError::BadRequest(m) => write!(f, "RPC Bad Request: {}", m),
            RpcError::Timeout => write!(f, "RPC Timeout"),
            RpcError::Runtime(m) => write!(f, "RPC Runtime Error: {}", m),
            RpcError::MaxDepthReached => write!(f, "RPC Max Call Depth Reached"),
        }
    }
}

impl std::error::Error for RpcError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcVisibility {
    Private,  // Только для самого модуля
    Internal, // Для других модулей
    Admin,    // Только для админ-контекста
    Public,   // Доступно через внешний API
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMethodDescriptor {
    pub name: String,
    pub handler: String, // Название Lua-функции или Rust-метода
    pub permission: Option<String>,
    pub visibility: RpcVisibility,
}

#[derive(Debug, Clone)]
pub struct RpcContext {
    pub caller: Option<String>, // ID вызывающего модуля
    pub trace_id: String,
    pub call_depth: u32,
    pub auth_admin_id: Option<i32>,
}

impl Default for RpcContext {
    fn default() -> Self {
        Self {
            caller: None,
            trace_id: uuid::Uuid::new_v4().to_string(),
            call_depth: 0,
            auth_admin_id: None,
        }
    }
}

impl RpcContext {
    pub fn next(&self, caller: String) -> Self {
        Self {
            caller: Some(caller),
            trace_id: self.trace_id.clone(),
            call_depth: self.call_depth + 1,
            auth_admin_id: self.auth_admin_id,
        }
    }
}

pub mod registry;
