use crate::error::AppError;
use cutline_analysis::{detect_scene_changes, detect_silence_in_wav, SilenceDetectorConfig};
use cutline_core::project::{Project, SilenceInterval, Source};
use cutline_core::time::FrameTime;
use cutline_media::{extract_audio_16k_mono, probe_media};
use cutline_render::{RenderOptions, RenderPipeline, SingleFrameInfo};
use cutline_stt::{transcribe_audio, ModelManager};
use std::fs;
use std::path::{Path, PathBuf};

pub fn op_probe(video_path: &Path) -> Result<serde_json::Value, AppError> {
    let probe = probe_media(video_path)
        .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;
    serde_json::to_value(probe).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize probe data.",
        )
    })
}

#[derive(serde::Serialize)]
struct ProjectInitData {
    project_id: String,
    project_path: String,
    name: String,
    dimensions: Dimensions,
    fps: cutline_core::time::Fps,
    duration_frames: i64,
}

#[derive(serde::Serialize)]
struct Dimensions {
    width: u32,
    height: u32,
}

pub fn op_project_init(
    video: &Path,
    b_roll: &[PathBuf],
    preset: &str,
    out: &Path,
) -> Result<serde_json::Value, AppError> {
    let probe =
        probe_media(video).map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;

    if probe.video_codec.is_none() || probe.width == 0 {
        return Err(AppError::new(
            "ERR_NO_VIDEO_STREAM",
            "The specified file contains no video track",
            "Please provide a video file (MP4, MOV, MKV) with a valid video stream.",
        ));
    }

    let primary_source = Source {
        id: "src_primary".to_string(),
        path: video.to_string_lossy().to_string(),
        duration_frames: probe.duration_frames,
        width: probe.width,
        height: probe.height,
        fps: probe.fps,
        audio_sample_rate: probe.sample_rate.unwrap_or(48000),
    };

    let project_name = video
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let mut project = Project::new(project_name, primary_source, preset);

    for (idx, b_path) in b_roll.iter().enumerate() {
        if let Ok(b_probe) = probe_media(b_path) {
            project.sources.push(Source {
                id: format!("src_broll_{}", idx + 1),
                path: b_path.to_string_lossy().to_string(),
                duration_frames: b_probe.duration_frames,
                width: b_probe.width,
                height: b_probe.height,
                fps: b_probe.fps,
                audio_sample_rate: b_probe.sample_rate.unwrap_or(48000),
            });
        }
    }

    let json_data = serde_json::to_string_pretty(&project).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize project model to JSON.",
        )
    })?;

    fs::write(out, json_data).map_err(|e| {
        AppError::new(
            "ERR_WRITE_FILE",
            e.to_string(),
            "Ensure the destination directory exists and is writable.",
        )
    })?;

    let init_data = ProjectInitData {
        project_id: project.project_id,
        project_path: out.to_string_lossy().to_string(),
        name: project.name,
        dimensions: Dimensions {
            width: project.output.width,
            height: project.output.height,
        },
        fps: project.output.fps,
        duration_frames: project.timeline.total_duration_frames(),
    };

    serde_json::to_value(init_data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize response.",
        )
    })
}

pub fn op_audio_extract(
    input: &Path,
    explicit_out: Option<&Path>,
) -> Result<serde_json::Value, AppError> {
    let video_path = if input.extension().map(|ext| ext == "json").unwrap_or(false) {
        let content = fs::read_to_string(input).map_err(|e| {
            AppError::new(
                "ERR_READ_FILE",
                e.to_string(),
                "Verify the project file exists and is accessible.",
            )
        })?;
        let proj = serde_json::from_str::<Project>(&content).map_err(|e| {
            AppError::new(
                "ERR_INVALID_PROJECT",
                e.to_string(),
                "Check that the project.json file is valid.",
            )
        })?;
        if let Some(src) = proj.sources.first() {
            PathBuf::from(&src.path)
        } else {
            return Err(AppError::new(
                "ERR_NO_SOURCE",
                "Project file contains no media sources",
                "Initialize the project with a valid video source using 'cutline project init'.",
            ));
        }
    } else {
        input.to_path_buf()
    };

    let target_out = if let Some(out) = explicit_out {
        out.to_path_buf()
    } else {
        video_path.with_extension("wav")
    };

    let info = extract_audio_16k_mono(&video_path, &target_out)
        .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;

    serde_json::to_value(info).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize audio info.",
        )
    })
}

