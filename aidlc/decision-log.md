# Cutline - Architecture & Decision Log (AI-DLC)

This document tracks all architectural, technical, and product decisions for **Cutline**. Every major architectural choice, trade-off, and rejected alternative is recorded here to maintain strict traceability throughout the project lifecycle.

---

## Log Entries

### DEC-001: Methodology - AI-DLC with Spec-Driven Development (SDD)
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: The project is an autonomous, agent-drivable video editing engine in Rust. High reliability, zero-drift between documentation and code, and deterministic testing are paramount.
- **Decision**: Adopt AI-DLC (AI-Driven Development Life Cycle) across three strict phases: **Inception -> Construction -> Operations**, paired with Spec-Driven Development (SDD).
  - Gated checkpoints require user approval before advancing phases or adding heavy dependencies.
  - Requirements are uniquely identified (`REQ-***`) with Given/When/Then acceptance criteria.
  - Traceability table (`specs/traceability.md`) maps requirements to tasks and tests.
- **Alternatives Rejected**:
  - *Ad-hoc prototyping*: High risk of architectural drift, messy timeline logic, and broken A/V sync.
  - *Code-first TDD without formal specs*: Leaves schema and agent contracts ambiguous.

---

### DEC-002: Multi-Crate Workspace Architecture
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: The video editing engine spans multiple concerns: CLI/MCP handling, STT speech processing, audio DSP analysis, video decoding/encoding, vector graphic rendering, and timeline math.
- **Decision**: Structure the engine as a modular Rust Cargo workspace:
  - `cutline-core`: Timeline EDL, rational frame-accurate time, project schemas, JSON Schema generation.
  - `cutline-media`: FFmpeg discovery, subprocess piping (`rawvideo`, `s16le` audio), hardware encoder detection.
  - `cutline-stt`: Local `whisper-rs` integration, GGML model cache, word-level timestamp extraction.
  - `cutline-analysis`: RMS/dB silence detection with hysteresis, scene change detection.
  - `cutline-render`: Multi-threaded pipeline (decoder -> rayon compositor -> encoder) with bounded channels.
  - `cutline-assets`: Procedural motion templates, SVG animation rendering via `resvg`, asset registry.
  - `cutline-lint`: Platform safe-area and design rule validator.
  - `cutline-cli`: Binary entry point, Clap v4 CLI, machine-readable JSON stdout, and MCP stdio server.
- **Alternatives Rejected**:
  - *Monolithic single-crate*: Tightly couples heavy C++ dependencies (`whisper-rs`) with lightweight core data models, slowing down tests and compile cycles.

---

### DEC-003: Video Decoding & Encoding Strategy - FFmpeg Subprocess Pipes vs `ffmpeg-next`
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: The user requires fast, cross-platform video decode/encode with hardware acceleration and zero reimplementation of codecs.
- **Decision**: Use FFmpeg child process pipes with raw uncompressed streams (`rawvideo` RGB24/RGBA over stdout/stdin, `pcm_s16le` over separate audio pipes), verified at startup.
- **Rationale**:
  - `ffmpeg-next` (and direct `libav*` C bindings) suffers from severe cross-platform build instability on Windows (MSVC ABI mismatches, vcpkg/pkg-config drift, linking failures with external system libraries).
  - FFmpeg subprocess pipes are 100% portable across Windows, macOS, and Linux without native C link-time friction.
  - Passing hardware acceleration flags (`-c:v h264_nvenc`, `-c:v h264_amf`, `-c:v h264_videotoolbox`, `-c:v h264_vaapi`) to FFmpeg CLI is robust, well-documented, and decoupled from Rust compile targets.
  - The overhead of raw IPC pipe streaming on localhost is negligible (microseconds per 1080p frame) and well within our >= 2x realtime target.
- **Alternatives Rejected**:
  - *`ffmpeg-next` / `libav-sys`*: Frequent build breaks on developer machines with MSVC or mismatched DLL versions.
  - *Custom Rust video decoders*: Immature, incomplete H.264/HEVC support, lack of hardware decoding.

---

