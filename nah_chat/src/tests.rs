/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::*;

#[test]
fn test_apply_text_and_reasoning_content_chunk() {
  let mut message = ChatMessage {
    role: "assistant".to_owned(),
    content: ChatMessageContentValue::Text("A".to_owned()),
    reasoning_content: None,
    tool_call_id: None,
    tool_calls: None,
  };

  message.apply_model_response_chunk(ChatResponseChunkDelta {
    role: Some("assistant".to_owned()),
    content: Some(" test".to_owned()),
    reasoning_content: Some("reason".to_owned()),
    tool_calls: None,
  });

  assert_eq!(message.role, "assistant");
  assert_eq!(
    message.content,
    ChatMessageContentValue::Text("A test".to_owned())
  );
  assert_eq!(message.reasoning_content.unwrap(), "reason");
}

#[test]
fn test_apply_tool_calls() {
  let mut message = ChatMessage {
    role: "assistant".to_owned(),
    content: ChatMessageContentValue::Text("A".to_owned()),
    reasoning_content: None,
    tool_call_id: None,
    tool_calls: None,
  };

  message.apply_model_response_chunk(ChatResponseChunkDelta {
    role: None,
    content: None,
    reasoning_content: None,
    tool_calls: Some(vec![ToolCallRequestChunkDelta {
      index: 0,
      id: Some("123".to_owned()),
      _type: Some("function".to_owned()),
      function: Some(FunctionCallRequestChunkDelta {
        name: Some("x".to_owned()),
        arguments: None,
      }),
    }]),
  });
  assert_eq!(message.role, "assistant");
  {
    let tool_calls = message.tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls[0].id, "123");
    assert_eq!(tool_calls[0].function.name, "x");
  }

  message.apply_model_response_chunk(ChatResponseChunkDelta {
    role: None,
    content: None,
    reasoning_content: None,
    tool_calls: Some(vec![ToolCallRequestChunkDelta {
      index: 0,
      id: None,
      _type: None,
      function: Some(FunctionCallRequestChunkDelta {
        name: Some("yz".to_owned()),
        arguments: Some("{\"a".to_owned()),
      }),
    }]),
  });
  {
    let tool_calls = message.tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls[0].id, "123");
    assert_eq!(tool_calls[0].function.name, "xyz");
    assert_eq!(tool_calls[0].function.arguments, "{\"a");
  }
}

#[test]
fn test_chat_completion_params_builder() {
  let mut params_builder = ChatCompletionParamsBuilder::new();
  params_builder
    .temperature(0.7)
    .top_p(0.9)
    .max_tokens(10000)
    .include_usage();
  params_builder.insert("customized_key", json!("customized_value"));
  let params = params_builder.build();
  assert_eq!(params["temperature"], 0.7);
  assert_eq!(params["top_p"], 0.9);
  assert_eq!(params["max_tokens"], 10000);
  assert_eq!(params["customized_key"], "customized_value");
  assert_eq!(params["stream_options"], json!({"include_usage": true}));
  assert_eq!(params.len(), 5);
}

#[test]
fn test_collect_chat_response_chunk_delta() {
  let delta = vec![
    ChatResponseChunkDelta {
      role: Some("assistant".to_owned()),
      content: None,
      reasoning_content: None,
      tool_calls: None,
    },
    ChatResponseChunkDelta {
      role: None,
      content: None,
      reasoning_content: Some("think a bit".to_owned()),
      tool_calls: None,
    },
    ChatResponseChunkDelta {
      role: None,
      content: Some("good content".to_owned()),
      reasoning_content: None,
      tool_calls: None,
    },
    ChatResponseChunkDelta {
      role: None,
      content: Some(" generated".to_owned()),
      reasoning_content: None,
      tool_calls: None,
    },
  ];
  let message: ChatMessage = delta.into_iter().collect();
  assert_eq!(message.role, "assistant");
  assert_eq!(
    message.content,
    ChatMessageContentValue::Text("good content generated".to_owned())
  );
  assert_eq!(message.reasoning_content, Some("think a bit".to_owned()));
}

#[test]
fn test_deserialize_text_content() {
  let data = r#"{
      "role": "user",
      "content": "Hello world"
    }
    "#;
  let message: ChatMessage = serde_json::from_str(data).unwrap();
  assert_eq!(message.role, "user");
  assert_eq!(
    message.content,
    ChatMessageContentValue::Text("Hello world".to_string())
  );
}

