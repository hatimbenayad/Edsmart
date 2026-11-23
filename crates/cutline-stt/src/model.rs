use crate::error::SttError;
use std::fs::{self, File};
use std::io::{copy, BufReader};
use std::path::PathBuf;
use tracing::info;

pub struct ModelManager;

impl ModelManager {
    pub fn cutline_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cutline")
    }

    pub fn models_dir() -> PathBuf {
        Self::cutline_dir().join("models")
    }

    pub fn bin_dir() -> PathBuf {
        Self::cutline_dir().join("bin")
    }

    pub fn model_filename(alias: &str) -> &'static str {
        match alias {
            "tiny" => "ggml-tiny.bin",
            "small" => "ggml-small.bin",
            _ => "ggml-base.bin",
        }
    }

    /// Ensures the Whisper GGML model is cached locally (REQ-STT-002)
    pub fn ensure_model(alias: &str) -> Result<PathBuf, SttError> {
        let path = PathBuf::from(alias);
        if path.exists() {
            return Ok(path);
        }

        let filename = Self::model_filename(alias);
        let dir = Self::models_dir();
        fs::create_dir_all(&dir)?;

        let target_path = dir.join(filename);
        if target_path.exists() {
            if let Ok(meta) = fs::metadata(&target_path) {
                if meta.len() > 10_000_000 {
                    return Ok(target_path);
                }
            }
        }

        info!(
            "Downloading local Whisper model '{}' into cache...",
            filename
        );
        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            filename
        );

        let temp_path = dir.join(format!("{}.download", filename));

        let response = ureq::get(&url)
            .call()
            .map_err(|e| SttError::ModelDownload(format!("HTTP error connecting to {url}: {e}")))?;

        if response.status() != 200 {
            return Err(SttError::ModelDownload(format!(
                "HTTP status {} received from {url}",
                response.status()
            )));
        }

        let mut reader = BufReader::new(response.into_reader());
        let mut file = File::create(&temp_path)?;
        copy(&mut reader, &mut file)?;
        drop(file);

        fs::rename(&temp_path, &target_path)?;
        info!(
            "Model successfully downloaded and cached at {:?}",
            target_path
        );

        Ok(target_path)
    }

    /// Locates or downloads the official native whisper.cpp runner
    pub fn ensure_whisper_cli() -> Result<PathBuf, SttError> {
        // 1. Check system PATH
        if let Ok(path) = which::which("whisper-cli") {
            return Ok(path);
        }

        // 2. Check local cutline bin cache
        let bin_dir = Self::bin_dir();
        let candidate1 = bin_dir.join("Release").join("whisper-cli.exe");
        if candidate1.exists() {
            return Ok(candidate1);
        }

        let candidate2 = bin_dir.join("whisper-cli.exe");
        if candidate2.exists() {
            return Ok(candidate2);
        }

        // 3. Download official precompiled release archive
        fs::create_dir_all(&bin_dir)?;
        let zip_path = bin_dir.join("whisper.zip");

        info!("Downloading native whisper.cpp engine...");
        let url =
            "https://github.com/ggml-org/whisper.cpp/releases/download/b5130/whisper-bin-x64.zip";

        let response = ureq::get(url).call().map_err(|e| {
            SttError::ModelDownload(format!("Failed to download whisper engine from {url}: {e}"))
        })?;

        let mut reader = BufReader::new(response.into_reader());
        let mut file = File::create(&zip_path)?;
        copy(&mut reader, &mut file)?;
        drop(file);

        // Unpack zip via powershell on Windows
        #[cfg(target_os = "windows")]
        {
            let status = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                        zip_path.display(),
                        bin_dir.display()
                    ),
                ])
                .status()
                .map_err(|e| SttError::ContextInit {
                    path: zip_path.to_string_lossy().to_string(),
                    details: format!("Failed to extract whisper zip: {e}"),
                })?;

            if !status.success() {
                return Err(SttError::ContextInit {
                    path: zip_path.to_string_lossy().to_string(),
                    details: "Failed to expand whisper.zip archive".to_string(),
                });
            }
        }

        if candidate1.exists() {
            Ok(candidate1)
        } else if candidate2.exists() {
            Ok(candidate2)
        } else {
            Err(SttError::ContextInit {
                path: bin_dir.to_string_lossy().to_string(),
                details: "whisper-cli binary not found after unpacking".to_string(),
            })
        }
    }
}
