use thiserror::Error;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Video decoding pipeline failed: {0}")]
    Decoder(String),

    #[error("Frame compositing pipeline failed: {0}")]
    Compositor(String),

    #[error("Video encoding pipeline failed: {0}")]
    Encoder(String),

    #[error("Audio pipeline failed: {0}")]
    Audio(String),

    #[error("Hardware acceleration test failed for '{encoder}': {details}")]
    HwAccel { encoder: String, details: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeline contains no video tracks or segments to render")]
    EmptyTimeline,
}

impl RenderError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Decoder(_) => "ERR_RENDER_DECODE",
            Self::Compositor(_) => "ERR_RENDER_COMPOSITE",
            Self::Encoder(_) => "ERR_RENDER_ENCODE",
            Self::Audio(_) => "ERR_RENDER_AUDIO",
            Self::HwAccel { .. } => "ERR_HW_ACCEL",
            Self::Io(_) => "ERR_IO",
            Self::EmptyTimeline => "ERR_EMPTY_TIMELINE",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::Decoder(_) => {
                "Verify source video file is readable and codec is supported by FFmpeg."
            }
            Self::Compositor(_) => "Check caption styling parameters and font assets.",
            Self::Encoder(_) => {
                "Try rendering with '--hw-accel none' to use the software libx264 encoder."
            }
            Self::Audio(_) => "Ensure source audio stream is valid and not corrupted.",
            Self::HwAccel { .. } => "Check graphics driver or fall back to software encoding.",
            Self::Io(_) => "Verify output directory exists and is writable.",
            Self::EmptyTimeline => {
                "Add at least one video segment to the timeline before rendering."
            }
        }
    }
}
