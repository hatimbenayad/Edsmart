use cutline_analysis::{detect_scene_changes, detect_silence_in_wav, SilenceDetectorConfig};
use cutline_core::time::Fps;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::process::Command;

#[test]
fn test_silence_detection_intervals() {
    // REQ-ANALYSIS-001: Energy silence detection with hysteresis
    let temp_dir = std::env::temp_dir().join("cutline_test_analysis");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let wav_path = temp_dir.join("test_silence_int.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&wav_path, spec).unwrap();
    // 0.5s speech
    for t in 0..8000 {
        let val =
            (16000.0 * (2.0 * std::f32::consts::PI * 440.0 * (t as f32 / 16000.0)).sin()) as i16;
        writer.write_sample(val).unwrap();
    }
    // 0.8s dead air
    for _ in 0..12800 {
        writer.write_sample(0i16).unwrap();
    }
    // 0.5s speech
    for t in 0..8000 {
        let val =
            (16000.0 * (2.0 * std::f32::consts::PI * 440.0 * (t as f32 / 16000.0)).sin()) as i16;
        writer.write_sample(val).unwrap();
    }
    writer.finalize().unwrap();

    let config = SilenceDetectorConfig::default();
    let res = detect_silence_in_wav(&wav_path, Fps::FPS_30, &config).unwrap();

    assert_eq!(res.silences_found, 1);
    let s = &res.silences[0];
    assert!(s.duration_ms >= 750 && s.duration_ms <= 850);
}

#[test]
fn test_scene_detection_filter() {
    // REQ-ANALYSIS-002: Scene change detection
    let temp_dir = std::env::temp_dir().join("cutline_test_scene");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let video_path = temp_dir.join("scene_test.mp4");

    // Create a 2-second clip with two radically different color scenes (1s white, 1s black)
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=white:duration=1:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:duration=1:size=320x240:rate=30",
            "-filter_complex",
            "[0:v][1:v]concat=n=2:v=1[v]",
            "-map",
            "[v]",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&video_path)
        .status()
        .expect("Failed to invoke ffmpeg for scene test");

    assert!(status.success());

    let res = detect_scene_changes(&video_path, 0.4).expect("Scene detection failed");
    // Should detect the shot change at frame ~30 (1s)
    assert!(res.scene_cuts_found >= 1);
}
