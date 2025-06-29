/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::AbstractMCPServer;
use nah_mcp_types::request::MCPRequest;
use nah_mcp_types::MCPResponse;
use serde_json::json;
/**
 * Process the initialize request
 */
pub fn process_initialize<T>(server: &mut T, request: MCPRequest) -> MCPResponse
where
    T: AbstractMCPServer,
{
    let id = request.id.as_str();
    let mut result = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        }
    });
    result.as_object_mut().unwrap().insert(
        "serverInfo".to_string(),
        serde_json::to_value(server.get_server_info()).unwrap(),
    );

    MCPResponse::new(id.to_string(), Some(result), None)
}
