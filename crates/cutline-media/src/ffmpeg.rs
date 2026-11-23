use crate::error::MediaError;
use std::process::Command;

/// Checks if ffmpeg is accessible and executable on PATH (REQ-MEDIA-001)
pub fn check_ffmpeg() -> Result<String, MediaError> {
    if which::which("ffmpeg").is_err() {
        return Err(MediaError::FfmpegMissing);
    }

    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map_err(|_| MediaError::FfmpegMissing)?;

    if !output.status.success() {
        return Err(MediaError::FfmpegMissing);
    }

    let banner = String::from_utf8_lossy(&output.stdout);
    let first_line = banner.lines().next().unwrap_or("ffmpeg unknown version");
    Ok(first_line.to_string())
}

/// Checks if ffprobe is accessible and executable on PATH
pub fn check_ffprobe() -> Result<String, MediaError> {
    if which::which("ffprobe").is_err() {
        return Err(MediaError::FfprobeMissing);
    }

    let output = Command::new("ffprobe")
        .arg("-version")
        .output()
        .map_err(|_| MediaError::FfprobeMissing)?;

    if !output.status.success() {
        return Err(MediaError::FfprobeMissing);
    }

    let banner = String::from_utf8_lossy(&output.stdout);
    let first_line = banner.lines().next().unwrap_or("ffprobe unknown version");
    Ok(first_line.to_string())
}

/// Validates both FFmpeg and FFprobe
pub fn verify_environment() -> Result<(), MediaError> {
    check_ffmpeg()?;
    check_ffprobe()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_detection() {
        // REQ-MEDIA-001: Verification on system with ffmpeg
        let res = check_ffmpeg();
        assert!(res.is_ok(), "Expected FFmpeg to be detected: {:?}", res);
        let banner = res.unwrap();
        assert!(banner.contains("ffmpeg version"));
    }
}
