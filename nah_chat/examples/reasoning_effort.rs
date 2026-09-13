/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Reasoning effort demo for both the chat completion API (`reasoning_effort`)
//! and the Responses API (`reasoning.effort`). DeepSeek's thinking mode is on by
//! default, so both calls should print a chain of thought.
//!
//! Run with:
//!   DEEPSEEK_API_KEY=... cargo run -p nah_chat --example reasoning_effort

use futures_util::{StreamExt, pin_mut};
use nah_chat::{
  ChatClient, ChatCompletionParamsBuilder, ChatCompletionStreamEvent, ChatMessage, ResponsesInput,
  ResponsesParamsBuilder, ResponsesStreamEvent,
};
use std::io::Write;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
  let auth_token = std::env::var("DEEPSEEK_API_KEY").unwrap();
  let base_url =
    std::env::var("BASE_URL").unwrap_or_else(|_| "https://api.deepseek.com".to_string());
  let model = std::env::var("MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());
  let client = ChatClient::init(base_url, Some(auth_token));

  println!("== Chat completion API: reasoning_effort ==");
  let messages = vec![ChatMessage::user_text_message(
    "9.11 and 9.8, which is greater? Answer with the number only.",
  )];
  let mut params = ChatCompletionParamsBuilder::new();
  params.reasoning_effort("high").include_usage();
  let stream = client
    .chat_completion_stream(&model, &messages, &params)
    .await
    .unwrap();
  pin_mut!(stream);
  let mut message = ChatMessage::new();
  while let Some(event) = stream.next().await {
    match event.unwrap() {
      ChatCompletionStreamEvent::Delta(delta) => message.apply_model_response_chunk(delta),
      ChatCompletionStreamEvent::Usage(usage) => println!(
        "\n[usage] prompt={:?} completion={:?} reasoning_tokens={:?}",
        usage.prompt_tokens,
        usage.completion_tokens,
        usage
          .completion_tokens_details
          .as_ref()
          .and_then(|d| d.reasoning_tokens)
      ),
    }
  }
  match &message.reasoning_content {
    Some(reasoning) => println!("[reasoning] {} chars of chain of thought", reasoning.len()),
    None => println!("[reasoning] none returned"),
  }
  println!("[answer] {}", message.content);

  println!("== Responses API: reasoning.effort ==");
  let input = ResponsesInput::Text("9.11 and 9.8, which is greater?".to_string());
  let mut params = ResponsesParamsBuilder::new();
  params
    .instructions("You are a helpful assistant.")
    .reasoning_effort("high");
  let stream = client
    .responses_stream(&model, &input, &params)
    .await
    .unwrap();
  pin_mut!(stream);
  while let Some(event) = stream.next().await {
    match event.unwrap() {
      ResponsesStreamEvent::ReasoningTextDelta { delta, .. } => eprint!("{}", delta),
      ResponsesStreamEvent::OutputTextDelta { delta, .. } => {
        print!("{}", delta);
        let _ = std::io::stdout().flush();
      }
      ResponsesStreamEvent::Completed(response) => println!(
        "\n[completed] status={}, reasoning_tokens={:?}",
        response.status,
        response
          .usage
          .as_ref()
          .and_then(|u| u.output_tokens_details.as_ref())
          .and_then(|d| d.reasoning_tokens)
      ),
      ResponsesStreamEvent::Incomplete(response) => {
        println!("\n[incomplete] status={}", response.status)
      }
      ResponsesStreamEvent::Failed(response) => println!("\n[failed] error={:?}", response.error),
      _ => {}
    }
  }
  Ok(())
}
