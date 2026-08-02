/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use futures_util::{StreamExt, pin_mut};
use nah_chat::{ChatClient, ResponsesInput, ResponsesParamsBuilder, ResponsesStreamEvent};
use serde_json::json;
use std::io::Write;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
  let auth_token = std::env::var("DEEPSEEK_API_KEY").unwrap();
  let base_url =
    std::env::var("BASE_URL").unwrap_or_else(|_| "https://api.deepseek.com".to_string());
  let model = std::env::var("MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());

  let client = ChatClient::init(base_url, Some(auth_token));

  // The Responses API tool format is FLAT: {"type":"function","name":...,"parameters":...}
  let tools = json!([{
    "type": "function",
    "name": "get_weather",
    "description": "Get the weather of a city",
    "parameters": {
      "type": "object",
      "properties": {"city": {"type": "string"}},
      "required": ["city"]
    }
  }]);

  let mut params = ResponsesParamsBuilder::new();
  params
    .instructions("You are a helpful assistant.")
    .tools(tools)
    .tool_choice(json!("auto"));

  let input = ResponsesInput::Text("What is the weather in San Francisco?".to_string());
  let stream = client
    .responses_stream(&model, &input, &params)
    .await
    .unwrap();
  pin_mut!(stream);
  while let Some(event_result) = stream.next().await {
    match event_result.unwrap() {
      ResponsesStreamEvent::FunctionCallArgumentsDelta { delta, .. } => {
        eprint!("{}", delta);
        let _ = std::io::stdout().flush();
      }
      ResponsesStreamEvent::OutputTextDelta { delta, .. } => {
        print!("{}", delta);
        let _ = std::io::stdout().flush();
      }
      ResponsesStreamEvent::Completed(response) => {
        println!("\n[completed] status={}", response.status);
        for call in response.function_calls() {
          println!(
            "[function_call] name={} arguments={}",
            call.name.as_deref().unwrap_or(""),
            call.arguments.as_deref().unwrap_or("")
          );
        }
      }
      ResponsesStreamEvent::Incomplete(response) => {
        println!("\n[incomplete] status={}", response.status)
      }
      ResponsesStreamEvent::Failed(response) => {
        println!("\n[failed] error={:?}", response.error)
      }
      _ => {}
    }
  }
  Ok(())
}
