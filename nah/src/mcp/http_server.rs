/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::{mcp::MCPServer, types::NahError};
use nah_mcp_types::{
  notification::MCPNotification, request::MCPRequest, MCPPromptDefinition, MCPResourceDefinition,
  MCPResponse, MCPToolDefinition, MCP_PROTOCOL_VERSION,
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, time::Duration};
use tokio::runtime::{Builder, Runtime};

#[derive(Debug, Deserialize)]
pub struct MCPRemoteServerConfig {
  pub url: String,
  pub headers: HashMap<String, String>,
  pub timeout_ms: Option<u64>,
}

pub struct MCPHTTPServerConnection {
  name: String,
  url: String,
  headers: HashMap<String, String>,
  tokio_runtime: Runtime,
  http_client: Client,
  tool_cache: HashMap<String, MCPToolDefinition>,
  resource_cache: HashMap<String, MCPResourceDefinition>,
  prompt_cache: HashMap<String, MCPPromptDefinition>,
  session_id: Option<String>,
}

impl MCPServer for MCPHTTPServerConnection {
  fn send_and_wait_for_response(
    &mut self,
    request: MCPRequest,
  ) -> Result<nah_mcp_types::MCPResponse, crate::types::NahError> {
    let data_str = serde_json::to_string(&request).unwrap();
    let mut req = self.http_client.post(self.url.to_owned());
    for (k, v) in self.headers.iter() {
      req = req.header(k, v);
    }
    req = req.header(reqwest::header::CONTENT_TYPE, "application/json");
    req = req.header(
      reqwest::header::ACCEPT,
      "application/json,text/event-stream",
    );
    req = req.header(reqwest::header::CONNECTION, "close");
    req = req.header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION);
    if self.session_id.is_some() {
      req = req.header("Mcp-Session-Id", self.session_id.as_ref().unwrap());
    }
    req = req.body(data_str);
    let response = match self.tokio_runtime.block_on(async { req.send().await }) {
      Ok(response) => response,
      Err(e) => {
        return Err(NahError::mcp_server_communication_error(
          &self.name,
          Some(Box::new(e)),
        ));
      }
    };
    let session_id = response.headers().get("Mcp-Session-Id");
    if session_id.is_some() && self.session_id.is_none() {
      let new_session_id = session_id.unwrap().to_str().unwrap().to_string();
      println!(
        "Initialized a new session with {}, id={}",
        self.name, new_session_id
      );
      self.session_id = Some(new_session_id);
    }
    let content_type = response.headers().get("Content-Type");

    let json_content = match content_type {
      Some(type_value) => {
        let content_type = type_value.as_bytes();
        match content_type {
          b"application/json" => match self.tokio_runtime.block_on(async { response.text().await })
          {
            Ok(s) => s,
            Err(e) => {
              return Err(NahError::mcp_server_invalid_response(
                &self.name,
                Some(Box::new(e)),
              ));
            }
          },
          b"text/event-stream" => {
            let all_response_data: String =
              match self.tokio_runtime.block_on(async { response.text().await }) {
                Ok(s) => s,
                Err(e) => {
                  return Err(NahError::mcp_server_invalid_response(
                    &self.name,
                    Some(Box::new(e)),
                  ));
                }
              };
            match all_response_data
              .split("\n")
              .filter(|s| s.starts_with("data: "))
              .next()
            {
              Some(s) => s.trim_start_matches("data: ").trim().to_string(),
              None => {
                return Err(NahError::mcp_server_invalid_response(&self.name, None));
              }
            }
          }
          _ => {
            let type_str = type_value.to_str().unwrap_or("UNKNOWN");
            return Err(NahError::mcp_server_error(
              &self.name,
              &format!("Unknown content type for MCP response: {}", type_str),
              None,
            ));
          }
        }
      }
      None => {
        return Err(NahError::mcp_server_error(
          &self.name,
          &format!("Missing content type for MCP response"),
          None,
        ));
      }
    };

    match serde_json::from_str::<MCPResponse>(&json_content) {
      Ok(r) => Ok(r),
      Err(e) => Err(NahError::mcp_server_invalid_response(
        &self.name,
        Some(Box::new(e)),
      )),
    }
  }

  fn kill(&mut self) -> std::io::Result<()> {
    match &self.session_id {
      None => Ok(()),
      Some(session_id) => {
        let mut req = self.http_client.delete(self.url.to_owned());
        for (k, v) in self.headers.iter() {
          req = req.header(k, v);
        }
        req = req.header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION);
        req = req.header("Mcp-Session-Id", session_id);
        match self.tokio_runtime.block_on(async { req.send().await }) {
          Ok(_) => Ok(()),
          Err(e) => std::io::Result::Err(std::io::Error::other(e)),
        }
      }
    }
  }

  fn set_timeout(&mut self, timeout_ms: u64) {
    self.http_client = Client::builder()
      .timeout(Duration::from_millis(timeout_ms))
      .build()
      .unwrap();
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

  fn _get_prompt_map<'a>(&'a self) -> &'a HashMap<String, MCPPromptDefinition> {
    &self.prompt_cache
  }

  fn _set_prompt_map(&mut self, data: HashMap<String, MCPPromptDefinition>) {
    self.prompt_cache = data;
  }
}

impl MCPHTTPServerConnection {
  fn send_notification(&mut self, request: MCPNotification) -> Result<(), crate::types::NahError> {
    let data_str = serde_json::to_string(&request).unwrap();
    let mut req = self.http_client.post(self.url.to_owned());
    for (k, v) in self.headers.iter() {
      req = req.header(k, v);
    }
    req = req.header(reqwest::header::CONTENT_TYPE, "application/json");
    req = req.header(
      reqwest::header::ACCEPT,
      "application/json,text/event-stream",
    );
    req = req.header(reqwest::header::CONNECTION, "close");
    req = req.header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION);
    if self.session_id.is_some() {
      req = req.header("Mcp-Session-Id", self.session_id.as_ref().unwrap());
    }
    req = req.body(data_str);
    let response = self.tokio_runtime.block_on(async { req.send().await });
    match response {
      Ok(r) => {
        if r.status().is_success() {
          Ok(())
        } else {
          Err(NahError::mcp_server_error(
            &self.name,
            "Initialization is not success.",
            None,
          ))
        }
      }
      Err(e) => Err(NahError::mcp_server_communication_error(
        &self.name,
        Some(Box::new(e)),
      )),
    }
  }

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
          Some(Box::new(e)),
        ));
      }
    };
    let mut conn = MCPHTTPServerConnection {
      name: name.to_string(),
      url: config.url.to_owned(),
      headers: config.headers.to_owned(),
      tokio_runtime: tokio_runtime,
      http_client: Client::new(),
      tool_cache: HashMap::new(),
      resource_cache: HashMap::new(),
      prompt_cache: HashMap::new(),
      session_id: None,
    };
    if config.timeout_ms.is_some() {
      conn.set_timeout(config.timeout_ms.unwrap());
    }
    let initialize_request = MCPRequest::initialize(
      &Value::String(uuid::Uuid::new_v4().to_string()),
      "nah",
      "0.1",
    );
    conn.send_and_wait_for_response(initialize_request)?;
    let initialized_notification = MCPNotification::initialized();
    conn.send_notification(initialized_notification)?;

    Ok(conn)
  }
}
