/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::MCP_PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
/**
 * MCP Request is a JSON-RPC request with id enabled.
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct MCPRequest {
  jsonrpc: String,
  pub method: String,
  pub id: Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub params: Option<Value>,
}

impl MCPRequest {
  /**
   * Request to initialize the server.
   */
  pub fn initialize(id: &Value, client_name: &str, client_version: &str) -> Self {
    let params = Some(json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": json!({}),
        "clientInfo": json!({
            "name": client_name,
            "version": client_version
        })
    }));
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "initialize".to_string(),
      id: id.clone(),
      params,
    }
  }

  /**
   * Request to fetch the list of available tools from `tools/list`.
   */
  pub fn tools_list(id: &Value) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "tools/list".to_string(),
      id: id.clone(),
      params: None,
    }
  }

  /**
   * Request to call a tool.
   */
  pub fn tools_call(id: &Value, tool_name: &str, args: &Value) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "tools/call".to_string(),
      id: id.clone(),
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
  pub fn resources_list(id: &Value) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "resources/list".to_owned(),
      id: id.clone(),
      params: None,
    }
  }

  /**
   * Request to fetch the list of available resource templates from `resources/templates/list`
   */
  pub fn resource_templates_list(id: &Value) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "resources/templates/list".to_owned(),
      id: id.clone(),
      params: None,
    }
  }

  /**
   * Request to read a resource.
   */
  pub fn resources_read(id: &Value, uri: &str) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "resources/read".to_owned(),
      id: id.clone(),
      params: Some(json!(
      {
        "uri": uri.to_string()
      })),
    }
  }

  /**
   * Request to fetch available prompts.
   */
  pub fn prompts_list(id: &Value) -> Self {
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "prompts/list".to_owned(),
      id: id.clone(),
      params: None,
    }
  }

  /**
   * Requeset to retrieve a prompt.
   */
  pub fn get_prompt<'a, I>(id: &Value, prompt_name: &str, args: I) -> Self
  where
    I: Iterator<Item = (&'a str, &'a str)>,
  {
    let mut arguments = serde_json::Map::new();
    for (key, value) in args {
      arguments.insert(key.to_owned(), Value::String(value.to_owned()));
    }
    MCPRequest {
      jsonrpc: "2.0".to_string(),
      method: "prompts/get".to_owned(),
      id: id.clone(),
      params: Some(json!({
        "name": Value::String(prompt_name.to_owned()),
        "arguments": if arguments.len() > 0 { Some(Value::Object(arguments))} else {None}
      })),
    }
  }
}
