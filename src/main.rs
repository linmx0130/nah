mod config;
mod json_schema;
mod mcp;
mod types;

use clap::Parser;
use config::load_mcp_servers_config;
use mcp::MCPServerProcess;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;
use types::NahError;

/// Read some lines of a file
#[derive(Debug, Parser)]
struct Cli {
  /// JSON config file that declares the `mcpServers` field.
  mcp_config_file: PathBuf,
  /// Path to store history records.
  #[arg(long, value_name = "PATH")]
  history_path: Option<PathBuf>,
}

/**
 * Global context for nah app.
 */
struct AppContext {
  pub server_processes: HashMap<String, MCPServerProcess>,
  pub current_server: Option<String>,
  pub history_path: PathBuf,
}

fn main() {
  let args = Cli::parse();
  println!("Config file: {:?}", args.mcp_config_file);
  let data = match load_mcp_servers_config(args.mcp_config_file) {
    Ok(d) => d,
    Err(e) => {
      println!("{}", e);
      return;
    }
  };
  println!("Found servers:");
  for server in data.keys() {
    println!(" - {}", server);
  }
  let timestamp = std::time::SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .unwrap()
    .as_secs();
  let history_path = match args.history_path {
    None => PathBuf::from(format!("nah_{}", timestamp)),
    Some(p) => p.clone(),
  };
  if std::fs::create_dir(history_path.clone()).is_err() {
    println!(
      "Failed to create history folder: {}",
      history_path.display()
    );
    return;
  } else {
    println!(
      "Nah communication history folder: {}",
      history_path.display()
    );
  }

  let mut context = AppContext {
    server_processes: HashMap::new(),
    current_server: None,
    history_path,
  };

  for (server_name, command) in data.iter() {
    println!("Launching server: {}", server_name);

    let process =
      match MCPServerProcess::start_and_init(server_name, command, &context.history_path) {
        Err(e) => {
          println!(
            "Fatal error while launching {}, give up this server.",
            server_name
          );
          println!("Error: {}", e);
          continue;
        }
        Ok(p) => p,
      };

    context
      .server_processes
      .insert(server_name.to_owned(), process);
  }

  let mut rl = rustyline::DefaultEditor::new().unwrap();
  loop {
    let prompt = match &context.current_server {
      Some(n) => format!("[{}] >> ", n),
      None => ">> ".to_owned(),
    };
    let inst = rl.readline(&prompt);
    match inst {
      Ok(command) => {
        if context.process_command(&command) {
          rl.add_history_entry(command).unwrap();
        }
      }
      Err(rustyline::error::ReadlineError::Interrupted) => {
        println!("Interrupted! To exit nah, type `exit`.");
      }
      Err(rustyline::error::ReadlineError::Eof) => break,
      Err(_) => {
        println!("Unexpected error. Terminated");
        break;
      }
    }
  }
  context.process_exit();
}

impl AppContext {
  /**
   * Process a command.
   */
  fn process_command(&mut self, command: &str) -> bool {
    let command_parts: Vec<&str> = command.split_ascii_whitespace().collect();
    match command_parts.first() {
      None => {
        // Empty input, not to append history
        false
      }
      Some(key) => {
        match *key {
          "help" => print_help(),
          "use" => self.process_use(&command_parts),
          "exit" => self.process_exit(),
          "list_tools" => self.process_list_tools(),
          "inspect_tool" => self.process_inspect_tool(&command_parts),
          "call_tool" => self.process_call_tool(&command_parts),
          "list_resources" => self.process_list_resources(),
          "inspect_resources" => self.process_inspect_resources(&command_parts),
          "read_resources" => self.process_read_resources(&command_parts),
          "list_prompts" => self.process_list_prompts(),
          "inspect_prompt" => self.process_inspect_prompt(&command_parts),
          "set_timeout" => self.process_set_timeout(&command_parts),
          _ => {
            println!("Invalid command: {}", key);
          }
        };
        true
      }
    }
  }

  fn process_use(&mut self, command_parts: &Vec<&str>) {
    if command_parts.len() != 2 {
      println!(
        "Usage of use:\n\
    >> use [server_name] \n\
MCP server of `server_name` will be used as the current server."
      );
      return;
    }
    let server_name = command_parts[1];
    if self.server_processes.contains_key(server_name) {
      self.current_server = Some(server_name.to_owned());
    } else {
      println!("Server {} not found. Available servers are:", server_name);
      for item in self.server_processes.keys() {
        println!("* {}", item);
      }
    }
  }

  fn process_exit(&mut self) {
    println!("Terminate MCP servers..");
    self.server_processes.iter_mut().for_each(|(name, server)| {
      if server.kill().is_err() {
        println!("Failed to terminate server: {}", name);
      }
    });
    std::process::exit(0);
  }

