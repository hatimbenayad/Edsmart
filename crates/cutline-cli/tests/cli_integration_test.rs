use std::path::PathBuf;
use std::process::Command;

fn get_cutline_bin() -> PathBuf {
    let mut bin = PathBuf::from(env!("CARGO_BIN_EXE_cutline"));
    if !bin.exists() {
        bin = PathBuf::from("target/debug/cutline.exe");
    }
    bin
}

fn ensure_test_video(path: &PathBuf) {
    if path.exists() {
        return;
    }
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=1080x1920:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=800:duration=2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ])
        .arg(path)
        .status()
        .expect("Failed to create test clip");
    assert!(status.success());
}

#[test]
fn test_cli_probe_json_envelope() {
    // REQ-AGENT-001, REQ-MEDIA-002: Pure JSON stdout envelope
    let temp_dir = std::env::temp_dir().join("cutline_cli_test");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let video_path = temp_dir.join("cli_test.mp4");
    ensure_test_video(&video_path);

    let bin = get_cutline_bin();
    let output = Command::new(&bin)
        .arg("probe")
        .arg(&video_path)
        .output()
        .expect("Failed to execute cutline probe");

    assert!(output.status.success());
    let stdout_str = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&stdout_str).expect("Stdout must be strictly valid JSON");

    assert_eq!(json["status"], "success");
    assert_eq!(json["schema_version"], "1.0.0");
    assert_eq!(json["command"], "probe");
    assert_eq!(json["data"]["width"], 1080);
    assert_eq!(json["data"]["height"], 1920);
    assert_eq!(json["data"]["fps"]["num"], 30);
}

#[test]
fn test_cli_project_init_and_audio_extract() {
    // REQ-CORE-001, REQ-CORE-003, REQ-MEDIA-003
    let temp_dir = std::env::temp_dir().join("cutline_cli_test_init");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let video_path = temp_dir.join("input.mp4");
    let proj_path = temp_dir.join("project.json");
    let wav_path = temp_dir.join("audio.wav");
    ensure_test_video(&video_path);

    let bin = get_cutline_bin();

    // 1. Project Init
    let init_out = Command::new(&bin)
        .args(["project", "init"])
        .arg(&video_path)
        .args(["--out", proj_path.to_str().unwrap()])
        .output()
        .expect("Failed to run project init");

    assert!(init_out.status.success());
    let init_json: serde_json::Value = serde_json::from_slice(&init_out.stdout).unwrap();
    assert_eq!(init_json["status"], "success");
    assert_eq!(init_json["command"], "project.init");
    assert!(proj_path.exists());

    // 2. Audio Extract from project file
    let audio_out = Command::new(&bin)
        .args(["audio", "extract"])
        .arg(&proj_path)
        .args(["--out", wav_path.to_str().unwrap()])
        .output()
        .expect("Failed to run audio extract");

    assert!(audio_out.status.success());
    let audio_json: serde_json::Value = serde_json::from_slice(&audio_out.stdout).unwrap();
    assert_eq!(audio_json["status"], "success");
    assert_eq!(audio_json["command"], "audio.extract");
    assert_eq!(audio_json["data"]["sample_rate"], 16000);
    assert_eq!(audio_json["data"]["channels"], 1);
    assert!(wav_path.exists());
}

#[test]
fn test_cli_structured_error_hints() {
    // REQ-AGENT-003: Structured error with { code, message, hint }
    let bin = get_cutline_bin();
    let output = Command::new(&bin)
        .args(["probe", "non_existent_file_12345.mp4"])
        .output()
        .expect("Failed to run probe on invalid file");

    assert!(!output.status.success());
    let stdout_str = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&stdout_str).expect("Error must be formatted as valid JSON");

    assert_eq!(json["status"], "error");
    assert_eq!(json["schema_version"], "1.0.0");
    assert_eq!(json["error"]["code"], "ERR_PROBE_FAILED");
    assert!(json["error"]["hint"].as_str().unwrap().len() > 10);
}

#[test]
fn test_cli_schema_export() {
    // REQ-CORE-003: cutline schema exports Draft JSON Schema
    let bin = get_cutline_bin();
    let output = Command::new(&bin)
        .arg("schema")
        .output()
        .expect("Failed to run cutline schema");

    assert!(output.status.success());
    let schema_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(schema_json["title"], "Project");
    assert!(schema_json["properties"]["timeline"].is_object());
}