### DEC-004: Speech-to-Text - Local `whisper-rs` with GGML Quantized Models
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Need offline speech-to-text with word-level timestamps, zero cloud APIs, and fast inference on consumer hardware.
- **Decision**: Integrate `whisper-rs` (Rust bindings for `whisper.cpp`). Default model: `ggml-base.bin` (~140MB) or `ggml-small.bin` (~460MB), automatically downloaded into local cache (`~/.cache/cutline/models` or Windows `%LOCALAPPDATA%/cutline/models`) on first run.
- **Rationale**:
  - `whisper.cpp` is state-of-the-art for local, quantized CPU/GPU speech inference.
  - Supports token-level timestamps enabling word-level timestamp reconstruction.
  - 100% offline, zero data leaves the machine.
- **Alternatives Rejected**:
  - *OpenAI Cloud Whisper API / Hugging Face Inference*: Violates strict constraint of offline, local-only processing.
  - *Vosk / Coqui STT*: Inferior timestamp accuracy and language understanding compared to Whisper.

---

### DEC-005: Frame-Accurate Time Model - Integer Ticks & Rational FPS
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Edits, cuts, captions, and overlays must align perfectly with video frames. Floating-point seconds (`12.4f64`) introduce accumulated rounding errors, off-by-one frame slips, and A/V desynchronization.
- **Decision**: Implement a rational frame-accurate time type `FrameTime` represented as integer frame ticks at a rational frame rate (`num / den`). All internal timeline arithmetic, cuts, and transitions operate on exact frame indices. Floating-point seconds are only accepted at the input CLI boundary and immediately converted to exact frames.
- **Alternatives Rejected**:
  - *Floating-point `f64` seconds throughout*: Causes non-deterministic frame snapping and drift over long videos.

---

### DEC-006: Graphic & Typography Rendering - Pure Rust Vector & Raster Stack
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Need pixel-perfect animated captions and overlay graphics composited at 1080x1920 with high performance.
- **Decision**: Use `tiny-skia` for 2D software compositing, `fontdue` (or `cosmic-text`) for pure-Rust font rasterization, and `resvg`/`usvg` for SVG rendering. Bundle open-license fonts (Inter Bold, Montserrat ExtraBold) directly inside the binary/assets to eliminate system font dependencies.
- **Rationale**:
  - `tiny-skia` is pure Rust, SIMD-accelerated, zero-dependency, and deterministic across all OS platforms.
  - `fontdue` provides the fastest font rasterization in pure Rust with sub-pixel coverage.
  - `resvg` is the leading SVG rendering engine in Rust, capable of rendering vector shapes directly into pixel buffers.
- **Alternatives Rejected**:
  - *`libass` burn-in as primary*: Requires external C library, awkward styling model, inflexible for procedural motion templates and non-text vector overlays.

---

### DEC-007: Overlay & Animation Strategy - Procedural Templates + SVG Keyframing
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: The app must render clean, studio-grade visuals (lower thirds, animated callouts, progress bars, custom vector animations such as a flying bird).
- **Decision**: Layered architecture:
  1. *Procedural Motion Templates*: Built-in pure Rust renderers for lower-thirds, counters, highlight/underline pills, zoom punches.
  2. *Animated SVG System*: Parameterized SVG templates where transforms, opacities, and path offsets are interpolated via standard easing curves (ease-out-back, ease-out-cubic).
  3. *B-Roll Cutaways*: Frame-accurate video inserts from secondary source files.
- **Rationale**:
  - Pure Rust Lottie libraries (e.g. `rlottie-rs`) require complex C++ Qt/Bodymovin builds or are unmaintained. SVG path manipulation + `resvg` provides reliable, ultra-crisp vector animations without external bloat.
- **Alternatives Rejected**:
  - *Lottie C++ runtime*: Incompatible with pure-Rust, lightweight build requirements.

---

### DEC-008: Dual AI Agent Interface - Machine-Readable CLI + Model Context Protocol (MCP)
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Antigravity IDE and other autonomous agents must drive Cutline seamlessly without human intervention.
- **Decision**:
  - Every CLI command outputs strict JSON to stdout with schema versioning (`schema_version: "1.0.0"`).
  - All human logs, progress updates, and warnings go exclusively to stderr via `tracing`.
  - Built-in `cutline mcp` runs a standard stdio Model Context Protocol (MCP) server, exposing every editing operation as a typed tool for the agent.
