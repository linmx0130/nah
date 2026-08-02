/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use async_stream::stream;
use futures_core::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value, json};

use crate::{ChatClient, Error, ErrorKind, Result};

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

// ---------- Params builder ----------

/**
 * A builder for creating parameters of Responses API requests.
 */
#[derive(Debug, Clone)]
pub struct ResponsesParamsBuilder {
  data: std::collections::HashMap<String, Value>,
}

impl ResponsesParamsBuilder {
  /**
   * Initialize a [ResponsesParamsBuilder] object.
   */
  pub fn new() -> Self {
    ResponsesParamsBuilder {
      data: std::collections::HashMap::new(),
    }
  }

  /**
   * Consume the data builder to get a hash map of the parameters for Responses API requests.
   */
  pub fn build(self) -> std::collections::HashMap<String, Value> {
    self.data
  }

  /** Set the `instructions` parameter (inserted as the first system message). */
  pub fn instructions(&mut self, s: &str) -> &mut Self {
    self
      .data
      .insert("instructions".to_owned(), json!(s.to_owned()));
    self
  }

  /** Set the `max_output_tokens` parameter. */
  pub fn max_output_tokens(&mut self, n: usize) -> &mut Self {
    self.data.insert(
      "max_output_tokens".to_owned(),
      Value::Number(Number::from_u128(n as u128).unwrap()),
    );
    self
  }

  /** Set the `temperature` parameter. */
  pub fn temperature(&mut self, t: f64) -> &mut Self {
    self.data.insert(
      "temperature".to_owned(),
      Value::Number(Number::from_f64(t).unwrap()),
    );
    self
  }

  /** Set the `top_p` parameter. */
  pub fn top_p(&mut self, p: f64) -> &mut Self {
    self.data.insert(
      "top_p".to_owned(),
      Value::Number(Number::from_f64(p).unwrap()),
    );
    self
  }

  /** Set the `top_logprobs` parameter (range [0, 20] on DeepSeek). */
  pub fn top_logprobs(&mut self, n: usize) -> &mut Self {
    self.data.insert(
      "top_logprobs".to_owned(),
      Value::Number(Number::from_u128(n as u128).unwrap()),
    );
    self
  }

  /**
   * Set the `tools` parameter. Note: the Responses API tool format is FLAT —
   * `{"type":"function","name":...,"description":...,"parameters":...}` —
   * unlike the nested `{"function":{...}}` shape of the chat completion API.
   */
  pub fn tools(&mut self, tools: Value) -> &mut Self {
    self.data.insert("tools".to_owned(), tools);
    self
  }

  /** Set the `tool_choice` parameter: "none" / "auto" / "required" / {"type":"function","name":...}. */
  pub fn tool_choice(&mut self, choice: Value) -> &mut Self {
    self.data.insert("tool_choice".to_owned(), choice);
    self
  }

  /** Set the `reasoning` parameter, e.g. `json!({"effort": "high"})`. */
  pub fn reasoning(&mut self, reasoning: Value) -> &mut Self {
    self.data.insert("reasoning".to_owned(), reasoning);
    self
  }

  /** Set the `text` parameter, e.g. `json!({"format": {"type": "json_object"}})`. */
  pub fn text(&mut self, text: Value) -> &mut Self {
    self.data.insert("text".to_owned(), text);
    self
  }

  /** Insert an arbitrary parameter with key `name` and value `value`. */
  pub fn insert(&mut self, name: &str, value: Value) -> &mut Self {
    self.data.insert(name.to_owned(), value);
    self
  }
}

impl<'a> std::iter::IntoIterator for &'a ResponsesParamsBuilder {
  type Item = (&'a String, &'a Value);
  type IntoIter = std::collections::hash_map::Iter<'a, String, Value>;

  fn into_iter(self) -> Self::IntoIter {
    (&self.data).into_iter()
  }
}

// ---------- Stream events ----------

/**
 * A parsed event from the Responses API streaming interface.
 */
