/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::AbstractMCPServer;
use nah_mcp_types::request::MCPRequest;
use nah_mcp_types::MCPResponse;
use serde_json::{json, Value};

/**
 * Process the initialize request
 */
pub fn process_initialize<T>(server: &mut T, request: MCPRequest) -> MCPResponse
where
    T: AbstractMCPServer,
{
    let id = request.id;
    let mut result = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {
                "listChanged": false
            },
            "resources": {
                "listChanged": false
            }
        }
    });
    result.as_object_mut().unwrap().insert(
        "serverInfo".to_string(),
        serde_json::to_value(server.get_server_info()).unwrap(),
    );

    MCPResponse::new(id.clone(), Some(result), None)
}

/**
 * Process tools/list request.
 */
pub fn process_tools_list<T>(server: &mut T, request: MCPRequest) -> MCPResponse
where
    T: AbstractMCPServer,
{
    let id = &request.id;
    let tools_list: Vec<Value> = server
        .get_tools_list()
        .into_iter()
        .map(|v| serde_json::to_value(v).unwrap())
        .collect();
    let mut result_map = serde_json::Map::new();
    result_map.insert("tools".to_string(), Value::Array(tools_list));
    let result = Value::Object(result_map);
    MCPResponse::new(id.clone(), Some(result), None)
}

/**
 * Process tools/call request.
 */
pub fn process_tools_call<T>(server: &mut T, mut request: MCPRequest) -> MCPResponse
where
    T: AbstractMCPServer,
{
    let id = &request.id;
    let params_value = request.params.take();
    let params = match params_value {
        Some(p) => match p.as_object() {
            Some(params) => params.clone(),
            None => {
                return invalid_params_error_response(
                    id,
                    "Invalid params in the tools/call request".to_string(),
                );
            }
        },
        None => {
            return invalid_params_error_response(
                id,
                "Missing params in the tools/call request".to_string(),
            );
        }
    };
    let name = match params.get("name").and_then(|s| s.as_str()) {
        Some(n) => n,
        None => {
            return invalid_params_error_response(
                id,
                "Missing or invalid name param for tools/call request".to_string(),
            );
        }
    };
    let args = params.get("arguments").and_then(|v| v.as_object());
    let response_content = server.on_tool_call(name, args);
    MCPResponse::new(
        id.clone(),
        Some(json!({
            "content": [{"type": "text", "text": response_content}]
        })),
        None,
    )
}

/**
 * Process resources/list request.
 */
pub fn process_resources_list<T>(server: &mut T, request: MCPRequest) -> MCPResponse
where
    T: AbstractMCPServer,
{
    let id = &request.id;
    let resources_list: Vec<Value> = server
        .get_resources_list()
        .into_iter()
        .filter(|v| v.uri.is_some())
        .filter(|v| v.is_valid_resource_definition())
        .map(|v| serde_json::to_value(v).unwrap())
        .collect();
    let mut result_map = serde_json::Map::new();
    result_map.insert("resources".to_string(), Value::Array(resources_list));
    let result = Value::Object(result_map);
    MCPResponse::new(id.clone(), Some(result), None)
}

/**
 * Process resources/read request.
 */
pub fn process_resources_read<T>(server: &mut T, request: MCPRequest) -> MCPResponse
where
    T: AbstractMCPServer,
{
    let id = &request.id;
    let uri: &str = match request
        .params
        .as_ref()
        .and_then(|params| params.as_object())
        .and_then(|params| params.get("uri"))
        .and_then(|v| v.as_str())
    {
        Some(uri) => uri,
        None => {
            return invalid_params_error_response(
                id,
                "Cannot find uri in the resources/read request".to_string(),
            );
        }
    };

    let contents = server.on_resources_read(uri);
    MCPResponse::new(id.clone(), Some(json!({"contents": contents})), None)
}

fn invalid_params_error_response(id: &Value, message: String) -> MCPResponse {
    MCPResponse::new(
        id.clone(),
        None,
        Some(json!({
            "code": -32603,
            "message": message
        })),
    )
}

pub fn invalid_request(id: &Value, message: String) -> MCPResponse {
    MCPResponse::new(
        id.clone(),
        None,
        Some(json!({
            "code":-32600,
            "message": message
        })),
    )
}