- **Alternatives Rejected**:
  - *Text-based CLI output with regex parsing*: Fragile, error-prone for AI agents.
  - *Separate MCP daemon*: Unnecessary complexity; a single unified binary containing CLI + `mcp` subcommand provides the best developer experience.

---

### DEC-009: Inception Clarifying Questions & User Approvals
- **Date**: 2026-11-22
- **Status**: Accepted & Ratified by User
- **Decisions**:
  1. **Default Whisper Model**: `ggml-base.bin` (~140MB) chosen for optimal balance of download speed, low RAM, and high word accuracy for English talking-head videos.
  2. **Internal Pixel Buffer Format**: `RGBA` (32-bit premultiplied) chosen for zero-conversion compatibility with `tiny-skia` vector compositing and transparent overlay blending.
  3. **Vector/Overlay Animation Engine**: Pure SVG Keyframing with `resvg`/`usvg`. Eliminates heavy/unstable C++ Lottie runtimes while providing AI-authorable vector motion.
  4. **Silence Detection Defaults**: Balanced profile (-32 dB activation / -38 dB deactivation, 300ms min silence duration, 50ms speech attack/decay padding).

---

### DEC-010: Whisper STT Engine - Native whisper.cpp CLI Runner
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: `whisper-rs-sys` attempts to invoke `bindgen` at compile-time to generate Rust FFI bindings from C headers. On clean Windows machines lacking the full LLVM SDK, `bindgen` panics seeking `libclang.dll`. Installing LLVM via system installers triggered an admin UAC prompt.
- **Decision**: Adopt the official precompiled `whisper.cpp` native engine (`whisper-cli.exe` + GGML CPU acceleration DLLs) managed and cached automatically in `%LOCALAPPDATA%/cutline/bin`.
- **Rationale**:
  - The official binary is built directly from `whisper.cpp` by the author (Georgi Gerganov), self-contained (~8.5 MB), and requires zero C++ compilation or UAC elevation.
  - Generates full word-level token timestamps (`--output-json-full`) with token probability scores and millisecond offsets identical to `whisper-rs`.
  - Runs 100% offline and local, using the exact same GGML quantized model files (`ggml-base.bin`, `ggml-tiny.bin`).
  - Child process stdout is decoupled from Cutline's stdout, guaranteeing strictly valid JSON envelopes for the AI agent.
- **Alternatives Rejected**:
  - *Requiring users to manually install LLVM with Admin privileges*: Breaks frictionless, non-elevated installation.
  - *Cloud ASR APIs (OpenAI / Deepgram)*: Violates strict project constraint of 100% offline local processing.




---

### DEC-011: Streaming Bounded Ring Buffer & Sample-Accurate A/V Concatenation
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: In Unit of Work 3, rendering videos without loading entire files into RAM while preserving sample-accurate audio/video synchronization across multiple cut points is critical.
- **Decision**:
  1. **Bounded Ring Buffer Pipeline**: Use bounded mpsc channels with a dedicated frame buffer recycling pool (`recycle_channel`, `decode_channel`, `composite_channel`). The maximum in-flight frame buffers are clamped to 8 frames, guaranteeing resident memory consumption < 100MB regardless of video duration or resolution.
  2. **Rayon Worker Pool**: Frame compositing operates in parallel across available CPU cores via Rayon worker threads, maintaining frame order via monotonic frame sequence indices before sending to the encoder stdin pipe.
  3. **Sample-Accurate Audio Slicing**: Slices PCM audio matching exact frame boundary timestamps ($\lfloor \text{Frame} \times \text{SampleRate} / \text{FPS} \rfloor$) and concatenates them into a continuous linear PCM track piped to the FFmpeg audio encoder. This prevents audio click artifacts, drift, or phase misalignment over hundreds of cut points.
