# Cutline - Functional & Non-Functional Requirements Specification (SDD)

This document serves as the formal specification of requirements for **Cutline**. Every requirement carries a unique, immutable identifier and testable Given/When/Then acceptance criteria.

---

## 1. User Stories

### US-01: Talking-Head Content Creator
> **As a** talking-head video creator producing content for TikTok, Reels, and YouTube Shorts,  
> **I want** an AI-drivable engine that can take my unedited footage, strip dead air, highlight my spoken words with kinetic captions, and insert tasteful visuals when I describe things,  
> **So that** I get an engaging, professional, polished video without spending hours manually editing on a timeline.

### US-02: AI Agent Editor Brain (Antigravity)
> **As an** autonomous AI agent operating inside the Antigravity IDE,  
> **I want** deterministic, machine-readable CLI commands and an MCP stdio server that output structured JSON, export schemas, generate visual preview frames, and lint my edits against design rules,  
> **So that** I can inspect footage, execute reversible cuts, position overlays, and verify final quality completely autonomously.

---

## 2. Functional Requirements

### 2.1 Core Timeline & Project Model (`REQ-CORE`)

#### `REQ-CORE-001`: Non-Destructive Project Model
- **Description**: The system must operate on an immutable source representation. All editing operations produce or update a virtual timeline in `project.json` without modifying the original media files.
- **Acceptance Criteria**:
  - *Given* an original raw video file `raw.mp4`,
  - *When* multiple cuts, trims, and overlay additions are applied,
  - *Then* `raw.mp4` hash and timestamp remain strictly identical, and all edits are recorded as time intervals in `project.json`.

#### `REQ-CORE-002`: Frame-Accurate Rational Time Math
- **Description**: All timeline operations, cut boundaries, caption highlights, and overlay timings must be calculated using integer frame ticks and rational frame rates (`num / den`), preventing floating-point drift.
- **Acceptance Criteria**:
  - *Given* a 30000/1001 (~29.97) fps or 30 fps video,
  - *When* an edit point is specified at second `12.4`,
  - *Then* it maps deterministically to an exact integer frame index without accumulating rounding errors over long sequences.

#### `REQ-CORE-003`: Project Initialization & JSON Schema Validation
- **Description**: The engine must initialize a valid `project.json` from a primary video and optional B-roll sources, validated against a published JSON Schema.
- **Acceptance Criteria**:
  - *Given* a valid media file `input.mp4`,
  - *When* `cutline project init input.mp4` is executed,
  - *Then* a compliant `project.json` is created with valid dimensions, fps, audio channels, and empty tracks, matching `cutline schema`.

---

### 2.2 Media Probing & Audio Pipeline (`REQ-MEDIA`)

#### `REQ-MEDIA-001`: FFmpeg Startup Verification & Error Handling
- **Description**: The system must check for an accessible `ffmpeg` executable on PATH upon invocation and return a structured machine-readable error if missing.
- **Acceptance Criteria**:
  - *Given* a system environment without `ffmpeg` on PATH,
  - *When* any Cutline command is invoked,
  - *Then* the process terminates with exit code 1 and outputs structured error code `ERR_FFMPEG_MISSING` with remediation hints.

#### `REQ-MEDIA-002`: Comprehensive Video Probing
- **Description**: Extract duration, exact frame count, rational frame rate, resolution, aspect ratio, video codec, audio sample rate, and audio channel count as structured JSON.
- **Acceptance Criteria**:
  - *Given* any valid MP4/MOV container,
  - *When* `cutline probe <file>` is called,
  - *Then* stdout returns JSON with fields `duration_ticks`, `fps_num`, `fps_den`, `width`, `height`, `vcodec`, `acodec`, and `channels`.

#### `REQ-MEDIA-003`: 16kHz Mono Audio Extraction
- **Description**: Extract the primary audio stream to a 16kHz 16-bit PCM mono WAV file optimized for Whisper transcription.
- **Acceptance Criteria**:
  - *Given* an input video with multi-channel or compressed audio,
  - *When* `cutline audio extract <video>` is run,
  - *Then* the output WAV is verified to be single-channel, 16000 Hz sample rate, and 16-bit signed PCM.

---

### 2.3 Speech-to-Text & Word Timestamps (`REQ-STT`)

#### `REQ-STT-001`: Local Offline Whisper Transcription
- **Description**: Perform speech recognition exclusively via a local `whisper-rs` model without network calls or external APIs during inference.
- **Acceptance Criteria**:
  - *Given* an extracted 16kHz audio file,
  - *When* `cutline transcribe <project>` is executed,
  - *Then* transcription succeeds completely offline, writing `transcript.json`.