#[test]
fn test_deserialize_image_url() {
  let data = r#"{
      "role": "user",
      "content": [{
        "type": "image_url",
        "image_url": {
          "url": "data:image/jpeg;base64"
        }
      }]
    }
    "#;
  let message: ChatMessage = serde_json::from_str(data).unwrap();
  assert_eq!(message.role, "user");
  assert_eq!(
    message.content,
    ChatMessageContentValue::TypedContentList(vec![TypedChatMessageContent {
      data_type: "image_url".to_string(),
      image_url: Some(URLObject {
        url: "data:image/jpeg;base64".to_string()
      }),
      text: None
    }])
  );
}

#[test]
fn test_responses_request_body() {
  use crate::responses::{ResponsesInput, ResponsesParamsBuilder};
  let client = ChatClient::init(
    "https://api.deepseek.com".to_string(),
    Some("k".to_string()),
  );
  let mut params = ResponsesParamsBuilder::new();
  params
    .instructions("You are helpful.")
    .reasoning(serde_json::json!({"effort": "high"}));
  let input = ResponsesInput::Text("Hi".to_string());
  let req = client
    .create_responses_request("deepseek-v4-flash", &input, true, &params)
    .build()
    .unwrap();
  assert_eq!(req.url().path(), "/responses");
  let body: serde_json::Value =
    serde_json::from_slice(req.body().unwrap().as_bytes().unwrap()).unwrap();
  assert_eq!(body["model"], "deepseek-v4-flash");
  assert_eq!(body["input"], "Hi");
  assert_eq!(body["stream"], true);
  assert_eq!(body["instructions"], "You are helpful.");
  assert_eq!(body["reasoning"]["effort"], "high");
}

#[test]
fn test_chat_stream_chunk_split_reassembly() {
  // SSE events split across network chunks must not be lost.
  use crate::responses::SseBuffer;
  let client = ChatClient::init("http://localhost".to_string(), None);
  let delta1 = r#"{"choices":[{"delta":{"role":"assistant","content":"Hello"}}]}"#;
  let delta2 = r#"{"choices":[{"delta":{"content":" world"}}]}"#;
  let fixture = format!("data: {}\n\ndata: {}\n\ndata: [DONE]\n", delta1, delta2);

  let mut buffer = SseBuffer::new();
  let mut deltas: Vec<String> = Vec::new();
  let mut done = false;

  // Cut the first delta payload in half; the boundary falls inside an event.
  let split_at = fixture.find("Hello").unwrap();
  for message in buffer.push(&fixture.as_bytes()[..split_at]) {
    for chunk in client.get_model_response_chunks(&message) {
      match chunk {
        ChatResponseChunk::Delta(d) => {
          if let Some(content) = d.content {
            deltas.push(content);
          }
        }
        ChatResponseChunk::Usage(_) => {}
        ChatResponseChunk::Done => done = true,
      }
    }
  }
  assert!(
    deltas.is_empty(),
    "no complete event should be emitted before the split point"
  );

  for message in buffer.push(&fixture.as_bytes()[split_at..]) {
    for chunk in client.get_model_response_chunks(&message) {
      match chunk {
        ChatResponseChunk::Delta(d) => {
          if let Some(content) = d.content {
            deltas.push(content);
          }
        }
        ChatResponseChunk::Usage(_) => {}
        ChatResponseChunk::Done => done = true,
      }
    }
  }
  for message in buffer.finish() {
    for chunk in client.get_model_response_chunks(&message) {
      match chunk {
        ChatResponseChunk::Delta(d) => {
          if let Some(content) = d.content {
            deltas.push(content);
          }
        }
        ChatResponseChunk::Usage(_) => {}
        ChatResponseChunk::Done => done = true,
      }
    }
  }

  assert_eq!(deltas, vec!["Hello".to_string(), " world".to_string()]);
  assert!(done, "the [DONE] marker must be detected");
}

#[test]
fn test_get_model_response_chunk_usage() {
  // The final chunk of an OpenAI-compatible stream carries `usage` with empty `choices`.
  let client = ChatClient::init("http://localhost".to_string(), None);
  // DeepSeek extras (`prompt_cache_hit_tokens` ...) must be tolerated/ignored.
  let usage_chunk = "{\"choices\": [], \"usage\": {\"prompt_tokens\": 12, \"completion_tokens\": 34, \"total_tokens\": 46, \"prompt_tokens_details\": {\"cached_tokens\": 5}, \"completion_tokens_details\": {\"reasoning_tokens\": 7}, \"prompt_cache_hit_tokens\": 100, \"prompt_cache_miss_tokens\": 200}}";
  let message = format!("data: {}\n", usage_chunk);
  let chunks = client.get_model_response_chunks(&message);
  assert_eq!(chunks.len(), 1, "only a Usage chunk expected: {chunks:?}");
  match &chunks[0] {
    ChatResponseChunk::Usage(usage) => {
      assert_eq!(usage.prompt_tokens, Some(12));
      assert_eq!(usage.completion_tokens, Some(34));
      assert_eq!(usage.total_tokens, Some(46));
      assert_eq!(
        usage.prompt_tokens_details.as_ref().unwrap().cached_tokens,
        Some(5)
      );
      assert_eq!(
        usage
          .completion_tokens_details
          .as_ref()
          .unwrap()
          .reasoning_tokens,
        Some(7)
      );
    }
    _ => panic!("expected a Usage chunk"),
  }
}

