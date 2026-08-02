/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------- Input ----------

/**
 * Input of a Responses API request: a plain text string, or a list of input items.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ResponsesInput {
  Text(String),
  Items(Vec<ResponseInputItem>),
}

impl From<String> for ResponsesInput {
  fn from(v: String) -> Self {
    ResponsesInput::Text(v)
  }
}
impl From<&str> for ResponsesInput {
  fn from(v: &str) -> Self {
    ResponsesInput::Text(v.to_owned())
  }
}
impl From<Vec<ResponseInputItem>> for ResponsesInput {
  fn from(v: Vec<ResponseInputItem>) -> Self {
    ResponsesInput::Items(v)
  }
}

/**
 * An input item of the Responses API. Unknown shapes are passed through as raw JSON.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ResponseInputItem {
  Message(ResponseMessageItem),
  FunctionCall(ResponseFunctionCallItem),
  FunctionCallOutput(ResponseFunctionCallOutputItem),
  Custom(Value),
}

/**
 * A message input/output item. Content can be a plain string or a list of content parts.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ResponseMessageItem {
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub item_type: Option<String>,
  pub role: String,
  pub content: ResponseMessageContent,
}

/**
 * Content of a message item: plain text or typed content parts.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ResponseMessageContent {
  Text(String),
  Parts(Vec<ResponseContentPart>),
}

/**
 * A content part of a message item. Supports `input_text` / `output_text` / `input_image`
 * via `part_type`; extra fields are tolerated.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ResponseContentPart {
  #[serde(rename = "type")]
  pub part_type: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub annotations: Option<Vec<Value>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image_url: Option<Value>,
}

impl ResponseContentPart {
  pub fn input_text(text: &str) -> ResponseContentPart {
    ResponseContentPart {
      part_type: "input_text".to_owned(),
      text: Some(text.to_owned()),
      annotations: None,
      image_url: None,
    }
  }
  pub fn output_text(text: &str) -> ResponseContentPart {
    ResponseContentPart {
      part_type: "output_text".to_owned(),
      text: Some(text.to_owned()),
      annotations: None,
      image_url: None,
    }
  }
}

/**
 * A `function_call` input/output item. The tool format of the Responses API is FLAT:
 * `{"type":"function_call","call_id":...,"name":...,"arguments":...}`.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ResponseFunctionCallItem {
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub item_type: Option<String>,
  pub call_id: String,
  pub name: String,
  pub arguments: String,
}

/**
 * A `function_call_output` input item carrying the result of a tool call.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ResponseFunctionCallOutputItem {
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub item_type: Option<String>,
  pub call_id: String,
  pub output: String,
}

// ---------- Output ----------

/**
 * An output item of a response. Kept as a tolerant struct (all fields optional except
 * `item_type`) because DeepSeek's compatibility with the OpenAI structure is partial.
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ResponseOutputItem {
  #[serde(rename = "type")]
  pub item_type: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub call_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub status: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub role: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub arguments: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub summary: Option<Vec<ResponseContentPart>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<Vec<ResponseContentPart>>,
}

impl ResponseOutputItem {
  pub fn is_message(&self) -> bool {
    self.item_type == "message"
  }
  pub fn is_function_call(&self) -> bool {
    self.item_type == "function_call"
  }

  /** Concatenate the text of all content parts (output_text / reasoning_text / ...). */
  pub fn text(&self) -> String {
    self
      .content
      .as_ref()
      .map(|parts| {
        parts
          .iter()
          .filter_map(|p| p.text.clone())
          .collect::<Vec<_>>()
          .join("")
      })
      .unwrap_or_default()
  }
  /** Concatenate summary + content text (for `reasoning` items). */
  pub fn reasoning_text(&self) -> String {
    let mut result = String::new();
    if let Some(summary) = &self.summary {
      result.push_str(
        &summary
          .iter()
          .filter_map(|p| p.text.clone())
          .collect::<Vec<_>>()
          .join(""),
      );
    }
    result.push_str(&self.text());
    result
  }
}

// ---------- Response object ----------