#[derive(serde::Serialize)]
struct TranscribeData {
    transcript_path: String,
    language: String,
    word_count: usize,
    words: Vec<cutline_stt::TranscriptWord>,
}

pub fn op_transcribe(
    input_path: &Path,
    model_alias: &str,
    explicit_out: Option<&Path>,
) -> Result<serde_json::Value, AppError> {
    let (wav_path, fps) = if input_path.extension().map(|e| e == "json").unwrap_or(false) {
        let content = fs::read_to_string(input_path).map_err(|e| {
            AppError::new("ERR_READ_FILE", e.to_string(), "Check project.json path.")
        })?;
        let project: Project = serde_json::from_str(&content).map_err(|e| {
            AppError::new(
                "ERR_INVALID_PROJECT",
                e.to_string(),
                "Ensure project.json is valid.",
            )
        })?;

        let primary_video = match project.sources.first() {
            Some(s) => PathBuf::from(&s.path),
            None => {
                return Err(AppError::new(
                    "ERR_NO_SOURCE",
                    "No source video in project.",
                    "Run 'cutline project init'.",
                ));
            }
        };

        let wav = primary_video.with_extension("wav");
        if !wav.exists() {
            extract_audio_16k_mono(&primary_video, &wav)
                .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;
        }
        (wav, project.output.fps)
    } else {
        let wav = if input_path.extension().map(|e| e == "wav").unwrap_or(false) {
            input_path.to_path_buf()
        } else {
            let wav = input_path.with_extension("wav");
            if !wav.exists() {
                extract_audio_16k_mono(input_path, &wav)
                    .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;
            }
            wav
        };
        (wav, cutline_core::time::Fps::FPS_30)
    };

    let model_path = ModelManager::ensure_model(model_alias)
        .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;

    let transcript = transcribe_audio(&model_path, &wav_path, fps)
        .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;

    let out_path = explicit_out
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| input_path.with_file_name("transcript.json"));

    if let Ok(json) = serde_json::to_string_pretty(&transcript) {
        let _ = fs::write(&out_path, json);
    }

    let data = TranscribeData {
        transcript_path: out_path.to_string_lossy().to_string(),
        language: transcript.language,
        word_count: transcript.word_count,
        words: transcript.words,
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize transcript.",
        )
    })
}

pub fn op_analyze_silence(
    project_path: &Path,
    threshold_db: f32,
    min_duration_ms: i64,
) -> Result<serde_json::Value, AppError> {
    let (wav_path, fps, is_project) = if project_path
        .extension()
        .map(|e| e == "json")
        .unwrap_or(false)
    {
        let content = fs::read_to_string(project_path).map_err(|e| {
            AppError::new("ERR_READ_FILE", e.to_string(), "Check project.json path.")
        })?;
        let project: Project = serde_json::from_str(&content).map_err(|e| {
            AppError::new(
                "ERR_INVALID_PROJECT",
                e.to_string(),
                "Ensure project.json is valid.",
            )
        })?;

        let primary_video = match project.sources.first() {
            Some(s) => PathBuf::from(&s.path),
            None => {
                return Err(AppError::new(
                    "ERR_NO_SOURCE",
                    "No source in project.",
                    "Run 'cutline project init'.",
                ));
            }
        };

        let wav = primary_video.with_extension("wav");
        if !wav.exists() {
            extract_audio_16k_mono(&primary_video, &wav)
                .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;
        }
        (wav, project.output.fps, true)
    } else {
        (
            project_path.to_path_buf(),
            cutline_core::time::Fps::FPS_30,
            false,
        )
    };

    let config = SilenceDetectorConfig {
        activation_db: threshold_db,
        deactivation_db: threshold_db - 6.0,
        min_duration_ms,
        window_size_ms: 20,
    };

    let result = detect_silence_in_wav(&wav_path, fps, &config)
        .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;

    if is_project {
        if let Ok(content) = fs::read_to_string(project_path) {
            if let Ok(mut proj) = serde_json::from_str::<Project>(&content) {
                proj.markers.silences = result
                    .silences
                    .iter()
                    .map(|s| SilenceInterval {
                        source_in_frame: s.start_frame,
                        source_out_frame: s.end_frame,
                        duration_ms: s.duration_ms,
                        status: "detected".to_string(),
                    })
                    .collect();
                if let Ok(updated_json) = serde_json::to_string_pretty(&proj) {
                    let _ = fs::write(project_path, updated_json);
                }
            }
        }
    }

    serde_json::to_value(result).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize silence data.",
        )
    })
}

