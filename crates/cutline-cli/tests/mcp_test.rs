use cutline_cli::mcp::run_mcp_server;
use serde_json::{json, Value};
use std::io::Cursor;

fn run_mcp_dialogue(requests: &[Value]) -> Vec<Value> {
    let mut input_bytes = Vec::new();
    for req in requests {
        let line = serde_json::to_string(req).unwrap();
        input_bytes.extend_from_slice(line.as_bytes());
        input_bytes.push(b'\n');
    }

    let cursor = Cursor::new(input_bytes);
    let mut output_bytes = Vec::new();

    run_mcp_server(cursor, &mut output_bytes).expect("MCP server should run without IO error");

    let output_str = String::from_utf8(output_bytes).expect("MCP output should be UTF-8");
    let mut responses = Vec::new();
    for line in output_str.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let val: Value = serde_json::from_str(trimmed)
                .unwrap_or_else(|e| panic!("Failed to parse line as JSON: {line} - Error: {e}"));
            responses.push(val);
        }
    }
    responses
}

#[test]
fn test_mcp_initialize() {
    let requests = vec![json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "antigravity", "version": "1.0.0" }
        }
    })];

    let responses = run_mcp_dialogue(&requests);
    assert_eq!(responses.len(), 1);

    let resp = &responses[0];
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(resp["result"]["serverInfo"]["name"], "cutline");
    assert!(resp["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn test_mcp_notifications_and_ping() {
    let requests = vec![
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "ping"
        }),
    ];

    let responses = run_mcp_dialogue(&requests);
    // Notification produces no response, only ping responds
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], 42);
    assert_eq!(responses[0]["result"], json!({}));
}

#[test]
fn test_mcp_tools_list() {
    let requests = vec![json!({
        "jsonrpc": "2.0",
        "id": "req_tools",
        "method": "tools/list"
    })];

    let responses = run_mcp_dialogue(&requests);
    assert_eq!(responses.len(), 1);

    let tools = responses[0]["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    // All core operations exposed
    assert!(tools.len() >= 14);

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(tool_names.contains(&"probe_media"));
    assert!(tool_names.contains(&"init_project"));
    assert!(tool_names.contains(&"extract_audio"));
    assert!(tool_names.contains(&"transcribe_audio"));
    assert!(tool_names.contains(&"detect_silence"));
    assert!(tool_names.contains(&"cut_timeline"));
    assert!(tool_names.contains(&"auto_cut_silence"));
    assert!(tool_names.contains(&"apply_captions"));
    assert!(tool_names.contains(&"add_overlay"));
    assert!(tool_names.contains(&"list_assets"));
    assert!(tool_names.contains(&"preview_frame"));
    assert!(tool_names.contains(&"preview_strip"));
    assert!(tool_names.contains(&"lint_project"));
    assert!(tool_names.contains(&"render_video"));
    assert!(tool_names.contains(&"get_schema"));
}

#[test]
fn test_mcp_tool_call_assets_and_schema() {
    let requests = vec![
        json!({
            "jsonrpc": "2.0",
            "id": 101,
            "method": "tools/call",
            "params": {
                "name": "list_assets",
                "arguments": {}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 102,
            "method": "tools/call",
            "params": {
                "name": "get_schema",
                "arguments": {}
            }
        }),
    ];

    let responses = run_mcp_dialogue(&requests);
    assert_eq!(responses.len(), 2);

    // list_assets
    assert_eq!(responses[0]["id"], 101);
    assert_eq!(responses[0]["result"]["isError"], false);
    let assets_text = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(assets_text.contains("svg:bird-flying"));
    assert!(assets_text.contains("template:lower-third"));

    // get_schema
    assert_eq!(responses[1]["id"], 102);
    assert_eq!(responses[1]["result"]["isError"], false);
    let schema_text = responses[1]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(schema_text.contains("\"$schema\""));
    assert!(schema_text.contains("\"Project\""));
}

#[test]
fn test_mcp_tool_call_error_handling() {
    let requests = vec![json!({
        "jsonrpc": "2.0",
        "id": 201,
        "method": "tools/call",
        "params": {
            "name": "probe_media",
            "arguments": {
                "path": "non_existent_file_xyz123.mp4"
            }
        }
    })];

    let responses = run_mcp_dialogue(&requests);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], 201);
    assert_eq!(responses[0]["result"]["isError"], true);

    let err_msg = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(err_msg.contains("ERR_"));
    assert!(err_msg.contains("Hint:"));
}

