# AGENTS.md - Guide for AI Agents Driving Cutline

This document provides definitive instructions for any AI agent (such as **Antigravity IDE**, Claude Code, or autonomous coding agents) operating **Cutline**.

---

## 1. Core Operating Philosophy

- **You are the editor brain; Cutline is the hands and eyes.**
  Cutline contains no internal LLM. It relies entirely on you to make creative editorial decisions, parse transcripts, identify bad takes, time punch-cuts, and illustrate spoken concepts.
- **Non-Destructive Editing**:
  Never modify or delete original raw video or audio files. All edits exist as non-destructive slices, trims, and layers within `project.json`.
- **Rational Frame-Accurate Time**:
  All timeline operations internally use integer frames and rational FPS (`30/1`, `60/1`). When specifying timestamps via CLI or MCP (`00:00:12.400` or `12.4s`), Cutline automatically snaps to the nearest exact frame boundary.

---

## 2. Dual Interface: MCP Server vs. CLI

Cutline provides two identical machine-readable interfaces:

### Option A: Model Context Protocol (MCP) Mode (Recommended for Antigravity IDE)
Run Cutline as an MCP stdio server:
```bash
cutline mcp
```
All capabilities are exposed as typed JSON-RPC tools (`probe_media`, `init_project`, `transcribe_audio`, `auto_cut_silence`, `apply_captions`, `add_overlay`, `preview_frame`, `lint_project`, `render_video`).

### Option B: CLI Mode (Command Line & Subprocesses)
Every command outputs pure JSON to `stdout` (`{"status": "success", "schema_version": "1.0.0", "data": ...}`).
All human-readable logs and diagnostics go exclusively to `stderr`.
Errors follow the structured schema:
```json
{
  "status": "error",
  "code": "ERR_INVALID_FRAME_RANGE",
  "message": "Start time must precede end time.",
  "hint": "Verify that start < end."
}
```

---

## 3. The Autonomous 6-Step Video Editing Playbook

Follow this strict sequence to edit talking-head videos from raw footage to final render:

```
[1. Probe & Init] -> [2. Audio & Transcribe] -> [3. Silence & Cuts] -> [4. Kinetic Captions] -> [5. Visual Overlays] -> [6. Quality Gate & Render]
```

### Step 1: Probe Media & Initialize Project
1. Run `probe_media` on the raw footage to inspect resolution, frame rate, and audio tracks.
2. Run `init_project` with `preset="tiktok"` (9:16 portrait 1080x1920) or `"youtube"` (16:9 1920x1080).
   *Command*: `cutline project init raw_talking_head.mp4 --preset tiktok --out project.json`

### Step 2: Extract Audio & Offline Transcription
1. Extract 16kHz mono audio for Whisper.
   *Command*: `cutline audio extract project.json --out audio.wav`
2. Run offline Whisper transcription to get word tokens with exact start and end frame timestamps.
   *Command*: `cutline transcribe project.json --model base --out transcript.json`
3. Read `transcript.json` to understand the dialogue, key phrases, and pauses.

### Step 3: Remove Silences & Bad Takes
1. Run silence detection to find pauses and dead air using dual-threshold hysteresis.
   *Command*: `cutline analyze silence project.json --threshold-db -32.0 --min-duration-ms 300`
2. Automatically excise silences while preserving 50ms speech padding.
   *Command*: `cutline timeline auto-cut-silence project.json --padding-ms 50`
3. If the speaker stumbled or repeated a take (e.g. said "Let me restart that"), excise the bad take manually:
   *Command*: `cutline timeline cut project.json --start 00:00:14.200 --end 00:00:19.500`

### Step 4: Apply Studio Kinetic Captions
1. Generate word-by-word animated captions tied to transcript timestamps:
   *Command*: `cutline captions apply project.json --style clean-yellow --transcript transcript.json`
2. Cutline automatically breaks text into 2-4 word groups, enforces TikTok safe margins, renders active words in warm yellow (`#FFD60A`) with a kinetic scale pop (1.15x), and renders inactive words in clean white with a 4px dark stroke outline and drop shadow.

### Step 5: Illustrate What the Speaker Says
When the speaker says something visual or emphatic (e.g. *"imagine a bird is flying"* or introduces a key topic):
1. Query available assets via `cutline assets list` or `list_assets`.
2. Place a procedural motion template or animated SVG onto the timeline matching the spoken word range.
3. Always supply `--reason` linking the visual to dialogue:
   *Command*:
   ```powershell
   cutline overlay add project.json `
     --asset svg:bird-flying `
     --at 00:00:04.200 `
     --duration 00:00:02.500 `
     --anchor top_center `
     --reason "Speaker said 'imagine a bird is flying'" `
     --word-range 12,13,14,15
   ```
4. Other available templates:
   - `template:keyword-badge`: High-contrast pill for key concepts.
   - `template:lower-third`: Speaker name and role tag.
   - `template:punch-in-zoom`: Dynamic 1.08x camera punch zoom centering attention.
   - `template:animated-progress`: 6px playback progress bar along top safe margin.

### Step 6: Visual Quality Gate & Final Render
1. **Never render blindly.** Use your "eyes" to inspect visual composition:
   *Command*: `cutline preview-frame project.json --time 00:00:05.000 --out preview_bird.png`
   *Strip*: `cutline preview-strip project.json --count 6 --out-dir previews/`
2. Run the automated quality gate linter:
   *Command*: `cutline lint project.json --transcript transcript.json`
   If `passed: false`, read the violation details and fix the issue (e.g. adjust overlay anchor if colliding with captions, or widen cut if splitting a word).
3. Once linting passes, export the final video:
   *Command*: `cutline render project.json --out final_edit.mp4 --preset tiktok`

---

## 4. MCP Server Registration in Antigravity IDE

To register Cutline as an MCP server in Antigravity or client config:

```json
{
  "mcpServers": {
    "cutline": {
      "command": "target/release/cutline.exe",
      "args": ["mcp"]
    }
  }
}
```

---

## 5. Summary of Design System Rules

| Element | Specification | Platform Rule |
|---|---|---|
| **Safe Area (9:16)** | Top: 100px, Bottom: 150px, Left: 90px, Right: 130px | Keep clear of TikTok UI buttons |
| **Active Caption** | Warm Amber Yellow `#FFD60A` + 1.15x Pop | 8-point stroke outline (4px) |
| **Inactive Caption** | Pure White `#FFFFFF` + Subtle Drop Shadow | Inter Bold / Montserrat ExtraBold |
| **Minimum Caption Duration** | $\ge 400\text{ms}$ (12 frames at 30fps) | Never split groups too fast |
| **Max Concurrent Overlays**| $\le 2$ simultaneous elements | Avoid screen congestion |