pub fn op_analyze_scenes(
    project_path: &Path,
    threshold: f32,
) -> Result<serde_json::Value, AppError> {
    let (video_path, is_project) = if project_path
        .extension()
        .map(|e| e == "json")
        .unwrap_or(false)
    {
        let content = fs::read_to_string(project_path).map_err(|e| {
            AppError::new("ERR_READ_FILE", e.to_string(), "Check project.json path.")
        })?;
        let project: Project = serde_json::from_str(&content).map_err(|e| {
            AppError::new(
                "ERR_INVALID_PROJECT",
                e.to_string(),
                "Ensure project.json is valid.",
            )
        })?;

        let primary_video = match project.sources.first() {
            Some(s) => PathBuf::from(&s.path),
            None => {
                return Err(AppError::new(
                    "ERR_NO_SOURCE",
                    "No source in project.",
                    "Run 'cutline project init'.",
                ));
            }
        };
        (primary_video, true)
    } else {
        (project_path.to_path_buf(), false)
    };

    let result = detect_scene_changes(&video_path, threshold)
        .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;

    if is_project {
        if let Ok(content) = fs::read_to_string(project_path) {
            if let Ok(mut proj) = serde_json::from_str::<Project>(&content) {
                proj.markers.scene_cuts = result.cut_frames.clone();
                if let Ok(updated_json) = serde_json::to_string_pretty(&proj) {
                    let _ = fs::write(project_path, updated_json);
                }
            }
        }
    }

    serde_json::to_value(result).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize scene cut data.",
        )
    })
}

#[derive(serde::Serialize)]
struct CutData {
    cut_start_frame: i64,
    cut_end_frame: i64,
    new_timeline_frames: i64,
    new_duration_seconds: f64,
}

pub fn op_timeline_cut(
    project_path: &Path,
    start_str: &str,
    end_str: &str,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path).map_err(|e| {
        AppError::new(
            "ERR_READ_FILE",
            e.to_string(),
            "Verify project path exists.",
        )
    })?;
    let mut project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Ensure project.json is valid.",
        )
    })?;

    let fps = project.output.fps;
    let t_start = FrameTime::parse(start_str, fps).map_err(|e| {
        AppError::new(
            "ERR_INVALID_TIME",
            e.to_string(),
            "Provide start time as HH:MM:SS.mmm, seconds, or frame:N.",
        )
    })?;
    let t_end = FrameTime::parse(end_str, fps).map_err(|e| {
        AppError::new(
            "ERR_INVALID_TIME",
            e.to_string(),
            "Provide end time as HH:MM:SS.mmm, seconds, or frame:N.",
        )
    })?;

    if t_start.frame >= t_end.frame {
        return Err(AppError::new(
            "ERR_INVALID_FRAME_RANGE",
            "Start time must precede end time.",
            "Verify that start < end.",
        ));
    }

    project.timeline.cut_range(t_start.frame, t_end.frame);

    if let Ok(json) = serde_json::to_string_pretty(&project) {
        let _ = fs::write(project_path, json);
    }

    let new_frames = project.timeline.total_duration_frames();
    let data = CutData {
        cut_start_frame: t_start.frame,
        cut_end_frame: t_end.frame,
        new_timeline_frames: new_frames,
        new_duration_seconds: new_frames as f64 / fps.as_f64(),
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize cut data.",
        )
    })
}

