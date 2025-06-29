/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
pub mod notification;
pub mod request;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

impl MCPResponse {
  pub fn new(id: String, result: Option<Value>, error: Option<Value>) -> MCPResponse {
    MCPResponse {
      jsonrpc: "2.0".to_string(),
      id,
      result,
      error,
    }
  }
}

/**
 * Describes how to launch a MCP server with a command.
 */
#[derive(Debug, Deserialize)]
pub struct MCPServerCommand {
  pub command: String,
  pub args: Vec<String>,
}

/**
 * Describe a MCP tool.
 */
#[derive(Debug, Deserialize, Clone)]
pub struct MCPToolDefinition {
  pub name: String,
  pub description: Option<String>,
  #[serde(rename = "inputSchema")]
  pub input_schema: Value,
}

/**
 * Describe a MCP Resource.
 */
#[derive(Debug, Deserialize, Clone)]
pub struct MCPResourceDefinition {
  pub uri: Option<String>,
  #[serde(rename = "uriTemplate")]
  pub uri_template: Option<String>,
  pub name: String,
  pub description: Option<String>,
  #[serde(rename = "mimeType")]
  pub mime_type: Option<String>,
  pub size: Option<usize>,
}

/**
 * Describe a MCP Resource content.
 */
#[derive(Debug, Deserialize, Serialize)]
pub struct MCPResourceContent {
  pub uri: String,
  pub mime: Option<String>,
  pub text: Option<String>,
  pub blob: Option<String>,
}

/**
 * Describe a MCP prompt.
 */
#[derive(Debug, Deserialize)]
pub struct MCPPromptDefinition {
  pub name: String,
  pub description: Option<String>,
  pub arguments: Option<Vec<MCPPromptArgument>>,
}

/**
 * Describe an argument that a prompt can accept.
 */
#[derive(Debug, Deserialize)]
pub struct MCPPromptArgument {
  pub name: String,
  pub description: Option<String>,
  pub required: Option<bool>,
}

/**
 * Describe a prompt message content. It could be text, image, audio or other supported data.
 */
#[derive(Debug, Deserialize, Serialize)]
pub struct PromptMessageContent {
  #[serde(rename = "type")]
  pub type_: String,
  pub text: Option<String>,
  pub data: Option<String>,
  #[serde(rename = "mimeType")]
  pub mime_type: Option<String>,
  pub resource: Option<Value>,
  pub annotations: Option<Value>,
}

/**
 * Describes a prompt message.
 */
#[derive(Debug, Deserialize, Serialize)]
pub struct PromptMessage {
  pub role: String,
  pub content: PromptMessageContent,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MCPPromptResult {
  pub description: Option<String>,
  pub messages: Vec<PromptMessage>,
}
