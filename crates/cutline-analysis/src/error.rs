use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Failed to read WAV audio for analysis: {0}")]
    WavRead(String),

    #[error("Scene detection failed: {0}")]
    SceneDetection(String),

    #[error("Invalid threshold parameter: {0}")]
    InvalidParameter(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl AnalysisError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::WavRead(_) => "ERR_WAV_READ",
            Self::SceneDetection(_) => "ERR_SCENE_DETECT",
            Self::InvalidParameter(_) => "ERR_INVALID_PARAM",
            Self::Io(_) => "ERR_IO",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::WavRead(_) => {
                "Ensure the project audio file was extracted and is a valid WAV file."
            }
            Self::SceneDetection(_) => {
                "Verify the video file has a valid video stream for scene analysis."
            }
            Self::InvalidParameter(_) => {
                "Ensure threshold values are in reasonable bounds (e.g. dB between -60 and 0)."
            }
            Self::Io(_) => "Check disk permissions and file availability.",
        }
    }
}
