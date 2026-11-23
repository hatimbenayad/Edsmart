use crate::error::MediaError;
use cutline_core::time::Fps;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaProbeResult {
    pub path: String,
    pub duration_seconds: f64,
    pub duration_frames: i64,
    pub width: u32,
    pub height: u32,
    pub fps: Fps,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: String,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    nb_frames: Option<String>,
    duration: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u16>,
}

pub fn probe_media(path: &Path) -> Result<MediaProbeResult, MediaError> {
    let path_str = path.to_string_lossy().to_string();

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(&path_str)
        .output()
        .map_err(|e| MediaError::ProbeFailed {
            path: path_str.clone(),
            details: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(MediaError::ProbeFailed {
            path: path_str,
            details: stderr,
        });
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)?;

    let v_stream = parsed.streams.iter().find(|s| s.codec_type == "video");
    let a_stream = parsed.streams.iter().find(|s| s.codec_type == "audio");

    if v_stream.is_none() && a_stream.is_none() {
        return Err(MediaError::ProbeFailed {
            path: path_str,
            details: "No video or audio streams detected in file".to_string(),
        });
    }

    // Determine duration
    let duration_seconds: f64 = parsed
        .format
        .as_ref()
        .and_then(|f| f.duration.as_ref())
        .and_then(|d| d.parse::<f64>().ok())
        .or_else(|| {
            v_stream
                .and_then(|v| v.duration.as_ref())
                .and_then(|d| d.parse::<f64>().ok())
        })
        .or_else(|| {
            a_stream
                .and_then(|a| a.duration.as_ref())
                .and_then(|d| d.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    let (width, height, fps, video_codec, duration_frames) = if let Some(v) = v_stream {
        let w = v.width.unwrap_or(1080);
        let h = v.height.unwrap_or(1920);
        let f = parse_fps(v.r_frame_rate.as_deref().or(v.avg_frame_rate.as_deref()))?;
        let vc = v.codec_name.clone();
        let frames = if let Some(nb) = v.nb_frames.as_ref().and_then(|n| n.parse::<i64>().ok()) {
            nb
        } else {
            (duration_seconds * f.as_f64()).round() as i64
        };
        (w, h, f, vc, frames)
    } else {
        let f = Fps::FPS_30;
        let frames = (duration_seconds * f.as_f64()).round() as i64;
        (0, 0, f, None, frames)
    };

    let audio_codec = a_stream.and_then(|a| a.codec_name.clone());
    let sample_rate = a_stream
        .and_then(|a| a.sample_rate.as_ref())
        .and_then(|sr| sr.parse::<u32>().ok());
    let channels = a_stream.and_then(|a| a.channels);

    Ok(MediaProbeResult {
        path: path_str,
        duration_seconds,
        duration_frames,
        width,
        height,
        fps,
        video_codec,
        audio_codec,
        sample_rate,
        channels,
    })
}

fn parse_fps(rate_str: Option<&str>) -> Result<Fps, MediaError> {
    let s = rate_str.unwrap_or("30/1");
    if let Some((num_str, den_str)) = s.split_once('/') {
        let num: u32 = num_str
            .parse()
            .map_err(|_| MediaError::InvalidFps(s.to_string()))?;
        let den: u32 = den_str
            .parse()
            .map_err(|_| MediaError::InvalidFps(s.to_string()))?;
        Fps::new(num, den).map_err(|_| MediaError::InvalidFps(s.to_string()))
    } else if let Ok(val) = s.parse::<f64>() {
        let num = (val * 1000.0).round() as u32;
        Fps::new(num, 1000).map_err(|_| MediaError::InvalidFps(s.to_string()))
    } else {
        Err(MediaError::InvalidFps(s.to_string()))
    }
}
