mod config;
mod types;

use clap::Parser;
use config::load_mcp_servers_config;
use std::collections::HashMap;
use std::io::BufReader;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Read some lines of a file
#[derive(Debug, Parser)]
struct Cli {
  /// JSON config file that declares the `mcpServers` field.
  mcp_config_file: PathBuf,
}

fn main() {
  let args = Cli::parse();
  println!("Config file: {:?}", args.mcp_config_file);
  let data = load_mcp_servers_config(args.mcp_config_file).unwrap();
  println!("Found servers:");
  for server in data.keys() {
    println!(" - {}", server);
  }

  // let mut server_processes : HashMap<String, std::process::Child> = HashMap::new();
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
    server_process.kill().expect("Failed to stop the server");
  }
}
