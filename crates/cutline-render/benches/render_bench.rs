use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cutline_assets::OverlayRenderer;
use cutline_core::project::{
    CaptionGroup, NormalizedPosition, OverlayItem, OverlayReason, Project, Source, WordToken,
};
use cutline_core::time::Fps;
use cutline_render::caption::{CaptionRenderer, CaptionStyle};

fn create_bench_project() -> Project {
    let source = Source {
        id: "src_bench".to_string(),
        path: "bench.mp4".to_string(),
        duration_frames: 300,
        width: 1080,
        height: 1920,
        fps: Fps::FPS_30,
        audio_sample_rate: 48000,
    };
    let mut project = Project::new("bench", source, "tiktok");
    project.captions.enabled = true;
    project.captions.style_id = "clean-yellow".to_string();
    project.captions.groups = vec![CaptionGroup {
        id: "grp_1".to_string(),
        timeline_in_frame: 0,
        timeline_out_frame: 60,
        words: vec![
            WordToken {
                text: "HELLO".to_string(),
                start_frame: 0,
                end_frame: 15,
                confidence: 0.99,
            },
            WordToken {
                text: "WORLD".to_string(),
                start_frame: 16,
                end_frame: 30,
                confidence: 0.99,
            },
            WordToken {
                text: "CUTLINE".to_string(),
                start_frame: 31,
                end_frame: 45,
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
            timeline_out_frame: 60,
            anchor: "top_center".to_string(),
            position: NormalizedPosition { x: 0.5, y: 0.22 },
            scale: 1.0,
            opacity: 1.0,
            in_animation: None,
            out_animation: None,
            reason: Some(OverlayReason {
                word_range: vec![0, 1],
                text: "Keyword highlight".to_string(),
            }),
            title: Some("KEYWORD".to_string()),
            subtitle: None,
        },
        OverlayItem {
            id: "ov_bird".to_string(),
            asset_id: "svg:bird-flying".to_string(),
            overlay_type: "svg_animation".to_string(),
            timeline_in_frame: 10,
            timeline_out_frame: 50,
            anchor: "top_center".to_string(),
            position: NormalizedPosition { x: 0.5, y: 0.35 },
            scale: 1.0,
            opacity: 1.0,
            in_animation: None,
            out_animation: None,
            reason: Some(OverlayReason {
                word_range: vec![2],
                text: "Flying illustration".to_string(),
            }),
            title: None,
            subtitle: None,
        },
    ];
    project
}

fn bench_caption_compositing(c: &mut Criterion) {
    let style = CaptionStyle::clean_yellow();
    let renderer = CaptionRenderer::new(style).expect("Failed to initialize CaptionRenderer");
    let project = create_bench_project();
    let width = 1080;
    let height = 1920;

    let mut group = c.benchmark_group("captions");
    group.bench_function("caption_render_frame_1080p", |b| {
        let mut buffer = vec![255u8; width as usize * height as usize * 4];
        b.iter(|| {
            renderer.render_frame(
                black_box(20),
                black_box(&mut buffer),
                black_box(width),
                black_box(height),
                black_box(&project.captions),
            );
        });
    });
}

fn bench_overlay_compositing(c: &mut Criterion) {
    let renderer = OverlayRenderer::new().expect("Failed to initialize OverlayRenderer");
    let project = create_bench_project();
    let width = 1080;
    let height = 1920;
    let total_frames = project.timeline.total_duration_frames();

    let mut group = c.benchmark_group("overlays");
    group.bench_function("overlay_render_svg_and_template_1080p", |b| {
        let mut buffer = vec![0u8; width as usize * height as usize * 4];
        b.iter(|| {
            renderer.render_frame(
                black_box(25),
                black_box(&mut buffer),
                black_box(width),
                black_box(height),
                black_box(&project.overlays),
                black_box(total_frames),
            );
        });
    });
}

fn bench_full_frame_compositing(c: &mut Criterion) {
    let project = create_bench_project();
    let style = CaptionStyle::clean_yellow();
    let caption_renderer =
        CaptionRenderer::new(style).expect("Failed to initialize CaptionRenderer");
    let overlay_renderer = OverlayRenderer::new().expect("Failed to initialize OverlayRenderer");
    let width = 1080;
    let height = 1920;
    let total_frames = project.timeline.total_duration_frames();

    let mut group = c.benchmark_group("full_frame");
    group.bench_function("full_compositing_pipeline_1080p", |b| {
        let mut buffer = vec![128u8; width as usize * height as usize * 4];
        b.iter(|| {
            // 1. Render active overlays (template badge + SVG animated bird)
            overlay_renderer.render_frame(
                black_box(25),
                black_box(&mut buffer),
                black_box(width),
                black_box(height),
                black_box(&project.overlays),
                black_box(total_frames),
            );
            // 2. Render kinetic word-by-word captions
            caption_renderer.render_frame(
                black_box(25),
                black_box(&mut buffer),
                black_box(width),
                black_box(height),
                black_box(&project.captions),
            );
        });
    });
}

criterion_group!(
    benches,
    bench_caption_compositing,
    bench_overlay_compositing,
    bench_full_frame_compositing
);
criterion_main!(benches);
