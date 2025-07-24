/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::mcp::MCPLocalServerCommand;
use crate::mcp::MCPRemoteServerConfig;
use crate::types::NahError;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug)]
pub struct NahConfig {
  pub mcp_servers: HashMap<String, MCPLocalServerCommand>,
  pub mcp_remote_servers: HashMap<String, MCPRemoteServerConfig>,
  pub model: Option<ModelConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
  #[serde(rename = "baseUrl")]
  pub base_url: String,
  pub model: String,
  #[serde(rename = "authToken")]
  pub auth_token: Option<String>,
  #[serde(rename = "extraParams")]
  pub extra_params: Option<Value>,
  #[serde(rename = "systemPrompt")]
  pub system_prompt: Option<String>,
}

/**
 * Load Nah config file.
 *
 * MCP Server config part follow the format follows Claude desktop app.
 */
pub fn load_config(path: PathBuf) -> Result<NahConfig, NahError> {
  let file = match File::open(&path) {
    Ok(f) => f,
    Err(e) => {
      return Err(NahError::io_error(
        &format!("Failed to open {}", path.display()),
        Some(Box::new(e)),
      ));
    }
  };

  let reader = BufReader::new(file);
  let data: Value = match serde_json::from_reader(reader) {
    Ok(v) => v,
    Err(e) => {
      return Err(NahError::io_error(
        &format!("Invalid mcp server config file {}", path.display()),
        Some(Box::new(e)),
      ))
    }
  };

  let (mcp_servers, mcp_remote_servers) = load_mcp_servers(&data, &path)?;
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
  Ok(NahConfig {
    mcp_servers,
    mcp_remote_servers,
    model,
  })
}

fn load_mcp_servers(
  data: &Value,
  path: &PathBuf,
) -> Result<
  (
    HashMap<String, MCPLocalServerCommand>,
    HashMap<String, MCPRemoteServerConfig>,
  ),
  NahError,
> {
  let mut mcp_servers = HashMap::new();
  let mut mcp_remote_servers = HashMap::new();
  match &data["mcpServers"] {
    Value::Object(servers) => {
      for (key, value) in servers.iter() {
        if value.as_object().is_some_and(|v| v.contains_key("command")) {
          let server_command = match serde_json::from_value(value.clone()) {
            Ok(v) => v,
            Err(e) => {
              return Err(NahError::invalid_value(
                &format!("invalid server command for tool {}", key),
                Some(Box::new(e)),
              ))
            }
          };
          mcp_servers.insert(key.to_string(), server_command);
        } else if value.as_object().is_some_and(|v| v.contains_key("url")) {
          let remote_server_config = match serde_json::from_value(value.clone()) {
            Ok(v) => v,
            Err(e) => {
              return Err(NahError::invalid_value(
                &format!("invalid server command for tool {}", key),
                Some(Box::new(e)),
              ))
            }
          };
          mcp_remote_servers.insert(key.to_string(), remote_server_config);
        } else {
          return Err(NahError::invalid_value(
            &format!("invalid server command for tool {}", key),
            None,
          ));
        }
      }
      Ok((mcp_servers, mcp_remote_servers))
    }
    _ => Err(NahError::io_error(
      &format!("Invalid mcp server config file {}", path.display()),
      None,
    )),
  }
}

#[cfg(test)]
mod tests {
  use super::load_mcp_servers;
  use serde_json::Value;
  use std::path::PathBuf;

  #[test]
  fn test_load_mcp_servers() {
    let test_data = r#"
      {
        "mcpServers": {
          "weather": {
            "command": "uv",
            "args": ["run", "weather.py"]
          },
          "huggingface": {
            "url": "https://huggingface.co/mcp",
            "headers": {
              "Authorization": "Bearer HF_TOKEN"
            }
          }
        }
      }"#;
    let test_value: Value = serde_json::from_str(test_data).unwrap();

    let (mcp_servers, mcp_remote_servers) = load_mcp_servers(&test_value, &PathBuf::new()).unwrap();
    assert_eq!(mcp_servers.len(), 1);
    assert!(mcp_servers.contains_key("weather"));
    assert!(mcp_remote_servers.contains_key("huggingface"));
    assert!(mcp_remote_servers["huggingface"]
      .headers
      .contains_key("Authorization"));
  }
}
