/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use core::time;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::thread::sleep;
use std::time::SystemTime;

use crate::editor::launch_editor;
use crate::types::NahError;
use crate::AppContext;
use crate::ModelConfig;
use bytes::Bytes;
use nah_chat::{ChatClient, ChatMessage, ChatResponseChunk, ToolCallRequest};
use reqwest::RequestBuilder;
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use tokio::runtime::{Builder, Runtime};

#[derive(Debug)]
struct ChatContext {
  tools: Vec<Value>,
  tool_name_to_server_map: HashMap<String, String>,
  model_config: ModelConfig,
  messages: Vec<ChatMessage>,
  tokio_runtime: Runtime,
  chat_client: ChatClient,
  history_file: File,
}
const MESSAGE_FILE_PATH: &'static str = ".nah_user_message";

pub fn process_chat(context: &mut AppContext) {
  let (tools, tool_name_to_server_map) = pull_tools(context).unwrap();
  let model_config = context.model_config.clone().unwrap();
  let timestamp = std::time::SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .unwrap()
    .as_secs();
  let mut history_file_path = context.history_path.clone();
  history_file_path.push(format!("chat_{}.jsonl", timestamp));
  let history_file = OpenOptions::new()
    .create(true)
    .append(true)
    .open(history_file_path)
    .unwrap();
  let chat_client = ChatClient::init(
    model_config.base_url.to_owned(),
    model_config.auth_token.to_owned(),
  );
  let mut chat_context = ChatContext {
    tools,
    tool_name_to_server_map,
    model_config,
    messages: Vec::new(),
    tokio_runtime: Builder::new_current_thread()
      .enable_io()
      .enable_time()
      .build()
      .unwrap(),
    chat_client,
    history_file,
  };
  chat_context
    .model_config
    .system_prompt
    .as_ref()
    .and_then(|sys| {
      chat_context.messages.push(ChatMessage {
        role: "system".to_string(),
        content: sys.to_string(),
        reasoning_content: None,
        tool_call_id: None,
        tool_calls: None,
      });
      Some(())
    });
  println!("Chat with model: {}", chat_context.model_config.model);

  let mut rl = rustyline::DefaultEditor::new().unwrap();
  loop {
    println!("Press [ENTER] to draft user message, `exit` to end this chat.");
    let command = rl.readline("[chat]>> ");
    match command {
      Ok(cmd) => {
        let trimed_cmd = cmd.trim();
        if trimed_cmd.len() == 0 {
          let message = match launch_editor_for_user_message() {
            Ok(l) => l,
            Err(e) => {
              println!("Error: {}", e.message);
              break;
            }
          };
          println!("[User]: {}", message);
          chat_context.user_message(message);
          let mut loop_end: bool;
          loop {
            match chat_context.generate() {
              Err(e) => {
                println!("Error: {}", e);
                println!("Retry after 30 seconds...");
                sleep(time::Duration::from_secs(30));
                loop_end = false;
              }
              Ok(msg) => {
                if msg.tool_calls.is_some() {
                  match chat_context.process_tool_calls(context) {
                    Err(e) => {
                      if e.code == 201 {
                        loop_end = true;
                      } else {
                        println!("Error happened during tool calls, may have wrong result!");
                        println!("Error message: {}", e.message);
                        loop_end = false;
                      }
                    }
                    Ok(()) => loop_end = false,
                  }
                } else {
                  if msg.reasoning_content.is_some() {
                    println!(
                      "[Assistant Reasoning]: {}",
                      msg.reasoning_content.as_ref().unwrap()
                    );
                  }
                  println!("[Assistant]: {}", msg.content);
                  loop_end = true;
                }
              }
            }
            if loop_end {
              break;
            }
          }
        } else if trimed_cmd == "exit" {
          break;
        }
      }
      Err(rustyline::error::ReadlineError::Interrupted) => break,
      Err(rustyline::error::ReadlineError::Eof) => break,
      Err(_) => {
        println!("Unexpected error. Terminated");
        break;
      }
    }
  }
  // Have a try to remove the user message draft file.
  let _ = fs::remove_file(MESSAGE_FILE_PATH);
}

/**
 * Pull all available tools from the app context as the tools parameter accepted by OpenAI API.
 */
fn pull_tools(context: &mut AppContext) -> Result<(Vec<Value>, HashMap<String, String>), NahError> {
  let mut result = Vec::new();
  let mut name_map = HashMap::new();
  for (server_name, server_process) in context.server_processes.iter_mut() {
    let tools = server_process.fetch_tools()?;
    for item in tools {
      let new_name = format!("{}_{}", server_name, item.name);
      let function_tool_object = json!({
        "type": "function",
        "function": {
          "name": new_name.to_owned(),
          "description": item.description.to_owned(),
          "parameters": item.input_schema.clone(),
          "strict": false
        }
      });
      result.push(function_tool_object);
      name_map.insert(new_name, server_name.to_string());
    }
  }
  Ok((result, name_map))
}