#### `REQ-STT-002`: Automated Model Caching
- **Description**: Automatically download the selected GGML model (default `ggml-base.bin`) into a local user cache directory on first run, verifying file integrity.
- **Acceptance Criteria**:
  - *Given* an empty model cache,
  - *When* transcription is requested,
  - *Then* the model is downloaded once, cached in `%LOCALAPPDATA%/cutline/models` (or `~/.cache/cutline/models`), and reused on subsequent runs.

#### `REQ-STT-003`: Word-Level Timestamp Extraction
- **Description**: Output must include individual word tokens with start time, end time, and probability confidence.
- **Acceptance Criteria**:
  - *Given* spoken phrase "Hello my name is Steven",
  - *When* transcription completes,
  - *Then* `transcript.json` contains separate objects for "Hello", "my", "name", "is", "Steven", each with `start_ms`, `end_ms`, and `confidence`.

---

### 2.4 Audio Analysis & Silence Detection (`REQ-ANALYSIS`)

#### `REQ-ANALYSIS-001`: RMS Energy Silence Detection with Hysteresis
- **Description**: Analyze PCM audio frames using dual-threshold hysteresis (activation threshold e.g. -32dB, deactivation threshold e.g. -38dB) to detect true pauses and prevent jittery cuts.
- **Acceptance Criteria**:
  - *Given* audio with natural pauses between sentences,
  - *When* `cutline analyze silence <project>` is executed,
  - *Then* a list of silence intervals is returned with `start_frame`, `end_frame`, and `duration_ms` exceeding the minimum silence threshold (default 300ms).

#### `REQ-ANALYSIS-002`: Scene Change Detection
- **Description**: Optionally detect visual scene cuts in source footage to assist in B-roll placement and avoid cuts immediately before natural scene transitions.
- **Acceptance Criteria**:
  - *Given* footage with multiple shot changes,
  - *When* `cutline analyze scenes <project>` is called,
  - *Then* frame indices of shot transitions exceeding the similarity threshold are reported.

---

### 2.5 Timeline Editing (`REQ-EDIT`)

#### `REQ-EDIT-001`: Frame-Accurate Cuts and Trims
- **Description**: Support `cut`, `trim`, `keep`, and `reorder` operations on timeline segments, recomputing the global timeline duration and segment mappings.
- **Acceptance Criteria**:
  - *Given* a 60-second video clip,
  - *When* a cut from 00:10.000 to 00:15.000 is applied,
  - *Then* the timeline duration becomes exactly 55 seconds, segment 1 spans 0-10s, segment 2 spans 15-60s, and both map to the original source frames.

#### `REQ-EDIT-002`: Automated Silence Removal
- **Description**: Automatically remove detected silence intervals while applying configurable padding (default 50ms) to preserve word attacks and natural cadence.
- **Acceptance Criteria**:
  - *Given* detected silences and a 50ms safety padding,
  - *When* `cutline timeline auto-cut-silence <project>` is run,
  - *Then* silences are excised from the timeline without truncating the initial consonants or final vowels of surrounding speech.

#### `REQ-EDIT-003`: Dynamic Transcript & Timing Remapping
- **Description**: When cuts are applied to the timeline, word timestamps and overlay timings must be dynamically remapped to match the new continuous timeline clock.
- **Acceptance Criteria**:
  - *Given* a cut from 5.0s to 10.0s,
  - *When* a word was originally spoken at 12.0s,
  - *Then* its active timeline display time is automatically remapped to 7.0s.

---

### 2.6 Word-by-Word Kinetic Captions (`REQ-CAP`)

#### `REQ-CAP-001`: Smart Word Grouping
- **Description**: Group words into readable visual chunks (2 to 4 words per group, maximum 2 lines) breaking on natural punctuation and pauses, with a minimum display duration of 400ms.
- **Acceptance Criteria**:
  - *Given* a continuous spoken sentence of 12 words,
  - *When* captions are generated,
  - *Then* words are clustered into segments of 2-4 words that never split a single word across screen transitions.

#### `REQ-CAP-002`: Native Pure-Rust Text Rasterization
- **Description**: Render caption typography natively using pure Rust (`fontdue` / `tiny-skia`) without requiring external system fonts or libass runtime dependencies.
- **Acceptance Criteria**:
  - *Given* bundled font Inter Bold,
  - *When* caption frames are rendered,
  - *Then* glyphs are rasterized with sub-pixel antialiasing and composited directly into the RGBA frame buffer.

