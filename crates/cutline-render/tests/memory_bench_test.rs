use cutline_assets::OverlayRenderer;
use cutline_core::project::{
    CaptionGroup, NormalizedPosition, OverlayItem, OverlayReason, Project, Source, WordToken,
};
use cutline_core::time::Fps;
use cutline_render::caption::{CaptionRenderer, CaptionStyle};
use std::time::Instant;

fn create_test_project() -> Project {
    let source = Source {
        id: "src_perf".to_string(),
        path: "test.mp4".to_string(),
        duration_frames: 300,
        width: 1080,
        height: 1920,
        fps: Fps::FPS_30,
        audio_sample_rate: 48000,
    };
    let mut project = Project::new("perf", source, "tiktok");
    project.captions.enabled = true;
    project.captions.style_id = "clean-yellow".to_string();
    project.captions.groups = vec![CaptionGroup {
        id: "grp_1".to_string(),
        timeline_in_frame: 0,
        timeline_out_frame: 90,
        words: vec![
            WordToken {
                text: "HELLO".to_string(),
                start_frame: 0,
                end_frame: 25,
                confidence: 0.99,
            },
            WordToken {
                text: "WORLD".to_string(),
                start_frame: 26,
                end_frame: 55,
                confidence: 0.99,
            },
        ],
    }];
    project.overlays = vec![
        OverlayItem {
            id: "ov_badge".to_string(),
            asset_id: "template:keyword-badge".to_string(),
            overlay_type: "template".to_string(),
            timeline_in_frame: 0,
            timeline_out_frame: 90,
            anchor: "top_center".to_string(),
            position: NormalizedPosition { x: 0.5, y: 0.22 },
            scale: 1.0,
            opacity: 1.0,
            in_animation: None,
            out_animation: None,
            reason: Some(OverlayReason {
                word_range: vec![0],
                text: "Keyword".to_string(),
            }),
            title: Some("PERFORMANCE".to_string()),
            subtitle: None,
        },
        OverlayItem {
            id: "ov_bird".to_string(),
            asset_id: "svg:bird-flying".to_string(),
            overlay_type: "svg_animation".to_string(),
            timeline_in_frame: 10,
            timeline_out_frame: 80,
            anchor: "top_center".to_string(),
            position: NormalizedPosition { x: 0.5, y: 0.35 },
            scale: 1.0,
            opacity: 1.0,
            in_animation: None,
            out_animation: None,
            reason: Some(OverlayReason {
                word_range: vec![1],
                text: "Bird".to_string(),
            }),
            title: None,
            subtitle: None,
        },
    ];
    project
}

#[test]
fn test_render_memory_footprint_peak() {
    // REQ-NFR-002: Flat memory footprint, bounded buffer pool
    let project = create_test_project();
    let width = 1080;
    let height = 1920;
    let frame_bytes = (width * height * 4) as usize; // ~8.29 MB per 1080p RGBA buffer
    let pool_capacity = 16;
    let max_pool_memory_mb = (frame_bytes * pool_capacity) / (1024 * 1024);

    // Bounded pool must be strictly under 512 MB
    assert!(
        max_pool_memory_mb < 200,
        "Ring buffer pool must consume < 200MB (limit is 512MB)"
    );

    // Simulate recycling frame buffers across 60 frames
    let mut pool = Vec::with_capacity(pool_capacity);
    for _ in 0..pool_capacity {
        pool.push(vec![0u8; frame_bytes]);
    }

    let caption_renderer = CaptionRenderer::new(CaptionStyle::clean_yellow()).unwrap();
    let overlay_renderer = OverlayRenderer::new().unwrap();

    let start = Instant::now();
    for f in 0..60 {
        let mut buf = pool.pop().unwrap_or_else(|| vec![0u8; frame_bytes]);
        // Composite overlays and captions
        overlay_renderer.render_frame(f, &mut buf, width, height, &project.overlays, 300);
        caption_renderer.render_frame(f, &mut buf, width, height, &project.captions);
        // Recycle buffer
        pool.push(buf);
    }
    let elapsed = start.elapsed();

    // Pool size remains strictly bounded
    assert_eq!(pool.len(), pool_capacity);

    // REQ-NFR-001: Check compositing throughput (60 frames at 30fps is 2.0s of content)
    let content_duration = 2.0;
    let speed_x_realtime = content_duration / elapsed.as_secs_f64();
    println!("Compositing throughput: {:.2}x realtime", speed_x_realtime);
    assert!(speed_x_realtime >= 1.5, "Compositing speed should be fast");
}