#[derive(serde::Serialize)]
struct AutoCutData {
    cuts_applied: usize,
    new_timeline_frames: i64,
    new_duration_seconds: f64,
}

pub fn op_timeline_auto_cut_silence(
    project_path: &Path,
    padding_ms: i64,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path)
        .map_err(|e| AppError::new("ERR_READ_FILE", e.to_string(), "Check project path."))?;
    let mut project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Invalid project file.",
        )
    })?;

    if project.markers.silences.is_empty() {
        let primary_video = match project.sources.first() {
            Some(s) => PathBuf::from(&s.path),
            None => {
                return Err(AppError::new(
                    "ERR_NO_SOURCE",
                    "No source in project.",
                    "Initialize with 'cutline project init'.",
                ));
            }
        };
        let wav = primary_video.with_extension("wav");
        if !wav.exists() {
            let _ = extract_audio_16k_mono(&primary_video, &wav);
        }
        let config = SilenceDetectorConfig::default();
        if let Ok(res) = detect_silence_in_wav(&wav, project.output.fps, &config) {
            project.markers.silences = res
                .silences
                .iter()
                .map(|s| SilenceInterval {
                    source_in_frame: s.start_frame,
                    source_out_frame: s.end_frame,
                    duration_ms: s.duration_ms,
                    status: "detected".to_string(),
                })
                .collect();
        }
    }

    let fps = project.output.fps;
    let padding_frames =
        ((padding_ms as f64 * fps.num as f64) / (1000.0 * fps.den as f64)).round() as i64;
    let silences = project.markers.silences.clone();
    let cuts_applied = project.timeline.auto_cut_silence(&silences, padding_frames);

    if let Ok(json) = serde_json::to_string_pretty(&project) {
        let _ = fs::write(project_path, json);
    }

    let new_frames = project.timeline.total_duration_frames();
    let data = AutoCutData {
        cuts_applied,
        new_timeline_frames: new_frames,
        new_duration_seconds: new_frames as f64 / fps.as_f64(),
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize auto-cut data.",
        )
    })
}

#[derive(serde::Serialize)]
struct TrimData {
    new_timeline_frames: i64,
    new_duration_seconds: f64,
}

pub fn op_timeline_trim(
    project_path: &Path,
    trim_in_str: &str,
    trim_out_str: &str,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path)
        .map_err(|e| AppError::new("ERR_READ_FILE", e.to_string(), "Check project path."))?;
    let mut project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Invalid project file.",
        )
    })?;

    let fps = project.output.fps;
    let t_in = FrameTime::parse(trim_in_str, fps)
        .map_err(|e| AppError::new("ERR_INVALID_TIME", e.to_string(), "Invalid in timestamp."))?;
    let t_out = FrameTime::parse(trim_out_str, fps)
        .map_err(|e| AppError::new("ERR_INVALID_TIME", e.to_string(), "Invalid out timestamp."))?;

    project.timeline.trim(t_in.frame, t_out.frame);

    if let Ok(json) = serde_json::to_string_pretty(&project) {
        let _ = fs::write(project_path, json);
    }

    let new_frames = project.timeline.total_duration_frames();
    let data = TrimData {
        new_timeline_frames: new_frames,
        new_duration_seconds: new_frames as f64 / fps.as_f64(),
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize trim data.",
        )
    })
}

