use crate::error::AppError;
use crate::ops::*;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

pub struct McpServer;

impl McpServer {
    pub fn new() -> Self {
        Self
    }

    pub fn list_tools() -> Value {
        json!({
            "tools": [
                {
                    "name": "probe_media",
                    "description": "Inspects video stream dimensions, fps, audio tracks, and codecs.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Path to video or media file" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "init_project",
                    "description": "Creates a new project.json workspace from a primary video.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "video_path": { "type": "string", "description": "Primary talking-head video file path" },
                            "b_roll": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional paths to B-roll cutaway footage files"
                            },
                            "preset": { "type": "string", "description": "Output preset (tiktok, youtube, fast-draft)", "default": "tiktok" },
                            "out_path": { "type": "string", "description": "Target project.json file output path", "default": "project.json" }
                        },
                        "required": ["video_path"]
                    }
                },
                {
                    "name": "extract_audio",
                    "description": "Extracts 16kHz mono audio for transcription.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Project file or media path" },
                            "out_path": { "type": "string", "description": "Optional destination WAV path" }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "transcribe_audio",
                    "description": "Transcribes dialogue with word-level timestamps using offline Whisper.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Project file or audio path" },
                            "model": { "type": "string", "description": "Whisper model variant (tiny, base, small)", "default": "base" },
                            "out_path": { "type": "string", "description": "Optional destination transcript.json path" }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "detect_silence",
                    "description": "Detects pauses and dead air in dialogue using dual-threshold RMS hysteresis.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Project file or audio WAV path" },
                            "threshold_db": { "type": "number", "description": "Activation threshold in dB (default: -32.0)", "default": -32.0 },
                            "min_duration_ms": { "type": "integer", "description": "Minimum silence duration in milliseconds (default: 300)", "default": 300 }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "detect_scenes",
                    "description": "Detects visual shot transitions and scene cuts in the primary video.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Project file or video file" },
                            "threshold": { "type": "number", "description": "Scene change sensitivity (default: 0.4)", "default": 0.4 }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "cut_timeline",
                    "description": "Excises a time range from the timeline.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "start_time": { "type": "string", "description": "Start timestamp or frame (e.g. 00:00:10.500, 10.5s, frame:315)" },
                            "end_time": { "type": "string", "description": "End timestamp or frame" }
                        },
                        "required": ["project_path", "start_time", "end_time"]
                    }
                },
                {
                    "name": "auto_cut_silence",
                    "description": "Excises all detected silences automatically while preserving speech padding.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "padding_ms": { "type": "integer", "description": "Safety padding in ms around speech (default: 50)", "default": 50 }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "trim_timeline",
                    "description": "Trims global project start and end points.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "trim_in": { "type": "string", "description": "Trim in timestamp or frame" },
                            "trim_out": { "type": "string", "description": "Trim out timestamp or frame" }
                        },
                        "required": ["project_path", "trim_in", "trim_out"]
                    }
                },
                {
                    "name": "apply_captions",
                    "description": "Formats and positions kinetic animated word-by-word captions tied to transcript timings and safe area.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "style": { "type": "string", "description": "Caption style preset (clean-yellow, mono-warm) or custom JSON style", "default": "clean-yellow" },
                            "transcript_path": { "type": "string", "description": "Optional explicit path to transcript.json" }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "add_overlay",
                    "description": "Places a visual overlay or procedural motion template onto the timeline.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "asset_id": { "type": "string", "description": "Asset ID (e.g. svg:bird-flying, template:lower-third, template:keyword-badge, template:animated-progress, template:punch-in-zoom)" },
                            "start_time": { "type": "string", "description": "Start timestamp or frame (e.g. 00:00:01.500)" },
                            "duration": { "type": "string", "description": "Duration timestamp or frame (e.g. 00:00:02.000)" },
                            "anchor": { "type": "string", "description": "Anchor position (top_center, center, lower_third, bottom_center)", "default": "top_center" },
                            "scale": { "type": "number", "description": "Scale multiplier (default: 1.0)", "default": 1.0 },
                            "opacity": { "type": "number", "description": "Opacity from 0.0 to 1.0 (default: 1.0)", "default": 1.0 },
                            "title": { "type": "string", "description": "Title text for template overlays" },
                            "subtitle": { "type": "string", "description": "Subtitle text for template overlays" },
                            "reason": { "type": "string", "description": "Semantic rationale (e.g. 'Speaker said: bird is flying')" },
                            "word_range": {
                                "type": "array",
                                "items": { "type": "integer" },
                                "description": "Associated transcript word indices"
                            }
                        },
                        "required": ["project_path", "asset_id", "start_time", "duration"]
                    }
                },
                {
                    "name": "list_overlays",
                    "description": "Lists active overlays in the project timeline.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "remove_overlay",
                    "description": "Removes an overlay from the project timeline by ID.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "overlay_id": { "type": "string", "description": "Overlay ID to remove" }
                        },
                        "required": ["project_path", "overlay_id"]
                    }
                },
                {
                    "name": "list_assets",
                    "description": "Returns all available motion templates, vector graphics, and registered assets.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "add_asset",
                    "description": "Registers an SVG or custom visual asset into the asset library.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Path to SVG or image file" },
                            "id": { "type": "string", "description": "Optional custom asset identifier" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "preview_frame",
                    "description": "Renders a single frame with composited video, captions, and overlays to PNG for visual inspection.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "time": { "type": "string", "description": "Timestamp or frame (e.g. 00:00:12.400, 12.4s, frame:372)" },
                            "out_path": { "type": "string", "description": "Target PNG path", "default": "frame.png" }
                        },
                        "required": ["project_path", "time"]
                    }
                },
                {
                    "name": "preview_strip",
                    "description": "Generates an evenly spaced series of thumbnail frames across the timeline.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "count": { "type": "integer", "description": "Number of preview frames (default: 8)", "default": 8 },
                            "out_dir": { "type": "string", "description": "Output directory for thumbnail images", "default": "previews" }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "lint_project",
                    "description": "Validates edit rules, safe areas, caption collisions, and design constraints.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "transcript_path": { "type": "string", "description": "Optional path to transcript.json for cut-word check" }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "render_video",
                    "description": "Executes high-speed multi-threaded final export.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Path to project.json" },
                            "out_path": { "type": "string", "description": "Output video file path", "default": "final.mp4" },
                            "preset": { "type": "string", "description": "Export preset (tiktok, youtube, fast-draft)" },
                            "hw_accel": { "type": "string", "description": "Hardware encoder preference (auto, nvenc, amf, qsv, vt, vaapi, none)" }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "get_schema",
                    "description": "Returns the JSON Schema for project.json.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        })
    }

    pub fn call_tool(name: &str, args: &Value) -> Result<Value, AppError> {
        match name {
            "probe_media" => {
                let path_str = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                    AppError::new(
                        "ERR_MISSING_ARG",
                        "Missing required argument 'path'",
                        "Specify media path.",
                    )
                })?;
                op_probe(Path::new(path_str))
            }
            "init_project" => {
                let video_path =
                    args.get("video_path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            AppError::new(
                                "ERR_MISSING_ARG",
                                "Missing required argument 'video_path'",
                                "Specify video path.",
                            )
                        })?;
                let b_roll: Vec<PathBuf> = args
                    .get("b_roll")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(PathBuf::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let preset = args
                    .get("preset")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tiktok");
                let out_path = args
                    .get("out_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("project.json");
                op_project_init(Path::new(video_path), &b_roll, preset, Path::new(out_path))
            }
            "extract_audio" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new(
                            "ERR_MISSING_ARG",
                            "Missing 'project_path'",
                            "Specify path to project.json or media file.",
                        )
                    })?;
                let explicit_out = args.get("out_path").and_then(|v| v.as_str()).map(Path::new);
                op_audio_extract(Path::new(project_path), explicit_out)
            }
            "transcribe_audio" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("base");
                let explicit_out = args.get("out_path").and_then(|v| v.as_str()).map(Path::new);
                op_transcribe(Path::new(project_path), model, explicit_out)
            }
            "detect_silence" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let threshold_db = args
                    .get("threshold_db")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(-32.0) as f32;
                let min_duration_ms = args
                    .get("min_duration_ms")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(300);
                op_analyze_silence(Path::new(project_path), threshold_db, min_duration_ms)
            }
            "detect_scenes" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let threshold = args
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.4) as f32;
                op_analyze_scenes(Path::new(project_path), threshold)
            }
            "cut_timeline" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let start_time =
                    args.get("start_time")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            AppError::new(
                                "ERR_MISSING_ARG",
                                "Missing 'start_time'",
                                "Specify start time.",
                            )
                        })?;
                let end_time = args
                    .get("end_time")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'end_time'", "Specify end time.")
                    })?;
                op_timeline_cut(Path::new(project_path), start_time, end_time)
            }
            "auto_cut_silence" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let padding_ms = args
                    .get("padding_ms")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(50);
                op_timeline_auto_cut_silence(Path::new(project_path), padding_ms)
            }
            "trim_timeline" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let trim_in = args
                    .get("trim_in")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new(
                            "ERR_MISSING_ARG",
                            "Missing 'trim_in'",
                            "Specify trim in time.",
                        )
                    })?;
                let trim_out = args
                    .get("trim_out")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new(
                            "ERR_MISSING_ARG",
                            "Missing 'trim_out'",
                            "Specify trim out time.",
                        )
                    })?;
                op_timeline_trim(Path::new(project_path), trim_in, trim_out)
            }
            "apply_captions" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let style = args
                    .get("style")
                    .and_then(|v| v.as_str())
                    .unwrap_or("clean-yellow");
                let explicit_transcript = args
                    .get("transcript_path")
                    .and_then(|v| v.as_str())
                    .map(Path::new);
                op_captions_apply(Path::new(project_path), style, explicit_transcript)
            }
            "add_overlay" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let asset_id = args
                    .get("asset_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'asset_id'", "Specify asset ID.")
                    })?;
                let start_time =
                    args.get("start_time")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            AppError::new(
                                "ERR_MISSING_ARG",
                                "Missing 'start_time'",
                                "Specify start time.",
                            )
                        })?;
                let duration = args
                    .get("duration")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'duration'", "Specify duration.")
                    })?;
                let anchor = args
                    .get("anchor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("top_center");
                let scale = args.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                let opacity = args.get("opacity").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                let title = args.get("title").and_then(|v| v.as_str());
                let subtitle = args.get("subtitle").and_then(|v| v.as_str());
                let reason = args.get("reason").and_then(|v| v.as_str());
                let word_range: Option<Vec<usize>> = args
                    .get("word_range")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|n| n.as_u64().map(|x| x as usize))
                            .collect()
                    });
                op_overlay_add(
                    Path::new(project_path),
                    asset_id,
                    start_time,
                    duration,
                    anchor,
                    scale,
                    opacity,
                    title,
                    subtitle,
                    reason,
                    word_range.as_deref(),
                )
            }
            "list_overlays" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                op_overlay_list(Path::new(project_path))
            }
            "remove_overlay" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let overlay_id =
                    args.get("overlay_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            AppError::new(
                                "ERR_MISSING_ARG",
                                "Missing 'overlay_id'",
                                "Specify overlay ID.",
                            )
                        })?;
                op_overlay_remove(Path::new(project_path), overlay_id)
            }
            "list_assets" => op_assets_list(),
            "add_asset" => {
                let path_str = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                    AppError::new("ERR_MISSING_ARG", "Missing 'path'", "Specify asset path.")
                })?;
                let explicit_id = args.get("id").and_then(|v| v.as_str());
                op_assets_add(Path::new(path_str), explicit_id)
            }
            "preview_frame" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let time_str = args.get("time").and_then(|v| v.as_str()).ok_or_else(|| {
                    AppError::new("ERR_MISSING_ARG", "Missing 'time'", "Specify timestamp.")
                })?;
                let out_path = args
                    .get("out_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("frame.png");
                op_preview_frame(Path::new(project_path), time_str, Path::new(out_path))
            }
            "preview_strip" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(8) as usize;
                let out_dir = args
                    .get("out_dir")
                    .and_then(|v| v.as_str())
                    .unwrap_or("previews");
                op_preview_strip(Path::new(project_path), count, Path::new(out_dir))
            }
            "lint_project" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let explicit_transcript = args
                    .get("transcript_path")
                    .and_then(|v| v.as_str())
                    .map(Path::new);
                op_lint(Path::new(project_path), explicit_transcript)
            }
            "render_video" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::new("ERR_MISSING_ARG", "Missing 'project_path'", "Specify path.")
                    })?;
                let out_path = args
                    .get("out_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("final.mp4");
                let preset = args.get("preset").and_then(|v| v.as_str());
                let hw_accel = args.get("hw_accel").and_then(|v| v.as_str());
                op_render(
                    Path::new(project_path),
                    Path::new(out_path),
                    preset,
                    hw_accel,
                )
            }
            "get_schema" => op_schema(),
            _ => Err(AppError::new(
                "ERR_UNKNOWN_TOOL",
                format!("Tool '{name}' is not recognized"),
                "Call 'tools/list' to see all registered Cutline tools.",
            )),
        }
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run_mcp_server<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    info!("Cutline MCP server started (stdio mode)");

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                error!("Invalid JSON-RPC line received: {e}");
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {e}")
                    }
                });
                writeln!(writer, "{}", serde_json::to_string(&err_resp).unwrap())?;
                writer.flush()?;
                line.clear();
                continue;
            }
        };

        let method = parsed.get("method").and_then(|m| m.as_str());
        let id = parsed.get("id");

        // Handle notifications (no id)
        if id.is_none() {
            if let Some(m) = method {
                debug!("Received MCP notification: {m}");
            }
            line.clear();
            continue;
        }

        let req_id = id.cloned().unwrap_or(Value::Null);

        match method {
            Some("initialize") => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "cutline",
                            "version": "0.1.0"
                        }
                    }
                });
                writeln!(writer, "{}", serde_json::to_string(&resp).unwrap())?;
                writer.flush()?;
            }
            Some("ping") => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {}
                });
                writeln!(writer, "{}", serde_json::to_string(&resp).unwrap())?;
                writer.flush()?;
            }
            Some("tools/list") => {
                let tools_data = McpServer::list_tools();
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": tools_data
                });
                writeln!(writer, "{}", serde_json::to_string(&resp).unwrap())?;
                writer.flush()?;
            }
            Some("tools/call") => {
                let params = parsed.get("params").unwrap_or(&Value::Null);
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").unwrap_or(&Value::Null);

                let resp = match McpServer::call_tool(tool_name, arguments) {
                    Ok(val) => {
                        let text_content = serde_json::to_string_pretty(&val).unwrap_or_default();
                        json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "content": [
                                    {
                                        "type": "text",
                                        "text": text_content
                                    }
                                ],
                                "isError": false
                            }
                        })
                    }
                    Err(app_err) => {
                        let text_content = format!(
                            "Error [{}]: {}\nHint: {}",
                            app_err.code, app_err.message, app_err.hint
                        );
                        json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "content": [
                                    {
                                        "type": "text",
                                        "text": text_content
                                    }
                                ],
                                "isError": true
                            }
                        })
                    }
                };

                writeln!(writer, "{}", serde_json::to_string(&resp).unwrap())?;
                writer.flush()?;
            }
            _ => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", method.unwrap_or("<none>"))
                    }
                });
                writeln!(writer, "{}", serde_json::to_string(&err_resp).unwrap())?;
                writer.flush()?;
            }
        }

        line.clear();
    }

    Ok(())
}