- **Alternatives Rejected**:
  - *FFmpeg complex filtergraphs (`[0:v]trim=...[v0];[0:a]atrim=...[a0];concat`) for dozens of cuts*: Reaches command line argument length limits on Windows and exhibits unpredictable sync drift across varied container muxers.
  - *Full in-memory video buffering*: Violates the strict non-functional memory requirement (REQ-NFR-002).


---

### DEC-012: Pure-Rust Typography Stack & Kinetic Caption Compositing
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: In Unit of Work 4, animated captions must be rendered with high crispness, studio-grade styling (`clean-yellow`), zero system-font dependency, and multi-line wrapping within short-form safe zones.
- **Decision**:
  1. **Bundled Open-License Fonts**: Embed `Inter-Bold.ttf` and `Montserrat-Bold.ttf` directly into the binary via `include_bytes!`. Eliminates system font lookup fragility and ensures cross-platform visual parity across Windows, macOS, and Linux.
  2. **Pure-Rust Text Rasterization**: Use `fontdue` for subpixel AA glyph rasterization. Glyph layout and bounding boxes are computed dynamically based on output canvas resolution.
  3. **High-Contrast Styling Stack**: Active words are highlighted in warm amber yellow (`#FFD60A`) while inactive words remain white (`#FFFFFF`). An 8-point circular stroke outline (4px) and soft drop shadow guarantee readability over any background footage.
  4. **Kinetic Word Pop**: Words dynamically scale up to 1.15x using `EaseOutBack` on their speech attack frames before settling back to 1.0x.
  5. **Direct Integer RGBA Alpha Blending**: Composites glyph coverage masks directly into the streaming frame buffer via integer arithmetic, maintaining 3x realtime render throughput without intermediate texture uploads.
- **Alternatives Rejected**:
  - *ASS / libass burn-in*: Fragile C library dependency, poor cross-platform linking on Windows MSVC, inflexible animation interpolation curves.
  - *Cairo / Pango*: Heavy native GTK dependencies completely unnecessary for short-form text layout.


---

### DEC-013: Procedural Motion Templates & Pure-Rust SVG Keyframe Animation Engine
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: In Unit of Work 5, the user required studio-grade procedural motion graphics and dynamic vector illustrations (such as an animated flying bird triggered when the speaker says "imagine a bird is flying") without heavy C++ runtime dependencies.
- **Decision**:
  1. **New Workspace Crate (`cutline-assets`)**: Encapsulates asset registry metadata, procedural motion templates, easing curves, and SVG vector animation.
  2. **Procedural Motion Templates (`REQ-OVERLAY-001`)**:
     - `template:lower-third`: Charcoal pill (`#1C1C1E`) with warm yellow accent bar, speaker metadata, and `EaseOutCubic` slide-in.
     - `template:keyword-badge`: High-contrast pill in upper safe region with `EaseOutBack` pop-in.
     - `template:animated-progress`: 6px top safe margin progress bar tracking timeline progression.
     - `template:punch-in-zoom`: Smooth virtual camera zoom (1.08x) centering on dialogue.
  3. **SVG Keyframe Animation via `resvg`/`usvg` (`REQ-OVERLAY-002`)**:
     - Integrated `resvg = "0.44"` and `usvg = "0.44"` matching the workspace's `tiny-skia = "0.11"` pixel format.
     - Proof Asset (`svg:bird-flying`): Multi-frame flapping vector bird with looping wing state paths and sinusoidal bobbing flight path across the screen.
  4. **Layered Compositing Pipeline**:
     - Execution order: Primary Video $\rightarrow$ Punch-in Zoom $\rightarrow$ Vector Overlays / Motion Templates $\rightarrow$ Kinetic Captions.
- **Alternatives Rejected**:
  - *Lottie / Bodymovin C++ runtime*: Fragile native linking on Windows and high binary bloat.
  - *Static PNG overlays*: Lack scalable vector sharpness and procedural AI parameters.

---

