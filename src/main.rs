mod types;

use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use types::{MCPServerCommand, NahError};

/// Read some lines of a file
#[derive(Debug, Parser)]
struct Cli {
    /// JSON config file that declares the `mcpServers` field.
    mcp_config_file: PathBuf,
}

fn load_mcp_servers_config(path: PathBuf) -> Result<HashMap<String, MCPServerCommand>, NahError> {
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

fn main() {
    let args = Cli::parse();
    println!("Config file: {:?}", args.mcp_config_file);
    let data = load_mcp_servers_config(args.mcp_config_file);
    println!("{:?}", data);
}
