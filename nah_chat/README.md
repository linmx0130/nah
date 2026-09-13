# Introduction
This crate exposes an async stream API for the widely-used OpenAI
[chat completion API](https://platform.openai.com/docs/api-reference/chat) and the
[Responses API](https://platform.openai.com/docs/api-reference/responses).

Supported features:
* Stream generation
* Tool calls
* Reasoning content (Qwen3, Deepseek R1, etc)
* Reasoning effort control (`reasoning_effort` in the chat completion API, `reasoning.effort` in
  the Responses API)
* Token usage in the chat completion stream (via `stream_options.include_usage`)
* Responses API (stream + non-stream, tool calls, reasoning)
This crate is built on top of `reqwest` and `serde_json`.

```rust
use nah_chat::{ChatClient, ChatCompletionStreamEvent, ChatMessage};
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
while let Some(event_result) = stream.next().await {
  match event_result {
    Ok(ChatCompletionStreamEvent::Delta(delta)) => {
      message.apply_model_response_chunk(delta);
    }
    Ok(ChatCompletionStreamEvent::Usage(usage)) => {
      // The final chunk carries the authoritative token usage of the call.
      eprintln!("Usage: {} prompt + {} completion tokens",
                usage.prompt_tokens.unwrap_or(0), usage.completion_tokens.unwrap_or(0));
    }
    Err(e) => {
      eprintln!("Error occurred while processing the chat completion: {}", e);
    }
  }
}
```

### Token usage

OpenAI-compatible streaming APIs only report token usage when the request sets
`stream_options.include_usage`. Use the params builder convenience:

```rust
let mut params = ChatCompletionParamsBuilder::new();
params.max_tokens(4096).include_usage();
```

The server then sends a final chunk with empty `choices` and a top-level `usage` object, which
`chat_completion_stream` surfaces as a `ChatCompletionStreamEvent::Usage(ChatCompletionUsage)`
event, yielded after the final delta and right before `[DONE]`. Consumers should treat it as the
*latest* (authoritative) token count for the call. `ChatCompletionUsage` mirrors `ResponseUsage`:
all fields are optional (`Option<u64>` + `#[serde(default)]`), so partial provider responses
deserialize; DeepSeek-specific extras are ignored.

### Reasoning effort

The two APIs spell the reasoning effort hint differently:

| API | Request field | Typical values |
|---|---|---|
| Chat completion | `reasoning_effort` (top level) | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` |
| Responses | `reasoning.effort` (nested in `reasoning`) | same |

```rust
use nah_chat::{ChatCompletionParamsBuilder, ResponsesParamsBuilder};

let mut params = ChatCompletionParamsBuilder::new();
params.max_tokens(4096).reasoning_effort("high");

let mut params = ResponsesParamsBuilder::new();
params.reasoning_effort("high");
```

The string is sent verbatim, so provider-specific values keep working; which values a model accepts
is model-dependent, and an unsupported value is rejected by the server (HTTP 400).
`reasoning_effort` merges into an existing `reasoning` object, so it can be combined with the raw
setter: `params.reasoning(serde_json::json!({"summary": "auto"})).reasoning_effort("high")`.

**DeepSeek thinking mode**: the effort is `low` / `high` / `max` (other values are mapped by the
server, e.g. `medium` -> `high`); thinking itself is toggled by a separate field — `thinking:
{"type": "enabled" | "disabled"}` in the chat completion API, or `reasoning.effort: "none"` in the
Responses API. Pass the toggle through the escape hatch:

```rust
let mut params = ChatCompletionParamsBuilder::new();
params.reasoning_effort("max");
params.insert("thinking", serde_json::json!({"type": "enabled"}));
```

In DeepSeek thinking mode `temperature`, `frequency_penalty` and `presence_penalty` have no effect
and `top_p` is clamped to >= 0.95.

## Responses API

The [Responses API](https://platform.openai.com/docs/api-reference/responses) is supported via
`responses()` (non-stream) and `responses_stream()` (SSE event stream). The endpoint is
`{base_url}/responses` — for DeepSeek use `https://api.deepseek.com`, for OpenAI use
`https://api.openai.com/v1` (the base URL must not end with `/`).

The stream yields typed `ResponsesStreamEvent`s and terminates on `response.completed` /
`response.incomplete` / `response.failed` (no `data: [DONE]`). See `examples/responses.rs`
and `examples/responses_tool.rs` for runnable examples.

# Notice
Copyright 2025, [Mengxiao Lin](linmx0130@gmail.com).
This is a part of [nah](https://github.com/linmx0130/nah) project. `nah` means "*N*ot *A*
*H*uman". Source code is available under [MPL-2.0](https://mozilla.org/MPL/2.0/).

