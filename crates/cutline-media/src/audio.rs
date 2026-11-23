use crate::error::MediaError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedAudioInfo {
    pub audio_path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: f64,
}

/// Extracts the audio stream into a 16kHz mono 16-bit PCM WAV file
/// Optimized for Whisper speech-to-text models.
/// Implements REQ-MEDIA-003.
pub fn extract_audio_16k_mono(
    input_path: &Path,
    output_path: &Path,
) -> Result<ExtractedAudioInfo, MediaError> {
    let in_str = input_path.to_string_lossy().to_string();
    let out_str = output_path.to_string_lossy().to_string();

    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &in_str,
            "-vn",
            "-acodec",
            "pcm_s16le",
            "-ar",
            "16000",
            "-ac",
            "1",
            &out_str,
        ])
        .output()
        .map_err(|e| MediaError::AudioExtractionFailed {
            path: in_str.clone(),
            details: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(MediaError::AudioExtractionFailed {
            path: in_str,
            details: stderr,
        });
    }

    // Probe the resulting WAV file to confirm exact properties
    let probe = crate::probe::probe_media(output_path)?;

    Ok(ExtractedAudioInfo {
        audio_path: out_str,
        sample_rate: 16000,
        channels: 1,
        duration_seconds: probe.duration_seconds,
    })
}
