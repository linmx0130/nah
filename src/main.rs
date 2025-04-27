mod config;
mod json_schema;
mod mcp;
mod types;

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

    let mut process = MCPServerProcess::start_and_init(server_name, command).unwrap();
    let tools = process.fetch_tools().unwrap();

    println!("Available tools:");
    for item in tools.iter() {
      println!(" - {}", item.name);
      println!("   input schema: {:?}", item.input_schema);
      println!(
        "   input template: \n{}",
        json_schema::create_instance_template(&item.input_schema).unwrap()
      )
    }

    println!("Termiante the server...");
    process.kill().unwrap();
  }
}
