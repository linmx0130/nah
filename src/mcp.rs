/**
 * Data structure and utilities to handle Model Context Protocol.
 */
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/**
 * MCP Request is a JSON-RPC request with id enabled.
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct MCPRequest {
  jsonrpc: String,
  pub method: String,
  pub id: String,
  pub params: Option<Value>,
}

impl MCPRequest {
  pub fn initialize(id: &str, client_name: &str, client_version: &str) -> Self {
    let params = Some(json!({
        "protocolVersion": "2024-11-05",
        "capabilities": json!({}),
        "clientInfo": json!({
            "name": client_name,
            "version": client_version
        })
    }));
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "initialize".to_string(),
      id: id.to_string(),
      params,
    }
  }
}

/**
 * MCP Response is a JSON-RPC response.
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct MCPResponse {
  jsonrpc: String,
  pub id: String,
  pub result: Option<Value>,
  pub error: Option<Value>,
}

/**
 * MCP Notification is a JSON-RPC request without id.
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct MCPNotification {
  jsonrpc: String,
  pub method: String,
  pub params: Option<Value>,
}

impl MCPNotification {
  pub fn initialized() -> Self {
    MCPNotification {
      jsonrpc: "2.0".to_owned(),
      method: "notifications/initialized".to_owned(),
      params: None,
    }
  }
}