#### `REQ-CAP-003`: Active Word Kinetic Highlighting (`clean-yellow`)
- **Description**: Render visible words in white while the currently spoken word is highlighted in warm yellow (`#FFD60A`) with a subtle scale pop animation.
- **Acceptance Criteria**:
  - *Given* visible group "hello my name is Steven",
  - *When* the current frame timestamp corresponds to "name",
  - *Then* "name" is rendered in `#FFD60A` with a 1.1x scale pop, while "hello", "my", "is", and "Steven" remain `#FFFFFF`.

#### `REQ-CAP-004`: Safe Area Compliance
- **Description**: Default caption position must sit within the vertical safe area (60% to 75% height for 9:16) and avoid platform UI overlays (TikTok bottom sound banner and right-side interactive icons).
- **Acceptance Criteria**:
  - *Given* a 1080x1920 canvas,
  - *When* caption bounds are computed,
  - *Then* the bounding box strictly resides inside `[x: 100..980, y: 1150..1440]`.

---

### 2.7 Overlays & Visual Asset System (`REQ-OVERLAY`)

#### `REQ-OVERLAY-001`: Built-In Procedural Motion Templates
- **Description**: Pure-code vector motion templates: lower-thirds, callout cards, animated progress pills, dynamic number counters, and punch-in zooms.
- **Acceptance Criteria**:
  - *Given* an overlay configured with template `lower-third`,
  - *When* rendered between frame 100 and 160,
  - *Then* a clean background pill, title text, and accent line animate in with ease-out-cubic and animate out with ease-in-cubic.

#### `REQ-OVERLAY-002`: Vector SVG Keyframe Animations
- **Description**: Support SVG assets rendered via `resvg`, with animated transform, scale, rotation, and opacity curves.
- **Acceptance Criteria**:
  - *Given* an animated SVG asset (e.g. `assets/bird.svg`),
  - *When* placed on the timeline,
  - *Then* it renders cleanly at any resolution without pixelation, following the specified flight path.

#### `REQ-OVERLAY-003`: Secondary B-Roll Video Inserts
- **Description**: Support cutaways to secondary video files on separate timeline video tracks.
- **Acceptance Criteria**:
  - *Given* a primary video and B-roll file,
  - *When* a B-roll segment is placed from timeline second 10 to 14,
  - *Then* the compositor renders B-roll frames on top of the primary track while maintaining primary dialogue audio.

#### `REQ-OVERLAY-004`: Word-Tied Semantic Association
- **Description**: Every overlay must record the semantic transcript word range that triggered it (`reason: { word_range, text }`).
- **Acceptance Criteria**:
  - *Given* an agent adding an overlay for "bird is flying",
  - *When* inspecting `project.json`,
  - *Then* the overlay object contains the exact word tokens and rationale.

---

### 2.8 Quality Gate & Inspection (`REQ-PREVIEW`, `REQ-LINT`)

#### `REQ-PREVIEW-001`: Precise Frame Capture
- **Description**: Export an individual composited frame at any specified timestamp or frame number to a PNG image for visual inspection.
- **Acceptance Criteria**:
  - *Given* a project with active captions and overlays,
  - *When* `cutline preview-frame --time 00:00:12.400 --out frame.png` is run,
  - *Then* `frame.png` is generated showing the exact composited output at that frame.

#### `REQ-PREVIEW-002`: Contact Sheet Preview Strip
- **Description**: Generate an evenly spaced grid of preview thumbnails across the timeline to allow rapid visual inspection of the entire edit.
- **Acceptance Criteria**:
  - *Given* a project file,
  - *When* `cutline preview-strip --count 8 --out-dir ./previews` is executed,
  - *Then* 8 PNG images representing key timeline intervals are saved to the target directory.

#### `REQ-LINT-001`: Automated Design & Safe Area Linting
- **Description**: Verify adherence to design system rules and output structured issues with severity and remediation suggestions.
- **Acceptance Criteria**:
  - *Given* a project where an overlay overlaps the active caption or exceeds the safe area,
  - *When* `cutline lint <project>` is executed,
  - *Then* the linter exits with code 1 and outputs structured violations `LINT_SAFE_AREA_VIOLATION` or `LINT_CAPTION_OVERLAY_COLLISION`.

---

### 2.9 Rendering & Hardware Acceleration (`REQ-RENDER`)

