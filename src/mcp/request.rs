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
  /**
   * Request to initialize the server.
   */
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

  /**
   * Request to fetch the list of available tools from `tools/list`.
   */
  pub fn tools_list(id: &str) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "tools/list".to_string(),
      id: id.to_string(),
      params: None,
    }
  }

  /**
   * Request to call a tool.
   */
  pub fn tools_call(id: &str, tool_name: &str, args: &Value) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "tools/call".to_string(),
      id: id.to_string(),
      params: Some(json!(
        {
        "name": tool_name,
        "arguments": args,
        }
      )),
    }
  }

  /**
   * Request to fetch the list of available resources from `resources/list`.
   */
  pub fn resources_list(id: &str) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "resources/list".to_owned(),
      id: id.to_string(),
      params: None,
    }
  }

  /**
   * Request to fetch the list of available resource templates from `resources/templates/list`
   */
  pub fn resource_templates_list(id: &str) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "resources/templates/list".to_owned(),
      id: id.to_string(),
      params: None,
    }
  }

  /**
   * Request to read a resource.
   */
  pub fn resources_read(id: &str, uri: &str) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "resources/read".to_owned(),
      id: id.to_string(),
      params: Some(json!(
      {
        "uri": uri.to_string()
      })),
    }
  }
}
