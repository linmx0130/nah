use core::time;
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
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use tokio::runtime::{Builder, Runtime};

/**
 * Data structure of a chat message, could be from the user, the assistant or the tool.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
  pub role: String,
  pub content: String,
  pub tool_call_id: Option<String>,
  pub tool_calls: Option<Vec<ToolCallRequest>>,
}

/**
 * A chunk of chat message response from the assistant.
 */
#[derive(Debug, Clone)]
enum ChatResponseChunk {
  Delta(ChatResponseChunkDelta),
  Done,
}

/**
 * Chunk delta of chat message from the assistant.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatResponseChunkDelta {
  pub role: Option<String>,
  pub content: Option<String>,
  pub tool_calls: Option<Vec<ToolCallRequestChunkDelta>>,
}

/**
 * A tool call request. Only function call is supported now.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolCallRequest {
  pub id: String,
  #[serde(rename = "type")]
  pub _type: String,
  pub function: FunctionCallRequest,
}

/**
 * A tool call request chunk received from stream api.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolCallRequestChunkDelta {
  pub index: usize,
  pub id: Option<String>,
  #[serde(rename = "type")]
  pub _type: Option<String>,
  pub function: Option<FunctionCallRequestChunkDelta>,
}

/**
 * A function call request.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
struct FunctionCallRequest {
  pub name: String,
  pub arguments: String,
}

/**
 * A function call request chunk received from stream api.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
struct FunctionCallRequestChunkDelta {
  pub name: Option<String>,
  pub arguments: Option<String>,
}

#[derive(Debug)]
struct ChatContext {
  tools: Vec<Value>,
  model_config: ModelConfig,
  messages: Vec<ChatMessage>,
  tokio_runtime: Runtime,
  history_file: File,
}
const MESSAGE_FILE_PATH: &'static str = ".nah_user_message";

pub fn process_chat(context: &mut AppContext) {
  let tools = pull_tools(context).unwrap();
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
  let mut chat_context = ChatContext {
    tools,
    model_config,
    messages: Vec::new(),
    tokio_runtime: Builder::new_current_thread()
      .enable_io()
      .enable_time()
      .build()
      .unwrap(),
    history_file,
  };
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
                  if chat_context.process_tool_calls(context).is_err() {
                    println!("Error happened during tool calls, may have wrong result!");
                  }
                  loop_end = false;
                } else {
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
fn pull_tools(context: &mut AppContext) -> Result<Vec<Value>, NahError> {
  let mut result: Vec<Value> = Vec::new();
  for (server_name, server_process) in context.server_processes.iter_mut() {
    let tools = server_process.fetch_tools()?;
    for item in tools {
      let function_tool_object = json!({
        "type": "function",
        "function": {
          "name": format!("{}.{}", server_name, item.name),
          "description": item.description.to_owned(),
          "parameters": item.input_schema.clone(),
          "strict": false
        }
      });
      result.push(function_tool_object);
    }
  }
  Ok(result)
}

impl ChatContext {
  /**
   * Append a user message to the chat context.
   */
  pub fn user_message(&mut self, message: String) {
    self.push_message(ChatMessage {
      role: "user".to_string(),
      content: message,
      tool_call_id: None,
      tool_calls: None,
    });
  }

  /**
   * Generate assistant message.
   */
  pub fn generate(&mut self) -> Result<&ChatMessage, NahError> {
    let req_stream = self.get_generate_request(true);
    let message: Result<ChatMessage, reqwest::Error> = self.tokio_runtime.block_on(async {
      let mut res = req_stream.send().await?;
      let mut message = ChatMessage {
        role: "".to_owned(),
        content: "".to_owned(),
        tool_call_id: None,
        tool_calls: None,
      };
      let mut reach_done = false;
      let mut chunk_received = 0usize;
      print!("Model is responding ...");
      let _ = std::io::stdout().flush();
      while !reach_done {
        let chunk = match res.chunk().await? {
          Some(chunk) => chunk,
          None => continue,
        };
        let delta = self.get_model_response_chunk(chunk);
        match delta {
          Some(ChatResponseChunk::Delta(d)) => {
            self.apply_model_response_chunk(&mut message, d);
            chunk_received += 1;
            print!("\rModel is responding ... {} chunks received.", chunk_received);
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
        self.messages.push(msg);
        Ok(&self.messages[self.messages.len() - 1])
      }
      Err(_e) => Err(NahError::model_invalid_response(&self.model_config.model)),
    }
  }

  fn get_generate_request(&self, is_stream: bool) -> RequestBuilder {
    let mut data = json!({
        "model": self.model_config.model,
        "messages": self.messages.clone(),
        "stream": is_stream,
        "max_tokens": 4096,
        "tools": self.tools.clone(),
        "n": 1,
        "temperature": 0.7,
        "top_p": 0.9,
        "frequency_penalty": 0.5
    });

    self
      .model_config
      .extra_params
      .as_ref()
      .and_then(|v| v.as_object())
      .and_then(|extra_params| {
        extra_params.iter().for_each(|(key, value)| {
          data
            .as_object_mut()
            .and_then(|o| o.insert(key.to_owned(), value.to_owned()));
        });
        Some(())
      });

    let client = reqwest::Client::new();
    let endpoint = format!("{}/chat/completions", self.model_config.base_url);
    let req = client
      .post(&endpoint)
      .bearer_auth(self.model_config.auth_token.clone())
      .header("Content-Type", "application/json")
      .body(serde_json::to_string(&data).unwrap());
    req
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

  /**
   * Consume the chunk delta return from the chat completion stream API and apply it on to the message.
   */
  fn apply_model_response_chunk(&self, message: &mut ChatMessage, chunk: ChatResponseChunkDelta) {
    chunk.role.and_then(|role| {
      message.role = role;
      Some(())
    });
    chunk.content.and_then(|content| {
      message.content.push_str(&content);
      Some(())
    });
    chunk.tool_calls.and_then(|tool_calls| {
      if message.tool_calls.is_none() {
        message.tool_calls = Some(Vec::new());
      }
      let message_tool_calls = message.tool_calls.as_mut().unwrap();
      for tool_call in tool_calls {
        let idx = tool_call.index;
        while idx >= message_tool_calls.len() {
          message_tool_calls.push(ToolCallRequest {
            id: "".to_owned(),
            _type: "".to_owned(),
            function: FunctionCallRequest {
              name: "".to_owned(),
              arguments: "".to_owned(),
            },
          });
        }
        let object_to_apply = message_tool_calls.get_mut(idx).unwrap();
        tool_call.id.and_then(|id| {
          object_to_apply.id.push_str(&id);
          Some(())
        });
        tool_call._type.and_then(|t| {
          object_to_apply._type.push_str(&t);
          Some(())
        });
        tool_call.function.and_then(|fcall| {
          fcall.name.and_then(|name| {
            object_to_apply.function.name.push_str(&name);
            Some(())
          });
          fcall.arguments.and_then(|arg| {
            object_to_apply.function.arguments.push_str(&arg);
            Some(())
          });
          Some(())
        });
      }
      Some(())
    });
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
        let name_parts: Vec<&str> = item.function.name.split(".").collect();
        let server_name = name_parts[0];
        let tool_name = name_parts[1];
        let args: Value = serde_json::from_str(&item.function.arguments).unwrap();

        let server = app.server_processes.get_mut(server_name).unwrap();
        let tool_result = server.call_tool(&tool_name, &args)?;
        let text_content = unpack_mcp_text_contents(server_name, &tool_result)?;
        println!("[Tool: {}]: {}", server_name, text_content);
        tool_call_responses.push(ChatMessage {
          role: "tool".to_owned(),
          content: text_content,
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
      return Err(NahError::mcp_server_invalid_response(server_name));
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
    Err(_) => {
      return Err(NahError::io_error("Failed to open the user message file."));
    }
  }
  let _ = launch_editor(MESSAGE_FILE_PATH)?;
  let mut file = open_file_or_throw()?;
  let mut buf: String = String::new();
  if file.read_to_string(&mut buf).is_err() {
    return Err(NahError::io_error("Failed to open the user message file."));
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
    Err(_) => return Err(NahError::io_error("Failed to open the user message file")),
  };
  Ok(file)
}
