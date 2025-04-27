mod config;
mod json_schema;
mod mcp;
mod types;

use std::collections::HashMap;
use clap::Parser;
use config::load_mcp_servers_config;
use mcp::MCPServerProcess;
use std::path::PathBuf;

/// Read some lines of a file
#[derive(Debug, Parser)]
struct Cli {
  /// JSON config file that declares the `mcpServers` field.
  mcp_config_file: PathBuf,
}

fn main() -> std::io::Result<()>{
  let args = Cli::parse();
  println!("Config file: {:?}", args.mcp_config_file);
  let data = load_mcp_servers_config(args.mcp_config_file).unwrap();
  println!("Found servers:");
  for server in data.keys() {
    println!(" - {}", server);
  }

  let mut server_processes : HashMap<String, MCPServerProcess> = HashMap::new();
  for (server_name, command) in data.iter() {
    println!("Launching server: {}", server_name);

    let mut process = MCPServerProcess::start_and_init(server_name, command).unwrap();
    let tools = process.fetch_tools().unwrap();

    println!("Available tools:");
    for item in tools.iter() {
      println!(" - {}", item.name);
    }
    server_processes.insert(server_name.to_owned(), process);
  }

  println!("Terminate MCP servers..");
  server_processes.iter_mut().for_each(|(name, server)| {
    if server.kill().is_err() {
        println!("Failed to terminate server: {}", name);
    }
  });

  Ok(())
}