#[derive(serde::Serialize)]
struct CaptionsApplyData {
    project_file: String,
    style: String,
    groups_count: usize,
    words_captioned: usize,
    first_caption_in_frame: Option<i64>,
    last_caption_out_frame: Option<i64>,
}

pub fn op_captions_apply(
    project_path: &Path,
    style_arg: &str,
    transcript_arg: Option<&Path>,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path).map_err(|e| {
        AppError::new(
            "ERR_READ_FILE",
            e.to_string(),
            "Verify project path exists and is accessible.",
        )
    })?;

    let mut project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Ensure project.json is valid.",
        )
    })?;

    let transcript_path = if let Some(t_path) = transcript_arg {
        t_path.to_path_buf()
    } else {
        let candidate = project_path.with_file_name("transcript.json");
        if candidate.exists() {
            candidate
        } else {
            let parent_candidate = project_path
                .parent()
                .unwrap_or(Path::new("."))
                .join("transcript.json");
            if parent_candidate.exists() {
                parent_candidate
            } else {
                return Err(AppError::new(
                    "ERR_MISSING_TRANSCRIPT",
                    "No transcript.json found for project",
                    "Run 'cutline transcribe <project>' first to generate word timestamps, or provide '--transcript <path>'.",
                ));
            }
        }
    };

    let trans_content = fs::read_to_string(&transcript_path).map_err(|e| {
        AppError::new(
            "ERR_READ_TRANSCRIPT",
            e.to_string(),
            "Verify transcript file exists and is accessible.",
        )
    })?;

    #[derive(serde::Deserialize)]
    struct TranscriptWrapper {
        #[serde(default)]
        words: Vec<cutline_core::project::WordToken>,
    }

    let raw_words: Vec<cutline_core::project::WordToken> =
        if let Ok(wrapper) = serde_json::from_str::<TranscriptWrapper>(&trans_content) {
            if !wrapper.words.is_empty() {
                wrapper.words
            } else {
                serde_json::from_str::<Vec<cutline_core::project::WordToken>>(&trans_content)
                    .unwrap_or_default()
            }
        } else if let Ok(direct_words) =
            serde_json::from_str::<Vec<cutline_core::project::WordToken>>(&trans_content)
        {
            direct_words
        } else {
            return Err(AppError::new(
            "ERR_INVALID_TRANSCRIPT",
            "Could not parse word timestamps from transcript JSON",
            "Ensure transcript.json contains a valid 'words' array with start_frame and end_frame.",
        ));
        };

    if raw_words.is_empty() {
        return Err(AppError::new(
            "ERR_EMPTY_TRANSCRIPT",
            "Transcript contains no word tokens",
            "Ensure the audio contains recognizable speech.",
        ));
    }

    let style = if style_arg.starts_with('{') {
        serde_json::from_str::<cutline_render::CaptionStyle>(style_arg).map_err(|e| {
            AppError::new(
                "ERR_INVALID_STYLE",
                format!("Failed to parse custom JSON style: {e}"),
                "Provide a valid JSON style definition or preset name (clean-yellow, mono-warm).",
            )
        })?
    } else {
        cutline_render::CaptionStyle::from_id(style_arg)
    };

    let remapped_words = project.timeline.remap_words_to_timeline(&raw_words);
    let groups =
        cutline_render::group_words_into_captions(&remapped_words, project.output.fps, &style);

    project.captions.enabled = true;
    project.captions.style_id = style.name.clone();
    project.captions.groups = groups.clone();

    if let Ok(updated_json) = serde_json::to_string_pretty(&project) {
        let _ = fs::write(project_path, updated_json);
    }

    let first_in = groups.first().map(|g| g.timeline_in_frame);
    let last_out = groups.last().map(|g| g.timeline_out_frame);

    let data = CaptionsApplyData {
        project_file: project_path.to_string_lossy().to_string(),
        style: style.name,
        groups_count: groups.len(),
        words_captioned: remapped_words.len(),
        first_caption_in_frame: first_in,
        last_caption_out_frame: last_out,
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize captions data.",
        )
    })
}

