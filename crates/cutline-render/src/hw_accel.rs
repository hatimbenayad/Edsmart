use std::process::Command;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderConfig {
    pub codec_name: String,
    pub extra_args: Vec<String>,
}

impl EncoderConfig {
    pub fn software_x264() -> Self {
        Self {
            codec_name: "libx264".to_string(),
            extra_args: vec![
                "-preset".to_string(),
                "veryfast".to_string(),
                "-crf".to_string(),
                "20".to_string(),
                "-pix_fmt".to_string(),
                "yuv420p".to_string(),
            ],
        }
    }
}

/// Detects and tests available hardware encoders, falling back to software libx264 (REQ-RENDER-003)
pub fn select_encoder(requested: Option<&str>) -> EncoderConfig {
    if let Some(req) = requested {
        match req.to_lowercase().as_str() {
            "none" | "sw" | "libx264" => return EncoderConfig::software_x264(),
            "nvenc" | "h264_nvenc" if test_encoder("h264_nvenc") => {
                return EncoderConfig {
                    codec_name: "h264_nvenc".to_string(),
                    extra_args: vec![
                        "-preset".to_string(),
                        "p4".to_string(),
                        "-tune".to_string(),
                        "hq".to_string(),
                        "-pix_fmt".to_string(),
                        "yuv420p".to_string(),
                    ],
                };
            }
            "amf" | "h264_amf" if test_encoder("h264_amf") => {
                return EncoderConfig {
                    codec_name: "h264_amf".to_string(),
                    extra_args: vec![
                        "-quality".to_string(),
                        "quality".to_string(),
                        "-pix_fmt".to_string(),
                        "yuv420p".to_string(),
                    ],
                };
            }
            "qsv" | "h264_qsv" if test_encoder("h264_qsv") => {
                return EncoderConfig {
                    codec_name: "h264_qsv".to_string(),
                    extra_args: vec![
                        "-preset".to_string(),
                        "medium".to_string(),
                        "-pix_fmt".to_string(),
                        "nv12".to_string(),
                    ],
                };
            }
            "vt" | "videotoolbox" | "h264_videotoolbox" if test_encoder("h264_videotoolbox") => {
                return EncoderConfig {
                    codec_name: "h264_videotoolbox".to_string(),
                    extra_args: vec![
                        "-realtime".to_string(),
                        "true".to_string(),
                        "-pix_fmt".to_string(),
                        "yuv420p".to_string(),
                    ],
                };
            }
            _ => {}
        }
    }

    // Auto-detection cascade
    if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
        if test_encoder("h264_nvenc") {
            info!("Hardware acceleration: NVIDIA NVENC active");
            return EncoderConfig {
                codec_name: "h264_nvenc".to_string(),
                extra_args: vec![
                    "-preset".to_string(),
                    "p4".to_string(),
                    "-tune".to_string(),
                    "hq".to_string(),
                    "-pix_fmt".to_string(),
                    "yuv420p".to_string(),
                ],
            };
        }
        if test_encoder("h264_amf") {
            info!("Hardware acceleration: AMD AMF active");
            return EncoderConfig {
                codec_name: "h264_amf".to_string(),
                extra_args: vec![
                    "-quality".to_string(),
                    "quality".to_string(),
                    "-pix_fmt".to_string(),
                    "yuv420p".to_string(),
                ],
            };
        }
        if test_encoder("h264_qsv") {
            info!("Hardware acceleration: Intel QSV active");
            return EncoderConfig {
                codec_name: "h264_qsv".to_string(),
                extra_args: vec![
                    "-preset".to_string(),
                    "medium".to_string(),
                    "-pix_fmt".to_string(),
                    "nv12".to_string(),
                ],
            };
        }
    }

    if cfg!(target_os = "macos") && test_encoder("h264_videotoolbox") {
        info!("Hardware acceleration: Apple VideoToolbox active");
        return EncoderConfig {
            codec_name: "h264_videotoolbox".to_string(),
            extra_args: vec![
                "-realtime".to_string(),
                "true".to_string(),
                "-pix_fmt".to_string(),
                "yuv420p".to_string(),
            ],
        };
    }

    info!("Using software encoder: libx264");
    EncoderConfig::software_x264()
}

fn test_encoder(name: &str) -> bool {
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=size=32x32:duration=0.1:rate=30",
            "-frames:v",
            "1",
            "-c:v",
            name,
            "-f",
            "null",
            "-",
        ])
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}
