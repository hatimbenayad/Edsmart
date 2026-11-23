# Cutline - CLI & MCP Interface Specification

This document defines the agent interface: the command-line interface (CLI) with machine-readable JSON input/output, error schemas, and the Model Context Protocol (MCP) server integration.

---

## 1. General Interface Protocol

### 1.1 Stdout / Stderr Separation
- **stdout**: Strictly reserved for machine-readable JSON responses. Every command prints exactly one valid JSON object upon completion.
- **stderr**: Diagnostic human logs, download progress, and tracing statements managed via `tracing-subscriber`. Stderr output must never pollute stdout.

### 1.2 Output Envelope Format
Every successful execution emits an envelope with metadata:

```json
{
  "status": "success",
  "schema_version": "1.0.0",
  "command": "project.init",
  "data": { ... }
}
```

### 1.3 Error Envelope Format
When an execution fails, it exits with non-zero exit code and emits:

```json
{
  "status": "error",
  "schema_version": "1.0.0",
  "command": "timeline.cut",
  "error": {
    "code": "ERR_INVALID_FRAME_RANGE",
    "message": "Start frame 450 is greater than end frame 300",
    "hint": "Verify that timeline cut start time precedes end time."
  }
}
```

---

## 2. CLI Command Specifications

### 2.1 Media & Project Initialization

#### `cutline probe <video>`
- **Description**: Probes container, video streams, audio streams, and metadata.
- **Arguments**:
  - `<video>`: Path to video file.
- **Output Data**:
  ```json
  {
    "path": "video.mp4",
    "duration_seconds": 60.0,
    "duration_frames": 1800,
    "width": 1080,
    "height": 1920,
    "fps": { "num": 30, "den": 1 },
    "video_codec": "h264",
    "audio_codec": "aac",
    "sample_rate": 48000,
    "channels": 2
  }
  ```

#### `cutline project init <video> [--b-roll <path>...] [--preset <tiktok|youtube>] [--out <project.json>]`
- **Description**: Initializes an empty non-destructive project with default tracks and safe area.
- **Arguments**:
  - `<video>`: Primary speaking-head video.
  - `--b-roll`: Optional secondary footage paths.
  - `--preset`: Platform preset (`tiktok` [9:16 1080x1920 30fps], `youtube` [16:9 1920x1080 30fps]). Default: `tiktok`.
  - `--out`: Target project file path (default `project.json`).
- **Output Data**:
  ```json
  {
    "project_id": "proj_abc123",
    "project_path": "project.json",
    "dimensions": { "width": 1080, "height": 1920 },
    "fps": { "num": 30, "den": 1 }
  }
  ```

#### `cutline audio extract <video|project> [--out <audio.wav>]`
- **Description**: Extracts dialogue audio to a 16kHz mono WAV file optimized for Whisper.
- **Output Data**:
  ```json
  {
    "audio_path": "audio_16k.wav",
    "sample_rate": 16000,
    "channels": 1,
    "duration_seconds": 60.0
  }
  ```

---

### 2.2 Speech-to-Text & Analysis

#### `cutline transcribe <project> [--model <tiny|base|small>] [--out <transcript.json>]`
- **Description**: Runs local offline Whisper speech recognition and outputs word-level timestamps.
- **Output Data**:
  ```json
  {
    "transcript_path": "transcript.json",
    "language": "en",
    "word_count": 142,
    "words": [
      { "text": "Hello", "start_frame": 15, "end_frame": 28, "start_ms": 500, "end_ms": 933, "confidence": 0.98 }
    ]
  }
  ```

#### `cutline analyze silence <project> [--threshold-db <db>] [--min-duration-ms <ms>]`
- **Description**: Analyzes audio energy using dual-threshold hysteresis to locate pauses and dead air.
- **Output Data**:
  ```json
  {
    "silences_found": 5,
    "total_silence_seconds": 8.4,
    "silences": [
      { "start_frame": 120, "end_frame": 180, "start_ms": 4000, "end_ms": 6000, "duration_ms": 2000 }
    ]
  }
  ```

#### `cutline analyze scenes <project> [--threshold <val>]`
- **Description**: Detects visual shot transitions in source video.
- **Output Data**:
  ```json
  {
    "scene_cuts_found": 3,
    "cut_frames": [450, 1120, 1540]
  }
  ```

---

### 2.3 Timeline Operations

#### `cutline timeline cut <project> --start <time> --end <time>`
- **Description**: Slices and excises a time interval from the timeline. Times accepted as `HH:MM:SS.mmm`, `seconds`, or `frame:N`.
- **Output Data**:
  ```json
  {
    "cut_interval": { "start_frame": 120, "end_frame": 180 },
    "new_timeline_frames": 1740,
    "new_duration_seconds": 58.0
  }
  ```

#### `cutline timeline auto-cut-silence <project> [--padding-ms <50>]`
- **Description**: Automatically cuts all detected silence intervals, preserving specified speech padding.
- **Output Data**:
  ```json
  {
    "cuts_applied": 5,
    "frames_removed": 252,
    "seconds_saved": 8.4,
    "new_timeline_frames": 1548
  }
  ```

#### `cutline timeline trim <project> --in <time> --out <time>`
- **Description**: Trims global project start and end points.