impl ChatContext {
  /**
   * Append a user message to the chat context.
   */
  pub fn user_message(&mut self, message: String) {
    self.push_message(ChatMessage {
      role: "user".to_string(),
      content: message,
      reasoning_content: None,
      tool_call_id: None,
      tool_calls: None,
    });
  }

  /**
   * Generate assistant message.
   */
  pub fn generate(&mut self) -> Result<&ChatMessage, NahError> {
    let req_stream = self.get_generate_request();
    let message: Result<ChatMessage, NahError> = self.tokio_runtime.block_on(async {
      let mut res = match req_stream.send().await {
        Ok(r) => r,
        Err(e) => {
          return Err(NahError::model_invalid_response(
            &self.model_config.model,
            Some(Box::new(e)),
          ))
        }
      };
      if !res.status().is_success() {
        let code = res.status().as_u16();
        let error_content = res.text().await.unwrap();
        return Err(NahError::model_error(
          &self.model_config.model,
          &format!(
            "Model server responded with error: HTTP status {}, error message = {}",
            code, error_content
          ),
          None,
        ));
      }
      let mut message = ChatMessage::new();
      let mut reach_done = false;
      let mut chunk_received = 0usize;
      print!("Model is responding ...");
      let _ = std::io::stdout().flush();
      while !reach_done {
        let chunk = match res.chunk().await {
          Ok(Some(chunk)) => chunk,
          Ok(None) => continue,
          Err(e) => {
            return Err(NahError::model_invalid_response(
              &self.model_config.model,
              Some(Box::new(e)),
            ))
          }
        };
        let delta = self.get_model_response_chunk(chunk);
        match delta {
          Some(ChatResponseChunk::Delta(d)) => {
            message.apply_model_response_chunk(d);
            chunk_received += 1;
            print!(
              "\rModel is responding ... {} chunks received.",
              chunk_received
            );
            let _ = std::io::stdout().flush();
          }
          Some(ChatResponseChunk::Done) => {
            reach_done = true;
            println!("\nModel finished generation!");
          }
          None => {}
        }
      }
      Ok(message)
    });

    match message {
      Ok(msg) => {
        self.push_message(msg);
        Ok(&self.messages[self.messages.len() - 1])
      }
      Err(e) => Err(NahError::model_invalid_response(
        &self.model_config.model,
        Some(Box::new(e)),
      )),
    }
  }

  fn get_generate_request(&self) -> RequestBuilder {
    let mut params = HashMap::from([
      ("max_token".to_owned(), json!(4096)),
      ("tools".to_owned(), json!(self.tools.clone())),
      ("n".to_owned(), json!(1)),
      ("temperature".to_owned(), json!(0.7)),
      ("top_p".to_owned(), json!(0.9)),
      ("frequency_penalty".to_owned(), json!(0.5)),
    ]);

    self
      .model_config
      .extra_params
      .as_ref()
      .and_then(|v| v.as_object())
      .and_then(|extra_params| {
        extra_params.iter().for_each(|(key, value)| {
          params.insert(key.to_owned(), value.to_owned());
        });
        Some(())
      });

    self.chat_client.create_chat_completion_request(
      &self.model_config.model,
      &self.messages,
      true,
      &params,
    )
  }

  /**
   * Parse the stream data from the stream chat completion API to obtain a chunk delta.
   */
  fn get_model_response_chunk(&self, chunk: Bytes) -> Option<ChatResponseChunk> {
    let data_str = match String::from_utf8(chunk.to_vec()) {
      Ok(v) => v,
      Err(_) => {
        return None;
      }
    };
    if !data_str.starts_with("data: ") {
      return None;
    }
    if data_str.starts_with("data: [DONE]") {
      return Some(ChatResponseChunk::Done);
    }
    let trim_data = data_str.strip_prefix("data: ").unwrap().trim();
    let chunk_value: Value = match serde_json::from_str(trim_data) {
      Ok(v) => v,
      Err(_) => return None,
    };
    let delta_value = chunk_value
      .as_object()
      .and_then(|chunk| chunk.get("choices"))
      .and_then(|choices_value| choices_value.as_array())
      .and_then(|choices_arr| choices_arr.get(0))
      .and_then(|choice_value| choice_value.as_object())
      .and_then(|choice_obj| choice_obj.get("delta"));

    delta_value.and_then(|v| match serde_json::from_value(v.to_owned()) {
      Ok(v) => Some(ChatResponseChunk::Delta(v)),
      Err(_) => None,
    })
  }