#[test]
fn test_get_model_response_chunk_both_delta_and_usage() {
  // Some providers (DeepSeek) attach `usage` to the *same* chunk as the
  // final delta instead of a dedicated empty-choices chunk. Both must be
  // parsed — dropping either loses content or token usage.
  let client = ChatClient::init("http://localhost".to_string(), None);
  let both_chunk = r#"{"choices": [{"delta": {"content": "hi"}}], "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}}"#;
  let message = format!("data: {}\n", both_chunk);
  let chunks = client.get_model_response_chunks(&message);
  assert_eq!(chunks.len(), 2, "delta and usage must both be parsed: {chunks:?}");
  match &chunks[0] {
    ChatResponseChunk::Delta(delta) => {
      assert_eq!(delta.content.as_deref(), Some("hi"));
    }
    _ => panic!("expected a Delta chunk"),
  }
  match &chunks[1] {
    ChatResponseChunk::Usage(usage) => {
      assert_eq!(usage.prompt_tokens, Some(1));
      assert_eq!(usage.total_tokens, Some(2));
    }
    _ => panic!("expected a Usage chunk"),
  }
}

#[test]
fn test_chat_completion_usage_partial_deserialize() {
  // Providers may omit fields; everything must stay optional.
  let usage: ChatCompletionUsage = serde_json::from_str(r#"{"prompt_tokens": 3}"#).unwrap();
  assert_eq!(usage.prompt_tokens, Some(3));
  assert_eq!(usage.completion_tokens, None);
  assert_eq!(usage.total_tokens, None);
  assert!(usage.prompt_tokens_details.is_none());
  assert!(usage.completion_tokens_details.is_none());
}

#[test]
fn test_chat_completion_stream_event_mapping() {
  // The stream loop maps ChatResponseChunk variants onto ChatCompletionStreamEvent.
  let client = ChatClient::init("http://localhost".to_string(), None);

  let usage_message = "data: {\"choices\": [], \"usage\": {\"prompt_tokens\": 12, \"completion_tokens\": 34, \"total_tokens\": 46}}\n";
  let event = match client.get_model_response_chunks(usage_message).into_iter().next() {
    Some(ChatResponseChunk::Usage(u)) => ChatCompletionStreamEvent::Usage(u),
    _ => panic!("expected a Usage chunk"),
  };
  match event {
    ChatCompletionStreamEvent::Usage(usage) => {
      assert_eq!(usage.prompt_tokens, Some(12));
      assert_eq!(usage.total_tokens, Some(46));
    }
    _ => panic!("expected a Usage event"),
  }

  let delta_message = "data: {\"choices\": [{\"delta\": {\"content\": \"hi\"}}]}\n";
  let event = match client.get_model_response_chunks(delta_message).into_iter().next() {
    Some(ChatResponseChunk::Delta(d)) => ChatCompletionStreamEvent::Delta(d),
    _ => panic!("expected a Delta chunk"),
  };
  match event {
    ChatCompletionStreamEvent::Delta(delta) => {
      assert_eq!(delta.content.as_deref(), Some("hi"));
    }
    _ => panic!("expected a Delta event"),
  }
}

#[test]
fn test_deserialize_reasoning_content_snake_case() {
  // DeepSeek (and most OpenAI-compatible providers) send `reasoning_content`.
  let data = r#"{
      "role": "assistant",
      "content": "Hi",
      "reasoning_content": "think a bit"
    }
    "#;
  let message: ChatMessage = serde_json::from_str(data).unwrap();
  assert_eq!(message.reasoning_content.as_deref(), Some("think a bit"));
}

#[test]
fn test_serialize_reasoning_content_snake_case() {
  let message = ChatMessage {
    role: "assistant".to_owned(),
    content: ChatMessageContentValue::Text("Hi".to_owned()),
    reasoning_content: Some("think".to_owned()),
    tool_call_id: None,
    tool_calls: None,
  };
  let value = serde_json::to_value(&message).unwrap();
  assert_eq!(value["reasoning_content"], "think");
  assert!(value.get("reasoningContent").is_none());
}