  fn process_list_tools(&mut self) {
    self.process_with_current_server(|_server_name, server_process| {
      let tools = match server_process.fetch_tools() {
        Ok(t) => t,
        Err(e) => {
          println!("Failed to fetch tool list: {}", e);
          return;
        }
      };

      for item in tools.iter() {
        println!(" * {}", item.name);
      }
    });
  }

  fn process_inspect_tool(&mut self, command_parts: &Vec<&str>) {
    if command_parts.len() != 2 {
      println!("Usage: inspect_tool [tool name]");
      return;
    }
    let tool_name = command_parts[1];
    self.process_with_current_server(|server_name, server_process| {
      let tool_def = server_process.get_tool_definition(tool_name);
      match tool_def {
        Ok(def) => {
          println!("{}", def.name);
          println!("======");
          def.description.as_ref().and_then(|desc| {
            println!("{desc}");
            println!("======");
            Some(())
          });
        }
        Err(e) => {
          println!(
            "Failed to load tool {} from server {} due to error: {}",
            tool_name, server_name, e
          );
        }
      }
    });
  }

  fn process_call_tool(&mut self, command_parts: &Vec<&str>) {
    if command_parts.len() != 2 {
      println!("Usage: call_tool [tool name]");
      return;
    }
    let tool_name = command_parts[1];
    self.process_with_current_server(|server_name, server_process| {
      let tool_def = server_process.get_tool_definition(tool_name);
      match tool_def {
        Ok(def) => {
          // create a temporary file for the request
          let param_template = json_schema::create_instance_template(&def.input_schema);
          match param_template {
            Ok(template) => {
              let temp_filename = format!(".nah_req.{}.args.js", tool_name);
              match File::create(&temp_filename).and_then(|file: File| {
                          let mut file = file;
                          file.write_all("// Please fill arguments for tool call here in JSON format. \n// Lines starts with '//' will be removed\n".as_bytes())?;
                          file.write_all(template.as_bytes())?;
                          file.flush()?;
                          Ok(())
                      }) {
                          Err(e) => println!("Failed to prepare file for argument template for tool calling {} > {} due to error: {}", server_name, tool_name, e),
                          Ok(()) => {
                              // load parameter and call tool
                              let launch_editor_outcome = launch_editor(&temp_filename);
                              if launch_editor_outcome.is_err() {
                                  println!("{}", launch_editor_outcome.unwrap_err());
                                  return;
                              }
                              let mut buf = String::new();
                              let mut file = File::open(&temp_filename).unwrap();
                              if file.read_to_string(&mut buf).is_err() {
                                  println!("Failed to load the argument file. Call tool operation is interrupted.");
                                  return;
                              }
                              let mut arg_json_buf = String::new();
                              buf.split('\n').filter(|l| !l.trim().starts_with("//")).for_each(|l| {
                                arg_json_buf.push_str(l);
                              });
                              match serde_json::from_str::<Value>(&arg_json_buf) {
                                  Err(_) => {
                                      println!("Provided argument is invalid in JSON Format. Call tool operation is interrupted.");
                                  }
                                  Ok(params) => {
                                      let result = server_process.call_tool(tool_name, &params);
                                      match result {
                                          Err(e) => {
                                              println!("Received error: {}", e);
                                          }
                                          Ok(result) => {
                                              println!("Result: \n{}\n", serde_json::to_string_pretty(&result).unwrap());
                                          }
                                      }
                                  }
                              }
                              if std::fs::remove_file(&temp_filename).is_err() {
                                  println!("Failed to clean up the temporary argument file.")
                              }
                          }
                      }
            }
            Err(e) => {
              println!(
                "Failed to prepare argment template for tool calling: {} > {} due to error: {}",
                server_name, tool_name, e
              );
            }
          }
        }
        Err(e) => {
          println!(
            "Failed to load tool {} from server {} due to error: {}",
            tool_name, server_name, e
          );
        }
      }
    });
  }

  fn process_list_resources(&mut self) {
    self.process_with_current_server(|_, server_process| {
      println!("Direct resources");
      match server_process.fetch_resources_list() {
        Ok(r) => {
          for item in r.iter() {
            println!(" * {}", item.uri.as_ref().unwrap());
          }
        }
        Err(e) => {
          println!("Failed to load resource list: {}", e);
        }
      };

      println!("Resource templates");
      match server_process.fetch_resource_templates_list() {
        Ok(r) => {
          for item in r.iter() {
            println!(" * {}", item.uri_template.as_ref().unwrap());
          }
        }
        Err(e) => {
          println!("Failed to load resource templates: {}", e);
        }
      };
    });
  }

