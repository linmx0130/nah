/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::AbstractMCPServer;
use nah_mcp_types::request::MCPRequest;
use nah_mcp_types::MCPResponse;
use serde_json::{json, Value};
use std::error::Error;
use std::io::{stderr, stdin, stdout, Write};

/**
 * Run the given MCP Server in STDIO.
 */
pub fn run_mcp_server_with_stdio<T>(server: &mut T) -> std::io::Result<()>
where
    T: AbstractMCPServer,
{
    let stdin = stdin();
    let mut buf = String::new();
    loop {
        buf.clear();
        stdin.read_line(&mut buf)?;
        let request: MCPRequest = match serde_json::from_str::<Value>(&buf.trim()) {
            Ok(r) => {
                if r.as_object().is_some_and(|r| r.contains_key("id")) {
                    match serde_json::from_value::<MCPRequest>(r) {
                        Ok(v) => v,
                        Err(e) => {
                            log_error(server, e);
                            continue;
                        }
                    }
                } else {
                    // TOOD: notification process
                    continue;
                }
            }
            Err(e) => {
                log_error(server, e);
                continue;
            }
        };

        match request.method.as_str() {
            "initialize" => {
                process_initialize(server, request)?;
            }
            _ => {
                println!("request: {}", request.method);
            }
        }
    }
}

/**
 * Log an error to stderr.
 */
fn log_error<S, T>(server: &S, e: T)
where
    S: AbstractMCPServer,
    T: Error,
{
    let mut stderr = stderr();
    use chrono::Utc;
    let time_str = Utc::now().to_rfc3339();
    let server_name = server.get_server_info().name;

    let message = format!(
        "[nah-server: {}, {}] {}\n",
        server_name,
        time_str,
        e.to_string()
    );
    let _ = stderr.write(message.as_bytes());
}

/**
 * Process the initialize request
 */
fn process_initialize<T>(server: &mut T, request: MCPRequest) -> std::io::Result<()>
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

    let resp = MCPResponse::new(id.to_string(), Some(result), None);
    let resp_str = serde_json::to_string(&resp).unwrap();
    let mut stdout = stdout();
    stdout.write(resp_str.as_bytes())?;
    stdout.write(b"\n")?;
    stdout.flush()?;
    Ok(())
}