#[derive(Debug, Clone)]
pub enum ResponsesStreamEvent {
  OutputTextDelta {
    item_id: String,
    output_index: usize,
    content_index: usize,
    delta: String,
  },
  OutputTextDone {
    item_id: String,
    output_index: usize,
    content_index: usize,
    text: String,
  },
  ReasoningTextDelta {
    item_id: String,
    output_index: usize,
    content_index: usize,
    delta: String,
  },
  ReasoningTextDone {
    item_id: String,
    output_index: usize,
    content_index: usize,
    text: String,
  },
  FunctionCallArgumentsDelta {
    item_id: String,
    output_index: usize,
    delta: String,
  },
  FunctionCallArgumentsDone {
    item_id: String,
    output_index: usize,
    arguments: String,
  },
  OutputItemDone {
    output_index: usize,
    item: ResponseOutputItem,
  },
  /** The response finished successfully; carries the full response object. */
  Completed(ResponseObject),
  /** The response was truncated (e.g. max_output_tokens reached). */
  Incomplete(ResponseObject),
  /** The response failed; `response.error` carries the error details. */
  Failed(ResponseObject),
  /** Any event type not explicitly modeled. */
  Unknown { event_type: String, data: Value },
}

impl ResponsesStreamEvent {
  fn is_terminal(&self) -> bool {
    matches!(
      self,
      ResponsesStreamEvent::Completed(_)
        | ResponsesStreamEvent::Incomplete(_)
        | ResponsesStreamEvent::Failed(_)
    )
  }
}

// ---------- SSE parsing ----------

/**
 * Incremental SSE parser. Buffers partial messages so events split across
 * network chunks are reassembled correctly.
 */
pub(crate) struct SseBuffer {
  pending: String,
}

impl SseBuffer {
  pub(crate) fn new() -> Self {
    SseBuffer {
      pending: String::new(),
    }
  }

  /** Feed bytes; returns all complete SSE messages (payloads between blank lines). */
  pub(crate) fn push(&mut self, bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    self.pending.push_str(&text.replace("\r\n", "\n"));
    let mut messages = Vec::new();
    while let Some(idx) = self.pending.find("\n\n") {
      let message = self.pending[..idx].trim_end_matches('\n').to_string();
      self.pending.drain(..idx + 2);
      messages.push(message);
    }
    messages
  }

  /** Flush any trailing message left at EOF. */
  pub(crate) fn finish(&mut self) -> Vec<String> {
    let rest = std::mem::take(&mut self.pending);
    let trimmed = rest.trim();
    if trimmed.is_empty() {
      Vec::new()
    } else {
      vec![trimmed.to_string()]
    }
  }
}

/**
 * Parse one SSE message into `(event_name, data_payload)`.
 * Comment/heartbeat lines (`: ...`) are ignored.
 */
fn parse_sse_message(message: &str) -> Option<(Option<String>, String)> {
  let mut event: Option<String> = None;
  let mut data_lines: Vec<&str> = Vec::new();
  for line in message.split('\n') {
    if line.starts_with(':') {
      continue;
    }
    if let Some(v) = line.strip_prefix("event:") {
      event = Some(v.trim().to_string());
    } else if let Some(v) = line.strip_prefix("data:") {
      data_lines.push(v.trim());
    }
  }
  if data_lines.is_empty() {
    return None;
  }
  Some((event, data_lines.join("\n")))
}

/**
 * Dispatch one SSE data payload to a typed event.
 * Prefers the `type` field of the JSON payload; falls back to the SSE `event:` name.
 * Unparseable or empty payloads are skipped.
 */
