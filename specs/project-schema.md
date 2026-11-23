# Cutline - Project File Schema Specification (`project.json`)

This document defines the schema and data model for `project.json`, the non-destructive timeline and Edit Decision List (EDL) driving Cutline.

---

## 1. Schema Overview

The project file represents the single source of truth for all editing decisions. It contains:
1. **Metadata**: Project ID, creation timestamp, schema version.
2. **Output Target**: Resolution, rational frame rate, audio configuration, export preset.
3. **Sources**: Primary speaking-head video and optional B-roll media files.
4. **Timeline Tracks**:
   - `video_tracks`: Ordered list of video layers containing frame segments.
   - `audio_tracks`: Dialogue audio and optional background music/SFX.
5. **Transcript**: Normalized word-level transcription with token timings.
6. **Captions**: Visual configuration, styles, safe-area bounds, and word groupings.
7. **Overlays**: Graphical motion templates, animated SVGs, and cutaways.
8. **Markers**: Detected silences, scene cuts, and agent semantic tags.

---

## 2. Example `project.json` Instance

```json
{
  "schema_version": "1.0.0",
  "project_id": "proj_9f82c47a",
  "name": "talking_head_demo",
  "created_at": "2026-11-22T16:00:00Z",
  "output": {
    "width": 1080,
    "height": 1920,
    "fps": { "num": 30, "den": 1 },
    "sample_rate": 48000,
    "channels": 2,
    "video_bitrate_kbps": 12000,
    "audio_bitrate_kbps": 192
  },
  "sources": [
    {
      "id": "src_primary",
      "path": "raw_footage.mp4",
      "duration_frames": 1800,
      "width": 1080,
      "height": 1920,
      "fps": { "num": 30, "den": 1 },
      "audio_sample_rate": 48000
    },
    {
      "id": "src_broll_1",
      "path": "city_broll.mp4",
      "duration_frames": 600,
      "width": 1080,
      "height": 1920,
      "fps": { "num": 30, "den": 1 },
      "audio_sample_rate": 48000
    }
  ],
  "timeline": {
    "video_tracks": [
      {
        "id": "track_v_main",
        "name": "Main Dialogue Track",
        "segments": [
          {
            "id": "seg_001",
            "source_id": "src_primary",
            "source_in_frame": 30,
            "source_out_frame": 330,
            "timeline_in_frame": 0,
            "timeline_out_frame": 300,
            "speed": 1.0
          },
          {
            "id": "seg_002",
            "source_id": "src_primary",
            "source_in_frame": 390,
            "source_out_frame": 690,
            "timeline_in_frame": 300,
            "timeline_out_frame": 600,
            "speed": 1.0
          }
        ]
      }
    ],
    "audio_tracks": [
      {
        "id": "track_a_main",
        "name": "Primary Dialogue Audio",
        "segments": [
          {
            "id": "seg_a_001",
            "source_id": "src_primary",
            "source_in_frame": 30,
            "source_out_frame": 330,
            "timeline_in_frame": 0,
            "timeline_out_frame": 300,
            "volume": 1.0
          },
          {
            "id": "seg_a_002",
            "source_id": "src_primary",
            "source_in_frame": 390,
            "source_out_frame": 690,
            "timeline_in_frame": 300,
            "timeline_out_frame": 600,
            "volume": 1.0
          }
        ]
      }
    ]
  },
  "captions": {
    "enabled": true,
    "style_id": "clean-yellow",
    "safe_area": {
      "top": 200,
      "bottom": 380,
      "left": 90,
      "right": 130
    },
    "groups": [
      {
        "id": "cap_001",
        "timeline_in_frame": 15,
        "timeline_out_frame": 75,
        "words": [
          { "text": "Hello", "start_frame": 15, "end_frame": 28, "confidence": 0.98 },
          { "text": "my", "start_frame": 29, "end_frame": 38, "confidence": 0.99 },
          { "text": "name", "start_frame": 39, "end_frame": 55, "confidence": 0.97 },
          { "text": "is", "start_frame": 56, "end_frame": 75, "confidence": 0.99 }
        ]
      }
    ]
  },
  "overlays": [
    {
      "id": "ovl_001",
      "asset_id": "bird_flying",
      "type": "svg_animation",
      "timeline_in_frame": 120,
      "timeline_out_frame": 210,
      "anchor": "top_center",
      "position": { "x": 0.5, "y": 0.28 },
      "scale": 1.0,
      "in_animation": { "kind": "slide_fade_in", "duration_frames": 12, "easing": "ease_out_cubic" },
      "out_animation": { "kind": "fade_out", "duration_frames": 8, "easing": "ease_in_cubic" },
      "reason": {
        "word_range": [8, 12],
        "text": "imagine a bird is flying"
      }
    }
  ],
  "markers": {
    "silences": [
      { "source_in_frame": 331, "source_out_frame": 389, "duration_ms": 1933, "status": "cut" }
    ],
    "scene_cuts": []
  }
}
```

---

