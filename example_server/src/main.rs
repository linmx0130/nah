/* This file is released in the public domain.
 */
use nah_mcp_types::{MCPResourceDefinition, MCPToolDefinition};
use nah_server::*;
use serde_json::{json, Value};

struct ExampleServer {}

impl AbstractMCPServer for ExampleServer {
    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "example-server".to_string(),
            version: "0.1.0".to_string(),
        }
    }

    fn get_tools_list(&self) -> Vec<MCPToolDefinition> {
        vec![MCPToolDefinition {
            name: "foo".to_string(),
            description: Some("First part of foobar".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "bar": {"type": "string"}
                }
            }),
            annotations: None,
        }]
    }

    fn on_tool_call(
        &mut self,
        name: &str,
        args: Option<&serde_json::Map<String, Value>>,
    ) -> String {
        "I don't know what you are requesting because I'm only an example.".to_string()
    }

    fn get_resources_list(&self) -> Vec<MCPResourceDefinition> {
        vec![MCPResourceDefinition::direct_resource(
            "files://text".to_string(),
            "text".to_string(),
            Some("A text file".to_string()),
            None,
            None,
        )]
    }
}

fn main() {
    let mut server = ExampleServer {};
    run_mcp_server_with_stdio(&mut server).unwrap();
}