pub fn op_assets_list() -> Result<serde_json::Value, AppError> {
    let registry = cutline_assets::AssetRegistry::new();
    serde_json::to_value(registry.list()).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize asset list.",
        )
    })
}

#[derive(serde::Serialize)]
struct AssetAddData {
    asset_id: String,
    kind: String,
    path: String,
}

pub fn op_assets_add(
    path: &Path,
    explicit_id: Option<&str>,
) -> Result<serde_json::Value, AppError> {
    if !path.exists() {
        return Err(AppError::new(
            "ERR_FILE_NOT_FOUND",
            "Asset file does not exist",
            "Verify path to SVG or image file.",
        ));
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let kind = match ext.as_str() {
        "svg" => "svg_animation",
        "png" | "jpg" | "jpeg" => "image",
        "mp4" | "mov" => "video_cutaway",
        _ => "unknown",
    };
    let asset_id = explicit_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}:{}", kind, path.file_stem().unwrap().to_string_lossy()));

    let data = AssetAddData {
        asset_id,
        kind: kind.to_string(),
        path: path.to_string_lossy().to_string(),
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize asset add response.",
        )
    })
}

#[derive(serde::Serialize)]
struct OverlayAddData {
    overlay_id: String,
    asset_id: String,
    timeline_in_frame: i64,
    timeline_out_frame: i64,
    duration_frames: i64,
    anchor: String,
    total_overlays: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn op_overlay_add(
    project_path: &Path,
    asset_id: &str,
    at_str: &str,
    dur_str: &str,
    anchor: &str,
    scale: f32,
    opacity: f32,
    title: Option<&str>,
    subtitle: Option<&str>,
    reason_text: Option<&str>,
    word_range: Option<&[usize]>,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path).map_err(|e| {
        AppError::new(
            "ERR_READ_FILE",
            e.to_string(),
            "Verify project path exists.",
        )
    })?;
    let mut project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Ensure project.json is valid.",
        )
    })?;

    let fps = project.output.fps;
    let t_start = FrameTime::parse(at_str, fps).map_err(|e| {
        AppError::new(
            "ERR_INVALID_TIME",
            e.to_string(),
            "Provide start time as HH:MM:SS.mmm, seconds, or frame:N.",
        )
    })?;
    let t_dur = FrameTime::parse(dur_str, fps).map_err(|e| {
        AppError::new(
            "ERR_INVALID_TIME",
            e.to_string(),
            "Provide duration as HH:MM:SS.mmm, seconds, or frame:N.",
        )
    })?;

    let in_frame = t_start.frame;
    let out_frame = t_start.frame + t_dur.frame.max(1);

    let (norm_x, norm_y) = match anchor {
        "top_center" => (0.5, 0.22),
        "lower_third" => (0.1, 0.72),
        "bottom_center" => (0.5, 0.75),
        _ => (0.5, 0.5),
    };

    let overlay_type = if asset_id.contains("svg") || asset_id.contains("bird") {
        "svg_animation"
    } else if asset_id.contains("cutaway") || asset_id.contains("b-roll") {
        "video_cutaway"
    } else {
        "template"
    };

    let overlay_id = format!(
        "ov_{}_{}",
        asset_id.replace(':', "_"),
        project.overlays.len() + 1
    );

    let reason = reason_text.map(|txt| cutline_core::project::OverlayReason {
        text: txt.to_string(),
        word_range: word_range.unwrap_or_default().to_vec(),
    });

    let item = cutline_core::project::OverlayItem {
        id: overlay_id.clone(),
        asset_id: asset_id.to_string(),
        overlay_type: overlay_type.to_string(),
        timeline_in_frame: in_frame,
        timeline_out_frame: out_frame,
        anchor: anchor.to_string(),
        position: cutline_core::project::NormalizedPosition {
            x: norm_x,
            y: norm_y,
        },
        scale,
        opacity,
        in_animation: Some(cutline_core::project::OverlayAnimation {
            kind: "slide_fade_in".to_string(),
            duration_frames: 10,
            easing: "ease_out_cubic".to_string(),
        }),
        out_animation: Some(cutline_core::project::OverlayAnimation {
            kind: "fade_out".to_string(),
            duration_frames: 10,
            easing: "ease_out_cubic".to_string(),
        }),
        reason,
        title: title.map(|s| s.to_string()),
        subtitle: subtitle.map(|s| s.to_string()),
    };

    project.overlays.push(item);

    if let Ok(json) = serde_json::to_string_pretty(&project) {
        let _ = fs::write(project_path, json);
    }

    let data = OverlayAddData {
        overlay_id,
        asset_id: asset_id.to_string(),
        timeline_in_frame: in_frame,
        timeline_out_frame: out_frame,
        duration_frames: out_frame - in_frame,
        anchor: anchor.to_string(),
        total_overlays: project.overlays.len(),
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize overlay data.",
        )
    })
}

