use crate::time::Fps;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub fps: Fps,
    pub sample_rate: u32,
    pub channels: u16,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self::tiktok_preset()
    }
}

impl OutputConfig {
    pub fn tiktok_preset() -> Self {
        Self {
            width: 1080,
            height: 1920,
            fps: Fps::FPS_30,
            sample_rate: 48000,
            channels: 2,
            video_bitrate_kbps: 12000,
            audio_bitrate_kbps: 192,
        }
    }

    pub fn youtube_preset() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: Fps::FPS_30,
            sample_rate: 48000,
            channels: 2,
            video_bitrate_kbps: 15000,
            audio_bitrate_kbps: 256,
        }
    }

    pub fn fast_draft_preset() -> Self {
        Self {
            width: 540,
            height: 960,
            fps: Fps::FPS_30,
            sample_rate: 48000,
            channels: 2,
            video_bitrate_kbps: 3000,
            audio_bitrate_kbps: 128,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Source {
    pub id: String,
    pub path: String,
    pub duration_frames: i64,
    pub width: u32,
    pub height: u32,
    pub fps: Fps,
    pub audio_sample_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VideoSegment {
    pub id: String,
    pub source_id: String,
    pub source_in_frame: i64,
    pub source_out_frame: i64,
    pub timeline_in_frame: i64,
    pub timeline_out_frame: i64,
    #[serde(default = "default_speed")]
    pub speed: f32,
}

fn default_speed() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AudioSegment {
    pub id: String,
    pub source_id: String,
    pub source_in_frame: i64,
    pub source_out_frame: i64,
    pub timeline_in_frame: i64,
    pub timeline_out_frame: i64,
    #[serde(default = "default_volume")]
    pub volume: f32,
}

fn default_volume() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VideoTrack {
    pub id: String,
    pub name: String,
    pub segments: Vec<VideoSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AudioTrack {
    pub id: String,
    pub name: String,
    pub segments: Vec<AudioSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct Timeline {
    pub video_tracks: Vec<VideoTrack>,
    pub audio_tracks: Vec<AudioTrack>,
}

impl Timeline {
    pub fn total_duration_frames(&self) -> i64 {
        self.video_tracks
            .iter()
            .flat_map(|t| t.segments.iter())
            .map(|s| s.timeline_out_frame)
            .max()
            .unwrap_or(0)
    }

    /// Recalculates continuous timeline_in_frame and timeline_out_frame across segments
    pub fn recalculate_timeline_offsets(&mut self) {
        for track in &mut self.video_tracks {
            let mut current_frame = 0;
            for seg in &mut track.segments {
                let duration = (seg.source_out_frame - seg.source_in_frame).max(0);
                seg.timeline_in_frame = current_frame;
                seg.timeline_out_frame = current_frame + duration;
                current_frame += duration;
            }
        }
        for track in &mut self.audio_tracks {
            let mut current_frame = 0;
            for seg in &mut track.segments {
                let duration = (seg.source_out_frame - seg.source_in_frame).max(0);
                seg.timeline_in_frame = current_frame;
                seg.timeline_out_frame = current_frame + duration;
                current_frame += duration;
            }
        }
    }

    /// Excises an interval [cut_start_frame, cut_end_frame] on the timeline clock (REQ-EDIT-001)
    pub fn cut_range(&mut self, cut_start: i64, cut_end: i64) {
        if cut_start >= cut_end {
            return;
        }

        for track in &mut self.video_tracks {
            let mut new_segments = Vec::new();
            for seg in &track.segments {
                let s_in = seg.timeline_in_frame;
                let s_out = seg.timeline_out_frame;

                if cut_end <= s_in || cut_start >= s_out {
                    // No overlap
                    new_segments.push(seg.clone());
                } else if cut_start <= s_in && cut_end >= s_out {
                    // Completely deleted
                    continue;
                } else if cut_start > s_in && cut_end < s_out {
                    // Cut splits segment in two
                    let left_duration = cut_start - s_in;
                    let right_offset = cut_end - s_in;

                    let mut seg1 = seg.clone();
                    seg1.id = format!("{}_a", seg.id);
                    seg1.source_out_frame = seg.source_in_frame + left_duration;

                    let mut seg2 = seg.clone();
                    seg2.id = format!("{}_b", seg.id);
                    seg2.source_in_frame = seg.source_in_frame + right_offset;

                    new_segments.push(seg1);
                    new_segments.push(seg2);
                } else if cut_start <= s_in && cut_end < s_out {
                    // Cut truncates left side
                    let offset = cut_end - s_in;
                    let mut truncated = seg.clone();
                    truncated.source_in_frame = seg.source_in_frame + offset;
                    new_segments.push(truncated);
                } else if cut_start > s_in && cut_end >= s_out {
                    // Cut truncates right side
                    let duration = cut_start - s_in;
                    let mut truncated = seg.clone();
                    truncated.source_out_frame = seg.source_in_frame + duration;
                    new_segments.push(truncated);
                }
            }
            track.segments = new_segments;
        }

        for track in &mut self.audio_tracks {
            let mut new_segments = Vec::new();
            for seg in &track.segments {
                let s_in = seg.timeline_in_frame;
                let s_out = seg.timeline_out_frame;

                if cut_end <= s_in || cut_start >= s_out {
                    new_segments.push(seg.clone());
                } else if cut_start <= s_in && cut_end >= s_out {
                    continue;
                } else if cut_start > s_in && cut_end < s_out {
                    let left_duration = cut_start - s_in;
                    let right_offset = cut_end - s_in;

                    let mut seg1 = seg.clone();
                    seg1.id = format!("{}_a", seg.id);
                    seg1.source_out_frame = seg.source_in_frame + left_duration;

                    let mut seg2 = seg.clone();
                    seg2.id = format!("{}_b", seg.id);
                    seg2.source_in_frame = seg.source_in_frame + right_offset;

                    new_segments.push(seg1);
                    new_segments.push(seg2);
                } else if cut_start <= s_in && cut_end < s_out {
                    let offset = cut_end - s_in;
                    let mut truncated = seg.clone();
                    truncated.source_in_frame = seg.source_in_frame + offset;
                    new_segments.push(truncated);
                } else if cut_start > s_in && cut_end >= s_out {
                    let duration = cut_start - s_in;
                    let mut truncated = seg.clone();
                    truncated.source_out_frame = seg.source_in_frame + duration;
                    new_segments.push(truncated);
                }
            }
            track.segments = new_segments;
        }

        self.recalculate_timeline_offsets();
    }

    /// Trims project to the interval [trim_in, trim_out]
    pub fn trim(&mut self, trim_in: i64, trim_out: i64) {
        let total = self.total_duration_frames();
        if trim_out < total {
            self.cut_range(trim_out, total);
        }
        if trim_in > 0 {
            self.cut_range(0, trim_in);
        }
    }

    /// Automatically excises detected silence intervals while applying padding (REQ-EDIT-002)
    pub fn auto_cut_silence(&mut self, silences: &[SilenceInterval], padding_frames: i64) -> usize {
        let mut cuts_count = 0;
        // Apply cuts in reverse order of source frame to preserve earlier coordinate stability
        let mut sorted_silences = silences.to_vec();
        sorted_silences.sort_by_key(|s| std::cmp::Reverse(s.source_in_frame));

        for s in sorted_silences {
            let padded_start = s.source_in_frame + padding_frames;
            let padded_end = s.source_out_frame - padding_frames;
            if padded_end > padded_start {
                // Find timeline interval corresponding to this source interval
                if let Some((t_start, t_end)) =
                    self.source_to_timeline_interval(padded_start, padded_end)
                {
                    self.cut_range(t_start, t_end);
                    cuts_count += 1;
                }
            }
        }
        cuts_count
    }

    /// Translates a source interval to the current timeline clock
    pub fn source_to_timeline_interval(&self, src_start: i64, src_end: i64) -> Option<(i64, i64)> {
        if let Some(track) = self.video_tracks.first() {
            for seg in &track.segments {
                if seg.source_in_frame <= src_start && seg.source_out_frame >= src_end {
                    let t_start = seg.timeline_in_frame + (src_start - seg.source_in_frame);
                    let t_end = seg.timeline_in_frame + (src_end - seg.source_in_frame);
                    return Some((t_start, t_end));
                }
            }
        }
        None
    }

    /// Dynamically remaps transcript word timestamps to the new continuous timeline clock (REQ-EDIT-003)
    pub fn remap_words_to_timeline(&self, words: &[WordToken]) -> Vec<WordToken> {
        let mut remapped = Vec::new();
        if let Some(track) = self.video_tracks.first() {
            for word in words {
                for seg in &track.segments {
                    if word.start_frame >= seg.source_in_frame
                        && word.end_frame <= seg.source_out_frame
                    {
                        let offset = seg.timeline_in_frame - seg.source_in_frame;
                        remapped.push(WordToken {
                            text: word.text.clone(),
                            start_frame: word.start_frame + offset,
                            end_frame: word.end_frame + offset,
                            confidence: word.confidence,
                        });
                        break;
                    }
                }
            }
        }
        remapped
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SafeArea {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

impl Default for SafeArea {
    fn default() -> Self {
        // Platform standard for TikTok/Reels 1080x1920
        Self {
            top: 200,
            bottom: 380,
            left: 90,
            right: 130,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WordToken {
    pub text: String,
    pub start_frame: i64,
    pub end_frame: i64,
    #[serde(default)]
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CaptionGroup {
    pub id: String,
    pub timeline_in_frame: i64,
    pub timeline_out_frame: i64,
    pub words: Vec<WordToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CaptionsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_caption_style")]
    pub style_id: String,
    pub safe_area: SafeArea,
    #[serde(default)]
    pub groups: Vec<CaptionGroup>,
}

fn default_true() -> bool {
    true
}

fn default_caption_style() -> String {
    "clean-yellow".to_string()
}

impl Default for CaptionsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            style_id: "clean-yellow".to_string(),
            safe_area: SafeArea::default(),
            groups: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OverlayAnimation {
    pub kind: String, // "slide_fade_in", "fade_out", "pop_in"
    pub duration_frames: i64,
    pub easing: String, // "ease_out_cubic", "ease_in_cubic", "ease_out_back"
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OverlayReason {
    pub word_range: Vec<usize>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct NormalizedPosition {
    pub x: f32,
    pub y: f32,
}

impl Default for NormalizedPosition {
    fn default() -> Self {
        Self { x: 0.5, y: 0.5 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OverlayItem {
    pub id: String,
    pub asset_id: String,
    pub overlay_type: String, // "template", "svg_animation", "video_cutaway", "image"
    pub timeline_in_frame: i64,
    pub timeline_out_frame: i64,
    #[serde(default = "default_anchor")]
    pub anchor: String, // "center", "top_center", "lower_third", "badge"
    #[serde(default)]
    pub position: NormalizedPosition,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub in_animation: Option<OverlayAnimation>,
    #[serde(default)]
    pub out_animation: Option<OverlayAnimation>,
    #[serde(default)]
    pub reason: Option<OverlayReason>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
}

fn default_anchor() -> String {
    "center".to_string()
}
fn default_scale() -> f32 {
    1.0
}
fn default_opacity() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SilenceInterval {
    pub source_in_frame: i64,
    pub source_out_frame: i64,
    pub duration_ms: i64,
    pub status: String, // "detected", "cut", "preserved"
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct Markers {
    #[serde(default)]
    pub silences: Vec<SilenceInterval>,
    #[serde(default)]
    pub scene_cuts: Vec<i64>,
}

/// Project root model representing project.json
/// Implements REQ-CORE-001 (Non-destructive source editing) and REQ-CORE-003 (JSON Schema compliance)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Project {
    pub schema_version: String,
    pub project_id: String,
    pub name: String,
    pub created_at: String,
    pub output: OutputConfig,
    pub sources: Vec<Source>,
    pub timeline: Timeline,
    #[serde(default)]
    pub captions: CaptionsConfig,
    #[serde(default)]
    pub overlays: Vec<OverlayItem>,
    #[serde(default)]
    pub markers: Markers,
}

impl Project {
    pub const CURRENT_SCHEMA_VERSION: &'static str = "1.0.0";

    pub fn new(name: impl Into<String>, primary_source: Source, preset: &str) -> Self {
        let output = match preset {
            "youtube" => OutputConfig::youtube_preset(),
            "fast-draft" => OutputConfig::fast_draft_preset(),
            _ => OutputConfig::tiktok_preset(),
        };

        let initial_segment_v = VideoSegment {
            id: "seg_v_001".to_string(),
            source_id: primary_source.id.clone(),
            source_in_frame: 0,
            source_out_frame: primary_source.duration_frames,
            timeline_in_frame: 0,
            timeline_out_frame: primary_source.duration_frames,
            speed: 1.0,
        };

        let initial_segment_a = AudioSegment {
            id: "seg_a_001".to_string(),
            source_id: primary_source.id.clone(),
            source_in_frame: 0,
            source_out_frame: primary_source.duration_frames,
            timeline_in_frame: 0,
            timeline_out_frame: primary_source.duration_frames,
            volume: 1.0,
        };

        let timeline = Timeline {
            video_tracks: vec![VideoTrack {
                id: "track_v_main".to_string(),
                name: "Primary Video".to_string(),
                segments: vec![initial_segment_v],
            }],
            audio_tracks: vec![AudioTrack {
                id: "track_a_main".to_string(),
                name: "Primary Audio".to_string(),
                segments: vec![initial_segment_a],
            }],
        };

        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION.to_string(),
            project_id: format!("proj_{}", uuid_short()),
            name: name.into(),
            created_at: chrono_timestamp_approx(),
            output,
            sources: vec![primary_source],
            timeline,
            captions: CaptionsConfig::default(),
            overlays: Vec::new(),
            markers: Markers::default(),
        }
    }

    /// Exports the formal Draft 2020-12 / Draft-07 JSON Schema for project.json
    pub fn json_schema() -> serde_json::Value {
        let schema = schemars::schema_for!(Project);
        serde_json::to_value(schema).unwrap_or_default()
    }
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}", now)
}

fn chrono_timestamp_approx() -> String {
    // 2026-11-22 ISO8601 string
    "2026-11-22T17:30:00Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_destructive_timeline_mutation() {
        // REQ-CORE-001: Sources remain untouched, timeline reflects cuts
        let source = Source {
            id: "src_1".to_string(),
            path: "raw.mp4".to_string(),
            duration_frames: 1800,
            width: 1080,
            height: 1920,
            fps: Fps::FPS_30,
            audio_sample_rate: 48000,
        };

        let mut project = Project::new("test", source.clone(), "tiktok");
        assert_eq!(project.sources[0].duration_frames, 1800);
        assert_eq!(project.timeline.total_duration_frames(), 1800);

        // Apply a cut: split first segment into two
        // Segment 1: frames 0..300, Segment 2: frames 600..1800 (frames 300..600 excised)
        project.timeline.video_tracks[0].segments = vec![
            VideoSegment {
                id: "seg_1".to_string(),
                source_id: "src_1".to_string(),
                source_in_frame: 0,
                source_out_frame: 300,
                timeline_in_frame: 0,
                timeline_out_frame: 300,
                speed: 1.0,
            },
            VideoSegment {
                id: "seg_2".to_string(),
                source_id: "src_1".to_string(),
                source_in_frame: 600,
                source_out_frame: 1800,
                timeline_in_frame: 300,
                timeline_out_frame: 1500,
                speed: 1.0,
            },
        ];

        // Total duration is now 1500 frames, but source duration_frames is still 1800
        assert_eq!(project.timeline.total_duration_frames(), 1500);
        assert_eq!(project.sources[0].duration_frames, 1800);
    }

    #[test]
    fn test_timeline_cut_and_trim_remapping() {
        // REQ-EDIT-001: Frame-accurate cuts and trims
        let source = Source {
            id: "src_1".to_string(),
            path: "test.mp4".to_string(),
            duration_frames: 1800,
            width: 1080,
            height: 1920,
            fps: Fps::FPS_30,
            audio_sample_rate: 48000,
        };

        let mut project = Project::new("test", source, "tiktok");
        assert_eq!(project.timeline.total_duration_frames(), 1800);

        // Cut frames 300 to 600 (10 seconds excised)
        project.timeline.cut_range(300, 600);
        assert_eq!(project.timeline.total_duration_frames(), 1500);

        // Verify segment splitting
        let segs = &project.timeline.video_tracks[0].segments;
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].timeline_in_frame, 0);
        assert_eq!(segs[0].timeline_out_frame, 300);
        assert_eq!(segs[1].timeline_in_frame, 300);
        assert_eq!(segs[1].timeline_out_frame, 1500);
        assert_eq!(segs[1].source_in_frame, 600);
        assert_eq!(segs[1].source_out_frame, 1800);
    }

    #[test]
    fn test_transcript_clock_remapping_after_cuts() {
        // REQ-EDIT-003: Remapping word timings to match the new continuous timeline
        let source = Source {
            id: "src_1".to_string(),
            path: "test.mp4".to_string(),
            duration_frames: 1800,
            width: 1080,
            height: 1920,
            fps: Fps::FPS_30,
            audio_sample_rate: 48000,
        };

        let mut project = Project::new("test", source, "tiktok");
        // Apply cut from frame 300 to 600 (excising source frames 300..600)
        project.timeline.cut_range(300, 600);

        let words = vec![
            WordToken {
                text: "Hello".to_string(),
                start_frame: 100,
                end_frame: 150,
                confidence: 0.99,
            },
            WordToken {
                text: "CutWord".to_string(),
                start_frame: 400,
                end_frame: 450,
                confidence: 0.99,
            }, // In cut region
            WordToken {
                text: "World".to_string(),
                start_frame: 900,
                end_frame: 950,
                confidence: 0.99,
            }, // After cut region
        ];

        let remapped = project.timeline.remap_words_to_timeline(&words);
        assert_eq!(remapped.len(), 2);
        // "Hello" is unchanged at [100, 150]
        assert_eq!(remapped[0].text, "Hello");
        assert_eq!(remapped[0].start_frame, 100);
        assert_eq!(remapped[0].end_frame, 150);

        // "World" was at source frame 900. After excising 300 frames, it maps to timeline frame 600!
        assert_eq!(remapped[1].text, "World");
        assert_eq!(remapped[1].start_frame, 600);
        assert_eq!(remapped[1].end_frame, 650);
    }

    #[test]
    fn test_project_json_schema_compliance() {
        // REQ-CORE-003: Valid JSON schema generation
        let schema = Project::json_schema();
        assert!(schema.is_object());
        assert_eq!(schema["title"], "Project");
        assert!(schema["properties"]["schema_version"].is_object());
    }
}
