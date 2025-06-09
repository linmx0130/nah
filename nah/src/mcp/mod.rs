use crate::types::NahError;
use nah_mcp_types::request;
use nah_mcp_types::*;
pub use request::MCPRequest;
/**
 * Data structure and utilities to handle Model Context Protocol.
 */
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
mod local_server;
pub use local_server::MCPLocalServerCommand;
pub use local_server::MCPLocalServerProcess;

#[derive(Debug, Deserialize)]
pub struct MCPRemoteServerConfig {
  pub url: String,
  pub headers: HashMap<String, String>,
}

pub trait MCPServer {
  /**
   * Send a MCP Request and wait for its response. This method will ignore all non-relevent messages for now.
   */
  fn send_and_wait_for_response(&mut self, request: MCPRequest) -> Result<MCPResponse, NahError>;

  /**
   * Kill the connection with the MCP server and try to release the resource.
   */
  fn kill(&mut self) -> std::io::Result<()>;

  /**
   * Fetch the list of tools from the MCP Server.
   */
  fn fetch_tools(&mut self) -> Result<Vec<&MCPToolDefinition>, NahError>;

  /**
   * Call the tool and wait for the response. Return value is the result object.
   */
  fn call_tool(&mut self, tool_name: &str, args: &Value) -> Result<Value, NahError>;

  /**
   * Get the definition of a given tool name. It may try to read the tool from cached results.
   */
  fn get_tool_definition(&mut self, tool_name: &str) -> Result<&MCPToolDefinition, NahError>;

  /**
   * Fetch the list of available resources.
   */
  fn fetch_resources_list(&mut self) -> Result<Vec<&MCPResourceDefinition>, NahError>;

  /**
   * Fetch the list of resource templates.
   */
  fn fetch_resource_templates_list(&mut self) -> Result<Vec<MCPResourceDefinition>, NahError>;

  /**
   * Get the definiton of a given resource URI.
   */
  fn get_resources_definition(&mut self, uri: &str) -> Result<&MCPResourceDefinition, NahError>;

  /**
   * Set timeout for waiting for a response.
   */
  fn set_timeout(&mut self, timeout_ms: u64);

  /**
   * Read the content of a resource URI.
   */
  fn read_resources(&mut self, uri: &str) -> Result<Vec<MCPResourceContent>, NahError>;

  /**
   * Fetch the list of promptss from the MCP Server.
   */
  fn fetch_prompts_list(&mut self) -> Result<Vec<&MCPPromptDefinition>, NahError>;

  /**
   * Get the definition of a given prompt name. It may try to read the prompt from cached results.
   */
  fn get_prompt_definition(&mut self, prompt_name: &str) -> Result<&MCPPromptDefinition, NahError>;

  /**
   * Get the prompt content through a given prompt name and arguments.
   */
  fn get_prompt_content(
    &mut self,
    prompt_name: &str,
    args: &HashMap<String, String>,
  ) -> Result<MCPPromptResult, NahError>;
}
