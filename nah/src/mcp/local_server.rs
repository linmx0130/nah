/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{
  io::{BufRead, BufReader, Write},
  process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use crate::mcp::MCPServer;
use crate::types::NahError;
use nah_mcp_types::notification;
use nah_mcp_types::request::MCPRequest;
use nah_mcp_types::*;
pub use notification::MCPNotification;

/**
 * Describes how to launch a MCP server with a command.
 */
#[derive(Debug, Deserialize)]
pub struct MCPLocalServerCommand {
  pub command: String,
  pub args: Vec<String>,
}
/**
 * Wrapper of a MCP local server process.
 */
pub struct MCPLocalServerProcess {
  pub server_name: String,
  history_file: File,
  process: Child,
  stdin: ChildStdin,
  stdout: Arc<Mutex<BufReader<ChildStdout>>>,
  tool_cache: HashMap<String, MCPToolDefinition>,
  resource_cache: HashMap<String, MCPResourceDefinition>,
  prompt_cache: HashMap<String, MCPPromptDefinition>,
  timeout_ms: u64,
}

impl MCPServer for MCPLocalServerProcess {
  fn send_and_wait_for_response(&mut self, request: MCPRequest) -> Result<MCPResponse, NahError> {
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
              Err(e) => Err(NahError::mcp_server_invalid_response(
                &self.server_name,
                Some(Box::new(e)),
              )),
            };
          }
        }
      }
    }
  }

  fn kill(&mut self) -> std::io::Result<()> {
    let _ = self.history_file.flush();
    self.process.kill()
  }

  fn set_timeout(&mut self, timeout_ms: u64) {
    self.timeout_ms = timeout_ms;
  }

  fn get_prompt_content(
    &mut self,
    prompt_name: &str,
    args: &HashMap<String, String>,
  ) -> Result<MCPPromptResult, NahError> {
    let id: String = uuid::Uuid::new_v4().to_string();
    let request = MCPRequest::get_prompt(
      &Value::String(id),
      prompt_name,
      args.into_iter().map(|(k, v)| (k.as_str(), v.as_str())),
    );
    let response = self.send_and_wait_for_response(request)?;

    match response.result {
      Some(r) => Ok(serde_json::from_value::<MCPPromptResult>(r).unwrap()),
      None => Err(self.parse_response_error(&response)),
    }
  }

  fn get_server_name(&self) -> &str {
    &self.server_name
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

impl MCPLocalServerProcess {
  /**
   * Start a MCP local server process.
   */
  pub fn start_and_init(
    name: &str,
    mcp_command: &MCPLocalServerCommand,
    history_path: &PathBuf,
  ) -> Result<Self, NahError> {
    let mut server_command = Command::new(&mcp_command.command);
    for arg in mcp_command.args.iter() {
      server_command.arg(&arg);
    }
    server_command.stdin(Stdio::piped());
    server_command.stdout(Stdio::piped());

    let mut history_file_path = history_path.clone();
    history_file_path.push(format!("{}.jsonl", name));
    let history_file = match OpenOptions::new()
      .create(true)
      .append(true)
      .open(history_file_path.as_path())
    {
      Ok(f) => f,
      Err(e) => {
        return Err(NahError::io_error(
          &format!(
            "Failed to create history file: {}",
            history_file_path.display(),
          ),
          Some(Box::new(e)),
        ));
      }
    };
    let mut stderr_file_path = history_path.clone();
    stderr_file_path.push(format!("{}.stderr", name));
    let stderr_file = match OpenOptions::new()
      .create(true)
      .append(true)
      .open(stderr_file_path.as_path())
    {
      Ok(f) => f,
      Err(e) => {
        return Err(NahError::io_error(
          &format!(
            "Failed to create stderr file: {}",
            stderr_file_path.display()
          ),
          Some(Box::new(e)),
        ));
      }
    };
    server_command.stderr(Stdio::from(stderr_file));

    let mut server_process = match server_command.spawn() {
      Ok(p) => p,
      Err(e) => {
        return Err(NahError::mcp_server_process_launch_error(
          name,
          Some(Box::new(e)),
        ));
      }
    };

    let stdin = server_process.stdin.take().unwrap();
    let stdout = server_process.stdout.take().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let mut result = MCPLocalServerProcess {
      server_name: name.to_string(),
      process: server_process,
      stdin,
      stdout: Arc::new(Mutex::new(stdout_reader)),
      tool_cache: HashMap::new(),
      resource_cache: HashMap::new(),
      prompt_cache: HashMap::new(),
      timeout_ms: 5000,
      history_file,
    };

    let initialize_request = MCPRequest::initialize(
      &Value::String(uuid::Uuid::new_v4().to_string()),
      "nah",
      "0.1",
    );
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
    let _ = self.history_file.write(data.as_bytes());
    if self.stdin.write_all(&data.as_bytes()).is_err() {
      return Err(NahError::mcp_server_communication_error(
        &self.server_name,
        None,
      ));
    }
    if self.stdin.flush().is_err() {
      return Err(NahError::mcp_server_communication_error(
        &self.server_name,
        None,
      ));
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
      match stdout.read_line(&mut buf) {
        Ok(_) => {}
        Err(e) => {
          let _ = tx.send(Err(e));
          return;
        }
      }
      let _ = tx.send(Ok(buf));
    });

    match rx.recv_timeout(Duration::from_millis(self.timeout_ms)) {
      Err(e) => {
        return Err(NahError::mcp_server_timeout(
          &self.server_name,
          Some(Box::new(e)),
        ));
      }
      Ok(result) => match result {
        Ok(bstr) => *buf = bstr,
        Err(e) => {
          return Err(NahError::mcp_server_communication_error(
            &server_name_copy,
            Some(Box::new(e)),
          ));
        }
      },
    }
    let _ = self.history_file.write(buf.as_bytes());

    let response_json = match buf.strip_suffix("\n") {
      Some(v) => v,
      None => buf,
    };
    match serde_json::from_str::<T>(response_json) {
      Ok(r) => Ok(r),
      Err(e) => Err(NahError::mcp_server_invalid_response(
        &self.server_name,
        Some(Box::new(e)),
      )),
    }
  }

  /**
   * Handling incoming notification.
   */
  fn process_notification(&mut self, notification: MCPNotification) {
    eprintln!("Received notification, method ={}", notification.method);
    // TODO: process the notification
  }
}
