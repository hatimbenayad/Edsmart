# Cutline - Autonomous AI Agent Playbook

This document serves as the comprehensive operational playbook for autonomous AI coding and workflow agents (e.g. Antigravity IDE, Claude Code, Cursor, and CLI scripts) driving **Cutline**.

---

## 1. Architecture Overview for Agents

Cutline separates the **brain** (you, the AI agent) from the **hands and eyes** (the Cutline Rust engine).

```
                      +-----------------------------+
                      |   AI AGENT (Antigravity)    |
                      |   - Understands semantics   |
                      |   - Decides cuts & rhythm   |
                      |   - Chooses motion visuals  |
                      |   - Inspects preview frames |
                      +--------------+--------------+
                                     |
               MCP JSON-RPC / CLI    | Machine-readable JSON
                                     v
                      +-----------------------------+
                      |       CUTLINE ENGINE        |
                      |   - Frame-accurate EDL      |
                      |   - Local Whisper STT       |
                      |   - Pure-Rust Compositor    |
                      |   - Hardware Encoder        |
                      +-----------------------------+
```

---

## 2. Standard Tool Interfaces

### 2.1 Model Context Protocol (MCP)

When launching with `cutline mcp`, standard JSON-RPC 2.0 messages are passed over stdio.

#### Example MCP Handshake:
Request:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": { "name": "antigravity", "version": "1.0.0" }
  }
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": { "tools": {} },
    "serverInfo": { "name": "cutline", "version": "0.1.0" }
  }
}
```

#### Example Tool Invocation (`tools/call`):
Request:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "apply_captions",
    "arguments": {
      "project_path": "project.json",
      "style": "clean-yellow"
    }
  }
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"first_caption_in_frame\": 12,\n  \"groups_count\": 18,\n  \"last_caption_out_frame\": 890,\n  \"project_file\": \"project.json\",\n  \"style\": \"clean-yellow\",\n  \"words_captioned\": 74\n}"
      }
    ],
    "isError": false
  }
}
```

---

## 3. Detailed Step-by-Step Autonomous Workflow

### Step 1: Media Intake & Inspection
Always inspect raw media to ensure video tracks and audio streams are healthy:
```bash
cutline probe input.mp4
```
Expected output:
```json
{
  "status": "success",
  "schema_version": "1.0.0",
  "command": "probe",
  "data": {
    "path": "input.mp4",
    "duration_seconds": 34.2,
    "duration_frames": 1026,
    "width": 1080,
    "height": 1920,
    "fps": { "num": 30, "den": 1 },
    "video_codec": "h264",
    "audio_codec": "aac",
    "sample_rate": 48000,
    "channels": 2
  }
}
```

### Step 2: Initialize Non-Destructive Project
```bash
cutline project init input.mp4 --preset tiktok --out project.json
```

### Step 3: Audio Extraction & Local Whisper Transcription
Extract 16kHz mono WAV:
```bash
cutline audio extract project.json --out audio.wav
```
Transcribe dialogue with word-level timestamps:
```bash
cutline transcribe project.json --model base --out transcript.json
```
Read `transcript.json`:
```json
{
  "language": "en",
  "word_count": 4,
  "words": [
    { "text": "Imagine", "start_frame": 10, "end_frame": 22, "confidence": 0.98 },
    { "text": "a", "start_frame": 23, "end_frame": 25, "confidence": 0.99 },
    { "text": "bird", "start_frame": 26, "end_frame": 38, "confidence": 0.99 },
    { "text": "flying", "start_frame": 39, "end_frame": 55, "confidence": 0.97 }
  ]
}
```

### Step 4: Silence Analysis & Rhythm Tightening
Detect dead air:
```bash
cutline analyze silence project.json --threshold-db -32.0 --min-duration-ms 300
```
Auto-cut silences while preserving 50ms speech safety margins:
```bash
cutline timeline auto-cut-silence project.json --padding-ms 50
```

### Step 5: Caption Generation
Apply the studio kinetic caption preset:
```bash
cutline captions apply project.json --style clean-yellow
```
*Tip*: Words are automatically remapped to the current cut timeline clock (`REQ-EDIT-003`).

### Step 6: Semantic Illustration
Query built-in assets:
```bash
cutline assets list
```
When dialogue contains descriptive or visual language, schedule vector illustrations or motion templates:
```bash
cutline overlay add project.json \
  --asset svg:bird-flying \
  --at 00:00:00.866 \
  --duration 00:00:01.833 \
  --anchor top_center \
  --reason "Speaker said: 'bird flying'" \
  --word-range 2,3
```

### Step 7: Eyes Inspection (`preview-frame` & `preview-strip`)
Examine the composite before rendering:
```bash
cutline preview-frame project.json --time 00:00:01.200 --out preview_frame.png
cutline preview-strip project.json --count 4 --out-dir previews/
```

### Step 8: Quality Gate (`cutline lint`)
Run the quality gate:
```bash
cutline lint project.json
```
If violations are detected, read the remediation hint:
```json
{
  "status": "success",
  "data": {
    "passed": false,
    "violations_count": 1,
    "warnings_count": 0,
    "details": [
      {
        "rule_id": "LINT-002",
        "severity": "Error",
        "message": "Overlay 'ov_bird' collides vertically with active captions",
        "frame": 35,
        "hint": "Relocate overlay to 'top_center' (y <= 0.35) or adjust position."
      }
    ]
  }
}
```
Fix the issue by modifying overlay position, then re-lint.

### Step 9: Final Hardware-Accelerated Export
```bash
cutline render project.json --out final.mp4 --preset tiktok
```

---

## 4. Error Codes & Recovery Protocol

| Error Code | Meaning | Remediation Strategy |
|---|---|---|
| `ERR_NO_VIDEO_STREAM` | Media contains no video stream | Check media file path; re-probe container. |
| `ERR_INVALID_FRAME_RANGE` | Start time >= End time | Verify that `start_time` strictly precedes `end_time`. |
| `ERR_MISSING_TRANSCRIPT` | Captions requested without word timestamps | Run `cutline transcribe <project>` first. |
| `ERR_NOT_FOUND` | Overlay or asset ID does not exist | Run `cutline overlay list` or `cutline assets list`. |
| `LINT_SAFE_AREA_VIOLATION` | Graphic extends into UI dead-zone | Adjust normalized position (`x`, `y`) into safe bounds. |
| `LINT_CAPTION_OVERLAY_COLLISION` | Overlay overlaps caption text box | Move overlay to `top_center` or adjust timeline in/out frames. |
| `LINT_SPLIT_WORD_ON_CUT` | Timeline cut slices through spoken word | Adjust cut start/end points to align with word boundaries. |
