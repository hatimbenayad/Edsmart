# Edsmart (Cutline Engine)

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![MCP Compatible](https://img.shields.io/badge/MCP-Protocol%202024--11--05-brightgreen.svg)](https://modelcontextprotocol.io/)

> **Autonomous AI-Driven Video Editing Engine for Short-Form Content (TikTok, YouTube Shorts, Reels)**

**Edsmart (Cutline)** is a high-performance, non-destructive video editing engine written in Rust. It is engineered from the ground up to be operated either by autonomous AI coding agents (such as Antigravity IDE, Claude Code) via the **Model Context Protocol (MCP)** or directly through a machine-readable **Command Line Interface (CLI)**.

---

## 🚀 Core Philosophy

1. **AI as Brain, Engine as Hands & Eyes**:
   Edsmart contains no internal LLM. It exposes frame-accurate timeline editing, offline Whisper transcription, kinetic typography, procedural motion graphics, and visual quality gating to external autonomous agents.
2. **Non-Destructive JSON Timeline**:
   Original footage and audio tracks are never mutated. All cuts, zooms, layers, and styles exist within a versioned `project.json` schema.
3. **Deterministic Frame-Accurate Time**:
   Timeline timestamps automatically snap to rational exact frame boundaries ($30/1$, $60/1$).
4. **Studio-Grade Visual Gating**:
   Autonomous agents never render blindly; they inspect individual frames and contact strips, and validate edits against rule-based quality linter gates before exporting.

---

## 🏗️ Workspace Architecture

Edsmart is structured as a modular Rust workspace consisting of 8 focused crates:

| Crate | Purpose | Key Technologies |
|---|---|---|
| `cutline-core` | Data models, rational time math, `project.json` schema serialization | `serde`, `serde_json`, rational frame math |
| `cutline-media` | Media probing, audio extraction, streaming FFmpeg process management | `ffmpeg`, `ffprobe` |
| `cutline-stt` | Offline local speech-to-text with word-level timestamps | `whisper-rs` (Whisper.cpp) |
| `cutline-analysis` | Silence detection with hysteresis, scene boundary analysis | Root Mean Square (RMS) energy analysis |
| `cutline-assets` | Procedural motion templates, easing math, animated vector SVGs | `resvg`, `tiny-skia`, procedural rasterizers |
| `cutline-render` | Multi-threaded frame compositing, kinetic typography, hardware acceleration | `fontdue`, `rayon`, `tiny-skia`, `libx264` |
| `cutline-lint` | Automated quality gate linter detecting safe area and visual violations | Rule engine (`LINT-001` .. `LINT-008`) |
| `cutline-cli` | Unified CLI binary and MCP stdio server | `clap`, `tokio`, JSON-RPC 2.0 |

---

## ⚡ The 6-Step Autonomous Editing Playbook

```
[1. Probe & Init] -> [2. Audio & Transcribe] -> [3. Silence & Cuts] -> [4. Kinetic Captions] -> [5. Visual Overlays] -> [6. Quality Gate & Render]
```

1. **Probe Media & Initialize Project**:
   ```bash
   cutline project init input.mp4 --preset tiktok --out project.json
   ```
2. **Extract Audio & Transcribe**:
   ```bash
   cutline audio extract project.json --out audio.wav
   cutline transcribe project.json --model base --out transcript.json
   ```
3. **Silence Analysis & Frame Cuts**:
   ```bash
   cutline analyze silence project.json --threshold-db -32.0 --min-duration-ms 300
   cutline timeline auto-cut-silence project.json --padding-ms 50
   ```
4. **Kinetic Captions**:
   ```bash
   cutline captions apply project.json --style clean-yellow --transcript transcript.json
   ```
5. **Procedural Motion Overlays & Illustrative Badges**:
   ```bash
   cutline overlay add project.json --asset template:keyword-badge --at 00:00:03.200 --duration 00:00:02.800 --reason "Highlight recipe ingredient"
   ```
6. **Visual Inspection & Final Render**:
   ```bash
   cutline preview-frame project.json --time 00:00:01.500 --out preview.png
   cutline lint project.json --transcript transcript.json
   cutline render project.json --out final_edit.mp4 --preset tiktok
   ```

---

## 🔌 Dual Interface: MCP Server vs. CLI

### Option A: Model Context Protocol (MCP) Mode
Register Edsmart in your AI IDE (e.g. Antigravity IDE, Claude Desktop):

```json
{
  "mcpServers": {
    "edsmart": {
      "command": "target/release/cutline.exe",
      "args": ["mcp"]
    }
  }
}
```

Exposes 20 typed JSON-RPC tools (`probe_media`, `init_project`, `transcribe_audio`, `auto_cut_silence`, `apply_captions`, `add_overlay`, `preview_frame`, `lint_project`, `render_video`).

### Option B: Machine-Readable CLI Mode
Every command outputs structured JSON to `stdout` while diagnostic tracing spans route exclusively to `stderr`:

```json
{
  "status": "success",
  "schema_version": "1.0.0",
  "command": "lint",
  "data": {
    "passed": true,
    "violations_count": 0,
    "warnings_count": 0,
    "details": []
  }
}
```

---

## 🎨 Design System & Motion Graphics

- **Safe Margins (9:16)**: Enforces TikTok UI safe margins (Top: 100px, Bottom: 150px, Left: 90px, Right: 130px).
- **Kinetic Word Highlighting**: Active spoken words pop with $1.15\times$ scale and warm amber yellow (`#FFD60A`), surrounded by 8-point stroke outlines.
- **Cinematic Viewfinder Overlay**: Camera HUD with blinking `● REC`, battery status, corner brackets, and resolution tags.
- **Torn Paper Tape Stickers**: Procedurally generated organic deckled edges with Georgia Bold serif typography and gold star accents.
- **Two-Tone Segmented Badges**: Frosted charcoal glass step tags with high-contrast amber yellow content pills.

---

## 🛠️ Building & Verification

### Prerequisites
- [Rust](https://rustup.rs/) (1.75+)
- [FFmpeg & FFprobe](https://ffmpeg.org/) on system `PATH`

### Build
```bash
# Debug build
cargo build

# Optimized production release
cargo build --release -p cutline-cli
```

### Run Tests & Benchmarks
```bash
# Run all 46 unit and integration tests across the 8 crates
cargo test --all

# Run Criterion performance benchmarks
cargo bench -p cutline-render
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