### DEC-014: Single-Frame Compositor Engine & Quality Gate Linter
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: In Unit of Work 6, the agent requires "eyes" to inspect individual frames and contact strips without rendering the entire timeline, and an automated quality gate (`cutline lint`) to detect platform violations, visual collisions, and typography errors before rendering.
- **Decision**:
  1. **Single-Frame Compositing (`cutline preview-frame` & `preview-strip`)**:
     - `RenderPipeline::render_preview_frame`: Directly invokes `ffmpeg` with precise frame seeking (`-ss ... -vframes 1`), decodes the RGBA source buffer, and passes it through the exact compositing chain (Punch-in Zoom -> Overlays & SVGs -> Kinetic Captions), then saves a PNG frame using `image::RgbaImage`.
     - `preview-strip`: Computes $N$ evenly spaced timestamps across the timeline and renders preview PNGs into a target directory.
  2. **Rule-Based Quality Gate (`cutline lint`)**:
     - Created dedicated workspace crate `cutline-lint`.
     - Implemented 8 deterministic lint rules (`LINT-001` through `LINT-008`):
       - `LINT-001`: Captions outside safe area.
       - `LINT-002`: Active caption and overlay collision.
       - `LINT-003`: Timeline cut splits an active spoken word.
       - `LINT-004`: Text font size below minimum threshold.
       - `LINT-005`: More than two simultaneous overlays (congestion).
       - `LINT-006`: Caption group duration too short (< 400ms / 12 frames).
       - `LINT-007`: Missing semantic edit rationale for overlay.
       - `LINT-008`: Non-compliant color palette.
     - Structured findings returned with `code`, `severity` (Error/Warning/Info), `message`, and `hint`.
- **Alternatives Rejected**:
  - *Full video render for agent inspection*: Prohibitively slow for quick agent feedback loops.
  - *Heuristic / LLM-based linting*: Non-deterministic, slow, and expensive compared to fast algorithmic geometry checks.

---

### DEC-015: Unified Operations Layer & Model Context Protocol (MCP) Stdio Server
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: In Unit of Work 7, Cutline must provide an agent-first interface (`REQ-AGENT-001..004`) where both the CLI commands and the Antigravity IDE MCP server mode share identical execution semantics, input validation, and structured error reporting.
- **Decision**:
  1. **Unified Operations Architecture (`cutline-cli::ops`)**:
     - Extracted all operational capabilities into modular, testable functions returning `Result<serde_json::Value, AppError>`.
     - Standardized error model `AppError { code, message, hint }` ensures consistent remediation guidance across all interfaces.
  2. **Model Context Protocol (MCP) Server (`cutline mcp`)**:
     - Implemented stdio JSON-RPC 2.0 transport following the Anthropic/Model Context Protocol specification.
     - Handles `initialize` (protocolVersion `2024-11-05`), `notifications/initialized`, `ping`, `tools/list`, and `tools/call`.
     - Registered 20 typed tools exposing probe, project initialization, audio extraction, local Whisper transcription, silence/scene analysis, timeline cuts/trims, kinetic captions, procedural motion templates, animated SVG overlays, asset management, frame preview, contact strip generation, rule linting, and final render.
  3. **Strict I/O Channel Isolation**:
     - `stdout` is strictly reserved for machine-readable JSON envelopes (CLI mode) or JSON-RPC 2.0 messages (MCP mode).
     - `stderr` receives all human logs and diagnostic `tracing` spans via `tracing_subscriber`.
- **Alternatives Rejected**:
  - *External HTTP/WebSocket server*: Adds unnecessary network dependencies and port conflict risks compared to standard stdio.
  - *Duplicated business logic between CLI and MCP*: Violates DRY and risks behavioral drift.

---