fn parse_responses_event(event_name: Option<&str>, data: &str) -> Option<ResponsesStreamEvent> {
  if data == "[DONE]" {
    return None; // defensive: not used by the Responses API
  }
  let value: Value = serde_json::from_str(data).ok()?;
  let obj = value.as_object()?;
  let type_name = obj.get("type").and_then(|t| t.as_str()).or(event_name)?;

  let str_field = |obj: &serde_json::Map<String, Value>, key: &str| -> String {
    obj
      .get(key)
      .and_then(|v| v.as_str())
      .unwrap_or("")
      .to_string()
  };
  let num_field = |obj: &serde_json::Map<String, Value>, key: &str| -> usize {
    obj.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as usize
  };

  match type_name {
    "response.output_text.delta" => Some(ResponsesStreamEvent::OutputTextDelta {
      item_id: str_field(obj, "item_id"),
      output_index: num_field(obj, "output_index"),
      content_index: num_field(obj, "content_index"),
      delta: str_field(obj, "delta"),
    }),
    "response.output_text.done" => Some(ResponsesStreamEvent::OutputTextDone {
      item_id: str_field(obj, "item_id"),
      output_index: num_field(obj, "output_index"),
      content_index: num_field(obj, "content_index"),
      text: str_field(obj, "text"),
    }),
    "response.reasoning_text.delta" => Some(ResponsesStreamEvent::ReasoningTextDelta {
      item_id: str_field(obj, "item_id"),
      output_index: num_field(obj, "output_index"),
      content_index: num_field(obj, "content_index"),
      delta: str_field(obj, "delta"),
    }),
    "response.reasoning_text.done" => Some(ResponsesStreamEvent::ReasoningTextDone {
      item_id: str_field(obj, "item_id"),
      output_index: num_field(obj, "output_index"),
      content_index: num_field(obj, "content_index"),
      text: str_field(obj, "text"),
    }),
    "response.function_call_arguments.delta" => {
      Some(ResponsesStreamEvent::FunctionCallArgumentsDelta {
        item_id: str_field(obj, "item_id"),
        output_index: num_field(obj, "output_index"),
        delta: str_field(obj, "delta"),
      })
    }
    "response.function_call_arguments.done" => {
      Some(ResponsesStreamEvent::FunctionCallArgumentsDone {
        item_id: str_field(obj, "item_id"),
        output_index: num_field(obj, "output_index"),
        arguments: str_field(obj, "arguments"),
      })
    }
    "response.output_item.done" => {
      let item: ResponseOutputItem = serde_json::from_value(obj.get("item")?.clone()).ok()?;
      Some(ResponsesStreamEvent::OutputItemDone {
        output_index: num_field(obj, "output_index"),
        item,
      })
    }
    "response.completed" => Some(ResponsesStreamEvent::Completed(parse_embedded_response(
      obj,
    )?)),
    "response.incomplete" => Some(ResponsesStreamEvent::Incomplete(parse_embedded_response(
      obj,
    )?)),
    "response.failed" => Some(ResponsesStreamEvent::Failed(parse_embedded_response(obj)?)),
    other => Some(ResponsesStreamEvent::Unknown {
      event_type: other.to_string(),
      data: value,
    }),
  }
}

fn parse_embedded_response(obj: &serde_json::Map<String, Value>) -> Option<ResponseObject> {
  let response_value = obj.get("response")?;
  serde_json::from_value(response_value.clone()).ok()
}

// ---------- Client methods ----------

