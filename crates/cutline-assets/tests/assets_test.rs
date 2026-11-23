use cutline_assets::{
    apply_punch_in_zoom, ease_in_cubic, ease_out_back, ease_out_cubic, render_animated_bird,
    render_animated_progress, render_keyword_badge, render_lower_third, AssetRegistry,
    OverlayRenderer,
};
use cutline_core::project::{NormalizedPosition, OverlayItem, OverlayReason};
use fontdue::{Font, FontSettings};

static TEST_FONT: &[u8] = include_bytes!("../../cutline-render/assets/fonts/Inter-Bold.ttf");

#[test]
fn test_asset_registry_builtins() {
    let registry = AssetRegistry::new();
    let list = registry.list();
    assert!(list.len() >= 5);

    let bird = registry.find("bird-flying");
    assert!(bird.is_some());
    assert_eq!(bird.unwrap().kind, "svg_animation");

    let lower_third = registry.find("lower-third");
    assert!(lower_third.is_some());
    assert_eq!(lower_third.unwrap().kind, "template");
}

#[test]
fn test_easing_functions() {
    assert!((ease_out_cubic(0.0) - 0.0).abs() < 1e-4);
    assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-4);
    assert!((ease_in_cubic(0.0) - 0.0).abs() < 1e-4);
    assert!((ease_in_cubic(1.0) - 1.0).abs() < 1e-4);
    assert!(ease_out_back(0.6) > 1.0, "EaseOutBack should overshoot");
}

#[test]
fn test_render_lower_third_template() {
    // REQ-OVERLAY-001: Lower third pill
    let font = Font::from_bytes(TEST_FONT, FontSettings::default()).unwrap();
    let width = 540u32;
    let height = 960u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    let overlay = OverlayItem {
        id: "ov_lower_third".to_string(),
        asset_id: "template:lower-third".to_string(),
        overlay_type: "template".to_string(),
        timeline_in_frame: 10,
        timeline_out_frame: 60,
        anchor: "lower_third".to_string(),
        position: NormalizedPosition { x: 0.1, y: 0.72 },
        scale: 1.0,
        opacity: 1.0,
        in_animation: None,
        out_animation: None,
        reason: None,
        title: Some("Jane Doe".to_string()),
        subtitle: Some("Director".to_string()),
    };

    // Render at frame 25 (mid-display)
    render_lower_third(&mut buffer, width, height, 25, &overlay, &font);

    // Check for charcoal background [28, 28, 30] and yellow accent [255, 214, 10]
    let mut has_bg_pixels = false;
    let mut has_accent_pixels = false;

    for chunk in buffer.chunks_exact(4) {
        let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
        if (r as i32 - 28).abs() < 10 && (g as i32 - 28).abs() < 10 && (b as i32 - 30).abs() < 10 {
            has_bg_pixels = true;
        }
        if r > 200 && g > 170 && b < 80 {
            has_accent_pixels = true;
        }
    }

    assert!(
        has_bg_pixels,
        "Lower third background pill must be rendered"
    );
    assert!(
        has_accent_pixels,
        "Lower third yellow accent bar must be rendered"
    );
}

#[test]
fn test_render_keyword_badge_template() {
    // REQ-OVERLAY-001: Keyword badge
    let font = Font::from_bytes(TEST_FONT, FontSettings::default()).unwrap();
    let width = 540u32;
    let height = 960u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    let overlay = OverlayItem {
        id: "ov_badge".to_string(),
        asset_id: "template:keyword-badge".to_string(),
        overlay_type: "template".to_string(),
        timeline_in_frame: 0,
        timeline_out_frame: 30,
        anchor: "top_center".to_string(),
        position: NormalizedPosition { x: 0.5, y: 0.20 },
        scale: 1.0,
        opacity: 1.0,
        in_animation: None,
        out_animation: None,
        reason: Some(OverlayReason {
            word_range: vec![0, 1],
            text: "IMPORTANT".to_string(),
        }),
        title: Some("STEP 1".to_string()),
        subtitle: None,
    };

    render_keyword_badge(&mut buffer, width, height, 15, &overlay, &font);

    let mut has_badge_yellow = false;
    for chunk in buffer.chunks_exact(4) {
        let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
        if r > 200 && g > 170 && b < 80 {
            has_badge_yellow = true;
            break;
        }
    }
    assert!(
        has_badge_yellow,
        "Badge pill must render with high-contrast amber yellow"
    );
}

