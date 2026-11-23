use crate::error::AnalysisError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneAnalysisResult {
    pub scene_cuts_found: usize,
    pub cut_frames: Vec<i64>,
}

/// Detects visual scene change cuts in source video (REQ-ANALYSIS-002)
pub fn detect_scene_changes(
    video_path: &Path,
    threshold: f32,
) -> Result<SceneAnalysisResult, AnalysisError> {
    let path_str = video_path.to_string_lossy().to_string();

    let filter_arg = format!("select='gt(scene,{})',showinfo", threshold);

    let output = Command::new("ffmpeg")
        .args(["-i", &path_str, "-filter:v", &filter_arg, "-f", "null", "-"])
        .output()
        .map_err(|e| AnalysisError::SceneDetection(e.to_string()))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut cut_frames = Vec::new();

    for line in stderr.lines() {
        if line.contains("showinfo") && line.contains("n:") {
            // e.g. [Parsed_showinfo_1 @ 00000...] n:   45 pts: 15360 ...
            if let Some(n_part) = line.split("n:").nth(1) {
                if let Some(frame_str) = n_part.split_whitespace().next() {
                    if let Ok(frame) = frame_str.parse::<i64>() {
                        cut_frames.push(frame);
                    }
                }
            }
        }
    }

    Ok(SceneAnalysisResult {
        scene_cuts_found: cut_frames.len(),
        cut_frames,
    })
}
