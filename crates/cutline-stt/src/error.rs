use thiserror::Error;

#[derive(Error, Debug)]
pub enum SttError {
    #[error("Failed to read WAV audio: {0}")]
    WavRead(String),

    #[error("Whisper model download failed: {0}")]
    ModelDownload(String),

    #[error("Whisper context initialization failed for model '{path}': {details}")]
    ContextInit { path: String, details: String },

    #[error("Whisper transcription inference failed: {0}")]
    Inference(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

impl SttError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::WavRead(_) => "ERR_WAV_READ",
            Self::ModelDownload(_) => "ERR_MODEL_DOWNLOAD",
            Self::ContextInit { .. } => "ERR_WHISPER_INIT",
            Self::Inference(_) => "ERR_WHISPER_INFERENCE",
            Self::Io(_) => "ERR_IO",
            Self::Json(_) => "ERR_JSON",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::WavRead(_) => {
                "Ensure the audio was extracted with 'cutline audio extract' as a 16kHz mono WAV."
            }
            Self::ModelDownload(_) => {
                "Check your internet connection to allow downloading the offline Whisper model."
            }
            Self::ContextInit { .. } => {
                "Verify the GGML model file in the cache directory is valid and not corrupted."
            }
            Self::Inference(_) => "Ensure system has sufficient RAM for Whisper inference.",
            Self::Io(_) => "Check disk permissions in your local cache directory.",
            Self::Json(_) => "Failed to serialize transcript.",
        }
    }
}
