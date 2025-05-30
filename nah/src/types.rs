#[derive(Debug)]
pub struct NahError {
  pub code: i32,
  pub message: String,
}

impl std::fmt::Display for NahError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "NahError {}: {}", self.code, self.message)
  }
}

impl std::error::Error for NahError {}

impl NahError {
  pub fn io_error(message: &str) -> NahError {
    NahError {
      code: 1,
      message: format!("IO Error: {}", message),
    }
  }
  pub fn invalid_value(message: &str) -> NahError {
    NahError {
      code: 2,
      message: format!("Invalid value error: {}", message),
    }
  }

  pub fn mcp_server_communication_error(server_name: &str) -> NahError {
    NahError {
      code: 3,
      message: format!("MCP server communication error with {}", server_name),
    }
  }

  pub fn mcp_server_process_launch_error(server_name: &str) -> NahError {
    NahError {
      code: 4,
      message: format!("Failed to launch MCP server process: {}", server_name),
    }
  }

  pub fn mcp_server_error(server_name: &str, message: &str) -> NahError {
    NahError {
      code: 5,
      message: format!("Error from MCP Server {}: {}", server_name, message),
    }
  }

  pub fn mcp_server_invalid_response(server_name: &str) -> NahError {
    NahError {
      code: 6,
      message: format!("Received invalid response from MCP Server {}", server_name),
    }
  }

  pub fn received_invalid_json_schema(message: &str) -> NahError {
    NahError {
      code: 7,
      message: format!("Received invalid JSON Schema: {}", message),
    }
  }

  pub fn editor_error(message: &str) -> NahError {
    NahError {
      code: 8,
      message: format!("Failed on running editor: {}", message),
    }
  }

  pub fn mcp_server_timeout(server_name: &str) -> NahError {
    NahError {
      code: 9,
      message: format!("Timeout when communicating with MCP server {}", server_name),
    }
  }

  pub fn invalid_argument_error(message: &str) -> NahError {
    NahError {
      code: 10,
      message: format!("Failed to load arguments: {}", message),
    }
  }

  pub fn model_error(model_name: &str, message: &str) -> NahError {
    NahError {
      code: 101,
      message: format!("Error from model {}: {}", model_name, message),
    }
  }
  pub fn model_invalid_response(model_name: &str) -> NahError {
    NahError {
      code: 102,
      message: format!("Invalid response from model: {}", model_name),
    }
  }
}
