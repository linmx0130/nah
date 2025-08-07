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
mod error;
mod message;
pub use error::{Error, ErrorKind, Result};
pub use message::*;

use async_stream::stream;
use bytes::Bytes;
use futures_core::stream::Stream;
use serde_json::{Number, Value, json};
/**
 * A builder for creating parameters for chat completion requests.
 */
#[derive(Debug, Clone)]
pub struct ChatCompletionParamsBuilder {
  data: std::collections::HashMap<String, Value>,
}

impl ChatCompletionParamsBuilder {
  /**
   * Initialize a [ChatCompletionParamsBuilder] object.
   */
  pub fn new() -> Self {
    ChatCompletionParamsBuilder {
      data: std::collections::HashMap::new(),
    }
  }

  /**
   * Consume the data builder to get a hash map of the parameters for chat completion requests.
   */
  pub fn build(self) -> std::collections::HashMap<String, Value> {
    self.data
  }

  /**
   * Set max token parameter.
   */
  pub fn max_tokens(&mut self, n: usize) -> &mut Self {
    self.data.insert(
      "max_tokens".to_owned(),
      Value::Number(Number::from_u128(n as u128).unwrap()),
    );
    self
  }

  /**
   * Set temperature parameter.
   */
  pub fn temperature(&mut self, t: f64) -> &mut Self {
    self.data.insert(
      "temperature".to_owned(),
      Value::Number(Number::from_f64(t).unwrap()),
    );
    self
  }

  /**
   * Set top_p parameter.
   */
  pub fn top_p(&mut self, p: f64) -> &mut Self {
    self.data.insert(
      "top_p".to_owned(),
      Value::Number(Number::from_f64(p).unwrap()),
    );
    self
  }

  /**
   * Set frequency_penalty parameter.
   */
  pub fn frequency_penalty(&mut self, p: f64) -> &mut Self {
    self.data.insert(
      "frequency_penalty".to_owned(),
      Value::Number(Number::from_f64(p).unwrap()),
    );
    self
  }

  /**
   * Set a parameter with key of `name` and value of `value`.
   */
  pub fn insert(&mut self, name: &str, value: Value) -> &mut Self {
    self.data.insert(name.to_owned(), value);
    self
  }
}

impl<'a> std::iter::IntoIterator for &'a ChatCompletionParamsBuilder {
  type Item = (&'a String, &'a Value);
  type IntoIter = std::collections::hash_map::Iter<'a, String, Value>;

  fn into_iter(self) -> Self::IntoIter {
    (&self.data).into_iter()
  }
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
      let mut reach_done = false;
      while !reach_done {
        let chunk_data = match res.chunk().await? {
          Some(chunk) => chunk,
          None => continue,
        };
        let chunk_data_str = match String::from_utf8(chunk_data.to_vec()) {
            Ok(v) => v,
            Err(e) => {
                yield Err(Error {
                    kind: ErrorKind::ModelServerError,
                    message: Some(format!("Failed to decode model server response")),
                    cause: Some(Box::new(e))
                });
                return;
            }
        };
        let chunks = chunk_data_str.split("\n\n");
        for chunk in chunks {
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
      }
    };
    Ok(stream)
  }

  /**
   * Parse the stream data from the stream chat completion API to obtain a chunk delta.
   */
  fn get_model_response_chunk(&self, data_str: &str) -> Option<ChatResponseChunk> {
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