  fn process_inspect_resources(&mut self, command_parts: &Vec<&str>) {
    if command_parts.len() != 2 {
      println!("Usage: read_resource [Resource URI]");
      return;
    }
    let uri = command_parts[1];
    self.process_with_current_server(|_, server_process| {
      match server_process.get_resources_definition(uri) {
        Ok(r) => {
          println!("Name: {}", r.name);
          println!("URI: {}", uri);
          let _ = r.size.as_ref().is_some_and(|v| {
            println!("Size: {}", v);
            true
          });
          let _ = r.mime_type.as_ref().is_some_and(|v| {
            println!("MIME Type: {}", v);
            true
          });
          let _ = r.description.as_ref().is_some_and(|v| {
            println!("======");
            println!("{}", v);
            println!("======");
            true
          });
        }
        Err(e) => {
          println!("Error: {}", e)
        }
      }
    });
  }

  fn process_read_resources(&mut self, command_parts: &Vec<&str>) {
    if command_parts.len() != 2 {
      println!("Usage: read_resource [Resource URI]");
      return;
    }
    let uri = command_parts[1];
    self.process_with_current_server(|_, server_process| {
      match server_process.read_resources(uri) {
        Ok(r) => {
          println!("Result: \n{}\n", serde_json::to_string_pretty(&r).unwrap());
        }
        Err(e) => {
          println!("Received error: {}", e);
        }
      }
    });
  }

  fn process_list_prompts(&mut self) {
    self.process_with_current_server(|_, server_process| {
      match server_process.fetch_prompts_list() {
        Ok(r) => {
          for item in r.iter() {
            println!("* {}", item.name);
          }
        }
        Err(e) => {
          println!("Failed to load prompt list: {}", e);
        }
      }
    });
  }

  fn process_inspect_prompt(&mut self, command_parts: &Vec<&str>) {
    if command_parts.len() != 2 {
      println!("Usage: inspect_prompt [Prompt name]");
      return;
    }
    let prompt_name = command_parts[1];
    self.process_with_current_server(|_, server_process| {
      match server_process.get_prompt_definition(prompt_name) {
        Ok(p) => {
          println!("Name: {}", p.name);
          let _ = p.description.as_ref().is_some_and(|desc| {
            println!("Description:\n  {}", desc);
            true
          });
          let _ = p.arguments.as_ref().is_some_and(|args| {
            if args.len() == 0 {
              false
            } else {
              println!("Args:");
              for arg in args.iter() {
                let mut desc = String::new();
                if arg.required.is_some_and(|v| v) {
                  desc.push_str("[REQUIRED] ");
                }
                let _ = arg.description.as_ref().is_some_and(|v| {
                  desc.push_str(v);
                  true
                });
                println!("  {}: {}", arg.name, desc);
              }
              true
            }
          });
        }
        Err(e) => {
          println!("Failed to load prompt {}: {}", prompt_name, e);
        }
      }
    });
  }

  fn process_set_timeout(&mut self, command_parts: &Vec<&str>) {
    if command_parts.len() != 2 {
      println!("Usage: set_timeout [timeout in milliseconds]");
      return;
    }
    let timeout_ms: u64 = match command_parts[1].parse() {
      Ok(t) => t,
      Err(_) => {
        println!("Timeout value must be an non-negative integer!");
        return;
      }
    };

    self.process_with_current_server(|server_name, server_process| {
      server_process.set_timeout(timeout_ms);
      println!(
        "Timeout for MCP server {} has been set to {}ms",
        server_name, timeout_ms
      );
    });
  }

  /**
   * Process a closure with the current server process as the parameter.
   * It will print out error message to ask users to select a server;
   */
  fn process_with_current_server<F>(&mut self, f: F)
  where
    F: Fn(&str, &mut MCPServerProcess),
  {
    match &self.current_server {
      Some(server_name) => {
        let server_process = self.server_processes.get_mut(server_name).unwrap();
        f(&server_name, server_process);
      }
      None => {
        println!("No server is selected. Run `use` command to select a server.")
      }
    }
  }
}

fn print_help() {
  println!(
    "\
Command list of nah: \n\
* use:               Select a MCP server to interactive with. \n\
* list_tools:        List all tools on the current server.\n\
* inspect_tool:      Inspect detailed info of a tool.\n\
* call_tool:         Call a tool on the current server.\n\
* list_resources:    List all resources on the current server\n\
* inspect_resources: Inspect detailed  info of a resource \n\
* read_resources:    Read resources with a URI\n\
* list_prompts:      List all prompts on the current server.\n\
* inspect_prompt:    Inspect detailed in of a prompt.\n\
* set_timeout:       Set communication timeout for the current server\n\
* exit:              Stop all server and exit nah."
  );
}

fn launch_editor(filename: &str) -> Result<(), NahError> {
  let editor = std::env::var("EDITOR").unwrap_or("vi".to_owned());
  match std::process::Command::new(editor)
    .arg(filename)
    .spawn()
    .unwrap()
    .wait()
  {
    Ok(exit_status) => {
      if exit_status.success() {
        Ok(())
      } else {
        Err(NahError::editor_error(&format!(
          "return value of the editor is {}",
          exit_status.code().unwrap_or(0)
        )))
      }
    }
    Err(e) => Err(NahError::editor_error(&format!("{}", e))),
  }
}
