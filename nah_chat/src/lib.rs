/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

pub enum ErrorKind {
  NetworkError,
  ModelServerError,
}

pub struct Error {
  kind: ErrorKind,
  cause: Option<Box<dyn std::error::Error>>,
}

pub type Result<T> = std::result::Result<T, Error>;

/**
 * Data structure of a chat message, could be from the user, the assistant or the tool.
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
      if self.tool_calls.is_none() {
        self.tool_calls = Some(Vec::new());
      }
      let message_tool_calls = self.tool_calls.as_mut().unwrap();
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

#[derive(Debug)]
pub struct ChatClient {
  pub base_url: String,
  pub auth_token: String,
  pub http_client: reqwest::Client,
}

impl ChatClient {
  /**
   * Create a new ChatClient instance, which hosts the basic information and reqwest client
   * for making the requests
   */
  pub fn init(base_url: String, auth_token: String) -> Self {
    let client = reqwest::Client::new();
    ChatClient {
      base_url: base_url,
      auth_token: auth_token,
      http_client: client,
    }
  }

  /**
   * Create a chat completion request.
   */
  pub fn create_chat_completion_request(
    &self,
    model: &str,
    messages: &Vec<ChatMessage>,
    is_stream: bool,
    params: &HashMap<String, Value>,
  ) -> reqwest::RequestBuilder {
    let mut data = json!({
        "model": model.to_owned(),
        "messages": messages.clone(),
        "stream": is_stream
    });

    params.iter().for_each(|(key, value)| {
      data
        .as_object_mut()
        .and_then(|o| o.insert(key.to_owned(), value.to_owned()));
    });

    let endpoint = format!("{}/chat/completions", self.base_url);
    self
      .http_client
      .post(&endpoint)
      .bearer_auth(self.auth_token.clone())
      .header(reqwest::header::CONTENT_TYPE, "application/json")
      .body(serde_json::to_string(&data).unwrap())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_apply_text_and_reasoning_content_chunk() {
    let mut message = ChatMessage {
      role: "assistant".to_owned(),
      content: "A".to_owned(),
      reasoning_content: None,
      tool_call_id: None,
      tool_calls: None,
    };

    message.apply_model_response_chunk(ChatResponseChunkDelta {
      role: Some("assistant".to_owned()),
      content: Some(" test".to_owned()),
      reasoning_content: Some("reason".to_owned()),
      tool_calls: None,
    });

    assert_eq!(message.role, "assistant");
    assert_eq!(message.content, "A test");
    assert_eq!(message.reasoning_content.unwrap(), "reason");
  }

  #[test]
  fn test_apply_tool_calls() {
    let mut message = ChatMessage {
      role: "assistant".to_owned(),
      content: "A".to_owned(),
      reasoning_content: None,
      tool_call_id: None,
      tool_calls: None,
    };

    message.apply_model_response_chunk(ChatResponseChunkDelta {
      role: None,
      content: None,
      reasoning_content: None,
      tool_calls: Some(vec![ToolCallRequestChunkDelta {
        index: 0,
        id: Some("123".to_owned()),
        _type: Some("function".to_owned()),
        function: Some(FunctionCallRequestChunkDelta {
          name: Some("x".to_owned()),
          arguments: None,
        }),
      }]),
    });
    assert_eq!(message.role, "assistant");
    {
      let tool_calls = message.tool_calls.as_ref().unwrap();
      assert_eq!(tool_calls[0].id, "123");
      assert_eq!(tool_calls[0].function.name, "x");
    }

    message.apply_model_response_chunk(ChatResponseChunkDelta {
      role: None,
      content: None,
      reasoning_content: None,
      tool_calls: Some(vec![ToolCallRequestChunkDelta {
        index: 0,
        id: None,
        _type: None,
        function: Some(FunctionCallRequestChunkDelta {
          name: Some("yz".to_owned()),
          arguments: Some("{\"a".to_owned()),
        }),
      }]),
    });
    {
      let tool_calls = message.tool_calls.as_ref().unwrap();
      assert_eq!(tool_calls[0].id, "123");
      assert_eq!(tool_calls[0].function.name, "xyz");
      assert_eq!(tool_calls[0].function.arguments, "{\"a");
    }
  }
}
