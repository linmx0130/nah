use std::error::Error;

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#[derive(Debug)]
pub struct NahError {
  pub code: i32,
  pub message: String,
  pub source: Option<Box<dyn std::error::Error>>,
}

impl std::fmt::Display for NahError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.source() {
      None => {
        write!(f, "NahError {}: {}", self.code, self.message)
      }
      Some(e) => {
        write!(f, "NahError {}: {}, source: {}", self.code, self.message, e)
      }
    }
  }
}

impl Error for NahError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match &self.source {
      Some(v) => Some(v.as_ref()),
      None => None,
    }
  }
}

impl NahError {
  pub fn io_error(message: &str, source: Option<Box<dyn std::error::Error>>) -> NahError {
    NahError {
      code: 1,
      message: format!("IO Error: {}", message),
      source,
    }
  }
  pub fn invalid_value(message: &str, source: Option<Box<dyn std::error::Error>>) -> NahError {
    NahError {
      code: 2,
      message: format!("Invalid value error: {}", message),
      source,
    }
  }

  pub fn mcp_server_communication_error(
    server_name: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 3,
      message: format!("MCP server communication error with {}", server_name),
      source,
    }
  }

  pub fn mcp_server_process_launch_error(
    server_name: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 4,
      message: format!("Failed to launch MCP server process: {}", server_name),
      source,
    }
  }

  pub fn mcp_server_error(
    server_name: &str,
    message: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 5,
      message: format!("Error from MCP Server {}: {}", server_name, message),
      source,
    }
  }

  pub fn mcp_server_invalid_response(
    server_name: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 6,
      message: format!("Received invalid response from MCP Server {}", server_name),
      source,
    }
  }

  pub fn received_invalid_json_schema(
    message: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 7,
      message: format!("Received invalid JSON Schema: {}", message),
      source,
    }
  }

  pub fn editor_error(message: &str, source: Option<Box<dyn std::error::Error>>) -> NahError {
    NahError {
      code: 8,
      message: format!("Failed on running editor: {}", message),
      source,
    }
  }

  pub fn mcp_server_timeout(
    server_name: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 9,
      message: format!("Timeout when communicating with MCP server {}", server_name),
      source,
    }
  }

  pub fn invalid_argument_error(
    message: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 10,
      message: format!("Failed to load arguments: {}", message),
      source,
    }
  }

  pub fn model_error(
    model_name: &str,
    message: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 101,
      message: format!("Error from model {}: {}", model_name, message),
      source,
    }
  }
  pub fn model_invalid_response(
    model_name: &str,
    source: Option<Box<dyn std::error::Error>>,
  ) -> NahError {
    NahError {
      code: 102,
      message: format!("Invalid response from model: {}", model_name),
      source,
    }
  }
  pub fn user_cancel_request() -> NahError {
    NahError {
      code: 201,
      message: format!("Request was cancelled by the user."),
      source: None,
    }
  }
}
