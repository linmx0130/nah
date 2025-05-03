use serde::{Deserialize, Serialize};
use serde_json::Value;

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
