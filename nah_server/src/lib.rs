/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use nah_mcp_types::{MCPResourceDefinition, MCPToolDefinition};
use serde::Serialize;
use serde_json::Value;

pub(crate) mod process_routine;
mod stdio_server;
pub use crate::stdio_server::run_mcp_server_with_stdio;

/**
 * MCP server info data class.
 */
#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/**
 * The trait for MCP Server instances. All routines in this package will
 * operate on the instance of this trait.
 */
pub trait AbstractMCPServer {
    /**
     * Return the name of this server.
     */
    fn get_server_info(&self) -> ServerInfo;
    /**
     * Return a Vec of all supported tools on this MCP server. The server
     * promises to process the requests to these tool calls.
     */
    fn get_tools_list(&self) -> Vec<MCPToolDefinition>;

    /**
     * Respond to the tool calls. Return a string as the response.
     *
     * Args:
     * * `name`: the name of the function to be caled.
     * * `args`: the arguments of the function call in JSON Value
     */
    fn on_tool_call(&mut self, name: &str, args: Option<&serde_json::Map<String, Value>>)
        -> String;

    /**
     * Return a Vec of all resource definitions.
     */
    fn get_resources_list(&self) -> Vec<MCPResourceDefinition>;
}
