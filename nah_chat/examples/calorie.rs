/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use futures_util::{StreamExt, pin_mut};
use nah_chat::{ChatClient, ChatMessage, ChatMessageContentValue, TypedChatMessageContent};
use std::collections::HashMap;
use std::io;

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
  let base_url = std::env::var("BASE_URL").unwrap();
  let auth_token = std::env::var("AUTH_TOKEN").unwrap();
  let model_name = std::env::var("MODEL").unwrap();

  let chat_client = ChatClient::init(base_url, Some(auth_token));
  let messages = vec![ChatMessage {
    role: "user".to_owned(),
    content: ChatMessageContentValue::TypedContentList(vec![
      TypedChatMessageContent::image_url_content(
        "https://upload.wikimedia.org/wikipedia/commons/thumb/a/ad/Katsudon_001.jpg/330px-Katsudon_001.jpg",
      ),
      TypedChatMessageContent::text_content(
        "Try to figure out the calorie value of this meal. The answer may not be very accurate, but should be informative.",
      ),
    ]),
    reasoning_content: None,
    tool_call_id: None,
    tool_calls: None,
  }];
  let params = HashMap::new();
  let stream = chat_client
    .chat_completion_stream(&model_name, &messages, &params)
    .await
    .unwrap();
  pin_mut!(stream);

  // buffer for the new message
  let mut message = ChatMessage::new();

  // consume the stream
  while let Some(delta_result) = stream.next().await {
    match delta_result {
      Ok(delta) => {
        println!("{:?}", delta);
        message.apply_model_response_chunk(delta);
      }
      Err(e) => {
        eprintln!("Error occurred while processing the chat completion: {}", e);
      }
    }
  }
  println!("{}", message.content);
  Ok(())
}
