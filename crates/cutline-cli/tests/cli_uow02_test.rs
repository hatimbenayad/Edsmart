use std::path::PathBuf;
use std::process::Command;

fn get_cutline_bin() -> PathBuf {
    let mut bin = PathBuf::from(env!("CARGO_BIN_EXE_cutline"));
    if !bin.exists() {
        bin = PathBuf::from("target/debug/cutline.exe");
    }
    bin
}

#[test]
fn test_cli_transcribe_and_silence_analysis() {
    let temp_dir = std::env::temp_dir().join("cutline_cli_uow02_test");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let video_path = temp_dir.join("test_video.mp4");
    let proj_path = temp_dir.join("project.json");
    let transcript_path = temp_dir.join("transcript.json");

    // Create 3-second test video
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=3:size=1080x1920:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=3",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ])
        .arg(&video_path)
        .status()
        .unwrap();
    assert!(status.success());

    let bin = get_cutline_bin();

    // 1. Init project
    let init_res = Command::new(&bin)
        .args(["project", "init"])
        .arg(&video_path)
        .args(["--out", proj_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(init_res.status.success());

    // 2. Transcribe (REQ-STT-001, REQ-STT-003)
    let trans_res = Command::new(&bin)
        .args(["transcribe"])
        .arg(&proj_path)
        .args([
            "--model",
            "tiny",
            "--out",
            transcript_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        trans_res.status.success(),
        "Transcribe stderr: {}",
        String::from_utf8_lossy(&trans_res.stderr)
    );
    let trans_json: serde_json::Value = serde_json::from_slice(&trans_res.stdout).unwrap();
    assert_eq!(trans_json["status"], "success");
    assert_eq!(trans_json["command"], "transcribe");
    assert!(transcript_path.exists());

    // 3. Analyze Silence (REQ-ANALYSIS-001)
    let silence_res = Command::new(&bin)
        .args(["analyze", "silence"])
        .arg(&proj_path)
        .output()
        .unwrap();

    assert!(silence_res.status.success());
    let silence_json: serde_json::Value = serde_json::from_slice(&silence_res.stdout).unwrap();
    assert_eq!(silence_json["status"], "success");
    assert_eq!(silence_json["command"], "analyze.silence");

    // 4. Analyze Scenes (REQ-ANALYSIS-002)
    let scene_res = Command::new(&bin)
        .args(["analyze", "scenes"])
        .arg(&proj_path)
        .output()
        .unwrap();

    assert!(scene_res.status.success());
    let scene_json: serde_json::Value = serde_json::from_slice(&scene_res.stdout).unwrap();
    assert_eq!(scene_json["status"], "success");
    assert_eq!(scene_json["command"], "analyze.scenes");
}