/**
 * The full response object returned by the Responses API (non-stream mode, or carried
 * inside the terminal streaming events).
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseObject {
  pub id: String,
  pub object: String,
  pub created_at: u64,
  pub status: String,
  pub model: String,
  #[serde(default)]
  pub output: Vec<ResponseOutputItem>,
  #[serde(default)]
  pub usage: Option<ResponseUsage>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub error: Option<Value>,
}

impl ResponseObject {
  /** Concatenated assistant text of all message items in output order. */
  pub fn output_text(&self) -> String {
    self
      .output
      .iter()
      .filter(|i| i.is_message())
      .map(|i| i.text())
      .collect::<Vec<_>>()
      .join("")
  }
  /** Concatenated chain-of-thought text of all reasoning items. */
  pub fn reasoning_text(&self) -> String {
    self
      .output
      .iter()
      .map(|i| i.reasoning_text())
      .collect::<Vec<_>>()
      .join("")
  }
  /** All function call items (for tool execution). */
  pub fn function_calls(&self) -> Vec<&ResponseOutputItem> {
    self
      .output
      .iter()
      .filter(|i| i.is_function_call())
      .collect()
  }
}

// ---------- Usage ----------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseUsage {
  #[serde(default)]
  pub input_tokens: Option<u64>,
  #[serde(default)]
  pub output_tokens: Option<u64>,
  #[serde(default)]
  pub total_tokens: Option<u64>,
  #[serde(default)]
  pub input_tokens_details: Option<ResponseInputTokensDetails>,
  #[serde(default)]
  pub output_tokens_details: Option<ResponseOutputTokensDetails>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseInputTokensDetails {
  #[serde(default)]
  pub cached_tokens: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseOutputTokensDetails {
  #[serde(default)]
  pub reasoning_tokens: Option<u64>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_serialize_input_text() {
    let input = ResponsesInput::Text("Hi".to_string());
    assert_eq!(
      serde_json::to_value(&input).unwrap(),
      serde_json::json!("Hi")
    );
  }

  #[test]
  fn test_serialize_input_items() {
    let input = ResponsesInput::Items(vec![
      ResponseInputItem::Message(ResponseMessageItem {
        item_type: None,
        role: "user".to_string(),
        content: ResponseMessageContent::Parts(vec![ResponseContentPart {
          part_type: "input_text".to_string(),
          text: Some("Hello".to_string()),
          annotations: None,
          image_url: None,
        }]),
      }),
      ResponseInputItem::FunctionCallOutput(ResponseFunctionCallOutputItem {
        item_type: Some("function_call_output".to_string()),
        call_id: "call_1".to_string(),
        output: "{\"temp\":20}".to_string(),
      }),
    ]);
    let v = serde_json::to_value(&input).unwrap();
    assert_eq!(v[0]["role"], "user");
    assert_eq!(v[0]["content"][0]["type"], "input_text");
    assert_eq!(v[0]["content"][0]["text"], "Hello");
    assert_eq!(v[1]["type"], "function_call_output");
    assert_eq!(v[1]["call_id"], "call_1");
    assert_eq!(v[1]["output"], "{\"temp\":20}");
  }

  #[test]
  fn test_deserialize_response_object() {
    let data = r#"{
      "id": "resp_1", "object": "response", "created_at": 1754000000,
      "status": "completed", "model": "deepseek-v4-flash",
      "output": [{
        "type": "message", "id": "msg_1", "status": "completed", "role": "assistant",
        "content": [{"type": "output_text", "text": "Hello world", "annotations": []}]
      }],
      "usage": {
        "input_tokens": 12, "output_tokens": 3, "total_tokens": 15,
        "input_tokens_details": {"cached_tokens": 0},
        "output_tokens_details": {"reasoning_tokens": 2}
      }
    }"#;
    let response: ResponseObject = serde_json::from_str(data).unwrap();
    assert_eq!(response.id, "resp_1");
    assert_eq!(response.status, "completed");
    assert_eq!(response.output_text(), "Hello world");
    assert_eq!(response.output.len(), 1);
    assert!(response.output[0].is_message());
    assert_eq!(
      response
        .usage
        .as_ref()
        .unwrap()
        .output_tokens_details
        .as_ref()
        .unwrap()
        .reasoning_tokens,
      Some(2)
    );
  }

  #[test]
  fn test_response_object_function_calls() {
    let data = r#"{
      "id": "resp_2", "object": "response", "created_at": 1754000000,
      "status": "completed", "model": "deepseek-v4-flash",
      "output": [{
        "type": "function_call", "id": "fc_1", "call_id": "call_1",
        "name": "get_weather", "arguments": "{\"city\":\"SF\"}", "status": "completed"
      }],
      "usage": {"input_tokens": 5, "output_tokens": 5, "total_tokens": 10,
                "input_tokens_details": {"cached_tokens": 0},
                "output_tokens_details": {"reasoning_tokens": 0}}
    }"#;
    let response: ResponseObject = serde_json::from_str(data).unwrap();
    let calls = response.function_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name.as_deref(), Some("get_weather"));
    assert_eq!(calls[0].arguments.as_deref(), Some("{\"city\":\"SF\"}"));
  }
}
