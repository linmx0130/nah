/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//!
//! # Introduction
//! This crate exposes an async stream API for the widely-used OpenAI
//! [chat completion API](https://platform.openai.com/docs/api-reference/chat).
//!
//! Supported features:
//! * Stream generation
//! * Tool calls
//! * Reasoning content (Qwen3, Deepseek R1, etc)
//!
//! This crate is built on top of `tokio`, `reqwest` and `serde_json`.
//!
//! ```rust
//! use nah_chat::{ChatClient, ChatMessage};
//! use futures_util::{pin_mut, StreamExt};
//!
//! # async fn make_request_example() {
//! # let base_url = "http://localhost:8080".to_string();
//! # let auth_token = None;
//! # let model_name = "deepseek-r1";
//! # let messages = vec![];
//! # let params = std::collections::HashMap::new();
//!
//! let chat_client = ChatClient::init(base_url, auth_token);
//!
//! // create and pin the stream
//! let stream = chat_client
//!        .chat_completion_stream(model_name, &messages, &params)
//!        .await
//!        .unwrap();
//! pin_mut!(stream);
//!
//! // buffer for the new message
//! let mut message = ChatMessage::new();
//!
//! // consume the stream
//! while let Some(delta_result) = stream.next().await {
//!   match delta_result {
//!     Ok(delta) => {
//!       message.apply_model_response_chunk(delta);
//!     }
//!     Err(e) => {
//!       eprintln!("Error occurred while processing the chat completion: {}", e);
//!     }
//!   }
//! }
//! # }
//! ```
//! # Notice
//! Copyright 2025, [Mengxiao Lin](linmx0130@gmail.com).
//! This is a part of [nah](https://github.com/linmx0130/nah) project. `nah` means "*N*ot *A*
//! *H*uman". Source code is available under [MPL-2.0](https://mozilla.org/MPL/2.0/).
//!
use async_stream::stream;
use bytes::Bytes;
use futures_core::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/**
 * Error kinds that may occur in `nah_chat`.
 */
#[derive(Debug)]
pub enum ErrorKind {
  NetworkError,
  ModelServerError,
}

impl std::fmt::Display for ErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorKind::NetworkError => {
        write!(f, "Network error")
      }
      ErrorKind::ModelServerError => {
        write!(f, "Model server error")
      }
    }
  }
}

/**
 * Error type of `nah_chat`.
 */
#[derive(Debug)]
pub struct Error {
  kind: ErrorKind,
  message: Option<String>,
  cause: Option<Box<dyn std::error::Error>>,
}

impl std::error::Error for Error {
  fn cause(&self) -> Option<&dyn std::error::Error> {
    self.cause.as_ref().and_then(|e| Some(e.as_ref()))
  }
}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}: {}",
      self.kind,
      self.message.clone().unwrap_or("None".to_string()),
    )
  }
}

pub type Result<T> = std::result::Result<T, Error>;

/**
 * Data structure of a chat message, could be from the user, the assistant or the tool.
 *
 * Fields:
 * * `role`: The role of the message.
 * * `content`: Text string content of the message.
 * * `reasoning_content`: Reasoning content in string.
 * * `tool_call_id`: Only valid for messages with `role` of `"tool"`. It indicates which tool call this
 *                    message is responding to.
 * * `tool_calls`: Only valid for messages with `role` of `"assistant"`. It is the tool calls
 *                 requested by the model.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
  pub role: String,
  pub content: String,
  #[serde(rename = "reasoningContent", skip_serializing_if = "Option::is_none")]
  pub reasoning_content: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_call_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_calls: Option<Vec<ToolCallRequest>>,
}

/**
 * A chunk of chat message response from the assistant.
 */
#[derive(Debug, Clone)]
pub enum ChatResponseChunk {
  Delta(ChatResponseChunkDelta),
  Done,
}

