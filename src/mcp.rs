/**
 * Data structure and utilities to handle Model Context Protocol.
 */
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
  io::{BufRead, BufReader, Write},
  process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
use uuid::Uuid;

use crate::types::NahError;
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
}

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
 * MCP Notification is a JSON-RPC request without id.
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct MCPNotification {
  jsonrpc: String,
  pub method: String,
  pub params: Option<Value>,
}

impl MCPNotification {
  pub fn initialized() -> Self {
    MCPNotification {
      jsonrpc: "2.0".to_owned(),
      method: "notifications/initialized".to_owned(),
      params: None,
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
#[derive(Debug, Deserialize)]
pub struct MCPToolDefinition {
  pub name: String,
  pub description: Option<String>,
  #[serde(rename = "inputSchema")]
  pub input_schema: Value,
  pub annotaions: Option<Value>,
}

/**
 * Wrapper of a MCP server process.
 */
pub struct MCPServerProcess {
  pub server_name: String,
  process: Child,
  stdin: ChildStdin,
  stdout: BufReader<ChildStdout>,
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
      stdout: stdout_reader,
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
    if self.stdout.read_line(buf).is_err() {
      return Err(NahError::mcp_server_communication_error(&self.server_name));
    }
    let response = serde_json::from_str::<T>(buf.strip_suffix("\n").unwrap()).unwrap();
    Ok(response)
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
      let incoming_data: Value = self.receive_data(&mut buf)?;
      match incoming_data
        .as_object()
        .and_then(|obj| obj.get("id"))
        .and_then(|v| v.as_str())
      {
        None => {
          // Not a response. Ignore for now.
        }
        Some(incoming_id) => {
          if incoming_id == id {
            return match serde_json::from_value::<MCPResponse>(incoming_data) {
              Ok(resp) => Ok(resp),
              Err(_e) => Err(NahError::mcp_server_invalid_response(&self.server_name)),
            };
          }
        }
      }
    }
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
  pub fn fetch_tools(&mut self) -> Result<Vec<MCPToolDefinition>, NahError> {
    let id = Uuid::new_v4().to_string();
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

        let mut result = Vec::with_capacity(tools.len());
        for item in tools.iter() {
          let tool: MCPToolDefinition = match serde_json::from_value(item.clone()) {
            Ok(t) => t,
            Err(_e) => {
              return Err(NahError::mcp_server_invalid_response(&self.server_name));
            }
          };
          result.push(tool);
        }
        result
      }
    };
    Ok(result)
  }
}
