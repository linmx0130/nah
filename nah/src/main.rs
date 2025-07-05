/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
mod chat;
mod config;
mod editor;
mod json_schema;
mod mcp;
mod types;
mod utils;

use clap::Parser;
use config::{load_config, ModelConfig};
use editor::launch_editor;
use mcp::{MCPLocalServerCommand, MCPLocalServerProcess, MCPServer};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;
use types::NahError;

use crate::mcp::{MCPHTTPServerConnection, MCPRemoteServerConfig};

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
  pub server_processes: HashMap<String, Box<dyn MCPServer>>,
  pub current_server: Option<String>,
  pub history_path: PathBuf,
  pub server_commands: HashMap<String, MCPLocalServerCommand>,
  pub remote_server_configs: HashMap<String, MCPRemoteServerConfig>,
  pub model_config: Option<ModelConfig>,
}

fn main() {
  let args = Cli::parse();
  println!("Config file: {:?}", args.mcp_config_file);
  let data = match load_config(args.mcp_config_file) {
    Ok(d) => d,
    Err(e) => {
      println!("{}", e);
      return;
    }
  };
  println!("Found servers:");
  for server in data.mcp_servers.keys() {
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
    server_commands: data.mcp_servers,
    remote_server_configs: data.mcp_remote_servers,
    model_config: data.model,
  };

  for (server_name, command) in context.server_commands.iter() {
    println!("Launching server: {}", server_name);

    let process =
      match MCPLocalServerProcess::start_and_init(server_name, command, &context.history_path) {
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
      .insert(server_name.to_owned(), Box::new(process));
  }

  for (server_name, config) in context.remote_server_configs.iter() {
    println!("Initializing remote server: {}", server_name);
    let conn = match MCPHTTPServerConnection::init(server_name, config) {
      Err(e) => {
        println!(
          "Fatal error while initializing {}, give up this server.",
          server_name
        );
        println!("Error: {}", e);
        continue;
      }
      Ok(p) => p,
    };
    context
      .server_processes
      .insert(server_name.to_owned(), Box::new(conn));
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
    let command_parts: Vec<&str> = split_command_into_parts(command);
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
          "list_servers" => self.process_list_servers(),
          "restart_server" => self.process_restart_server(&command_parts),
          "list_tools" => self.process_list_tools(),
          "inspect_tool" => self.process_inspect_tool(&command_parts),
          "call_tool" => self.process_call_tool(&command_parts),
          "list_resources" => self.process_list_resources(),
          "inspect_resources" => self.process_inspect_resources(&command_parts),
          "read_resources" => self.process_read_resources(&command_parts),
          "list_prompts" => self.process_list_prompts(),
          "inspect_prompt" => self.process_inspect_prompt(&command_parts),
          "get_prompt" => self.process_get_prompt(&command_parts),
          "set_timeout" => self.process_set_timeout(&command_parts),
          "chat" => self.process_chat(),
          _ => {
            println!("Invalid command: {}.", key);
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

  fn process_list_servers(&mut self) {
    self.server_processes.iter().for_each(|(name, _)| {
      println!("* {}", name);
    });
  }

  fn process_restart_server(&mut self, command_parts: &Vec<&str>) {
    let help_message = "Usage of use:\n\
    >> restart_server [server_name] \n\
MCP server of `server_name` will be restarted. If no `server_name` is provided, current server will be restarted.";
    if command_parts.len() > 2 {
      println!("{}", help_message);
      return;
    }
    let server_name = match command_parts.get(1) {
      Some(s) => *s,
      None => match &self.current_server {
        Some(s) => s,
        None => {
          println!("No current server is selected!");
          println!("{}", help_message);
          return;
        }
      },
    };

    let command = match self.server_commands.get(server_name) {
      Some(p) => p,
      None => {
        println!("MCP Server {} not found!", server_name);
        return;
      }
    };

    let process =
      match MCPLocalServerProcess::start_and_init(server_name, command, &self.history_path) {
        Err(e) => {
          println!(
            "Fatal error while launching {}, give up this server.",
            server_name
          );
          println!("Error: {}", e);
          return;
        }
        Ok(p) => p,
      };

    let old_process = self
      .server_processes
      .insert(server_name.to_owned(), Box::new(process));

    let _ = old_process.is_some_and(|mut p| p.kill().is_ok());
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
          def.description.as_ref().and_then(|desc| {
            println!("= Description =");
            println!("{desc}");
            println!("======");
            Some(())
          });
          def.annotations.as_ref().and_then(|annotations| {
            println!("= Annotations =");
            annotations.title.as_ref().and_then(|title| {
              println!("Title: {}", title);
              Some(())
            });
            annotations.read_only_hint.as_ref().and_then(|h| {
              println!("Read only hint: {}", h);
              Some(())
            });
            annotations.destructive_hint.as_ref().and_then(|h| {
              println!("Destructive hint: {}", h);
              Some(())
            });
            annotations.idempotent_hint.as_ref().and_then(|h| {
              println!("Idempotent hint: {}", h);
              Some(())
            });
            annotations.open_world_hint.as_ref().and_then(|h| {
              println!("Open world hint: {}", h);
              Some(())
            });
            println!("=====");
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
          if def.is_destructive() {
            if ! utils::ask_for_user_confirmation(
            &format!("Tool {} is annotated as destructive. Do you still want to call? [N/y] > ", def.name),
              &format!("Tool {} has not been called.", def.name),
            ) {
              return
            }
          }
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
                            if !json_schema::is_empty_object(&def.input_schema.as_object().and_then(|obj| obj.get("properties")).unwrap()) {
                              // load parameter and call tool
                              let launch_editor_outcome = launch_editor(&temp_filename);
                              if launch_editor_outcome.is_err() {
                                  println!("{}", launch_editor_outcome.unwrap_err());
                                  return;
                              }
                            } else {
                              println!("No argument is requested. Directly call the function...");
                            }
                            let arguments= match load_json_arguments(&temp_filename) {
                              Err(e) => {
                                println!("{}", e);
                                return;
                              },
                              Ok(v) => v
                            };
                            let result = server_process.call_tool(tool_name, &arguments);
                            match result {
                              Err(e) => {
                                println!("Received error: {}", e);
                              }
                              Ok(result) => {
                                println!("Result: \n{}\n", serde_json::to_string_pretty(&result).unwrap());
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

  fn process_get_prompt(&mut self, command_parts: &Vec<&str>) {
    if command_parts.len() != 2 {
      println!("Usage: get_prompt [Prompt name]");
      return;
    }
    let prompt_name = drop_quotes(command_parts[1]);
    self.process_with_current_server(|_, server_process| {
      let prompt_def = server_process.get_prompt_definition(&prompt_name);
      match prompt_def {
        Ok(def) => {
          match &def.arguments {
            Some(args) => {
              if args.len() > 0 {
                let temp_filename = format!(".nah_req.{}.args.js", prompt_name);
                let template_lines: Vec<String> = args.iter().map(|arg| format!("    \"{}\": \"<FILL ARGUMENT HERE>\"", arg.name)).collect();
                let write_template_result = File::create(&temp_filename).and_then(|file| {
                  let mut file = file;
                  file.write_all(b"// Please fill arguments for prompt call here in JSON format \n// Lines starts with '//' will be removed\n")?;
                  file.write_all(b"{\n")?;
                  for (idx, line) in template_lines.iter().enumerate() {
                    file.write_all(line.as_bytes())?;
                    if idx + 1 != template_lines.len() {
                      file.write_all(b",\n")?;
                    } else {
                      file.write_all(b"\n")?;
                    }
                  }
                  file.write_all(b"}\n")?;
                  Ok(())
                });
                if write_template_result.is_err() {
                  println!("Failed to prepare the argument template file for getting prompt {} due to error {}", prompt_name, write_template_result.err().unwrap());
                  return;
                }
                // load parameter and call tool
                let launch_editor_outcome = launch_editor(&temp_filename);
                if launch_editor_outcome.is_err() {
                  println!("{}", launch_editor_outcome.unwrap_err());
                  return;
                }
                let argument_value = load_json_arguments(&temp_filename);
                let arguments= match argument_value.and_then(|v| match v.as_object() {
                  Some(v) => Ok(v.clone()),
                  None => Err(NahError::invalid_argument_error("Arguments should be a JSON Object!"))
                }) {
                  Err(e) => {
                    println!("{}", e);
                    return;
                  },
                  Ok(v) => v
                };
                let args_map = arguments.iter().map(|(k, v)| {
                  (k.to_owned(), v.as_str().unwrap_or("").to_owned())
                }).collect();
                let result = server_process.get_prompt_content(&prompt_name, &args_map);
                match result {
                  Err(e) => {
                    println!("Received error: {}", e);
                  }
                  Ok(result) => {
                    println!("Result: \n{}\n", serde_json::to_string_pretty(&result).unwrap());
                  }
                };
                if std::fs::remove_file(&temp_filename).is_err() {
                  println!("Failed to clean up the temporary argument file.")
                }
              } else {
                println!("Prompt {} doesn't need arguments.", prompt_name);
              }
            },
            None => {
              println!("Prompt {} doesn't need arguments.", prompt_name);
            }
          };

        }
        Err(e) => {
          println!("Failed to prepare argument template for getting prompt {} due to error: {}", prompt_name, e);
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
    F: Fn(&str, &mut Box<dyn MCPServer>),
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

  fn process_chat(&mut self) {
    if self.model_config.is_none() {
      println!("No model is supplied! Please set model config.");
      return;
    }
    chat::process_chat(self)
  }
}

fn print_help() {
  println!(
    "\
Command list of nah: \n\
* use:               Select a MCP server to interactive with. \n\
* list_servers:      List all available MCP servers. \n\
* restart_server:    Restart a MCP server. \n\
* list_tools:        List all tools on the current server.\n\
* inspect_tool:      Inspect detailed info of a tool.\n\
* call_tool:         Call a tool on the current server.\n\
* list_resources:    List all resources on the current server\n\
* inspect_resources: Inspect detailed info of a resource \n\
* read_resources:    Read resources with a URI\n\
* list_prompts:      List all prompts on the current server.\n\
* inspect_prompt:    Inspect detailed in of a prompt.\n\
* get_prompt:        Get a prompt from current server.\n\
* set_timeout:       Set communication timeout for the current server\n\
* chat:              Chat with a LLM equiped with tools\n \
* exit:              Stop all server and exit nah."
  );
}

fn load_json_arguments(filename: &str) -> Result<Value, NahError> {
  let mut buf = String::new();
  let mut file = match File::open(&filename) {
    Ok(f) => f,
    Err(_) => return Err(NahError::io_error("Failed to open the argument file")),
  };
  if file.read_to_string(&mut buf).is_err() {
    return Err(NahError::io_error("Failed to open the argument file."));
  }
  let mut arg_json_buf = String::new();
  buf
    .split('\n')
    .filter(|l| !l.trim().starts_with("//"))
    .for_each(|l| {
      arg_json_buf.push_str(l);
    });
  match serde_json::from_str::<Value>(&arg_json_buf) {
    Err(_) => {
      return Err(NahError::invalid_argument_error(
        "Provided argument is invalid in JSON Format",
      ));
    }
    Ok(args) => Ok(args),
  }
}

fn split_command_into_parts(command: &str) -> Vec<&str> {
  let mut ret = Vec::new();
  let mut command_chars = command.chars();
  let mut last_pos = 0usize;
  let mut current_pos = 0usize;
  let mut in_string = false;
  let mut in_escape = false;
  let mut in_chunk = false;
  while current_pos < command.len() {
    let current_char = command_chars.next();
    current_pos += 1;
    match current_char {
      None => {
        break;
      }
      Some(c) => {
        if in_string {
          // if in string, continue to move forward until hit a non-escaping quote
          if c == '"' && !in_escape {
            in_string = false;
            in_chunk = false;
            ret.push(&command[last_pos..current_pos]);
            last_pos = current_pos
          } else if c == '\\' {
            in_escape = true;
          } else {
            in_escape = false;
          }
        } else {
          if in_chunk {
            if c.is_ascii_whitespace() {
              in_chunk = false;
              ret.push(&command[last_pos..current_pos - 1]);
              last_pos = current_pos;
            }
          } else {
            if c.is_ascii_whitespace() {
              last_pos = current_pos;
            } else if c == '"' {
              in_string = true;
              in_chunk = true;
              last_pos = current_pos - 1;
            } else {
              in_chunk = true;
              last_pos = current_pos - 1;
            }
          }
        }
      }
    }
  }
  if in_chunk {
    ret.push(&command[last_pos..current_pos]);
  }
  return ret;
}

fn drop_quotes(raw_content: &str) -> String {
  if !raw_content.starts_with("\"") {
    return raw_content.to_string();
  }
  let mut ret = String::new();
  let mut chars = raw_content.chars();
  chars.next();
  let mut in_escape = false;
  let mut c = chars.next();
  while c.is_some() {
    match c {
      Some('\\') => {
        in_escape = true;
      }
      Some('"') => {
        if in_escape {
          ret.push('"');
          in_escape = false;
        } else {
          break;
        }
      }
      Some(v) => {
        ret.push(v);
        in_escape = false;
      }
      None => {
        break;
      }
    }
    c = chars.next();
  }
  return ret;
}