pub fn op_overlay_list(project_path: &Path) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path).map_err(|e| {
        AppError::new(
            "ERR_READ_FILE",
            e.to_string(),
            "Verify project path exists.",
        )
    })?;
    let project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Ensure project.json is valid.",
        )
    })?;

    serde_json::to_value(&project.overlays).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize overlays.",
        )
    })
}

#[derive(serde::Serialize)]
struct RemoveData {
    removed_id: String,
    remaining_count: usize,
}

pub fn op_overlay_remove(project_path: &Path, id: &str) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path).map_err(|e| {
        AppError::new(
            "ERR_READ_FILE",
            e.to_string(),
            "Verify project path exists.",
        )
    })?;
    let mut project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Ensure project.json is valid.",
        )
    })?;

    let prev_len = project.overlays.len();
    project.overlays.retain(|o| o.id != id);
    if project.overlays.len() == prev_len {
        return Err(AppError::new(
            "ERR_NOT_FOUND",
            format!("No overlay found with ID '{id}'"),
            "Use 'cutline overlay list' to view active overlay IDs.",
        ));
    }
    if let Ok(json) = serde_json::to_string_pretty(&project) {
        let _ = fs::write(project_path, json);
    }

    let data = RemoveData {
        removed_id: id.to_string(),
        remaining_count: project.overlays.len(),
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize remove response.",
        )
    })
}

pub fn op_preview_frame(
    project_path: &Path,
    time_str: &str,
    out_path: &Path,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path).map_err(|e| {
        AppError::new(
            "ERR_READ_FILE",
            e.to_string(),
            "Verify project path exists.",
        )
    })?;
    let project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Ensure project.json is valid.",
        )
    })?;

    let fps = project.output.fps;
    let t = FrameTime::parse(time_str, fps).map_err(|e| {
        AppError::new(
            "ERR_INVALID_TIME",
            e.to_string(),
            "Provide timestamp as HH:MM:SS.mmm, seconds, or frame:N.",
        )
    })?;

    let info = RenderPipeline::render_preview_frame(&project, t.frame, out_path)
        .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;

    serde_json::to_value(info).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize preview frame info.",
        )
    })
}

#[derive(serde::Serialize)]
struct StripData {
    count: usize,
    out_dir: String,
    frames: Vec<SingleFrameInfo>,
}

