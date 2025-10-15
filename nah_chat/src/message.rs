/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use serde::{Deserialize, Serialize};
/**
 * This is used to represent a URL in the message content. Currently, only
 * images will be represented in this format.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct URLObject {
  pub url: String,
}

/**
 * This is used to represent a message content that can have multiple types
 * with a type annotation. Currently, it supports text and image_url.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TypedChatMessageContent {
  #[serde(rename = "type")]
  pub data_type: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image_url: Option<URLObject>,
}

impl TypedChatMessageContent {
  pub fn text_content(text: &str) -> TypedChatMessageContent {
    TypedChatMessageContent {
      data_type: "text".to_owned(),
      text: Some(text.to_owned()),
      image_url: None,
    }
  }

  pub fn image_url_content(image_url: &str) -> TypedChatMessageContent {
    TypedChatMessageContent {
      data_type: "image_url".to_owned(),
      text: None,
      image_url: Some(URLObject {
        url: image_url.to_owned(),
      }),
    }
  }
}

/**
 * Type for message contents.
 *
 * It can be either a text message or a `TypedChatMessageContent`.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ChatMessageContentValue {
  Text(String),
  TypedContentList(Vec<TypedChatMessageContent>),
}

impl std::fmt::Display for ChatMessageContentValue {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ChatMessageContentValue::Text(text) => write!(f, "{}", text),
      ChatMessageContentValue::TypedContentList(typed_content_list) => {
        let mut buffer = String::new();
        for typed_content in typed_content_list {
          if let Some(text) = &typed_content.text {
            buffer += text
          } else if let Some(image_url) = &typed_content.image_url {
            buffer += &format!("[Image URL: {}]", image_url.url);
          } else {
            buffer += &format!("[Unknown type: {}]", typed_content.data_type);
          }
        }
        write!(f, "{}", buffer)
      }
    }
  }
}

/**
 * Data structure of a chat message, could be from the user, the assistant or the tool.
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
  /** The role of the message. */
  pub role: String,

  /** Content of the message */
  pub content: ChatMessageContentValue,

  /** Reasoning content in string */
  #[serde(rename = "reasoningContent", skip_serializing_if = "Option::is_none")]
  pub reasoning_content: Option<String>,
  /**
   * Which tool call this message is responding to.
   *
   * Only valid for messages with `role` of `"tool"`.
   */

  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_call_id: Option<String>,

  /**
   * The tool calls requested by the model.
   *
   * Only valid for messages with `role` of `"assistant"`.
   */
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_calls: Option<Vec<ToolCallRequest>>,
}

impl ChatMessage {
  /**
   * Create an empty ChatMessage object.
   */
  pub fn new() -> Self {
    ChatMessage {
      role: String::new(),
      content: ChatMessageContentValue::Text(String::new()),
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
      match &mut self.content {
        ChatMessageContentValue::Text(t) => t.push_str(&content),
        _ => {
          // Cannot apply the chunk. Ignore.
        }
      }
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

  /**
   * Create a pure text user message.
   */
  pub fn user_text_message(text: &str) -> ChatMessage {
    ChatMessage {
      role: "user".to_owned(),
      content: ChatMessageContentValue::Text(text.to_owned()),
      reasoning_content: None,
      tool_call_id: None,
      tool_calls: None,
    }
  }
}

impl FromIterator<ChatResponseChunkDelta> for ChatMessage {
  fn from_iter<T: IntoIterator<Item = ChatResponseChunkDelta>>(iter: T) -> Self {
    let mut message = ChatMessage::default();
    message.extend(iter);
    message
  }
}

impl Default for ChatMessage {
  fn default() -> Self {
    ChatMessage::new()
  }
}

impl Extend<ChatResponseChunkDelta> for ChatMessage {
  fn extend<T>(&mut self, iter: T)
  where
    T: IntoIterator<Item = ChatResponseChunkDelta>,
  {
    for item in iter {
      self.apply_model_response_chunk(item)
    }
  }
}

/**
 * A chunk of chat message response from the assistant.
 */
#[derive(Debug, Clone)]
pub enum ChatResponseChunk {
  Delta(ChatResponseChunkDelta),
  Done,
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
