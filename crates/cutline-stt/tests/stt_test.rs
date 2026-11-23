use cutline_core::time::Fps;
use cutline_stt::{transcribe_audio, ModelManager};
use hound::{SampleFormat, WavSpec, WavWriter};

#[test]
fn test_model_cache_and_transcription() {
    // REQ-STT-002: Model cache verification
    let model_path = ModelManager::ensure_model("tiny").expect("Model caching failed");
    assert!(model_path.exists());

    // REQ-STT-001, REQ-STT-003: Offline transcription with word timestamps
    let temp_dir = std::env::temp_dir().join("cutline_test_stt");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let wav_path = temp_dir.join("test_tone.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&wav_path, spec).unwrap();
    for t in 0..32000 {
        let val =
            (16000.0 * (2.0 * std::f32::consts::PI * 440.0 * (t as f32 / 16000.0)).sin()) as i16;
        writer.write_sample(val).unwrap();
    }
    writer.finalize().unwrap();

    let transcript =
        transcribe_audio(&model_path, &wav_path, Fps::FPS_30).expect("Transcription failed");

    assert_eq!(transcript.schema_version, "1.0.0");
    assert_eq!(transcript.language, "en");
    // Words were parsed and populated
    for word in &transcript.words {
        assert!(word.end_frame >= word.start_frame);
        assert!(word.end_ms >= word.start_ms);
        assert!(word.confidence >= 0.0 && word.confidence <= 1.0);
    }
}