impl ChatClient {
  /**
   * Create a Responses API request.
   *
   * Args:
   * * `model` Name of the model to be called.
   * * `input` The input of the request: a text string or a list of input items.
   * * `is_stream` Whether the request is stream-based.
   * * `params` Other parameters to be sent (see [ResponsesParamsBuilder]).
   */
  pub fn create_responses_request<'a, P>(
    &self,
    model: &str,
    input: &ResponsesInput,
    is_stream: bool,
    params: P,
  ) -> reqwest::RequestBuilder
  where
    P: IntoIterator<Item = (&'a String, &'a Value)>,
  {
    let mut data = json!({
      "model": model.to_owned(),
      "input": input,
      "stream": is_stream,
    });
    params.into_iter().for_each(|(key, value)| {
      data
        .as_object_mut()
        .and_then(|o| o.insert(key.to_owned(), value.to_owned()));
    });

    let endpoint = format!("{}/responses", self.base_url);

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
   * Request a response in the non-stream approach.
   *
   * Args:
   * * `model` Name of the model to be called.
   * * `input` The input of the request: a text string or a list of input items.
   * * `params` Other parameters to be sent (see [ResponsesParamsBuilder]).
   */
  pub async fn responses<'a, P>(
    &self,
    model: &str,
    input: &ResponsesInput,
    params: P,
  ) -> Result<ResponseObject>
  where
    P: IntoIterator<Item = (&'a String, &'a Value)>,
  {
    let req = self.create_responses_request(model, input, false, params);
    let res_text = req.send().await?.text().await?;
    parse_response_object(&res_text)
  }

  /**
   * Request a response in the async stream approach. The stream yields typed
   * [ResponsesStreamEvent]s and terminates on `response.completed` /
   * `response.incomplete` / `response.failed` (the Responses API does not use
   * `data: [DONE]`).
   *
   * Args:
   * * `model` Name of the model to be called.
   * * `input` The input of the request: a text string or a list of input items.
   * * `params` Other parameters to be sent (see [ResponsesParamsBuilder]).
   */
  pub async fn responses_stream<'a, P>(
    &self,
    model: &str,
    input: &ResponsesInput,
    params: P,
  ) -> Result<impl Stream<Item = Result<ResponsesStreamEvent>>>
  where
    P: IntoIterator<Item = (&'a String, &'a Value)>,
  {
    let req = self.create_responses_request(model, input, true, params);
    let mut res = req.send().await?;

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
      let mut buffer = SseBuffer::new();
      let mut finished = false;
      while !finished {
        let Some(chunk_data) = res.chunk().await? else {
          break;
        };
        for message in buffer.push(&chunk_data) {
          if let Some((event_name, data)) = parse_sse_message(&message)
            && let Some(event) = parse_responses_event(event_name.as_deref(), &data)
          {
            if event.is_terminal() {
              finished = true;
            }
            yield Ok(event);
          }
        }
      }
      for message in buffer.finish() {
        if let Some((event_name, data)) = parse_sse_message(&message)
          && let Some(event) = parse_responses_event(event_name.as_deref(), &data)
        {
          yield Ok(event);
        }
      }
    };
    Ok(stream)
  }
}

/**
 * Parse the JSON body of a non-stream Responses API response.
 */
