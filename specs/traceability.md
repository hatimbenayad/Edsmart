# Cutline - Requirements Traceability Matrix (SDD)

This document maps all functional and non-functional requirements to their implementing Units of Work, task breakdown, verification tests, and current lifecycle status.

---

## Traceability Table

| Requirement ID | Unit of Work | Implementation Tasks | Verification Test Case | Status |
|---|---|---|---|---|
| `REQ-CORE-001` (Non-destructive) | UOW-01 | Define `Project`, `Timeline`, `Segment` structs; ensure source file read-only | `test_non_destructive_timeline_mutation` | 🟢 Implemented |
| `REQ-CORE-002` (Rational Time) | UOW-01 | Implement `FrameTime`, `Fps`, rational arithmetic | `test_rational_time_conversion_and_fps_snap` | 🟢 Implemented |
| `REQ-CORE-003` (Project Schema) | UOW-01 | JSON serialization and `schemars` schema export | `test_project_json_schema_compliance` | 🟢 Implemented |
| `REQ-MEDIA-001` (FFmpeg Detection) | UOW-01 | `cutline-media::ffmpeg_check` with structured error | `test_ffmpeg_detection` | 🟢 Implemented |
| `REQ-MEDIA-002` (Probe Video) | UOW-01 | Probe JSON parser for video/audio streams | `test_probe_and_audio_extraction` | 🟢 Implemented |
| `REQ-MEDIA-003` (Audio Extract) | UOW-01 | FFmpeg audio export (16kHz mono WAV) | `test_extract_16khz_mono_wav` | 🟢 Implemented |
| `REQ-STT-001` (Local Whisper) | UOW-02 | `cutline-stt` wrapper over `whisper-rs` | `test_model_cache_and_transcription` | 🟢 Implemented |
| `REQ-STT-002` (Model Cache) | UOW-02 | GGML model cache directory manager | `test_model_cache_and_transcription` | 🟢 Implemented |
| `REQ-STT-003` (Word Timestamps) | UOW-02 | Word token alignment and confidence extraction | `test_model_cache_and_transcription` | 🟢 Implemented |
| `REQ-ANALYSIS-001` (Silence Hysteresis)| UOW-02 | Dual-threshold RMS energy silence detector | `test_silence_detection_hysteresis` | 🟢 Implemented |
| `REQ-ANALYSIS-002` (Scene Detection) | UOW-02 | Shot change detector | `test_scene_detection_filter` | 🟢 Implemented |
| `REQ-EDIT-001` (Cuts & Trims) | UOW-03 | Timeline segment slicing and duration recalculation | `test_timeline_cut_and_trim_remapping` | 🟢 Implemented |
| `REQ-EDIT-002` (Auto-cut Silence) | UOW-03 | Bulk cut application with 50ms padding | `test_cli_timeline_cut_and_render_pipeline` | 🟢 Implemented |
| `REQ-EDIT-003` (Time Remap) | UOW-03 | Dynamic transcript clock adjustment | `test_transcript_clock_remapping_after_cuts`| 🟢 Implemented |
| `REQ-RENDER-001` (Streaming Ring) | UOW-03 | Bounded channel Decoder -> Compositor -> Encoder | `test_cli_timeline_cut_and_render_pipeline` | 🟢 Implemented |
| `REQ-RENDER-002` (Rayon Compositor) | UOW-03 | Multi-threaded frame compositor with Rayon | `test_cli_timeline_cut_and_render_pipeline` | 🟢 Implemented |
| `REQ-RENDER-003` (HW Encoder) | UOW-03 / UOW-08 | Hardware encoder detection & fallback cascade | `test_cli_timeline_cut_and_render_pipeline` | 🟢 Implemented |
| `REQ-RENDER-004` (A/V Sync) | UOW-03 | Audio segment concatenation matching video frames | `test_cli_timeline_cut_and_render_pipeline` | 🟢 Implemented |
| `REQ-CAP-001` (Word Grouping) | UOW-04 | 2-4 word clustering with punctuation boundaries | `test_smart_caption_word_grouping_punctuation_and_count` | 🟢 Implemented |
| `REQ-CAP-002` (Pure-Rust Text) | UOW-04 | `fontdue` glyph rasterization with subpixel AA | `test_font_manager_initialization` | 🟢 Implemented |
| `REQ-CAP-003` (Clean Yellow Pop) | UOW-04 | Active word highlight `#FFD60A` with pop-in scale | `test_caption_rasterization_active_yellow_and_inactive_white` | 🟢 Implemented |
| `REQ-CAP-004` (Safe Area Caption) | UOW-04 | Lower-middle caption placement in safe zone | `test_caption_rasterization_active_yellow_and_inactive_white` | 🟢 Implemented |
| `REQ-OVERLAY-001` (Templates) | UOW-05 | Procedural lower thirds, pills, counters | `test_render_lower_third_template` | 🟢 Implemented |
| `REQ-OVERLAY-002` (Animated SVG) | UOW-05 | `resvg` vector rendering with animated bird proof | `test_render_animated_bird_svg` | 🟢 Implemented |
| `REQ-OVERLAY-003` (B-Roll Cutaway) | UOW-05 | Secondary video stream frame extraction | `test_overlay_renderer_pipeline_integration` | 🟢 Implemented |
| `REQ-OVERLAY-004` (Word Semantic) | UOW-05 | Associating overlays with transcript words | `test_render_keyword_badge_template` | 🟢 Implemented |
| `REQ-PREVIEW-001` (Frame Capture) | UOW-06 | Single frame rasterization to PNG | `test_render_preview_frame_generation` | 🟢 Implemented |
| `REQ-PREVIEW-002` (Preview Strip) | UOW-06 | Multi-frame contact sheet generation | `test_render_preview_frame_generation` | 🟢 Implemented |
| `REQ-LINT-001` (Rule Engine) | UOW-06 | Rules LINT-001 through LINT-008 checks | `test_lint_rule_001_safe_area_violation` | 🟢 Implemented |
| `REQ-AGENT-001` (JSON Stdout) | UOW-07 | Strict JSON envelope formatter | `test_cli_probe_json_envelope` | 🟢 Implemented |
| `REQ-AGENT-002` (Stderr Logs) | UOW-07 | Diagnostic logging isolation to stderr | `test_cli_probe_json_envelope` | 🟢 Implemented |
| `REQ-AGENT-003` (Error Hints) | UOW-07 | Actionable structured error serialization | `test_cli_structured_error_hints` | 🟢 Implemented |
| `REQ-AGENT-004` (MCP Server) | UOW-07 | Stdio MCP JSON-RPC tool server | `test_mcp_end_to_end_agent_flow` | 🟢 Implemented |
| `REQ-NFR-001` (>= 2x Realtime) | UOW-08 | Criterion benchmark suite for rendering | `bench_full_frame_compositing` | 🟢 Implemented |
| `REQ-NFR-002` (Memory < 512MB) | UOW-08 | Frame pool memory leak & peak RSS check | `test_render_memory_footprint_peak` | 🟢 Implemented |