#[test]
fn test_render_punch_in_zoom() {
    // REQ-OVERLAY-001: Punch-in virtual zoom
    let width = 320u32;
    let height = 240u32;
    let mut buffer = vec![128u8; (width * height * 4) as usize];

    let overlay = OverlayItem {
        id: "ov_zoom".to_string(),
        asset_id: "template:punch-in-zoom".to_string(),
        overlay_type: "template".to_string(),
        timeline_in_frame: 0,
        timeline_out_frame: 30,
        anchor: "center".to_string(),
        position: NormalizedPosition { x: 0.5, y: 0.5 },
        scale: 1.08,
        opacity: 1.0,
        in_animation: None,
        out_animation: None,
        reason: None,
        title: None,
        subtitle: None,
    };

    apply_punch_in_zoom(&mut buffer, width, height, 15, &overlay);
    assert_eq!(buffer.len(), (width * height * 4) as usize);
}

#[test]
fn test_render_animated_progress() {
    // REQ-OVERLAY-001: Progress indicator
    let width = 540u32;
    let height = 960u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    render_animated_progress(&mut buffer, width, height, 45, 90);

    let mut has_progress_yellow = false;
    for chunk in buffer.chunks_exact(4) {
        let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
        if r > 200 && g > 170 && b < 80 {
            has_progress_yellow = true;
            break;
        }
    }
    assert!(
        has_progress_yellow,
        "Animated progress bar must render fill at 50% progression"
    );
}

#[test]
fn test_render_animated_bird_svg() {
    // REQ-OVERLAY-002: Animated SVG vector bird
    let width = 540u32;
    let height = 960u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    let overlay = OverlayItem {
        id: "ov_bird".to_string(),
        asset_id: "svg:bird-flying".to_string(),
        overlay_type: "svg_animation".to_string(),
        timeline_in_frame: 0,
        timeline_out_frame: 60,
        anchor: "top_center".to_string(),
        position: NormalizedPosition { x: 0.5, y: 0.22 },
        scale: 1.0,
        opacity: 1.0,
        in_animation: None,
        out_animation: None,
        reason: Some(OverlayReason {
            word_range: vec![4, 5, 6],
            text: "imagine a bird is flying".to_string(),
        }),
        title: None,
        subtitle: None,
    };

    // Render wing flap frame 0 (wings up)
    render_animated_bird(&mut buffer, width, height, 10, &overlay);

    let mut has_bird_yellow = false;
    for chunk in buffer.chunks_exact(4) {
        let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
        if r > 200 && g > 170 && b < 80 {
            has_bird_yellow = true;
            break;
        }
    }
    assert!(
        has_bird_yellow,
        "Animated vector bird must be rendered into frame buffer (REQ-OVERLAY-002)"
    );
}

#[test]
fn test_overlay_renderer_pipeline_integration() {
    let renderer = OverlayRenderer::new().expect("Renderer creation failed");
    let width = 540u32;
    let height = 960u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    let overlays = vec![OverlayItem {
        id: "ov_bird".to_string(),
        asset_id: "svg:bird-flying".to_string(),
        overlay_type: "svg_animation".to_string(),
        timeline_in_frame: 0,
        timeline_out_frame: 30,
        anchor: "top_center".to_string(),
        position: NormalizedPosition { x: 0.5, y: 0.22 },
        scale: 1.0,
        opacity: 1.0,
        in_animation: None,
        out_animation: None,
        reason: Some(OverlayReason {
            word_range: vec![2, 3],
            text: "flying bird".to_string(),
        }),
        title: None,
        subtitle: None,
    }];

    renderer.render_frame(15, &mut buffer, width, height, &overlays, 60);

    let mut painted_pixels = 0;
    for chunk in buffer.chunks_exact(4) {
        if chunk[0] > 0 || chunk[1] > 0 || chunk[2] > 0 {
            painted_pixels += 1;
        }
    }
    assert!(
        painted_pixels > 100,
        "OverlayRenderer must composite active overlays onto frame"
    );
}
