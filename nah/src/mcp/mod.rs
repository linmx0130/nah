/**
 * Data structure and utilities to handle Model Context Protocol.
 */
use crate::types::NahError;
use nah_mcp_types::request;
use nah_mcp_types::*;
pub use request::MCPRequest;
use serde_json::Value;
use std::collections::HashMap;
mod local_server;
pub use local_server::MCPLocalServerCommand;
pub use local_server::MCPLocalServerProcess;
mod http_server;
pub use http_server::MCPHTTPServerConnection;
pub use http_server::MCPRemoteServerConfig;

/**
 * The trait for all MCP Server adapter implementations. Nah interacts with different MCP servers
 * in this same interface.
 */
pub trait MCPServer {
  /**
   * Send a MCP Request and wait for its response. This method will ignore all non-relevent messages for now.
   */
  fn send_and_wait_for_response(&mut self, request: MCPRequest) -> Result<MCPResponse, NahError>;

  /**
   * Get the name of this server.
   */
  fn get_server_name(&self) -> &str;

  /**
   * Kill the connection with the MCP server and try to release the resource.
   */
  fn kill(&mut self) -> std::io::Result<()>;

  /**
   * Return a reference to the tool definiton map
   */
  fn _get_tool_map<'a>(&'a self) -> &'a HashMap<String, MCPToolDefinition>;

  /**
   * Set the tool definition map to a new value.
   */
  fn _set_tool_map(&mut self, data: HashMap<String, MCPToolDefinition>);

  /**
   * Return a reference to the resource definition map.
   */
  fn _get_resource_map<'a>(&'a self) -> &'a HashMap<String, MCPResourceDefinition>;

  /**
   * Set the resource definition map to a new value.
   */
  fn _set_resource_map(&mut self, data: HashMap<String, MCPResourceDefinition>);

  /**
   * Fetch the list of tools from the MCP Server.
   */
  fn fetch_tools(&mut self) -> Result<Vec<&MCPToolDefinition>, NahError> {
    let id: String = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::tools_list(&id);
    let response = self.send_and_wait_for_response(request)?;

    let tool_list = parse_tools_list_from_response(self.get_server_name(), response)?;
    let mut tool_map = HashMap::new();
    for item in tool_list {
      tool_map.insert(item.name.to_owned(), item);
    }
    self._set_tool_map(tool_map);
    Ok(self._get_tool_map().values().collect())
  }

  /**
   * Call the tool and wait for the response. Return value is the result object.
   */
  fn call_tool(&mut self, tool_name: &str, args: &Value) -> Result<Value, NahError> {
    let id: String = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::tools_call(&id, tool_name, args);
    let response = self.send_and_wait_for_response(request)?;

    match response.result {
      Some(r) => Ok(r),
      None => Err(self.parse_response_error(&response)),
    }
  }

  /**
   * Get the definition of a given tool name. It may try to read the tool from cached results.
   */
  fn get_tool_definition(&mut self, tool_name: &str) -> Result<&MCPToolDefinition, NahError> {
    if self._get_tool_map().contains_key(tool_name) {
      Ok(self._get_tool_map().get(tool_name).unwrap())
    } else {
      // re-fetch tool list
      self.fetch_tools()?;
      match self._get_tool_map().get(tool_name) {
        Some(p) => Ok(p),
        None => Err(NahError::invalid_value(&format!(
          "Invalid tool name: {}",
          tool_name
        ))),
      }
    }
  }

  /**
   * Fetch the list of available resources.
   */
  fn fetch_resources_list(&mut self) -> Result<Vec<&MCPResourceDefinition>, NahError> {
    let id: String = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::resources_list(&id);
    let response = self.send_and_wait_for_response(request)?;
    match response.result {
      Some(res) => {
        let resources = res
          .as_object()
          .and_then(|obj| obj.get("resources"))
          .and_then(|v| v.as_array());
        if resources.is_none() {
          return Err(NahError::mcp_server_invalid_response(
            self.get_server_name(),
          ));
        }
        let mut resource_map = HashMap::new();
        resources
          .unwrap()
          .iter()
          .map(|v| serde_json::from_value::<MCPResourceDefinition>(v.clone()))
          .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(_) => None,
          })
          .for_each(|v| {
            resource_map.insert(v.name.to_owned(), v);
          });
        self._set_resource_map(resource_map);
        Ok(self._get_resource_map().values().collect())
      }
      None => Err(self.parse_response_error(&response)),
    }
  }

  /**
   * Fetch the list of resource templates.
   */
  fn fetch_resource_templates_list(&mut self) -> Result<Vec<MCPResourceDefinition>, NahError> {
    let id: String = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::resource_templates_list(&id);
    let response = self.send_and_wait_for_response(request)?;
    match response.result {
      Some(res) => {
        let resources = res
          .as_object()
          .and_then(|obj| obj.get("resourceTemplates"))
          .and_then(|v: &Value| v.as_array());
        if resources.is_none() {
          return Err(NahError::mcp_server_invalid_response(
            self.get_server_name(),
          ));
        }
        let result: Vec<MCPResourceDefinition> = resources
          .unwrap()
          .iter()
          .map(|v| serde_json::from_value::<MCPResourceDefinition>(v.clone()))
          .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(_) => None,
          })
          .collect();
        Ok(result)
      }
      None => Err(self.parse_response_error(&response)),
    }
  }

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

  fn parse_response_error(&self, response: &MCPResponse) -> NahError {
    match &response.error {
      Some(e) => {
        NahError::mcp_server_error(self.get_server_name(), &serde_json::to_string(e).unwrap())
      }
      None => NahError::mcp_server_error(self.get_server_name(), "unknown error"),
    }
  }
}

pub(in crate::mcp) fn parse_tools_list_from_response(
  server_name: &str,
  response: MCPResponse,
) -> Result<Vec<MCPToolDefinition>, NahError> {
  let mut result = Vec::new();
  match response.result {
    None => {
      return Err(match response.error {
        None => NahError::mcp_server_communication_error(server_name),
        Some(err) => {
          NahError::mcp_server_error(server_name, &serde_json::to_string_pretty(&err).unwrap())
        }
      });
    }
    Some(res) => {
      let tools = match res
        .as_object()
        .and_then(|v| v.get("tools"))
        .and_then(|v| v.as_array())
      {
        None => {
          return Err(NahError::mcp_server_invalid_response(server_name));
        }
        Some(t) => t,
      };

      for item in tools.iter() {
        let tool: MCPToolDefinition = match serde_json::from_value(item.clone()) {
          Ok(t) => t,
          Err(_e) => {
            return Err(NahError::mcp_server_invalid_response(server_name));
          }
        };
        result.push(tool);
      }
    }
  };
  return Ok(result);
}
