# Introduction
This crate exposes an async stream API for the widely-used OpenAI
[chat completion API](https://platform.openai.com/docs/api-reference/chat).

Supported features:
* Stream generation
* Tool calls
* Reasoning content (Qwen3, Deepseek R1, etc)
This crate is built on top of `reqwest` and `serde_json`.

```rust
use nah_chat::{ChatClient, ChatMessage};
use futures_util::{pin_mut, StreamExt};

let chat_client = ChatClient::init(base_url, auth_token);

// create and pin the stream
let stream = chat_client
       .chat_completion_stream(model_name, &messages, &params)
       .await
       .unwrap();
pin_mut!(stream);

// buffer for the new message
let mut message = ChatMessage::new();

// consume the stream
while let Some(delta_result) = stream.next().await {
  match delta_result {
    Ok(delta) => {
      message.apply_model_response_chunk(delta);
    }
    Err(e) => {
      eprintln!("Error occurred while processing the chat completion: {}", e);
    }
  }
}
```

# Notice
Copyright 2025, [Mengxiao Lin](linmx0130@gmail.com).
This is a part of [nah](https://github.com/linmx0130/nah) project. `nah` means "*N*ot *A*
*H*uman". Source code is available under [MPL-2.0](https://mozilla.org/MPL/2.0/).

