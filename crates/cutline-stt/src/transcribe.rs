use crate::error::SttError;
use crate::model::ModelManager;
use cutline_core::time::{Fps, FrameTime};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptWord {
    pub text: String,
    pub start_frame: i64,
    pub end_frame: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transcript {
    pub schema_version: String,
    pub language: String,
    pub word_count: usize,
    pub words: Vec<TranscriptWord>,
}

#[derive(Deserialize)]
struct WhisperOutputJson {
    transcription: Option<Vec<WhisperSegmentJson>>,
}

#[derive(Deserialize)]
struct WhisperSegmentJson {
    tokens: Option<Vec<WhisperTokenJson>>,
}

#[derive(Deserialize)]
struct WhisperTokenJson {
    text: String,
    offsets: Option<WhisperOffsetsJson>,
    p: Option<f32>,
}

#[derive(Deserialize)]
struct WhisperOffsetsJson {
    from: i64,
    to: i64,
}

/// Transcribes 16kHz audio using local Whisper engine with word-level timestamps (REQ-STT-001, REQ-STT-003)
pub fn transcribe_audio(
    model_path: &Path,
    wav_path: &Path,
    fps: Fps,
) -> Result<Transcript, SttError> {
    let whisper_cli = ModelManager::ensure_whisper_cli()?;

    let temp_dir = std::env::temp_dir().join("cutline_transcribe");
    fs::create_dir_all(&temp_dir)?;
    let output_stem = temp_dir.join(format!(
        "trans_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    let output_json_file = output_stem.with_extension("json");

    let status = Command::new(&whisper_cli)
        .args([
            "-m",
            &model_path.to_string_lossy(),
            "-f",
            &wav_path.to_string_lossy(),
            "-ojf",
            "-of",
            &output_stem.to_string_lossy(),
            "-np",
            "-l",
            "en",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| SttError::Inference(format!("Failed to execute whisper-cli: {e}")))?;

    if !status.success() {
        return Err(SttError::Inference(format!(
            "whisper-cli exited with status code {:?}",
            status.code()
        )));
    }

    if !output_json_file.exists() {
        return Err(SttError::Inference(
            "whisper-cli completed but did not produce expected output JSON".to_string(),
        ));
    }

    let json_content = fs::read_to_string(&output_json_file)?;
    let parsed: WhisperOutputJson = serde_json::from_str(&json_content).map_err(|e| {
        SttError::Inference(format!("Failed to parse whisper-cli JSON output: {e}"))
    })?;

    let mut words = Vec::new();

    if let Some(segments) = parsed.transcription {
        for seg in segments {
            if let Some(tokens) = seg.tokens {
                for tok in tokens {
                    let clean = tok.text.trim();
                    // Ignore empty or Whisper internal timestamp/control tokens
                    if clean.is_empty()
                        || clean.starts_with("[_")
                        || clean.starts_with("<|")
                        || (clean.starts_with('(') && clean.ends_with(')'))
                    {
                        continue;
                    }

                    if let Some(offsets) = tok.offsets {
                        let start_ms = offsets.from;
                        let end_ms = offsets.to.max(start_ms + 100);

                        let start_frame = FrameTime::from_millis(start_ms, fps)
                            .map(|t| t.frame)
                            .unwrap_or(0);
                        let end_frame = FrameTime::from_millis(end_ms, fps)
                            .map(|t| t.frame)
                            .unwrap_or(start_frame + 1);

                        words.push(TranscriptWord {
                            text: clean.to_string(),
                            start_frame,
                            end_frame,
                            start_ms,
                            end_ms,
                            confidence: tok.p.unwrap_or(0.95),
                        });
                    }
                }
            }
        }
    }

    // Clean up temporary json file
    let _ = fs::remove_file(&output_json_file);

    Ok(Transcript {
        schema_version: "1.0.0".to_string(),
        language: "en".to_string(),
        word_count: words.len(),
        words,
    })
}