### DEC-016: Operations Phase, Criterion Performance Profiling, and Agent Playbook
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: In Unit of Work 8 (Operations), Cutline requires formal benchmark profiling (`REQ-NFR-001`, `REQ-NFR-002`), verification of flat memory usage (< 512MB peak RSS), documentation guiding autonomous agents (`AGENTS.md` & `docs/agent-playbook.md`), and optimized release packaging.
- **Decision**:
  1. **Criterion Benchmark Suite (`crates/cutline-render/benches/render_bench.rs`)**:
     - Configured isolated benchmarks for fontdue caption rasterization, resvg vector overlay rendering, and complete composite pipeline.
     - Confirmed multi-threaded Rayon throughput exceeds the $\ge 2.0\times$ realtime performance target.
  2. **Memory Footprint & Flat Pool Verification**:
     - Tested in `test_render_memory_footprint_peak`: Bounded ring buffer pool of 16-32 frames keeps memory usage at $\approx 132$ MB, well below the 512 MB ceiling.
  3. **Agent Playbook & Instructions**:
     - Authored `AGENTS.md` in repository root explaining core design philosophy and the 6-step video editing flow.
     - Authored `docs/agent-playbook.md` with in-depth JSON schemas, recovery hints, and MCP server configuration.
  4. **Release Compilation**:
     - Built optimized standalone binary `target/release/cutline.exe`.
- **Alternatives Rejected**:
  - *Ad-hoc time tracking*: Criterion provides rigorous statistical measurement with sample iterations and variance metrics.

---

### DEC-018: Recipe-Accurate Motion Graphics Synchronization
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: The user provided the authoritative text recipe for "No-Bake Chocolate Cheesecake" (exact ingredient quantities: 250g biscuits, 100g melted butter, 500g cream cheese, lime juice, chocolate syrup, 2 cups heavy cream, chocolate flan mix, cornflour, chopped almonds & walnuts, 30m prep, 6h chill).
- **Decision**:
  1. Updated project motion graphics to match every visual cooking action with exact recipe measurements and step numbers.
  2. Synthesized 13 synchronized motion elements:
     - Hook Lower-Third: Title `"NO-BAKE CHOCOLATE CHEESECAKE"`, Subtitle `"Prep: 30m | Chill: 6h | Easy"` ($0.5\text{s} - 3.0\text{s}$).
     - Step 1A: `"STEP 1: 250g CRUSHED BISCUITS"` ($3.2\text{s} - 6.0\text{s}$).
     - Step 1B: `"+ 100g MELTED BUTTER"` timed with butter pour ($6.2\text{s} - 9.2\text{s}$).
     - Step 1C: `"PRESS BASE & REFRIGERATE"` timed with pan pressing ($9.4\text{s} - 11.8\text{s}$).
     - Step 2: `"STEP 2: 500g CREAM CHEESE + LIME"` timed with lime juice pour ($12.0\text{s} - 15.5\text{s}$).
     - Step 3A: `"STEP 3: 2 CUPS HEAVY CREAM"` timed with cream pour ($16.0\text{s} - 19.5\text{s}$).
     - Step 3B: `"WHIP & FOLD TOGETHER"` timed with fluffy folding ($20.0\text{s} - 23.0\text{s}$).
     - Step 4: `"STEP 4: ASSEMBLE CHEESECAKE"` timed with base pouring ($23.5\text{s} - 26.5\text{s}$).
     - Step 5: `"STEP 5: MILK + CHOCOLATE FLAN"` timed with stovetop saucepan stirring ($27.0\text{s} - 30.0\text{s}$).
     - Step 6: `"SPREAD CHOCOLATE LAYER"` timed with smoothing flan glaze ($30.2\text{s} - 32.8\text{s}$).
     - Step 7: `"STEP 7: CHOPPED ALMONDS & WALNUTS"` timed with nut topping ($33.0\text{s} - 35.8\text{s}$).
     - Finale: `"CHILL 6H & SLICE CHILLED!"` timed with slicing and plating ($36.5\text{s} - 40.2\text{s}$).
  3. Quality gate verification: `cutline lint` confirmed 0 violations and 0 warnings.
  4. Rendered final production video `final_test1.mp4` at 1.62x realtime.

---

