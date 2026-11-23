# Cutline - Risks, Trade-offs & Open Questions Specification

This document identifies technical risks, architectural trade-offs, and critical platform variables considered during the Inception phase of **Cutline**.

---

## 1. Technical Risks & Mitigation Strategies

### Risk 1: `whisper-rs` (C++ bindings) Build and Native Toolchain Dependencies on Windows
- **Severity**: High
- **Description**: `whisper-rs` binds to `whisper.cpp` written in C/C++. On Windows, building from source via `cc-rs` requires a working C++ compiler (MSVC or Clang) and CMake. Any environment mismatch can break `cargo build`.
- **Observed Environment**:
  - `rustc 1.99.1`, `cargo 1.99.1` installed.
  - CMake 4.2.1 is present.
  - Visual Studio 18 Community (MSVC) is present in `C:\Program Files\Microsoft Visual Studio\18\Community`.
- **Mitigation Strategy**:
  1. Primary: Use `whisper-rs` with CPU inference (`whisper-rs = "0.13"`). Ensure `cc` and `cmake` build scripts locate MSVC.
  2. Fallback / Alternative: If native compilation issues arise on certain developer machines, Cutline can support executing the pre-compiled `whisper-cli` binary or FFmpeg's built-in whisper filter (`--enable-whisper`, already compiled into the local FFmpeg build!), preserving an identical JSON contract for the agent.

---

### Risk 2: Lottie Animation Support in Pure Rust
- **Severity**: Medium
- **Description**: The prompt states: *"Lottie (only if a solid Rust renderer exists; if not, say so and propose SVG keyframe animation)"*. Pure Rust Lottie renderers (such as `rlottie-rs`) require external C++ dependencies, while pure-Rust ports remain incomplete.
- **Decision & Mitigation**:
  - We explicitly reject a heavy C++ Lottie runtime to maintain clean, reliable builds.
  - We adopt **Animated SVG Keyframing with `resvg`/`usvg`**:
    - SVGs are lightweight, vector-crisp, editable in text by AI agents, and render at microsecond latency via `resvg`.
    - Keyframed parameters (opacity, translation X/Y, rotation, scale, path morphology) are calculated per frame using standard easing curves (`EaseOutCubic`, `EaseOutBack`) and rendered into frame pixmaps.

---

### Risk 3: Hardware Video Encoding Variations & Platform Drift
- **Severity**: Medium
- **Description**: Hardware encoders (NVENC on NVIDIA, AMF on AMD, QSV on Intel, VideoToolbox on macOS, VAAPI on Linux) depend on driver support and GPU hardware availability. If a hardware encoder fails mid-run, rendering could fail.
- **Mitigation Strategy**:
  - Cutline implements an automatic **Hardware Probing & Fallback Cascade**:
    - During initialization or render setup, Cutline tests the hardware encoder with a 1-frame null pipeline.
    - If the hardware encoder errors or is absent, it seamlessly defaults to `libx264` (`-preset veryfast -crf 20`).
    - The CLI and MCP outputs report the exact `encoder_used` (e.g. `h264_nvenc` vs `libx264`).

---

### Risk 4: Audio-Video Desynchronization Across Multi-Segment Cuts
- **Severity**: High
- **Description**: Slicing both video frames and audio streams at multiple arbitrary cut points can accumulate fractional sample offsets, resulting in noticeable lip-sync drift over a 60-second video.
- **Mitigation Strategy**:
  - Cutline uses **Sample-Exact Time Alignment**:
    - Audio is sliced based on the exact integer frame boundaries converted to PCM sample counts: $\text{Sample} = \lfloor \text{Frame} \times 48000 / \text{FPS} \rfloor$.
    - Audio segments are concatenated in-memory or streamed through an FFmpeg filtergraph with explicit timestamps (`asetpts=N/SR/TB`), guaranteeing zero cumulative slip.
    - Integration tests will verify audio duration equals video frame count divided by fps within 1 audio sample tolerance.

---

### Risk 5: High Memory Consumption During 1080x1920 Compositing
- **Severity**: Medium
- **Description**: A 1080x1920 RGBA frame is 8.3 MB. Storing full uncompressed videos in RAM (e.g. 1800 frames for 60s) would consume ~15 GB of RAM, violating our performance constraints.
- **Mitigation Strategy**:
  - Strict **Bounded Ring Buffer Pipeline**:
    - Maximum 16 frames in the decode-to-composite queue, and 16 frames in the composite-to-encode queue.
    - Buffers are pre-allocated and recycled (`FrameBufferPool`).
    - Maximum process memory is bounded to < 350 MB, regardless of whether the source video is 10 seconds or 2 hours.

---

## 2. Inception Trade-offs & Decisions Summary

| Dimension | Chosen Approach | Rejected Alternative | Key Reason |
|---|---|---|---|
| **FFmpeg Integration** | Subprocess stdio pipes (`rawvideo`, `s16le`) | C bindings (`ffmpeg-next`, `libav-sys`) | C library linking on Windows MSVC is fragile; CLI pipes are robust, fast, and cross-platform. |
| **Model Scope** | Speech-to-Text ONLY (local Whisper) | Adding face detection / LLMs | The Antigravity agent is the brain; the engine is pure deterministic Rust code. |
| **Vector Animation** | Parameterized SVG + `resvg` | Lottie (C++ runtime) | Pure-Rust reliability, AI-agent authorable, zero heavy dependencies. |
| **Time Model** | Rational ticks (`FrameTime`) | Floating-point seconds (`f64`) | Eliminates floating-point drift and off-by-one frame rounding errors. |
| **Agent Interface** | Unified CLI (JSON stdout) + `mcp` subcommand | Separate standalone daemon | Single compiled binary, easier distribution and testing. |
