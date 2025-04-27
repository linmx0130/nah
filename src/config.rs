use crate::mcp::MCPServerCommand;
use crate::types::NahError;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

/**
 * Load MCP servers config file. The format follows Claude desktop app.
 */
pub fn load_mcp_servers_config(
  path: PathBuf,
) -> Result<HashMap<String, MCPServerCommand>, NahError> {
  let file = match File::open(&path) {
    Ok(f) => f,
    Err(_) => {
      return Err(NahError::io_error(&format!(
        "Failed to open {}",
        path.display()
      )))
    }
  };

  let reader = BufReader::new(file);
  let data: Value = match serde_json::from_reader(reader) {
    Ok(v) => v,
    Err(_) => {
      return Err(NahError::io_error(&format!(
        "Invalid mcp server config file {}",
        path.display()
      )))
    }
  };

  let mut result = HashMap::new();
  match &data["mcpServers"] {
    Value::Object(servers) => {
      for (key, value) in servers.iter() {
        let server_command = match serde_json::from_value(value.clone()) {
          Ok(v) => v,
          Err(_) => {
            return Err(NahError::invalid_value(&format!(
              "invalid server command for tool {}",
              key
            )))
          }
        };
        result.insert(key.to_string(), server_command);
      }
    }
    _ => {
      return Err(NahError::io_error(&format!(
        "Invalid mcp server config file {}",
        path.display()
      )))
    }
  }
  Ok(result)
}