fn parse_response_object(text: &str) -> Result<ResponseObject> {
  serde_json::from_str(text).map_err(|e| Error {
    kind: ErrorKind::ModelServerError,
    message: Some("Failed to parse model server response".to_string()),
    cause: Some(Box::new(e)),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  const STREAM_FIXTURE_TEXT: &str = "\
event: response.output_text.delta
data: {\"type\":\"response.output_text.delta\",\"sequence_number\":4,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hello\"}

event: response.output_text.delta
data: {\"type\":\"response.output_text.delta\",\"sequence_number\":5,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\" world\"}

event: response.output_text.done
data: {\"type\":\"response.output_text.done\",\"sequence_number\":6,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"text\":\"Hello world\",\"annotations\":[]}

event: response.output_item.done
data: {\"type\":\"response.output_item.done\",\"sequence_number\":7,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello world\",\"annotations\":[]}]}}

event: response.completed
data: {\"type\":\"response.completed\",\"sequence_number\":8,\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"created_at\":1754000000,\"status\":\"completed\",\"model\":\"deepseek-v4-flash\",\"output\":[{\"type\":\"message\",\"id\":\"msg_1\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello world\",\"annotations\":[]}]}],\"usage\":{\"input_tokens\":12,\"output_tokens\":3,\"total_tokens\":15,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens_details\":{\"reasoning_tokens\":0}}}}
";

  fn parse_fixture(fixture: &str) -> Vec<ResponsesStreamEvent> {
    let mut buffer = SseBuffer::new();
    let mut events = Vec::new();
    for message in buffer.push(fixture.as_bytes()) {
      if let Some((name, data)) = parse_sse_message(&message)
        && let Some(event) = parse_responses_event(name.as_deref(), &data)
      {
        events.push(event);
      }
    }
    for message in buffer.finish() {
      if let Some((name, data)) = parse_sse_message(&message)
        && let Some(event) = parse_responses_event(name.as_deref(), &data)
      {
        events.push(event);
      }
    }
    events
  }

  fn typed_events(events: Vec<ResponsesStreamEvent>) -> Vec<ResponsesStreamEvent> {
    events
      .into_iter()
      .filter(|e| !matches!(e, ResponsesStreamEvent::Unknown { .. }))
      .collect()
  }

  #[test]
  fn test_parse_simple_stream() {
    let events = typed_events(parse_fixture(STREAM_FIXTURE_TEXT));
    assert_eq!(events.len(), 5); // delta, delta, done, item.done, completed
  }

  #[test]
  fn test_stream_event_shapes() {
    let events = typed_events(parse_fixture(STREAM_FIXTURE_TEXT));
    match &events[0] {
      ResponsesStreamEvent::OutputTextDelta {
        delta,
        output_index,
        ..
      } => {
        assert_eq!(delta, "Hello");
        assert_eq!(*output_index, 0);
      }
      _ => panic!("expected OutputTextDelta"),
    }
    match &events[1] {
      ResponsesStreamEvent::OutputTextDelta { delta, .. } => assert_eq!(delta, " world"),
      _ => panic!("expected OutputTextDelta"),
    }
    match &events[2] {
      ResponsesStreamEvent::OutputTextDone { text, .. } => assert_eq!(text, "Hello world"),
      _ => panic!("expected OutputTextDone"),
    }
    match &events[3] {
      ResponsesStreamEvent::OutputItemDone { item, .. } => assert!(item.is_message()),
      _ => panic!("expected OutputItemDone"),
    }
    match &events[4] {
      ResponsesStreamEvent::Completed(response) => {
        assert_eq!(response.status, "completed");
        assert_eq!(response.output_text(), "Hello world");
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, Some(15));
        assert_eq!(response.output.len(), 1);
      }
      _ => panic!("expected Completed"),
    }
  }

  #[test]
  fn test_parse_reasoning_and_function_call_events() {
    let fixture = "\
event: response.reasoning_text.delta
data: {\"type\":\"response.reasoning_text.delta\",\"sequence_number\":3,\"item_id\":\"rs_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"Let me think\"}

event: response.reasoning_text.done
data: {\"type\":\"response.reasoning_text.done\",\"sequence_number\":4,\"item_id\":\"rs_1\",\"output_index\":0,\"content_index\":0,\"text\":\"Let me think step by step\"}

event: response.function_call_arguments.delta
data: {\"type\":\"response.function_call_arguments.delta\",\"sequence_number\":11,\"item_id\":\"fc_1\",\"output_index\":0,\"delta\":\"{\\\"city\\\":\"}

event: response.function_call_arguments.delta
data: {\"type\":\"response.function_call_arguments.delta\",\"sequence_number\":12,\"item_id\":\"fc_1\",\"output_index\":0,\"delta\":\"\\\"SF\\\"}\"}

event: response.function_call_arguments.done
data: {\"type\":\"response.function_call_arguments.done\",\"sequence_number\":13,\"item_id\":\"fc_1\",\"output_index\":0,\"arguments\":\"{\\\"city\\\":\\\"SF\\\"}\"}
";
    let events = parse_fixture(fixture);
    assert_eq!(events.len(), 5);
    match &events[0] {
      ResponsesStreamEvent::ReasoningTextDelta { delta, .. } => assert_eq!(delta, "Let me think"),
      _ => panic!("expected ReasoningTextDelta"),
    }
    match &events[1] {
      ResponsesStreamEvent::ReasoningTextDone { text, .. } => {
        assert_eq!(text, "Let me think step by step")
      }
      _ => panic!("expected ReasoningTextDone"),
    }
    match &events[2] {
      ResponsesStreamEvent::FunctionCallArgumentsDelta { delta, .. } => {
        assert_eq!(delta, "{\"city\":")
      }
      _ => panic!("expected FunctionCallArgumentsDelta"),
    }
    match &events[3] {
      ResponsesStreamEvent::FunctionCallArgumentsDelta { delta, .. } => {
        assert_eq!(delta, "\"SF\"}")
      }
      _ => panic!("expected FunctionCallArgumentsDelta"),
    }
    match &events[4] {
      ResponsesStreamEvent::FunctionCallArgumentsDone { arguments, .. } => {
        assert_eq!(arguments, "{\"city\":\"SF\"}")
      }
      _ => panic!("expected FunctionCallArgumentsDone"),
    }
  }

  #[test]
  fn test_sse_chunk_split_reassembly() {
    let fixture = STREAM_FIXTURE_TEXT;
    let split_at = fixture.find("Hello").unwrap(); // cut mid-event
    let mut buffer = SseBuffer::new();
    let mut events = Vec::new();
    for message in buffer.push(&fixture.as_bytes()[..split_at]) {
      if let Some((name, data)) = parse_sse_message(&message)
        && let Some(event) = parse_responses_event(name.as_deref(), &data)
      {
        events.push(event);
      }
    }
    assert!(
      events.is_empty(),
      "no complete event should be emitted before the split point"
    );
    for message in buffer.push(&fixture.as_bytes()[split_at..]) {
      if let Some((name, data)) = parse_sse_message(&message)
        && let Some(event) = parse_responses_event(name.as_deref(), &data)
      {
        events.push(event);
      }
    }
    for message in buffer.finish() {
      if let Some((name, data)) = parse_sse_message(&message)
        && let Some(event) = parse_responses_event(name.as_deref(), &data)
      {
        events.push(event);
      }
    }
    assert_eq!(events.len(), 5);
  }

  #[test]
  fn test_unknown_event_and_heartbeat_are_handled() {
    let fixture = "\
: ping

event: response.whatever.custom
data: {\"type\":\"response.whatever.custom\",\"sequence_number\":1,\"foo\":\"bar\"}

event: response.output_text.delta
data: {\"type\":\"response.output_text.delta\",\"sequence_number\":2,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"X\"}
";
    let events = parse_fixture(fixture);
    assert_eq!(events.len(), 2);
    match &events[0] {
      ResponsesStreamEvent::Unknown { event_type, .. } => {
        assert_eq!(event_type, "response.whatever.custom")
      }
      _ => panic!("expected Unknown passthrough"),
    }
    match &events[1] {
      ResponsesStreamEvent::OutputTextDelta { delta, .. } => assert_eq!(delta, "X"),
      _ => panic!("expected OutputTextDelta"),
    }
  }

  #[test]
  fn test_terminal_events_are_terminal() {
    let events = typed_events(parse_fixture(STREAM_FIXTURE_TEXT));
    assert!(!events[0].is_terminal());
    assert!(events[4].is_terminal()); // Completed
  }

  #[test]
  fn test_responses_params_builder() {
    let mut params = ResponsesParamsBuilder::new();
    params
      .instructions("You are helpful.")
      .max_output_tokens(2048)
      .temperature(0.7)
      .top_p(0.9)
      .top_logprobs(5)
      .reasoning(serde_json::json!({"effort": "high"}))
      .insert("customized_key", serde_json::json!("customized_value"));
    let data = params.build();
    assert_eq!(data["instructions"], "You are helpful.");
    assert_eq!(data["max_output_tokens"], 2048);
    assert_eq!(data["temperature"], 0.7);
    assert_eq!(data["top_p"], 0.9);
    assert_eq!(data["top_logprobs"], 5);
    assert_eq!(data["reasoning"]["effort"], "high");
    assert_eq!(data["customized_key"], "customized_value");
    assert_eq!(data.len(), 7);
  }

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