  fn process_tool_calls(&mut self, app: &mut AppContext) -> Result<(), NahError> {
    let mut tool_call_responses = Vec::new();
    {
      let last_message = &self.messages[self.messages.len() - 1];
      let tool_calls: &Vec<ToolCallRequest> = last_message.tool_calls.as_ref().unwrap();
      for item in tool_calls {
        println!(
          "[Assistant - tool call request] {}({})",
          item.function.name, item.function.arguments
        );
        let server_name = match self.tool_name_to_server_map.get(&item.function.name) {
          Some(n) => n,
          None => {
            return Err(NahError::model_invalid_response(
              &self.model_config.model,
              None,
            ))
          }
        };
        let tool_name = item
          .function
          .name
          .strip_prefix(&format!("{}_", server_name))
          .unwrap();
        let args: Value = match serde_json::from_str(&item.function.arguments) {
          Ok(args) => args,
          Err(e) => {
            return Err(NahError::invalid_argument_error(
              "Argument should be a JSON Object, but received an invalid value from the model!",
              Some(Box::new(e)),
            ));
          }
        };

        let server = app.server_processes.get_mut(server_name).unwrap();
        let tool_definition = server.get_tool_definition(tool_name)?;
        if tool_definition.is_destructive() {
          if !crate::utils::ask_for_user_confirmation(
              &format!("Model requests to call tool {}, which is annotated as destructive. Do you still want to call? [N/y] > ", tool_definition.name),
                  &format!("Cancel the tool call!")
            ) {
              return Err(NahError::user_cancel_request());
            }
        }
        let tool_result = server.call_tool(&tool_name, &args)?;
        let text_content = unpack_mcp_text_contents(server_name, &tool_result)?;
        println!("[Tool: {}]: {}", server_name, text_content);
        tool_call_responses.push(ChatMessage {
          role: "tool".to_owned(),
          content: text_content,
          reasoning_content: None,
          tool_call_id: Some(item.id.to_owned()),
          tool_calls: None,
        });
      }
    }
    for item in tool_call_responses {
      self.push_message(item);
    }
    Ok(())
  }

  fn push_message(&mut self, msg: ChatMessage) {
    let _ = self
      .history_file
      .write(serde_json::to_string(&msg).unwrap().as_bytes());
    let _ = self.history_file.write(b"\n");
    let _ = self.history_file.flush();
    self.messages.push(msg);
  }
}

fn unpack_mcp_text_contents(server_name: &str, result: &Value) -> Result<String, NahError> {
  let contents = match result
    .as_object()
    .and_then(|o| o.get("content"))
    .and_then(|c| c.as_array())
  {
    Some(p) => p,
    None => {
      return Err(NahError::mcp_server_invalid_response(server_name, None));
    }
  };
  let mut text = String::new();
  for item in contents.iter() {
    let text_part = item
      .as_object()
      .and_then(|o| o.get("text"))
      .and_then(|t| t.as_str());
    if text_part.is_some() {
      text.push_str(text_part.unwrap());
    }
  }
  Ok(text)
}

fn launch_editor_for_user_message() -> Result<String, NahError> {
  let _ = fs::remove_file(MESSAGE_FILE_PATH);
  match OpenOptions::new()
    .create(true)
    .write(true)
    .open(MESSAGE_FILE_PATH)
  {
    Ok(mut f) => {
      let _ = f.write(b"# Draft your message here. Lines start wtih # will be ignored.\n");
    }
    Err(e) => {
      return Err(NahError::io_error(
        "Failed to open the user message file.",
        Some(Box::new(e)),
      ));
    }
  }
  let _ = launch_editor(MESSAGE_FILE_PATH)?;
  let mut file = open_file_or_throw()?;
  let mut buf: String = String::new();
  match file.read_to_string(&mut buf) {
    Ok(_) => {}
    Err(e) => {
      return Err(NahError::io_error(
        "Failed to open the user message file.",
        Some(Box::new(e)),
      ));
    }
  }
  let mut message = String::new();
  buf
    .split('\n')
    .filter(|l| !l.starts_with('#'))
    .for_each(|l| {
      message.push_str(l);
      message.push('\n');
    });
  Ok(message)
}

fn open_file_or_throw() -> Result<File, NahError> {
  let file = match File::open(&MESSAGE_FILE_PATH) {
    Ok(f) => f,
    Err(e) => {
      return Err(NahError::io_error(
        "Failed to open the user message file",
        Some(Box::new(e)),
      ))
    }
  };
  Ok(file)
}
