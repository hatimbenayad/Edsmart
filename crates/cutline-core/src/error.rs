use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Time error: {0}")]
    Time(#[from] crate::time::TimeError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Source not found: {0}")]
    SourceNotFound(String),

    #[error(
        "Invalid timeline range: start frame {start} is greater than or equal to end frame {end}"
    )]
    InvalidTimeRange { start: i64, end: i64 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl CoreError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Time(_) => "ERR_INVALID_TIME",
            Self::Serialization(_) => "ERR_JSON_SERIALIZE",
            Self::SourceNotFound(_) => "ERR_SOURCE_NOT_FOUND",
            Self::InvalidTimeRange { .. } => "ERR_INVALID_FRAME_RANGE",
            Self::Io(_) => "ERR_IO",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::Time(_) => {
                "Check the time format (e.g. '00:00:12.400', '12.4s', or 'frame:372')."
            }
            Self::Serialization(_) => {
                "Ensure the project JSON adheres to the schema exported by 'cutline schema'."
            }
            Self::SourceNotFound(_) => "Verify the media path exists and is readable.",
            Self::InvalidTimeRange { .. } => "Ensure that the start time precedes the end time.",
            Self::Io(_) => "Check disk permissions and available storage space.",
        }
    }
}
