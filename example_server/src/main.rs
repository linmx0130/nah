/* This file is released in the public domain.
 */
use nah_server::*;
use nah_mcp_types::MCPToolDefinition;
use serde_json::Value;

struct ExampleServer {}

impl AbstractMCPServer for ExampleServer {
    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "example-server".to_string(),
            version: "0.1.0".to_string()
        }
    }

    fn get_tools_list(&self) -> Vec<MCPToolDefinition> {
        vec![]
    }

    fn on_tool_call(&mut self, name: String, args: Value) -> String {
        "I don't know what you are requesting because I'm only an example.".to_string()
    }
}

fn main() {
    let mut server = ExampleServer {};
    run_mcp_server_with_stdio(&mut server).unwrap();
}
