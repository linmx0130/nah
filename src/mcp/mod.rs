/**
 * Data structure and utilities to handle Model Context Protocol.
 */
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{
  io::{BufRead, BufReader, Write},
  process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use crate::types::NahError;

mod notification;
mod request;
pub use notification::MCPNotification;
pub use request::MCPRequest;

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
#[derive(Debug, Deserialize)]
pub struct MCPToolDefinition {
  pub name: String,
  pub description: Option<String>,
  #[serde(rename = "inputSchema")]
  pub input_schema: Value,
}

/**
 * Describe a MCP Resource.
 */
#[derive(Debug, Deserialize)]
pub struct MCPResourceDefinition {
  pub uri: String,
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
 * Wrapper of a MCP server process.
 */
pub struct MCPServerProcess {
  pub server_name: String,
  process: Child,
  stdin: ChildStdin,
  stdout: Arc<Mutex<BufReader<ChildStdout>>>,
  tool_cache: HashMap<String, MCPToolDefinition>,
  timeout_ms: u64,
}

impl MCPServerProcess {
  /**
   * Start and initialize a MCP Server.
   */
  pub fn start_and_init(name: &str, mcp_command: &MCPServerCommand) -> Result<Self, NahError> {
    let mut server_command = Command::new(&mcp_command.command);
    for arg in mcp_command.args.iter() {
      server_command.arg(&arg);
    }
    server_command.stdin(Stdio::piped());
    server_command.stdout(Stdio::piped());
    server_command.stderr(Stdio::null());
    let mut server_process = match server_command.spawn() {
      Ok(p) => p,
      Err(_e) => {
        return Err(NahError::mcp_server_process_launch_error(name));
      }
    };

    let stdin = server_process.stdin.take().unwrap();
    let stdout = server_process.stdout.take().unwrap();
    let stdout_reader = BufReader::new(stdout);

    let mut result = MCPServerProcess {
      server_name: name.to_string(),
      process: server_process,
      stdin,
      stdout: Arc::new(Mutex::new(stdout_reader)),
      tool_cache: HashMap::new(),
      timeout_ms: 5000,
    };

    let initialize_request =
      MCPRequest::initialize(&uuid::Uuid::new_v4().to_string(), "nah", "0.1");
    let response: MCPResponse = result.send_and_wait_for_response(initialize_request)?;
    let initialized_notification = MCPNotification::initialized();
    result.send_data(initialized_notification)?;

    println!(
      "Server initialized. Info: {:?}",
      response
        .result
        .unwrap()
        .as_object()
        .unwrap()
        .get("serverInfo")
    );
    Ok(result)
  }

  /**
   * Set communication timeout.
   *
   * Args:
   * * timeout_ms: timeout in milliseconds.
   */
  pub fn set_timeout(&mut self, timeout_ms: u64) {
    self.timeout_ms = timeout_ms;
  }

  /**
   * Send a piece of data to the MCP Server.
   */
  fn send_data<T>(&mut self, request: T) -> Result<(), NahError>
  where
    T: serde::Serialize,
  {
    let mut data = serde_json::to_string(&request).unwrap();
    data.push_str("\n");
    if self.stdin.write_all(&data.as_bytes()).is_err() {
      return Err(NahError::mcp_server_communication_error(&self.server_name));
    }
    if self.stdin.flush().is_err() {
      return Err(NahError::mcp_server_communication_error(&self.server_name));
    }
    Ok(())
  }

  /**
   * Load and deserialize a piece of data from the MCP Server.
   */
  fn receive_data<'b, T>(&mut self, buf: &'b mut String) -> Result<T, NahError>
  where
    T: serde::Deserialize<'b>,
  {
    let (tx, rx) = channel();
    let stdout_thread = self.stdout.clone();
    let server_name_copy = self.server_name.clone();

    thread::spawn(move || {
      let mut buf = String::new();
      let mut stdout = stdout_thread.lock().unwrap();
      if stdout.read_line(&mut buf).is_err() {
        let _ = tx.send(Err(NahError::mcp_server_communication_error(
          &server_name_copy,
        )));
        return;
      }
      let _ = tx.send(Ok(buf));
    });

    match rx.recv_timeout(Duration::from_millis(self.timeout_ms)) {
      Err(_) => {
        return Err(NahError::mcp_server_timeout(&self.server_name));
      }
      Ok(result) => match result {
        Ok(bstr) => *buf = bstr,
        Err(e) => {
          return Err(e);
        }
      },
    }

    let response_json = match buf.strip_suffix("\n") {
      Some(v) => v,
      None => buf,
    };
    match serde_json::from_str::<T>(response_json) {
      Ok(r) => Ok(r),
      Err(_e) => Err(NahError::mcp_server_invalid_response(&self.server_name)),
    }
  }

  /**
   * Send a MCP Request and wait for its response. This method will ignore all non-relevent messages for now.
   */
  pub fn send_and_wait_for_response(
    &mut self,
    request: MCPRequest,
  ) -> Result<MCPResponse, NahError> {
    let id = request.id.clone();
    self.send_data(request)?;
    let mut buf = String::new();
    loop {
      let incoming_msg = self.receive_data::<Value>(&mut buf)?;
      let incoming_obj = incoming_msg.as_object();
      if incoming_obj.is_none() {
        continue;
      }
      let incoming_data = incoming_obj.unwrap();
      match incoming_data.get("id").and_then(|v| v.as_str()) {
        None => {
          // Try to unpack the message as a notification
          match serde_json::from_value::<MCPNotification>(incoming_msg) {
            Ok(notif) => {
              self.process_notification(notif);
            }
            _ => {
              // Unknown message. Ignore it for now.
            }
          }
        }
        Some(incoming_id) => {
          if incoming_id == id {
            return match serde_json::from_value::<MCPResponse>(incoming_msg) {
              Ok(resp) => Ok(resp),
              Err(_e) => Err(NahError::mcp_server_invalid_response(&self.server_name)),
            };
          }
        }
      }
    }
  }

  /**
   * Handling incoming notification.
   */
  fn process_notification(&mut self, notification: MCPNotification) {
    eprintln!("Received notification, method ={}", notification.method);
    // TODO: process the notification
  }

  /**
   * Kill the process.
   */
  pub fn kill(&mut self) -> std::io::Result<()> {
    self.process.kill()
  }

  /**
   * Fetch the list of tools from the MCP Server.
   */
  pub fn fetch_tools(&mut self) -> Result<Vec<&MCPToolDefinition>, NahError> {
    let id: String = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::tools_list(&id);
    let response = self.send_and_wait_for_response(request)?;

    let result = match response.result {
      None => {
        return Err(match response.error {
          None => NahError::mcp_server_communication_error(&self.server_name),
          Some(err) => NahError::mcp_server_error(
            &self.server_name,
            &serde_json::to_string_pretty(&err).unwrap(),
          ),
        });
      }
      Some(res) => {
        let tools = match res
          .as_object()
          .and_then(|v| v.get("tools"))
          .and_then(|v| v.as_array())
        {
          None => {
            return Err(NahError::mcp_server_invalid_response(&self.server_name));
          }
          Some(t) => t,
        };

        self.tool_cache.clear();
        for item in tools.iter() {
          let tool: MCPToolDefinition = match serde_json::from_value(item.clone()) {
            Ok(t) => t,
            Err(_e) => {
              return Err(NahError::mcp_server_invalid_response(&self.server_name));
            }
          };
          self.tool_cache.insert(tool.name.clone(), tool);
        }
        self.tool_cache.values().collect()
      }
    };
    Ok(result)
  }

  /**
   * Get the definition of a given tool name. It may try to read the tool from cached results.
   */
  pub fn get_tool_definition(&mut self, tool_name: &str) -> Result<&MCPToolDefinition, NahError> {
    if self.tool_cache.contains_key(tool_name) {
      Ok(self.tool_cache.get(tool_name).unwrap())
    } else {
      // re-fetch tool list
      self.fetch_tools()?;
      match self.tool_cache.get(tool_name) {
        Some(p) => Ok(p),
        None => Err(NahError::invalid_value(&format!(
          "Invalid tool name: {}",
          tool_name
        ))),
      }
    }
  }

  /**
   * Call the tool and wait for the response. Return value is the result object.
   */
  pub fn call_tool(&mut self, tool_name: &str, args: &Value) -> Result<Value, NahError> {
    let id: String = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::tools_call(&id, tool_name, args);
    let response = self.send_and_wait_for_response(request)?;

    match response.result {
      Some(r) => Ok(r),
      None => Err(self.parse_response_error(&response)),
    }
  }

  /**
   * Fetch the list of available resources.
   */
  pub fn resources_list(&mut self) -> Result<Vec<MCPResourceDefinition>, NahError> {
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
          return Err(NahError::mcp_server_invalid_response(&self.server_name));
        }
        Ok(
          resources
            .unwrap()
            .iter()
            .map(|v| serde_json::from_value::<MCPResourceDefinition>(v.clone()))
            .filter_map(|r| match r {
              Ok(v) => Some(v),
              Err(_) => None,
            })
            .collect(),
        )
      }
      None => Err(self.parse_response_error(&response)),
    }
  }

  pub fn read_resources(&mut self, uri: &str) -> Result<Vec<MCPResourceContent>, NahError> {
    let id = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::resources_read(&id, uri);
    let response = self.send_and_wait_for_response(request)?;
    let contents = match response
      .result
      .as_ref()
      .and_then(|result| result.as_object())
      .and_then(|result_obj| result_obj.get("contents"))
      .and_then(|contents| contents.as_array())
    {
      Some(r) => r,
      None => return Err(self.parse_response_error(&response)),
    };

    Ok(
      contents
        .iter()
        .map(|v| serde_json::from_value::<MCPResourceContent>(v.clone()))
        .filter_map(|v| match v {
          Ok(r) => {
            if r.text.is_none() && r.blob.is_none() {
              None
            } else {
              Some(r)
            }
          }
          Err(_) => None,
        })
        .collect(),
    )
  }

  fn parse_response_error(&self, response: &MCPResponse) -> NahError {
    match &response.error {
      Some(e) => NahError::mcp_server_error(&self.server_name, &serde_json::to_string(e).unwrap()),
      None => NahError::mcp_server_error(&self.server_name, "unknown error"),
    }
  }
}