#[test]
fn test_mcp_unknown_method_error() {
    let requests = vec![json!({
        "jsonrpc": "2.0",
        "id": 301,
        "method": "non_existent_method"
    })];

    let responses = run_mcp_dialogue(&requests);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], 301);
    assert_eq!(responses[0]["error"]["code"], -32601);
}

fn ensure_test_video(path: &std::path::Path) {
    if path.exists() {
        return;
    }
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=1080x1920:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=800:duration=2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ])
        .arg(path)
        .status()
        .expect("Failed to create test clip");
    assert!(status.success());
}

#[test]
fn test_mcp_end_to_end_agent_flow() {
    let temp_dir = std::env::temp_dir().join("cutline_test_mcp_e2e");
    let _ = std::fs::create_dir_all(&temp_dir);
    let video_path = temp_dir.join("agent_raw.mp4");
    ensure_test_video(&video_path);

    let project_path = temp_dir.join("agent_project.json");
    let preview_out = temp_dir.join("agent_preview.png");

    let requests = vec![
        // 1. Agent initializes project from raw footage
        json!({
            "jsonrpc": "2.0",
            "id": "step1_init",
            "method": "tools/call",
            "params": {
                "name": "init_project",
                "arguments": {
                    "video_path": video_path.to_string_lossy().to_string(),
                    "out_path": project_path.to_string_lossy().to_string(),
                    "preset": "tiktok"
                }
            }
        }),
        // 2. Agent adds an animated flying bird overlay
        json!({
            "jsonrpc": "2.0",
            "id": "step2_overlay",
            "method": "tools/call",
            "params": {
                "name": "add_overlay",
                "arguments": {
                    "project_path": project_path.to_string_lossy().to_string(),
                    "asset_id": "svg:bird-flying",
                    "start_time": "00:00:00.500",
                    "duration": "00:00:01.000",
                    "anchor": "top_center",
                    "reason": "Agent illustrated speaker concept"
                }
            }
        }),
        // 3. Agent checks quality gate via linter
        json!({
            "jsonrpc": "2.0",
            "id": "step3_lint",
            "method": "tools/call",
            "params": {
                "name": "lint_project",
                "arguments": {
                    "project_path": project_path.to_string_lossy().to_string()
                }
            }
        }),
        // 4. Agent renders preview frame to visually check layout
        json!({
            "jsonrpc": "2.0",
            "id": "step4_preview",
            "method": "tools/call",
            "params": {
                "name": "preview_frame",
                "arguments": {
                    "project_path": project_path.to_string_lossy().to_string(),
                    "time": "00:00:00.800",
                    "out_path": preview_out.to_string_lossy().to_string()
                }
            }
        }),
    ];

    let responses = run_mcp_dialogue(&requests);
    assert_eq!(responses.len(), 4);

    assert_eq!(responses[0]["id"], "step1_init");
    assert_eq!(responses[0]["result"]["isError"], false);

    assert_eq!(responses[1]["id"], "step2_overlay");
    assert_eq!(responses[1]["result"]["isError"], false);

    assert_eq!(responses[2]["id"], "step3_lint");
    assert_eq!(responses[2]["result"]["isError"], false);
    let lint_text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(lint_text.contains("\"passed\": true"));

    assert_eq!(responses[3]["id"], "step4_preview");
    assert_eq!(responses[3]["result"]["isError"], false);
    assert!(preview_out.exists());
    assert!(std::fs::metadata(&preview_out).unwrap().len() > 0);
}
