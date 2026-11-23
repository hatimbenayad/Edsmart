pub mod error;
pub mod scene;
pub mod silence;

pub use error::AnalysisError;
pub use scene::{detect_scene_changes, SceneAnalysisResult};
pub use silence::{
    detect_silence_in_wav, DetectedSilence, SilenceAnalysisResult, SilenceDetectorConfig,
};
