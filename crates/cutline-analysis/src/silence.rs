use crate::error::AnalysisError;
use cutline_core::time::{Fps, FrameTime};
use hound::WavReader;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilenceDetectorConfig {
    /// Activation threshold in dB (below this enters silence). Default: -32.0 dB
    pub activation_db: f32,
    /// Deactivation threshold in dB (above this exits silence). Default: -38.0 dB
    pub deactivation_db: f32,
    /// Minimum contiguous duration in ms to be classified as silence. Default: 300 ms
    pub min_duration_ms: i64,
    /// Analysis window size in ms. Default: 20 ms
    pub window_size_ms: u32,
}

impl Default for SilenceDetectorConfig {
    fn default() -> Self {
        Self {
            activation_db: -32.0,
            deactivation_db: -38.0,
            min_duration_ms: 300,
            window_size_ms: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedSilence {
    pub start_frame: i64,
    pub end_frame: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilenceAnalysisResult {
    pub silences_found: usize,
    pub total_silence_seconds: f64,
    pub silences: Vec<DetectedSilence>,
}

/// Analyzes audio using RMS energy with dual-threshold hysteresis to locate dead air (REQ-ANALYSIS-001)
pub fn detect_silence_in_wav(
    wav_path: &Path,
    fps: Fps,
    config: &SilenceDetectorConfig,
) -> Result<SilenceAnalysisResult, AnalysisError> {
    let mut reader = WavReader::open(wav_path)
        .map_err(|e| AnalysisError::WavRead(format!("Could not open WAV {:?}: {e}", wav_path)))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f32;
    let window_samples = ((sample_rate * config.window_size_ms as f32) / 1000.0).round() as usize;
    if window_samples == 0 {
        return Err(AnalysisError::InvalidParameter(
            "Window size cannot be zero".into(),
        ));
    }

    let samples: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    let raw_samples = samples.map_err(|e| AnalysisError::WavRead(e.to_string()))?;

    // Downmix or normalize samples to f32
    let float_samples: Vec<f32> = if spec.channels == 1 {
        raw_samples
            .into_iter()
            .map(|s| s as f32 / 32768.0)
            .collect()
    } else {
        let channels = spec.channels as usize;
        raw_samples
            .chunks(channels)
            .map(|chunk| {
                let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
                (sum / channels as i32) as f32 / 32768.0
            })
            .collect()
    };

    let total_samples = float_samples.len();
    let mut silences = Vec::new();
    let mut in_silence = false;
    let mut silence_start_sample: Option<usize> = None;

    let mut idx = 0;
    while idx + window_samples <= total_samples {
        let window = &float_samples[idx..idx + window_samples];
        let sum_sq: f32 = window.iter().map(|&x| x * x).sum();
        let rms = (sum_sq / window_samples as f32).sqrt();
        let db = 20.0 * (rms.max(1e-7)).log10();

        if !in_silence {
            if db < config.activation_db {
                in_silence = true;
                silence_start_sample = Some(idx);
            }
        } else if db > config.deactivation_db {
            in_silence = false;
            if let Some(start_s) = silence_start_sample {
                let duration_samples = idx - start_s;
                let duration_ms = ((duration_samples as f32 / sample_rate) * 1000.0).round() as i64;
                if duration_ms >= config.min_duration_ms {
                    let start_ms = ((start_s as f32 / sample_rate) * 1000.0).round() as i64;
                    let end_ms = start_ms + duration_ms;
                    let start_frame = FrameTime::from_millis(start_ms, fps)
                        .map(|t| t.frame)
                        .unwrap_or(0);
                    let end_frame = FrameTime::from_millis(end_ms, fps)
                        .map(|t| t.frame)
                        .unwrap_or(start_frame);

                    silences.push(DetectedSilence {
                        start_frame,
                        end_frame,
                        start_ms,
                        end_ms,
                        duration_ms,
                    });
                }
            }
            silence_start_sample = None;
        }

        idx += window_samples;
    }

    // Handle trailing silence at the end of the file
    if in_silence {
        if let Some(start_s) = silence_start_sample {
            let duration_samples = total_samples - start_s;
            let duration_ms = ((duration_samples as f32 / sample_rate) * 1000.0).round() as i64;
            if duration_ms >= config.min_duration_ms {
                let start_ms = ((start_s as f32 / sample_rate) * 1000.0).round() as i64;
                let end_ms = start_ms + duration_ms;
                let start_frame = FrameTime::from_millis(start_ms, fps)
                    .map(|t| t.frame)
                    .unwrap_or(0);
                let end_frame = FrameTime::from_millis(end_ms, fps)
                    .map(|t| t.frame)
                    .unwrap_or(start_frame);

                silences.push(DetectedSilence {
                    start_frame,
                    end_frame,
                    start_ms,
                    end_ms,
                    duration_ms,
                });
            }
        }
    }

    let total_silence_ms: i64 = silences.iter().map(|s| s.duration_ms).sum();
    let total_silence_seconds = total_silence_ms as f64 / 1000.0;

    Ok(SilenceAnalysisResult {
        silences_found: silences.len(),
        total_silence_seconds,
        silences,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};

    #[test]
    fn test_silence_detection_hysteresis() {
        let temp_dir = std::env::temp_dir().join("cutline_test_silence");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let wav_path = temp_dir.join("synthetic_speech_silence.wav");

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let mut writer = WavWriter::create(&wav_path, spec).unwrap();

        // 1.0 second of speech (loud sine tone ~ -6 dB)
        for t in 0..16000 {
            let val = (16000.0 * (2.0 * std::f32::consts::PI * 440.0 * (t as f32 / 16000.0)).sin())
                as i16;
            writer.write_sample(val).unwrap();
        }

        // 1.0 second of silence (zeros ~ -inf dB)
        for _ in 0..16000 {
            writer.write_sample(0i16).unwrap();
        }

        // 1.0 second of speech
        for t in 0..16000 {
            let val = (16000.0 * (2.0 * std::f32::consts::PI * 440.0 * (t as f32 / 16000.0)).sin())
                as i16;
            writer.write_sample(val).unwrap();
        }

        writer.finalize().unwrap();

        let config = SilenceDetectorConfig::default();
        let result = detect_silence_in_wav(&wav_path, Fps::FPS_30, &config).unwrap();

        assert_eq!(result.silences_found, 1);
        let s = &result.silences[0];
        // Silence should start around 1000ms and end around 2000ms
        assert!(
            s.start_ms >= 980 && s.start_ms <= 1020,
            "start_ms was {}",
            s.start_ms
        );
        assert!(
            s.end_ms >= 1980 && s.end_ms <= 2020,
            "end_ms was {}",
            s.end_ms
        );
        assert!(
            s.duration_ms >= 960 && s.duration_ms <= 1040,
            "duration was {}",
            s.duration_ms
        );
    }
}
