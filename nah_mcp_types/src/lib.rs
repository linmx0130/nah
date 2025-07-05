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
  pub id: Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<Value>,
}

impl MCPResponse {
  pub fn new(id: Value, result: Option<Value>, error: Option<Value>) -> MCPResponse {
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
 * MCP tool annotations, which contains some optional metadata.
 *
 * Following https://modelcontextprotocol.io/docs/concepts/tools#tool-definition-structure
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPToolAnnotations {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none", rename = "readOnlyHint")]
  pub read_only_hint: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none", rename = "destructiveHint")]
  pub destructive_hint: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none", rename = "idempotentHint")]
  pub idempotent_hint: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none", rename = "openWorldHint")]
  pub open_world_hint: Option<bool>,
}

/**
 * Describe a MCP tool.
 *
 * Following https://modelcontextprotocol.io/docs/concepts/tools#tool-definition-structure
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPToolDefinition {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(rename = "inputSchema")]
  pub input_schema: Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub annotations: Option<MCPToolAnnotations>,
}

impl MCPToolDefinition {
  #[inline]
  pub fn is_destructive(&self) -> bool {
    match self.annotations.as_ref().and_then(|a| a.destructive_hint) {
      Some(true) => true,
      _ => false,
    }
  }

  #[inline]
  pub fn is_open_world(&self) -> bool {
    match self.annotations.as_ref().and_then(|a| a.open_world_hint) {
      Some(true) => true,
      _ => false,
    }
  }
}

/**
 * Describe a MCP Resource.
 */
#[derive(Debug, Deserialize, Clone)]
pub struct MCPResourceDefinition {
  pub uri: Option<String>,
  #[serde(rename = "uriTemplate", skip_serializing_if = "Option::is_none")]
  pub uri_template: Option<String>,
  pub name: String,
  pub description: Option<String>,
  #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
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
