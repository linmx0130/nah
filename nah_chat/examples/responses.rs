/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use futures_util::{StreamExt, pin_mut};
use nah_chat::{ChatClient, ResponsesInput, ResponsesParamsBuilder, ResponsesStreamEvent};
use std::io::Write;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
  let auth_token = std::env::var("DEEPSEEK_API_KEY").unwrap();
  let base_url =
    std::env::var("BASE_URL").unwrap_or_else(|_| "https://api.deepseek.com".to_string());
  let model = std::env::var("MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());

  let client = ChatClient::init(base_url, Some(auth_token));
  let input =
    ResponsesInput::Text("Hi, how are you? Please introduce yourself in one sentence.".to_string());
  let mut params = ResponsesParamsBuilder::new();
  params
    .instructions("You are a helpful assistant.")
    .reasoning(serde_json::json!({"effort": "high"}));

  println!("== Non-stream ==");
  let response = client.responses(&model, &input, &params).await.unwrap();
  println!("{}", response.output_text());
  if let Some(usage) = &response.usage {
    println!(
      "usage: input={:?} output={:?} total={:?} (reasoning_tokens={:?})",
      usage.input_tokens,
      usage.output_tokens,
      usage.total_tokens,
      usage
        .output_tokens_details
        .as_ref()
        .and_then(|d| d.reasoning_tokens)
    );
  }

  println!("== Stream ==");
  let stream = client
    .responses_stream(&model, &input, &params)
    .await
    .unwrap();
  pin_mut!(stream);
  while let Some(event_result) = stream.next().await {
    match event_result.unwrap() {
      ResponsesStreamEvent::ReasoningTextDelta { delta, .. } => {
        eprint!("{}", delta);
      }
      ResponsesStreamEvent::OutputTextDelta { delta, .. } => {
        print!("{}", delta);
        let _ = std::io::stdout().flush();
      }
      ResponsesStreamEvent::Completed(response) => {
        println!(
          "\n[completed] status={}, total_tokens={:?}",
          response.status,
          response.usage.as_ref().and_then(|u| u.total_tokens)
        );
      }
      ResponsesStreamEvent::Incomplete(response) => {
        println!("\n[incomplete] status={}", response.status);
      }
      ResponsesStreamEvent::Failed(response) => {
        println!("\n[failed] error={:?}", response.error);
      }
      _ => {}
    }
  }
  Ok(())
}
