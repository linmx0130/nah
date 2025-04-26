mod config;
mod mcp;
mod types;

use clap::Parser;
use config::load_mcp_servers_config;
use mcp::{MCPNotification, MCPRequest, MCPResponse};
use std::cell::RefCell;
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
    let child_stdin = RefCell::new(server_process.stdin.take().unwrap());
    let initialize_request = MCPRequest::initialize("1", "nah", "0.1");
    let data = serde_json::to_string(&initialize_request).unwrap();
    child_stdin.borrow_mut().write_all(data.as_bytes()).unwrap();
    child_stdin.borrow_mut().write(b"\n").unwrap();
    child_stdin.borrow_mut().flush().unwrap();

    let child_stdout = server_process.stdout.take().unwrap();
    let child_stdout_reader = RefCell::new(BufReader::new(child_stdout));
    let mut buf = String::new();
    println!("Waiting for result");
    child_stdout_reader
      .borrow_mut()
      .read_line(&mut buf)
      .unwrap();
    let response = serde_json::from_str::<MCPResponse>(buf.strip_suffix("\n").unwrap()).unwrap();
    let initialized_notification = MCPNotification::initialized();
    child_stdin
      .borrow_mut()
      .write_all(
        serde_json::to_string(&initialized_notification)
          .unwrap()
          .as_bytes(),
      )
      .unwrap();
    child_stdin.borrow_mut().write(b"\n").unwrap();
    child_stdin.borrow_mut().flush().unwrap();
    println!(
      "Server initialized. Info: {:?}",
      response
        .result
        .unwrap()
        .as_object()
        .unwrap()
        .get("serverInfo")
    );

    println!("Termiante the server...");
    server_process.kill().expect("Failed to stop the server");
  }
}