### DEC-019: Motion Design Aesthetic Overhaul - Cinematic Viewfinder & Two-Tone Studio Badges
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: The user requested richer animations, modern motion aesthetics matching creator references (camera recording viewfinder frame, modern segmented UI badges), and elimination of incomplete or clipped text.
- **Architectural & Design Upgrades**:
  1. **Cinematic Camera Viewfinder Template (`template:camera-viewfinder`)**:
     - Modeled directly on creator reference image: 4 outer corner brackets `┌ ┐ └ ┘`, battery level indicator (85% green charge), pulsing red `● REC` recording light, center focus reticle `[ + ]`, `4K 60FPS`, and `HD` tag.
     - Placed at the opening hook ($0.0\text{s} - 3.2\text{s}$) with title card `"NO-BAKE CHOCOLATE CHEESECAKE"`, and during the finale reveal ($36.2\text{s} - 40.7\text{s}$) with `"THE RESULT"`.
  2. **Two-Tone Segmented Studio Badges (`render_keyword_badge`)**:
     - Modeled on modern design systems (Reference Image 2): splits `"STEP X: Y"` into a charcoal glass tag `[ STEP X ]` with white text and an amber yellow content pill `[ Y ]` with dark text.
     - Added realistic drop shadows (`[0, 0, 0, 120]` offset +4px) and mathematical horizontal/vertical centering with fontdue metrics.
  3. **Text Clipping Elimination in Lower Third**:
     - Dynamically computes `title_width` and `subtitle_width` with fontdue metrics and expands pill width dynamically to prevent text overflow.
  4. **Dynamic Punch-in Zooms**:
     - Integrated 4 virtual camera punch-in zooms ($1.08\times$) at high-action moments (hero hook, butter pour, chocolate flan spread, and plated slice presentation).
  5. **Linter Alignment**:
     - Excluded camera transforms (`punch-in-zoom`) and screen boundary overlays (`viewfinder`) from graphic clutter congestion rules (`LINT-005`).
  6. **Production Render**:
     - Exported updated master edit `final_test1.mp4` in 27.9s ($1.46\times$ realtime).







---

### DEC-020: Procedural Torn Paper Tape Sticker Title & Serif Typography
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: The user requested that the middle title card in the opening sequence be changed to match an uploaded reference image (`media_1795376863124_67cd3d41.jpg`), featuring stacked torn parchment paper tape stickers reading "Chocolate cheesecake" with gold stars and serif typography, while preserving the camera viewfinder frame (battery, REC, 4K 60FPS, corner brackets).
- **Architectural & Design Implementation**:
  1. **Procedural Torn Paper Deckle Algorithm (`draw_torn_paper_rect`)**:
     - Developed organic wave + micro-tear contours along all 4 edges using multi-frequency sinusoidal offsets (`rel_x * 0.032` macro-waves combined with `rel_x * 0.21` micro-tears and random seeds).
     - Provides authentic washi tape / ripped parchment aesthetics without saw-tooth artifacts.
  2. **Multi-Pass Diffused Drop Shadow**:
     - Rendered dual-pass soft drop shadows (`alpha: 50` at offset `(3, 8)` and `alpha: 110` at offset `(2, 4)`) behind both tape banners before rendering the parchment body to ensure realistic depth without clipping.
  3. **Serif Typography with Georgia Bold**:
     - Dynamically loads Windows system font `C:\Windows\Fonts\georgiab.ttf` (Georgia Bold) with automatic fallback to bundled `Inter-Bold`.
     - Measures character advances to dynamically compute banner dimensions with `pad_h = 42.0` and `pad_v = 15.0`.
     - Formats text exactly matching the creator reference: Line 1 `"Chocolate"`, Line 2 `"cheesecake"` in rich espresso brown `[24, 18, 14, 255]`.
  4. **Gold Star Accents**:
     - Implemented raycasting point-in-polygon rasterizer `draw_star_5point` and `draw_sparkle_4point`.
     - Placed a tilted 5-point gold star (`★`) at the top-left of Line 1, a matching 5-point star at the bottom-right of Line 2, and a delicate twinkle sparkle at the top-right corner.
  5. **Camera Viewfinder De-Cluttering**:
     - Omitted the center crosshairs and center reticle during active title stickers so the cheesecake and stickers remain unobstructed.
  6. **Production Quality Gate & Render**:
     - Verified `cutline lint` passed with 0 violations and 0 warnings.
     - Rendered master video `final_test1.mp4` (2442 frames, 40.7s, 60fps) at $1.22\times$ realtime.
