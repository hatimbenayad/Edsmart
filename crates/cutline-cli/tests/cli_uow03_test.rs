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
fn test_cli_timeline_cut_and_render_pipeline() {
    let temp_dir = std::env::temp_dir().join("cutline_cli_uow03_test");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let video_path = temp_dir.join("test_render_src.mp4");
    let proj_path = temp_dir.join("project.json");
    let out_mp4 = temp_dir.join("rendered_output.mp4");

    // 1. Generate 4-second test video (120 frames at 30fps) with audio
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=4:size=540x960:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=4",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-ar",
            "44100",
            "-ac",
            "2",
        ])
        .arg(&video_path)
        .status()
        .unwrap();
    assert!(status.success());

    let bin = get_cutline_bin();

    // 2. Initialize project with fast-draft preset (540x960)
    let init_res = Command::new(&bin)
        .args(["project", "init"])
        .arg(&video_path)
        .args([
            "--preset",
            "fast-draft",
            "--out",
            proj_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(init_res.status.success());

    // 3. Cut out 1 second (frames 30 to 60) -> REQ-EDIT-001
    let cut_res = Command::new(&bin)
        .args(["timeline", "cut"])
        .arg(&proj_path)
        .args(["--start", "00:00:01.000", "--end", "00:00:02.000"])
        .output()
        .unwrap();
    assert!(
        cut_res.status.success(),
        "Cut stderr: {}",
        String::from_utf8_lossy(&cut_res.stderr)
    );

    let cut_json: serde_json::Value = serde_json::from_slice(&cut_res.stdout).unwrap();
    assert_eq!(cut_json["status"], "success");
    // Original was 120 frames (4s). After 30 frames cut (1s), duration is 90 frames (3s).
    assert_eq!(cut_json["data"]["new_timeline_frames"], 90);
    assert_eq!(cut_json["data"]["new_duration_seconds"], 3.0);

    // 4. Render video through streaming pipeline -> REQ-RENDER-001, REQ-RENDER-002, REQ-RENDER-003, REQ-RENDER-004
    let render_res = Command::new(&bin)
        .args(["render"])
        .arg(&proj_path)
        .args(["--out", out_mp4.to_str().unwrap(), "--preset", "fast-draft"])
        .output()
        .unwrap();

    assert!(
        render_res.status.success(),
        "Render stderr: {}",
        String::from_utf8_lossy(&render_res.stderr)
    );
    let render_json: serde_json::Value = serde_json::from_slice(&render_res.stdout).unwrap();
    assert_eq!(render_json["status"], "success");
    assert_eq!(render_json["command"], "render");
    assert_eq!(render_json["data"]["frames_rendered"], 90);
    assert!(render_json["data"]["speed_x_realtime"].as_f64().unwrap() > 0.0);
    assert!(out_mp4.exists());

    // 5. Probe final rendered video to verify container properties and A/V sync (REQ-RENDER-004)
    let probe_res = Command::new(&bin)
        .args(["probe", out_mp4.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(probe_res.status.success());
    let probe_json: serde_json::Value = serde_json::from_slice(&probe_res.stdout).unwrap();
    assert_eq!(probe_json["data"]["width"], 540);
    assert_eq!(probe_json["data"]["height"], 960);
    assert_eq!(probe_json["data"]["duration_frames"], 90);
    let dur = probe_json["data"]["duration_seconds"].as_f64().unwrap();
    assert!(
        (dur - 3.0).abs() <= 0.1,
        "Rendered duration was {dur}, expected ~3.0s"
    );
}
