/**
 * Data structure and utilities to handle Model Context Protocol.
 */
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
  cell::{Ref, RefCell},
  io::{BufRead, BufReader, Stdin, Stdout, Write},
  process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use crate::{mcp, types::NahError};
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
 * Wrapper of a MCP server process.
 */
pub struct MCPServerProcess {
  pub server_name: String,
  pub process: Child,
  pub stdin: ChildStdin,
  pub stdout: BufReader<ChildStdout>,
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

    let initialize_request = MCPRequest::initialize("1", "nah", "0.1");
    result.send_data(initialize_request)?;
    let initialized_notification = MCPNotification::initialized();
    let mut buf = String::new();
    let response: MCPResponse = result.receive_data(&mut buf)?;
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
  pub fn send_data<T>(&mut self, request: T) -> Result<(), NahError>
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
  pub fn receive_data<'b, T>(&mut self, buf: &'b mut String) -> Result<T, NahError>
  where
    T: serde::Deserialize<'b>,
  {
    if self.stdout.read_line(buf).is_err() {
      return Err(NahError::mcp_server_communication_error(&self.server_name));
    }
    let response = serde_json::from_str::<T>(buf.strip_suffix("\n").unwrap()).unwrap();
    Ok(response)
  }
}
