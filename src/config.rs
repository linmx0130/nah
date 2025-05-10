use crate::mcp::MCPServerCommand;
use crate::types::NahError;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug)]
pub struct NahConfig {
  pub mcp_servers: HashMap<String, MCPServerCommand>,
  pub model: Option<ModelConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
  #[serde(rename = "baseUrl")]
  pub base_url: String,
  pub model: String,
  #[serde(rename = "authToken")]
  pub auth_token: String,
}

/**
 * Load Nah config file.
 *
 * MCP Server config part follow the format follows Claude desktop app.
 */
pub fn load_config(path: PathBuf) -> Result<NahConfig, NahError> {
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

  let mut mcp_servers = HashMap::new();
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
        mcp_servers.insert(key.to_string(), server_command);
      }
    }
    _ => {
      return Err(NahError::io_error(&format!(
        "Invalid mcp server config file {}",
        path.display()
      )))
    }
  }
  let model =
    data.as_object().and_then(|obj| obj.get("model")).and_then(
      |model| match serde_json::from_value::<ModelConfig>(model.clone()) {
        Ok(v) => Some(v),
        Err(e) => {
          println!("{:?}", e);
          None
        }
      },
    );
  Ok(NahConfig { mcp_servers, model })
}
