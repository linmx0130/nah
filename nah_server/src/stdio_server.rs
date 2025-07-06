/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::process_routine::{
    invalid_request, process_initialize, process_resources_list, process_resources_read,
    process_tools_call, process_tools_list,
};
use crate::AbstractMCPServer;
use nah_mcp_types::request::MCPRequest;
use nah_mcp_types::MCPResponse;
use serde_json::Value;
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
                    // TODO: notification process
                    continue;
                }
            }
            Err(e) => {
                log_error(server, e);
                continue;
            }
        };

        let response = match request.method.as_str() {
            "initialize" => process_initialize(server, request),
            "tools/list" => process_tools_list(server, request),
            "tools/call" => process_tools_call(server, request),
            "resources/list" => process_resources_list(server, request),
            "resources/read" => process_resources_read(server, request),
            _ => invalid_request(&request.id, format!("Unknown method: {}", request.method)),
        };
        send_response(response)?;
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
 * Send a response to stdout.
 */
fn send_response(resp: MCPResponse) -> std::io::Result<()> {
    let resp_str = serde_json::to_string(&resp).unwrap();
    let mut stdout = stdout();
    stdout.write(resp_str.as_bytes())?;
    stdout.write(b"\n")?;
    stdout.flush()?;
    Ok(())
}
