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
    content: "A".to_owned(),
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
  assert_eq!(message.content, "A test");
  assert_eq!(message.reasoning_content.unwrap(), "reason");
}

#[test]
fn test_apply_tool_calls() {
  let mut message = ChatMessage {
    role: "assistant".to_owned(),
    content: "A".to_owned(),
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
  params_builder.temperature(0.7).top_p(0.9).max_tokens(10000);
  params_builder.insert("customized_key", json!("customized_value"));
  let params = params_builder.build();
  assert_eq!(params["temperature"], 0.7);
  assert_eq!(params["top_p"], 0.9);
  assert_eq!(params["max_tokens"], 10000);
  assert_eq!(params["customized_key"], "customized_value");
  assert_eq!(params.len(), 4);
}
