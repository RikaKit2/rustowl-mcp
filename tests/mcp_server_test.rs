use rustowl_mcp::analysis::normalize_decorations;
use rustowl_mcp::mcp::server::McpServer;
use rustowl_mcp::mcp::types::*;
use rustowl_mcp::owl::protocol::*;
use serde_json::json;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_mcp_initialize_and_tools_list() {
    let server = McpServer::new();

    // 1. Test initialize
    let init_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0" }
        })),
    };

    let init_res = server.handle_request(init_req).await;
    assert_eq!(init_res.id, Some(json!(1)));
    assert!(init_res.result.is_some());
    assert!(init_res.error.is_none());

    // 2. Test tools/list
    let list_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "tools/list".to_string(),
        params: None,
    };

    let list_res = server.handle_request(list_req).await;
    assert_eq!(list_res.id, Some(json!(2)));
    let result_val = list_res.result.expect("Expected result from tools/list");
    let tools_list: ListToolsResult =
        serde_json::from_value(result_val).expect("Failed to parse ListToolsResult");
    assert_eq!(tools_list.tools.len(), 2);
    assert_eq!(tools_list.tools[0].name, "rustowl_inspect_cursor");
    assert_eq!(tools_list.tools[1].name, "rustowl_inspect_line");
}

#[tokio::test]
async fn test_normalization_decorations_and_conflicts() {
    let dummy_path = Path::new("src/main.rs");
    let decorations = vec![
        RustOwlDecoration::new(
            Range {
                start: Position {
                    line: 9,
                    character: 4,
                },
                end: Position {
                    line: 24,
                    character: 1,
                },
            },
            DecorationType::DefinitelyLive,
        ),
        RustOwlDecoration::new(
            Range {
                start: Position {
                    line: 11,
                    character: 8,
                },
                end: Position {
                    line: 13,
                    character: 1,
                },
            },
            DecorationType::ImmBorrow,
        ),
        RustOwlDecoration::new(
            Range {
                start: Position {
                    line: 17,
                    character: 8,
                },
                end: Position {
                    line: 21,
                    character: 1,
                },
            },
            DecorationType::MutBorrow,
        ),
        RustOwlDecoration::new(
            Range {
                start: Position {
                    line: 20,
                    character: 4,
                },
                end: Position {
                    line: 20,
                    character: 12,
                },
            },
            DecorationType::Outlive,
        ),
    ];

    let report = normalize_decorations(dummy_path, 10, 5, &decorations);

    assert_eq!(report.file, "src/main.rs");
    assert_eq!(report.target.line, 10);
    assert_eq!(report.target.col, 5);
    assert!(report.live_span.is_some());
    let live = report.live_span.unwrap();
    assert_eq!(live.start_line, 10);
    assert_eq!(live.end_line, 25);
    assert_eq!(live.certainty, "definitely_live");

    assert_eq!(report.active_borrows.len(), 2);
    assert_eq!(report.active_borrows[0].kind, "immutable");
    assert_eq!(report.active_borrows[1].kind, "mutable");

    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(report.conflicts[0].kind, "outlive_conflict");
    assert!(!report.actionable_fixes.is_empty());
    assert!(report.summary.contains("ALERT"));
}

#[tokio::test]
async fn test_tool_call_flow_with_temp_file() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "fn calculate() {{").unwrap();
    writeln!(temp, "    let mut data = vec![1, 2, 3];").unwrap();
    writeln!(temp, "    let r = &mut data;").unwrap();
    writeln!(temp, "    r.push(4);").unwrap();
    writeln!(temp, "}}").unwrap();
    temp.flush().unwrap();

    let server = McpServer::new();

    let call_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(42)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "rustowl_inspect_cursor",
            "arguments": {
                "path": temp.path().to_str().unwrap(),
                "line": 2,
                "col": 13
            }
        })),
    };

    let res = server.handle_request(call_req).await;
    assert_eq!(res.id, Some(json!(42)));
    assert!(res.result.is_some());

    let tool_res: CallToolResult = serde_json::from_value(res.result.unwrap()).unwrap();
    assert!(!tool_res.is_error);
    assert!(!tool_res.content.is_empty());

    match &tool_res.content[0] {
        ToolContent::Text { text } => {
            assert!(text.contains("data") || text.contains("live_span"));
        }
    }
}
