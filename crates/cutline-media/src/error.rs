use thiserror::Error;

#[derive(Error, Debug)]
pub enum MediaError {
    #[error("FFmpeg not found on PATH. Cutline requires FFmpeg for media operations.")]
    FfmpegMissing,

    #[error("FFprobe not found on PATH.")]
    FfprobeMissing,

    #[error("Failed to execute media command: {0}")]
    ProcessExecution(#[from] std::io::Error),

    #[error("Media probing failed for '{path}': {details}")]
    ProbeFailed { path: String, details: String },

    #[error("Audio extraction failed for '{path}': {details}")]
    AudioExtractionFailed { path: String, details: String },

    #[error("Failed to parse media probe JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Media file has no video stream: '{0}'")]
    NoVideoStream(String),

    #[error("Invalid rational frame rate '{0}'")]
    InvalidFps(String),
}

impl MediaError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::FfmpegMissing => "ERR_FFMPEG_MISSING",
            Self::FfprobeMissing => "ERR_FFPROBE_MISSING",
            Self::ProcessExecution(_) => "ERR_PROCESS_EXEC",
            Self::ProbeFailed { .. } => "ERR_PROBE_FAILED",
            Self::AudioExtractionFailed { .. } => "ERR_AUDIO_EXTRACT_FAILED",
            Self::JsonParse(_) => "ERR_PROBE_JSON_PARSE",
            Self::NoVideoStream(_) => "ERR_NO_VIDEO_STREAM",
            Self::InvalidFps(_) => "ERR_INVALID_FPS",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::FfmpegMissing => "Install FFmpeg and ensure 'ffmpeg' is accessible in your system PATH environment variable.",
            Self::FfprobeMissing => "Ensure 'ffprobe' is installed alongside FFmpeg and added to system PATH.",
            Self::ProcessExecution(_) => "Check operating system process execution permissions.",
            Self::ProbeFailed { .. } => "Verify the video file is not corrupted and is supported by FFmpeg.",
            Self::AudioExtractionFailed { .. } => "Check that the media contains a valid audio stream or that destination directory is writable.",
            Self::JsonParse(_) => "Verify that ffprobe output is valid JSON.",
            Self::NoVideoStream(_) => "Provide a video container that contains at least one video track.",
            Self::InvalidFps(_) => "The media stream has an unrecognizable or variable framerate.",
        }
    }
}
