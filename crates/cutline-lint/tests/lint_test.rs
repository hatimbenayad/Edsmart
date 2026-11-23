use cutline_core::project::{
    CaptionGroup, NormalizedPosition, OverlayItem, OverlayReason, Project, SafeArea, Source,
    VideoSegment, VideoTrack, WordToken,
};
use cutline_core::time::Fps;
use cutline_lint::{lint_project, Severity};

fn create_base_project() -> Project {
    let source = Source {
        id: "src_1".to_string(),
        path: "test.mp4".to_string(),
        duration_frames: 120,
        width: 1080,
        height: 1920,
        fps: Fps::FPS_30,
        audio_sample_rate: 48000,
    };
    Project::new("lint_test", source, "tiktok")
}

#[test]
fn test_lint_clean_project_passes() {
    let mut project = create_base_project();
    project.captions.enabled = true;
    project.captions.groups = vec![CaptionGroup {
        id: "grp_1".to_string(),
        timeline_in_frame: 0,
        timeline_out_frame: 30, // 30 frames >= 10 frames
        words: vec![WordToken {
            text: "Hello".to_string(),
            start_frame: 0,
            end_frame: 15,
            confidence: 0.99,
        }],
    }];
    project.overlays = vec![OverlayItem {
        id: "ov_1".to_string(),
        asset_id: "svg:bird-flying".to_string(),
        overlay_type: "svg_animation".to_string(),
        timeline_in_frame: 35,
        timeline_out_frame: 65,
        anchor: "top_center".to_string(),
        position: NormalizedPosition { x: 0.5, y: 0.22 },
        scale: 1.0,
        opacity: 1.0,
        in_animation: None,
        out_animation: None,
        reason: Some(OverlayReason {
            word_range: vec![0],
            text: "Illustration of bird".to_string(),
        }),
        title: None,
        subtitle: None,
    }];

    let report = lint_project(&project, None);
    assert!(report.valid, "Clean project should pass all linter checks");
    assert_eq!(report.error_count, 0);
}

#[test]
fn test_lint_rule_001_safe_area_violation() {
    // LINT-001: Safe Area Margin Violation
    let mut project = create_base_project();
    project.captions.safe_area = SafeArea {
        top: 20,    // Too small (< 100)
        bottom: 50, // Too small (< 150)
        left: 10,
        right: 10,
    };

    let report = lint_project(&project, None);
    assert!(!report.valid);
    assert!(report.issues.iter().any(|i| i.rule_id == "LINT-001"));
}

#[test]
fn test_lint_rule_002_caption_overlay_collision() {
    // LINT-002: Caption-Overlay Collision
    let mut project = create_base_project();
    project.captions.groups = vec![CaptionGroup {
        id: "grp_collide".to_string(),
        timeline_in_frame: 10,
        timeline_out_frame: 40,
        words: vec![],
    }];
    project.overlays = vec![OverlayItem {
        id: "ov_lower".to_string(),
        asset_id: "template:lower-third".to_string(),
        overlay_type: "template".to_string(),
        timeline_in_frame: 20, // Overlaps [10, 40] in lower third region!
        timeline_out_frame: 50,
        anchor: "lower_third".to_string(),
        position: NormalizedPosition { x: 0.1, y: 0.72 },
        scale: 1.0,
        opacity: 1.0,
        in_animation: None,
        out_animation: None,
        reason: Some(OverlayReason {
            word_range: vec![],
            text: "Lower third speaker".to_string(),
        }),
        title: None,
        subtitle: None,
    }];

    let report = lint_project(&project, None);
    assert!(!report.valid);
    assert!(report.issues.iter().any(|i| i.rule_id == "LINT-002"));
}

#[test]
fn test_lint_rule_003_cut_splits_word() {
    // LINT-003: Cut point inside spoken word
    let mut project = create_base_project();
    // Two video segments with cut point at source frame 45
    project.timeline.video_tracks = vec![VideoTrack {
        id: "v_track".to_string(),
        name: "Video".to_string(),
        segments: vec![
            VideoSegment {
                id: "seg_1".to_string(),
                source_id: "src_1".to_string(),
                source_in_frame: 0,
                source_out_frame: 45, // Cut at 45
                timeline_in_frame: 0,
                timeline_out_frame: 45,
                speed: 1.0,
            },
            VideoSegment {
                id: "seg_2".to_string(),
                source_id: "src_1".to_string(),
                source_in_frame: 60,
                source_out_frame: 120,
                timeline_in_frame: 45,
                timeline_out_frame: 105,
                speed: 1.0,
            },
        ],
    }];

    // Word token spans [40, 50] -> cut at 45 splits it!
    let words = vec![WordToken {
        text: "splitting".to_string(),
        start_frame: 40,
        end_frame: 50,
        confidence: 0.95,
    }];

    let report = lint_project(&project, Some(&words));
    assert!(!report.valid);
    assert!(report.issues.iter().any(|i| i.rule_id == "LINT-003"));
}

#[test]
fn test_lint_rule_006_caption_group_too_fast() {
    // LINT-006: Caption group visible for < 10 frames
    let mut project = create_base_project();
    project.captions.groups = vec![CaptionGroup {
        id: "fast_grp".to_string(),
        timeline_in_frame: 10,
        timeline_out_frame: 15, // Only 5 frames!
        words: vec![],
    }];

    let report = lint_project(&project, None);
    assert!(report.issues.iter().any(|i| i.rule_id == "LINT-006"));
    assert_eq!(
        report
            .issues
            .iter()
            .find(|i| i.rule_id == "LINT-006")
            .unwrap()
            .severity,
        Severity::Warning
    );
}

#[test]
fn test_lint_rule_007_missing_overlay_rationale() {
    // LINT-007: Overlay without semantic rationale
    let mut project = create_base_project();
    project.overlays = vec![OverlayItem {
        id: "ov_no_reason".to_string(),
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
        reason: None, // Missing reason!
        title: None,
        subtitle: None,
    }];

    let report = lint_project(&project, None);
    assert!(report.issues.iter().any(|i| i.rule_id == "LINT-007"));
}

#[test]
fn test_lint_rule_008_non_compliant_palette() {
    // LINT-008: Non-compliant color palette
    let mut project = create_base_project();
    project.captions.style_id = "neon-gradient-wild".to_string();

    let report = lint_project(&project, None);
    assert!(!report.valid);
    assert!(report.issues.iter().any(|i| i.rule_id == "LINT-008"));
}
