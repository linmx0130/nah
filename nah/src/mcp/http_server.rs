use crate::{mcp::MCPServer, types::NahError};
use nah_mcp_types::{request::MCPRequest, MCPResourceDefinition, MCPResponse, MCPToolDefinition};
use reqwest::Client;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
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
  resource_cache: HashMap<String, MCPResourceDefinition>,
}

impl MCPServer for MCPHTTPServerConnection {
  fn send_and_wait_for_response(
    &mut self,
    request: MCPRequest,
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

  fn get_resources_definition(
    &mut self,
    uri: &str,
  ) -> Result<&nah_mcp_types::MCPResourceDefinition, crate::types::NahError> {
    todo!()
  }

  fn set_timeout(&mut self, timeout_ms: u64) {
    self.http_client = Client::builder()
      .timeout(Duration::from_millis(timeout_ms))
      .build()
      .unwrap();
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

  fn get_server_name(&self) -> &str {
    &self.name
  }

  fn _get_tool_map<'a>(&'a self) -> &'a HashMap<String, MCPToolDefinition> {
    &self.tool_cache
  }

  fn _set_tool_map(&mut self, data: HashMap<String, MCPToolDefinition>) {
    self.tool_cache = data;
  }

  fn _get_resource_map<'a>(&'a self) -> &'a HashMap<String, MCPResourceDefinition> {
    &self.resource_cache
  }

  fn _set_resource_map(&mut self, data: HashMap<String, MCPResourceDefinition>) {
    self.resource_cache = data;
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
      Err(_e) => {
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
      resource_cache: HashMap::new(),
    })
  }
}