## 3. Formal JSON Schema (Draft 2020-12 / Draft-07)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://cutline.dev/schemas/v1/project.json",
  "title": "CutlineProject",
  "type": "object",
  "required": ["schema_version", "project_id", "output", "sources", "timeline"],
  "properties": {
    "schema_version": { "type": "string", "enum": ["1.0.0"] },
    "project_id": { "type": "string" },
    "name": { "type": "string" },
    "created_at": { "type": "string", "format": "date-time" },
    "output": {
      "type": "object",
      "required": ["width", "height", "fps"],
      "properties": {
        "width": { "type": "integer", "minimum": 128 },
        "height": { "type": "integer", "minimum": 128 },
        "fps": {
          "type": "object",
          "required": ["num", "den"],
          "properties": {
            "num": { "type": "integer", "minimum": 1 },
            "den": { "type": "integer", "minimum": 1 }
          }
        },
        "sample_rate": { "type": "integer", "default": 48000 },
        "channels": { "type": "integer", "default": 2 },
        "video_bitrate_kbps": { "type": "integer", "default": 12000 },
        "audio_bitrate_kbps": { "type": "integer", "default": 192 }
      }
    },
    "sources": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "path", "duration_frames", "width", "height", "fps"],
        "properties": {
          "id": { "type": "string" },
          "path": { "type": "string" },
          "duration_frames": { "type": "integer", "minimum": 1 },
          "width": { "type": "integer" },
          "height": { "type": "integer" },
          "fps": {
            "type": "object",
            "required": ["num", "den"],
            "properties": {
              "num": { "type": "integer" },
              "den": { "type": "integer" }
            }
          },
          "audio_sample_rate": { "type": "integer" }
        }
      }
    },
    "timeline": {
      "type": "object",
      "required": ["video_tracks", "audio_tracks"],
      "properties": {
        "video_tracks": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["id", "segments"],
            "properties": {
              "id": { "type": "string" },
              "name": { "type": "string" },
              "segments": {
                "type": "array",
                "items": {
                  "type": "object",
                  "required": ["id", "source_id", "source_in_frame", "source_out_frame", "timeline_in_frame", "timeline_out_frame"],
                  "properties": {
                    "id": { "type": "string" },
                    "source_id": { "type": "string" },
                    "source_in_frame": { "type": "integer", "minimum": 0 },
                    "source_out_frame": { "type": "integer", "minimum": 0 },
                    "timeline_in_frame": { "type": "integer", "minimum": 0 },
                    "timeline_out_frame": { "type": "integer", "minimum": 0 },
                    "speed": { "type": "number", "default": 1.0 }
                  }
                }
              }
            }
          }
        },
        "audio_tracks": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["id", "segments"],
            "properties": {
              "id": { "type": "string" },
              "name": { "type": "string" },
              "segments": {
                "type": "array",
                "items": {
                  "type": "object",
                  "required": ["id", "source_id", "source_in_frame", "source_out_frame", "timeline_in_frame", "timeline_out_frame"],
                  "properties": {
                    "id": { "type": "string" },
                    "source_id": { "type": "string" },
                    "source_in_frame": { "type": "integer", "minimum": 0 },
                    "source_out_frame": { "type": "integer", "minimum": 0 },
                    "timeline_in_frame": { "type": "integer", "minimum": 0 },
                    "timeline_out_frame": { "type": "integer", "minimum": 0 },
                    "volume": { "type": "number", "default": 1.0 }
                  }
                }
              }
            }
          }
        }
      }
    },
    "captions": {
      "type": "object",
      "properties": {
        "enabled": { "type": "boolean", "default": true },
        "style_id": { "type": "string", "default": "clean-yellow" },
        "safe_area": {
          "type": "object",
          "properties": {
            "top": { "type": "integer" },
            "bottom": { "type": "integer" },
            "left": { "type": "integer" },
            "right": { "type": "integer" }
          }
        },
        "groups": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["id", "timeline_in_frame", "timeline_out_frame", "words"],
            "properties": {
              "id": { "type": "string" },
              "timeline_in_frame": { "type": "integer" },
              "timeline_out_frame": { "type": "integer" },
              "words": {
                "type": "array",
                "items": {
                  "type": "object",
                  "required": ["text", "start_frame", "end_frame"],
                  "properties": {
                    "text": { "type": "string" },
                    "start_frame": { "type": "integer" },
                    "end_frame": { "type": "integer" },
                    "confidence": { "type": "number" }
                  }
                }
              }
            }
          }
        }
      }
    },
    "overlays": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "asset_id", "timeline_in_frame", "timeline_out_frame"],
        "properties": {
          "id": { "type": "string" },
          "asset_id": { "type": "string" },
          "type": { "type": "string", "enum": ["template", "svg_animation", "video_cutaway", "image"] },
          "timeline_in_frame": { "type": "integer" },
          "timeline_out_frame": { "type": "integer" },
          "anchor": { "type": "string", "default": "center" },
          "position": {
            "type": "object",
            "properties": {
              "x": { "type": "number" },
              "y": { "type": "number" }
            }
          },
          "scale": { "type": "number", "default": 1.0 },
          "opacity": { "type": "number", "default": 1.0 },
          "in_animation": { "type": "object" },
          "out_animation": { "type": "object" },
          "reason": {
            "type": "object",
            "properties": {
              "word_range": { "type": "array", "items": { "type": "integer" } },
              "text": { "type": "string" }
            }
          }
        }
      }
    },
    "markers": {
      "type": "object",
      "properties": {
        "silences": { "type": "array" },
        "scene_cuts": { "type": "array" }
      }
    }
  }
}
```
