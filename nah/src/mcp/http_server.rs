use crate::{
  mcp::{parse_tools_list_from_response, MCPServer},
  types::NahError,
};
use nah_mcp_types::{request::MCPRequest, MCPResponse, MCPToolDefinition};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::runtime::{Builder, Runtime};

#[derive(Debug, Deserialize)]
pub struct MCPRemoteServerConfig {
  pub url: String,
  pub headers: HashMap<String, String>,
}

pub struct MCPHTTPServerConnection {
  name: String,
  url: String,
  headers: HashMap<String, String>,
  tokio_runtime: Runtime,
  http_client: Client,
  tool_cache: HashMap<String, MCPToolDefinition>,
}

impl MCPServer for MCPHTTPServerConnection {
  fn send_and_wait_for_response(
    &mut self,
    request: nah_mcp_types::request::MCPRequest,
  ) -> Result<nah_mcp_types::MCPResponse, crate::types::NahError> {
    let id = request.id.clone();
    let data_str = serde_json::to_string(&request).unwrap();
    let mut req = self.http_client.post(self.url.to_owned());
    for (k, v) in self.headers.iter() {
      req = req.header(k, v);
    }
    req = req.header("Content-Type", "application/json");
    req = req.header("Accept", "application/json,text/event-stream");
    req = req.body(data_str);
    let res = match self
      .tokio_runtime
      .block_on(async { req.send().await?.text().await })
    {
      Ok(s) => s,
      Err(e) => {
        return Err(NahError::mcp_server_error(
          &self.name,
          &format!("Error in fetching MCP remote server response: {}", e),
        ));
      }
    };
    match serde_json::from_str::<MCPResponse>(&res) {
      Ok(r) => Ok(r),
      Err(_e) => Err(NahError::mcp_server_invalid_response(&self.name)),
    }
  }

  fn kill(&mut self) -> std::io::Result<()> {
    Ok(())
  }

  fn fetch_tools(
    &mut self,
  ) -> Result<Vec<&nah_mcp_types::MCPToolDefinition>, crate::types::NahError> {
    let id: String = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::tools_list(&id);
    let response = self.send_and_wait_for_response(request)?;
    let tool_list = parse_tools_list_from_response(&self.name, response)?;
    self.tool_cache.clear();
    for item in tool_list {
      self.tool_cache.insert(item.name.to_owned(), item);
    }
    Ok(self.tool_cache.values().collect())
  }

  fn call_tool(
    &mut self,
    tool_name: &str,
    args: &serde_json::Value,
  ) -> Result<serde_json::Value, crate::types::NahError> {
    todo!()
  }

  fn get_tool_definition(
    &mut self,
    tool_name: &str,
  ) -> Result<&nah_mcp_types::MCPToolDefinition, crate::types::NahError> {
    todo!()
  }

  fn fetch_resources_list(
    &mut self,
  ) -> Result<Vec<&nah_mcp_types::MCPResourceDefinition>, crate::types::NahError> {
    todo!()
  }

  fn fetch_resource_templates_list(
    &mut self,
  ) -> Result<Vec<nah_mcp_types::MCPResourceDefinition>, crate::types::NahError> {
    todo!()
  }

  fn get_resources_definition(
    &mut self,
    uri: &str,
  ) -> Result<&nah_mcp_types::MCPResourceDefinition, crate::types::NahError> {
    todo!()
  }

  fn set_timeout(&mut self, timeout_ms: u64) {
    todo!()
  }

  fn read_resources(
    &mut self,
    uri: &str,
  ) -> Result<Vec<nah_mcp_types::MCPResourceContent>, crate::types::NahError> {
    todo!()
  }

  fn fetch_prompts_list(
    &mut self,
  ) -> Result<Vec<&nah_mcp_types::MCPPromptDefinition>, crate::types::NahError> {
    todo!()
  }

  fn get_prompt_definition(
    &mut self,
    prompt_name: &str,
  ) -> Result<&nah_mcp_types::MCPPromptDefinition, crate::types::NahError> {
    todo!()
  }

  fn get_prompt_content(
    &mut self,
    prompt_name: &str,
    args: &HashMap<String, String>,
  ) -> Result<nah_mcp_types::MCPPromptResult, crate::types::NahError> {
    todo!()
  }
}

impl MCPHTTPServerConnection {
  pub fn init(name: &str, config: &MCPRemoteServerConfig) -> Result<Self, NahError> {
    let tokio_runtime = match Builder::new_current_thread()
      .enable_io()
      .enable_time()
      .build()
    {
      Ok(r) => r,
      Err(e) => {
        return Err(NahError::io_error(
          "Failed to create tokio runtime for network connection",
        ));
      }
    };
    Ok(MCPHTTPServerConnection {
      name: name.to_string(),
      url: config.url.to_owned(),
      headers: config.headers.to_owned(),
      tokio_runtime: tokio_runtime,
      http_client: Client::new(),
      tool_cache: HashMap::new(),
    })
  }
}
