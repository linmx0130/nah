mod types;

use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use types::{MCPServerCommand, NahError};
use std::process::{Command, Stdio};
use std::io::{Read, Write, BufRead};
use std::cell::Ref;

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
    let data = load_mcp_servers_config(args.mcp_config_file).unwrap();
    println!("Found servers:");
    for server in data.keys() {
        println!(" - {}", server);
    }
    
    let mut server_processes : HashMap<String, std::process::Child> = HashMap::new();
    for (server_name, command) in data.iter() {
        println!("Launching server: {}", server_name);
        let mut server_command = Command::new(&command.command);
        for arg in command.args.iter() {
            server_command.arg(&arg);
        }
        server_command.stdin(Stdio::piped());
        server_command.stdout(Stdio::piped());
        // server_command.stdout(Stdio::inherit());

        let mut server_process = server_command.spawn().unwrap();
        let mut child_stdin = server_process.stdin.take().unwrap();
        child_stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"id\":1, \"params\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"clientInfo\":{\"name\":\"nah\",\"version\":\"0.1\"}}}\n").unwrap();
        child_stdin.flush().unwrap();

        let mut child_stdout = server_process.stdout.take().unwrap();
        let mut child_stdout_reader = BufReader::new(child_stdout);
        let mut buf = String::new();
        println!("Waiting for result");
        child_stdout_reader.read_line(&mut buf).unwrap();
        println!("{:?}", buf);
        
        println!("Termiante the server...");
        server_process.kill();
    }

}