#### `REQ-RENDER-001`: Streaming Compositing Pipeline
- **Description**: Video frames must stream continuously through bounded memory channels (Decoder -> Compositor -> Encoder) to ensure flat memory usage regardless of video duration.
- **Acceptance Criteria**:
  - *Given* a 30-minute 1080p source video,
  - *When* rendering the entire timeline,
  - *Then* process RAM usage remains stable (< 350 MB) without unbound frame buffering.

#### `REQ-RENDER-002`: Parallel Multi-Threaded Compositing
- **Description**: Utilize `rayon` to parallelize text rasterization, overlay blending, and pixel operations across available CPU cores.
- **Acceptance Criteria**:
  - *Given* a multi-core CPU,
  - *When* compositing multiple graphic layers,
  - *Then* CPU utilization scales across all cores and achieves >= 2x realtime on draft presets.

#### `REQ-RENDER-003`: Hardware Accelerated Encoding with Software Fallback
- **Description**: Automatically detect and utilize platform hardware encoders (`h264_nvenc` for NVIDIA, `h264_amf` for AMD, `h264_qsv` for Intel, `h264_videotoolbox` for macOS, `h264_vaapi` for Linux), gracefully falling back to `libx264`.
- **Acceptance Criteria**:
  - *Given* a machine with NVIDIA GPU and NVENC enabled FFmpeg,
  - *When* rendering with default auto settings,
  - *Then* `h264_nvenc` is selected and verified in the render summary.

#### `REQ-RENDER-004`: Accurate Audio-Video Multiplexing
- **Description**: Re-mux cut audio segments and composited video frames into the final MP4 container with zero A/V desynchronization across cut boundaries.
- **Acceptance Criteria**:
  - *Given* 20 cut points across a 2-minute dialogue clip,
  - *When* final render is inspected,
  - *Then* audio and video remain strictly synchronized within +/- 1 frame tolerance throughout the entire duration.

---

### 2.10 AI Agent Interfaces (`REQ-AGENT`)

#### `REQ-AGENT-001`: Pure JSON Output on Stdout
- **Description**: All CLI commands must output strictly valid JSON to stdout when executed.
- **Acceptance Criteria**:
  - *Given* any Cutline command,
  - *When* stdout is piped to a JSON parser (`jq` or `serde_json`),
  - *Then* it parses without syntax errors and contains `"status": "success"` or `"status": "error"`.

#### `REQ-AGENT-002`: Isolated Diagnostic Logging on Stderr
- **Description**: All human-readable diagnostic messages, download progress, and debug traces must be written exclusively to stderr.
- **Acceptance Criteria**:
  - *Given* stdout redirected to a file,
  - *When* `cutline transcribe` or `cutline render` is executed,
  - *Then* stdout contains only the final JSON envelope while stderr displays tracing logs.

#### `REQ-AGENT-003`: Structured Actionable Errors
- **Description**: Every error must adhere to `{ code, message, hint }` where `hint` gives specific, actionable instructions for how the AI agent can recover.
- **Acceptance Criteria**:
  - *Given* an invalid timeline cut range where `start > end`,
  - *When* the command executes,
  - *Then* stderr/stdout return error code `ERR_INVALID_TIME_RANGE` with hint "Verify that start_time precedes end_time."

#### `REQ-AGENT-004`: Model Context Protocol (MCP) Server
- **Description**: Provide `cutline mcp` implementing the MCP standard over stdio, exposing all engine capabilities as agent tools.
- **Acceptance Criteria**:
  - *Given* Antigravity IDE connecting via stdio MCP,
  - *When* `tools/list` is queried,
  - *Then* all Cutline editing commands are registered as typed tools with parameter schemas.

---

## 3. Non-Functional Requirements (`REQ-NFR`)

#### `REQ-NFR-001`: Performance & Throughput
- 1080x1920 30fps rendering with captions and 2 active overlays must achieve **>= 2.0x realtime** on modern consumer laptop hardware with hardware encoding.
- Draft preset (`fast-draft` 540x960) must achieve **>= 4.0x realtime**.

#### `REQ-NFR-002`: Memory Footprint
- Peak resident set size (RSS) during rendering must not exceed **512 MB**, achieved via a bounded ring buffer of 16-32 frames.

#### `REQ-NFR-003`: Cross-Platform Compatibility
- The engine must compile and run cleanly on Windows (MSVC), macOS (Apple Silicon & Intel), and Linux (x86_64).

#### `REQ-NFR-004`: Offline Operation & Zero Telemetry
- No telemetry, analytics, cloud LLM, or external web service calls during editing, transcription, or rendering. Network access is restricted to initial model caching if absent.