impl ChatMessage {
  /**
   * Create an empty ChatMessage object.
   */
  pub fn new() -> Self {
    ChatMessage {
      role: String::new(),
      content: String::new(),
      reasoning_content: None,
      tool_call_id: None,
      tool_calls: None,
    }
  }
  /**
   * Consume the chunk delta return from the chat completion stream API and apply it on to the message.
   */
  pub fn apply_model_response_chunk(&mut self, chunk: ChatResponseChunkDelta) {
    chunk.role.and_then(|role| {
      self.role = role;
      Some(())
    });
    chunk.content.and_then(|content| {
      self.content.push_str(&content);
      Some(())
    });
    chunk
      .reasoning_content
      .and_then(|reasoning_content: String| {
        match &mut self.reasoning_content {
          Some(r) => {
            r.push_str(&reasoning_content);
          }
          None => self.reasoning_content = Some(reasoning_content),
        }
        Some(())
      });
    chunk.tool_calls.and_then(|tool_calls| {
      let message_tool_calls = match self.tool_calls.as_mut() {
        Some(t) => t,
        None => {
          self.tool_calls = Some(Vec::new());
          self.tool_calls.as_mut().unwrap()
        }
      };
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
}

/**
 * Chunk delta of chat message from the assistant.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatResponseChunkDelta {
  pub role: Option<String>,
  pub content: Option<String>,
  #[serde(rename = "reasoning_content")]
  pub reasoning_content: Option<String>,
  pub tool_calls: Option<Vec<ToolCallRequestChunkDelta>>,
}

/**
 * A tool call request. Only function call is supported now.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallRequest {
  pub id: String,
  #[serde(rename = "type")]
  pub _type: String,
  pub function: FunctionCallRequest,
}

/**
 * A tool call request chunk received from stream api.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallRequestChunkDelta {
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
pub struct FunctionCallRequest {
  pub name: String,
  pub arguments: String,
}

/**
 * A function call request chunk received from stream api.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCallRequestChunkDelta {
  pub name: Option<String>,
  pub arguments: Option<String>,
}

/**
 * The object to hold information about the model server and `reqwest` HTTP client.
 */
#[derive(Debug)]
pub struct ChatClient {
  pub base_url: String,
  pub auth_token: Option<String>,
  pub http_client: reqwest::Client,
}

impl ChatClient {
  /**
   * Create a new ChatClient instance, which hosts the basic information and reqwest client
   * for making the requests
   *
   * Args:
   * * `base_url` Base url of the API server. This URL should NOT end with '/'.
   * * `auth_token` Bearer authentication token. It is often called "API Key".
   */
  pub fn init(base_url: String, auth_token: Option<String>) -> Self {
    let client = reqwest::Client::new();
    ChatClient {
      base_url: base_url,
      auth_token: auth_token,
      http_client: client,
    }
  }

  /**
   * Create a chat completion request.
   *
   * Args:
   * * `model` Name of the model to be called.
   * * `messages` A list of [ChatMessage] as the context.
   * * `is_stream` Whether the request is stream-based.
   * * `params` Other parameters to be sent.
   */
  pub fn create_chat_completion_request<'a, 'b, P, M>(
    &self,
    model: &str,
    messages: M,
    is_stream: bool,
    params: P,
  ) -> reqwest::RequestBuilder
  where
    P: IntoIterator<Item = (&'a String, &'a Value)>,
    M: IntoIterator<Item = &'b ChatMessage>,
  {
    let mut data = json!({
        "model": model.to_owned(),
        "messages": messages.into_iter().collect::<Vec<_>>(),
        "stream": is_stream,
        "n": 1,
    });

    params.into_iter().for_each(|(key, value)| {
      data
        .as_object_mut()
        .and_then(|o| o.insert(key.to_owned(), value.to_owned()));
    });

    let endpoint = format!("{}/chat/completions", self.base_url);

    let mut req = self
      .http_client
      .post(&endpoint)
      .header(reqwest::header::CONTENT_TYPE, "application/json")
      .body(serde_json::to_string(&data).unwrap());
    if self.auth_token.is_some() {
      req = req.bearer_auth(self.auth_token.as_ref().unwrap().as_str());
    }

    req
  }

  /**
   * Request chat completion in the async stream approach.
   *
   * Args:
   * * `model` Name of the model to be called.
   * * `messages` An list of [ChatMessage] as the context.
   * * `params` Other parameters to be sent.
   */
  pub async fn chat_completion_stream<'a, 'b, P, M>(
    &self,
    model: &str,
    messages: M,
    params: P,
  ) -> Result<impl Stream<Item = Result<ChatResponseChunkDelta>>>
  where
    P: IntoIterator<Item = (&'a String, &'a Value)>,
    M: IntoIterator<Item = &'b ChatMessage>,
  {
    let req = self.create_chat_completion_request(model, messages, true, params);
    let mut res = match req.send().await {
      Ok(r) => r,
      Err(e) => {
        return Err(Error {
          kind: ErrorKind::NetworkError,
          cause: Some(Box::new(e)),
          message: None,
        });
      }
    };

    if !res.status().is_success() {
      let code = res.status().as_u16();
      let error_content = res.text().await.unwrap();
      return Err(Error {
        kind: ErrorKind::ModelServerError,
        message: Some(format!(
          "Model server responded with error: HTTP status {}, error message = {}",
          code, error_content
        )),
        cause: None,
      });
    }

    let stream = stream! {
      let mut reach_done = false;
      while !reach_done {
        let chunk = match res.chunk().await {
          Ok(Some(chunk)) => chunk,
          Ok(None) => continue,
          Err(e) => {
            yield Err(Error{
                kind: ErrorKind::NetworkError,
                message: None,
                cause: Some(Box::new(e))
            });
            break;
          }
        };
        let delta = self.get_model_response_chunk(chunk);
        match delta {
          Some(ChatResponseChunk::Delta(d)) => {
            yield Ok(d);
          }
          Some(ChatResponseChunk::Done) => {
            reach_done = true;
          }
          None => {}
        }
      }
    };
    Ok(stream)
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
}

#[cfg(test)]
mod tests;