pub fn op_preview_strip(
    project_path: &Path,
    count: usize,
    out_dir: &Path,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path).map_err(|e| {
        AppError::new(
            "ERR_READ_FILE",
            e.to_string(),
            "Verify project path exists.",
        )
    })?;
    let project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Ensure project.json is valid.",
        )
    })?;

    let total_frames = project.timeline.total_duration_frames();
    if total_frames <= 0 {
        return Err(AppError::new(
            "ERR_EMPTY_TIMELINE",
            "Timeline contains no frames to preview",
            "Initialize project with video media first.",
        ));
    }

    fs::create_dir_all(out_dir).map_err(|e| {
        AppError::new(
            "ERR_CREATE_DIR",
            e.to_string(),
            "Verify destination directory permissions.",
        )
    })?;

    let n = count.max(1);
    let mut frames_info = Vec::new();

    for i in 0..n {
        let frame = if n == 1 {
            0
        } else {
            (i as i64 * (total_frames - 1)) / (n as i64 - 1)
        };
        let out_file = out_dir.join(format!("strip_{:02}.png", i + 1));

        let info = RenderPipeline::render_preview_frame(&project, frame, &out_file)
            .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;
        frames_info.push(info);
    }

    let data = StripData {
        count: frames_info.len(),
        out_dir: out_dir.to_string_lossy().to_string(),
        frames: frames_info,
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize strip data.",
        )
    })
}

#[derive(serde::Serialize)]
pub struct LintData {
    pub passed: bool,
    pub violations_count: usize,
    pub warnings_count: usize,
    pub details: Vec<cutline_lint::LintIssue>,
}

pub fn op_lint(
    project_path: &Path,
    explicit_transcript: Option<&Path>,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path).map_err(|e| {
        AppError::new(
            "ERR_READ_FILE",
            e.to_string(),
            "Verify project path exists.",
        )
    })?;
    let project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Ensure project.json is valid.",
        )
    })?;

    let transcript_path = if let Some(p) = explicit_transcript {
        Some(p.to_path_buf())
    } else {
        let candidate = project_path.with_file_name("transcript.json");
        if candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    };

    #[derive(serde::Deserialize)]
    struct TranscriptWrapper {
        #[serde(default)]
        words: Vec<cutline_core::project::WordToken>,
    }

    let words: Option<Vec<cutline_core::project::WordToken>> =
        if let Some(ref t_path) = transcript_path {
            if let Ok(c) = fs::read_to_string(t_path) {
                if let Ok(wrapper) = serde_json::from_str::<TranscriptWrapper>(&c) {
                    Some(wrapper.words)
                } else {
                    serde_json::from_str::<Vec<cutline_core::project::WordToken>>(&c).ok()
                }
            } else {
                None
            }
        } else {
            None
        };

    let report = cutline_lint::lint_project(&project, words.as_deref());

    let data = LintData {
        passed: report.valid,
        violations_count: report.error_count,
        warnings_count: report.warning_count,
        details: report.issues,
    };

    serde_json::to_value(data).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize lint data.",
        )
    })
}

pub fn op_render(
    project_path: &Path,
    output_path: &Path,
    preset: Option<&str>,
    hw_accel: Option<&str>,
) -> Result<serde_json::Value, AppError> {
    let content = fs::read_to_string(project_path)
        .map_err(|e| AppError::new("ERR_READ_FILE", e.to_string(), "Check project path."))?;
    let mut project: Project = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            "ERR_INVALID_PROJECT",
            e.to_string(),
            "Invalid project file.",
        )
    })?;

    if let Some(p) = preset {
        match p {
            "youtube" => project.output = cutline_core::project::OutputConfig::youtube_preset(),
            "fast-draft" => {
                project.output = cutline_core::project::OutputConfig::fast_draft_preset()
            }
            _ => project.output = cutline_core::project::OutputConfig::tiktok_preset(),
        }
    }

    let options = RenderOptions {
        preset: preset.map(|s| s.to_string()),
        hw_accel: hw_accel.map(|s| s.to_string()),
    };

    let result = RenderPipeline::render(&project, output_path, &options)
        .map_err(|e| AppError::new(e.error_code(), e.to_string(), e.hint()))?;

    serde_json::to_value(result).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize render result.",
        )
    })
}

pub fn op_schema() -> Result<serde_json::Value, AppError> {
    let schema = Project::json_schema();
    serde_json::to_value(schema).map_err(|e| {
        AppError::new(
            "ERR_SERIALIZE",
            e.to_string(),
            "Failed to serialize JSON schema.",
        )
    })
}