---

### 2.4 Captions & Overlays

#### `cutline captions apply <project> [--style <name|file.json>] [--words-per-group <n>]`
- **Description**: Generates kinetic word-by-word captions tied to transcript timings and safe area.
- **Output Data**:
  ```json
  {
    "captions_generated": 36,
    "style": "clean-yellow",
    "safe_area": { "top": 200, "bottom": 380, "left": 90, "right": 130 }
  }
  ```

#### `cutline overlay add <project> --asset <id> --at <time> --duration <time> --anchor <pos> [--animation <in/out>] [--reason <json>]`
- **Description**: Adds an overlay element (template, animated SVG, or B-roll cutaway) to the timeline.
- **Output Data**:
  ```json
  {
    "overlay_id": "ovl_001",
    "asset_id": "bird_flying",
    "start_frame": 120,
    "end_frame": 210,
    "anchor": "top_center"
  }
  ```

#### `cutline assets list`
- **Description**: Lists built-in procedural templates and registered vector/SVG assets.
- **Output Data**:
  ```json
  {
    "assets": [
      { "id": "lower_third", "type": "template", "description": "Minimal animated speaker lower third" },
      { "id": "bird_flying", "type": "svg_animation", "description": "Clean vector flying bird" },
      { "id": "progress_pill", "type": "template", "description": "Top screen time indicator" }
    ]
  }
  ```

#### `cutline assets add <project> --name <id> --file <path> --type <svg|template|image>`
- **Description**: Registers a user or agent-authored SVG or visual asset into the project asset library.

---

### 2.5 Inspection, Quality Gate & Rendering

#### `cutline preview-frame <project> --time <time> --out <frame.png>`
- **Description**: Renders a single frame with composited video, captions, and overlays to PNG.
- **Output Data**:
  ```json
  {
    "frame_index": 372,
    "timestamp_seconds": 12.4,
    "output_image": "frame.png",
    "dimensions": { "width": 1080, "height": 1920 }
  }
  ```

#### `cutline preview-strip <project> --count <n> --out-dir <dir>`
- **Description**: Renders an evenly spaced series of thumbnail frames across the timeline.
- **Output Data**:
  ```json
  {
    "count": 8,
    "frames": [
      { "index": 0, "frame": 0, "file": "previews/frame_000.png" },
      { "index": 1, "frame": 220, "file": "previews/frame_001.png" }
    ]
  }
  ```

#### `cutline lint <project>`
- **Description**: Validates edit rules, platform safe area bounds, caption collision, and color constraints.
- **Output Data**:
  ```json
  {
    "passed": true,
    "violations_count": 0,
    "warnings_count": 0,
    "details": []
  }
  ```

#### `cutline render <project> --out <out.mp4> [--preset <tiktok|youtube|fast-draft>] [--hw-accel <auto|nvenc|amf|qsv|vt|vaapi|none>]`
- **Description**: Renders final video with multi-threaded compositing and hardware encoding.
- **Output Data**:
  ```json
  {
    "output_file": "final.mp4",
    "duration_seconds": 51.6,
    "frames_rendered": 1548,
    "render_time_seconds": 18.2,
    "speed_x_realtime": 2.83,
    "encoder_used": "h264_nvenc"
  }
  ```

#### `cutline schema`
- **Description**: Prints the complete JSON Schema for `project.json`.

---

## 3. Model Context Protocol (MCP) Server Specification

When invoked with `cutline mcp`, the process initializes a standard JSON-RPC 2.0 stdio transport adhering to the Anthropic/Model Context Protocol specification.

### 3.1 Registered MCP Tools

| Tool Name | Parameters | Description |
|---|---|---|
| `probe_media` | `path: string` | Inspects video stream dimensions, fps, audio tracks, and codecs. |
| `init_project` | `video_path: string, b_roll?: string[], preset?: string` | Creates a new `project.json` workspace. |
| `extract_audio` | `project_path: string` | Extracts 16kHz mono audio for transcription. |
| `transcribe_audio` | `project_path: string, model?: string` | Transcribes dialogue with word-level timestamps. |
| `detect_silence` | `project_path: string, threshold_db?: number, min_ms?: number` | Detects pauses in dialogue. |
| `cut_timeline` | `project_path: string, start_time: string, end_time: string` | Excises a time range from the timeline. |
| `auto_cut_silence` | `project_path: string, padding_ms?: number` | Excises all detected silences automatically. |
| `apply_captions` | `project_path: string, style?: string` | Formats and positions kinetic captions. |
| `add_overlay` | `project_path: string, asset_id: string, start_time: string, duration: string, anchor: string, props?: object` | Places a visual overlay onto the timeline. |
| `list_assets` | `project_path: string` | Returns all available templates and vector graphics. |
| `preview_frame` | `project_path: string, time: string, out_path: string` | Renders a single frame for visual inspection. |
| `preview_strip` | `project_path: string, count: number, out_dir: string` | Generates a multi-frame visual contact sheet. |
| `lint_project` | `project_path: string` | Validates safe area and layout design rules. |
| `render_video` | `project_path: string, out_path: string, preset?: string` | Executes high-speed final export. |
