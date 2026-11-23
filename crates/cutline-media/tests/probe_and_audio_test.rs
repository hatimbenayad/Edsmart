use cutline_media::{extract_audio_16k_mono, probe_media};
use std::path::PathBuf;
use std::process::Command;

fn generate_synthetic_video(path: &PathBuf) {
    if path.exists() {
        return;
    }
    // 2-second 1080x1920 30fps clip with 1kHz sine audio
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
            "sine=frequency=1000:duration=2",
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
        .arg(path)
        .status()
        .expect("Failed to invoke ffmpeg to create test fixture");
    assert!(status.success(), "Failed to create synthetic test video");
}

#[test]
fn test_probe_and_audio_extraction() {
    let temp_dir = std::env::temp_dir().join("cutline_test_uow01");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let video_path = temp_dir.join("synthetic.mp4");
    generate_synthetic_video(&video_path);

    // REQ-MEDIA-002: Comprehensive video probing
    let probe = probe_media(&video_path).expect("Probing synthetic video failed");
    assert_eq!(probe.width, 1080);
    assert_eq!(probe.height, 1920);
    assert_eq!(probe.fps.num, 30);
    assert_eq!(probe.fps.den, 1);
    assert_eq!(probe.duration_frames, 60);
    assert!(probe.duration_seconds >= 1.9 && probe.duration_seconds <= 2.1);
    assert_eq!(probe.channels, Some(2));

    // REQ-MEDIA-003: 16kHz mono audio extraction
    let audio_path = temp_dir.join("extracted_16k.wav");
    let audio_info =
        extract_audio_16k_mono(&video_path, &audio_path).expect("Audio extraction failed");
    assert_eq!(audio_info.sample_rate, 16000);
    assert_eq!(audio_info.channels, 1);
    assert!(audio_info.duration_seconds >= 1.9 && audio_info.duration_seconds <= 2.1);
    assert!(audio_path.exists());
}
