use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::types::*;
use crate::analysis;

pub struct McpServer {
    server_info: ServerInfo,
}

impl Default for McpServer {
    fn default() -> Self {
        Self {
            server_info: ServerInfo {
                name: "rustowl-mcp".to_string(),
                version: "0.1.0".to_string(),
            },
        }
    }
}

impl McpServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "rustowl_inspect_cursor".to_string(),
                description: "Inspect Rust variable lifetime, immutable/mutable borrows, moves, and borrow-checker conflicts at a specific file coordinate.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the Rust source file"
                        },
                        "line": {
                            "type": "integer",
                            "description": "1-indexed line number of the variable/token"
                        },
                        "col": {
                            "type": "integer",
                            "description": "1-indexed column number of the variable/token"
                        }
                    },
                    "required": ["path", "line", "col"]
                }),
            },
            McpTool {
                name: "rustowl_inspect_line".to_string(),
                description: "Inspect all Rust variable lifetimes, borrows, and moves on an entire line of code.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the Rust source file"
                        },
                        "line": {
                            "type": "integer",
                            "description": "1-indexed line number to inspect"
                        }
                    },
                    "required": ["path", "line"]
                }),
            },
        ]
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id;
        match req.method.as_str() {
            "initialize" => {
                let res = InitializeResult {
                    protocol_version: "2024-11-05".to_string(),
                    capabilities: ServerCapabilities {
                        tools: Some(ToolsCapability {
                            list_changed: Some(false),
                        }),
                    },
                    server_info: self.server_info.clone(),
                };
                JsonRpcResponse::success(id, json!(res))
            }
            "notifications/initialized" | "initialized" => JsonRpcResponse::success(id, json!({})),
            "tools/list" => {
                let res = ListToolsResult {
                    tools: self.get_tools(),
                };
                JsonRpcResponse::success(id, json!(res))
            }
            "tools/call" => {
                let params = match req.params {
                    Some(p) => match serde_json::from_value::<CallToolParams>(p) {
                        Ok(call) => call,
                        Err(e) => {
                            return JsonRpcResponse::error(
                                id,
                                JsonRpcError::invalid_params(e.to_string()),
                            )
                        }
                    },
                    None => {
                        return JsonRpcResponse::error(
                            id,
                            JsonRpcError::invalid_params("Missing params"),
                        )
                    }
                };

                let tool_res = self.dispatch_tool(params).await;
                JsonRpcResponse::success(id, json!(tool_res))
            }
            _ => JsonRpcResponse::error(id, JsonRpcError::method_not_found(&req.method)),
        }
    }

    async fn dispatch_tool(&self, call: CallToolParams) -> CallToolResult {
        let args = call.arguments.unwrap_or_else(|| json!({}));
        match call.name.as_str() {
            "rustowl_inspect_cursor" => {
                let path_str = match args.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => return CallToolResult::error("Missing 'path' argument"),
                };
                let line = match args.get("line").and_then(|v| v.as_u64()) {
                    Some(l) => l as u32,
                    None => return CallToolResult::error("Missing or invalid 'line' argument"),
                };
                let col = match args.get("col").and_then(|v| v.as_u64()) {
                    Some(c) => c as u32,
                    None => return CallToolResult::error("Missing or invalid 'col' argument"),
                };

                let path = std::path::Path::new(path_str);
                match analysis::inspect_cursor(path, line, col).await {
                    Ok(report) => match serde_json::to_string_pretty(&report) {
                        Ok(json) => CallToolResult::text(json),
                        Err(e) => {
                            CallToolResult::error(format!("Failed to serialize report: {}", e))
                        }
                    },
                    Err(e) => CallToolResult::error(format!("RustOwl analysis error: {}", e)),
                }
            }
            "rustowl_inspect_line" => {
                let path_str = match args.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => return CallToolResult::error("Missing 'path' argument"),
                };
                let line = match args.get("line").and_then(|v| v.as_u64()) {
                    Some(l) => l as u32,
                    None => return CallToolResult::error("Missing or invalid 'line' argument"),
                };

                let path = std::path::Path::new(path_str);
                match analysis::inspect_line(path, line).await {
                    Ok(reports) => match serde_json::to_string_pretty(&reports) {
                        Ok(json) => CallToolResult::text(json),
                        Err(e) => {
                            CallToolResult::error(format!("Failed to serialize reports: {}", e))
                        }
                    },
                    Err(e) => CallToolResult::error(format!("RustOwl analysis error: {}", e)),
                }
            }
            _ => CallToolResult::error(format!("Unknown tool: {}", call.name)),
        }
    }
}

pub async fn run_stdio_server() -> Result<()> {
    let server = Arc::new(McpServer::new());
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    while let Some(line) = reader.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_res =
                    JsonRpcResponse::error(None, JsonRpcError::parse_error(e.to_string()));
                let mut out = serde_json::to_string(&err_res)?;
                out.push('\n');
                stdout.write_all(out.as_bytes()).await?;
                stdout.flush().await?;
                continue;
            }
        };

        // If it's a notification without id, we handle it but don't respond unless protocol requires
        let has_id = req.id.is_some();
        let res = server.handle_request(req).await;

        if has_id {
            let mut out = serde_json::to_string(&res)?;
            out.push('\n');
            stdout.write_all(out.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
