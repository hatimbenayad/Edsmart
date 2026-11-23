use cutline_core::project::{CaptionGroup, CaptionsConfig, SafeArea, WordToken};
use cutline_core::time::Fps;
use cutline_render::caption::{
    ease_out_back, group_words_into_captions, CaptionRenderer, CaptionStyle, FontManager,
};

#[test]
fn test_font_manager_initialization() {
    let font_mgr = FontManager::new().expect("Failed to initialize FontManager with bundled fonts");
    let inter = font_mgr.get_font("Inter");
    assert!(inter.metrics('A', 40.0).advance_width > 0.0);

    let montserrat = font_mgr.get_font("Montserrat");
    assert!(montserrat.metrics('A', 40.0).advance_width > 0.0);
}

#[test]
fn test_smart_caption_word_grouping_punctuation_and_count() {
    // REQ-CAP-001: Grouping 2-4 words, splitting on punctuation and pauses
    let fps = Fps::FPS_30;
    let style = CaptionStyle::clean_yellow();

    let words = vec![
        WordToken {
            text: "Hello".to_string(),
            start_frame: 0,
            end_frame: 8,
            confidence: 0.95,
        },
        WordToken {
            text: "world,".to_string(), // Ends with comma -> should split here
            start_frame: 9,
            end_frame: 16,
            confidence: 0.92,
        },
        WordToken {
            text: "this".to_string(),
            start_frame: 18,
            end_frame: 24,
            confidence: 0.98,
        },
        WordToken {
            text: "is".to_string(),
            start_frame: 25,
            end_frame: 30,
            confidence: 0.99,
        },
        WordToken {
            text: "Cutline".to_string(),
            start_frame: 31,
            end_frame: 42,
            confidence: 0.97,
        },
        WordToken {
            text: "video".to_string(),
            start_frame: 43,
            end_frame: 50,
            confidence: 0.96,
        },
        WordToken {
            text: "editor.".to_string(), // Ends with period -> should split here
            start_frame: 51,
            end_frame: 60,
            confidence: 0.99,
        },
    ];

    let groups = group_words_into_captions(&words, fps, &style);
    assert!(!groups.is_empty(), "Should generate at least one group");

    // Group 1 should contain "Hello" and "world,"
    assert_eq!(groups[0].words.len(), 2);
    assert_eq!(groups[0].words[0].text, "Hello");
    assert_eq!(groups[0].words[1].text, "world,");

    // Ensure groups do not overlap
    for i in 0..groups.len().saturating_sub(1) {
        assert!(
            groups[i].timeline_out_frame <= groups[i + 1].timeline_in_frame,
            "Group {} out ({}) overlaps Group {} in ({})",
            i,
            groups[i].timeline_out_frame,
            i + 1,
            groups[i + 1].timeline_in_frame
        );
    }

    // Ensure all words are accounted for
    let total_grouped_words: usize = groups.iter().map(|g| g.words.len()).sum();
    assert_eq!(total_grouped_words, words.len());
}

#[test]
fn test_smart_caption_word_grouping_pause_split() {
    let fps = Fps::FPS_30;
    let style = CaptionStyle::clean_yellow();

    // Word 1 ends at frame 10; Word 2 starts at frame 25 (gap = 15 frames = 500ms > 200ms pause)
    let words = vec![
        WordToken {
            text: "First".to_string(),
            start_frame: 0,
            end_frame: 10,
            confidence: 0.95,
        },
        WordToken {
            text: "Second".to_string(),
            start_frame: 25,
            end_frame: 35,
            confidence: 0.95,
        },
    ];

    let groups = group_words_into_captions(&words, fps, &style);
    assert_eq!(groups.len(), 2, "Should split across 500ms pause");
    assert_eq!(groups[0].words.len(), 1);
    assert_eq!(groups[1].words.len(), 1);
}

#[test]
fn test_ease_out_back_curve() {
    // REQ-DESIGN-001: Easing curve verification
    let t0 = ease_out_back(0.0);
    assert!((t0 - 0.0).abs() < 1e-4);

    let t1 = ease_out_back(1.0);
    assert!((t1 - 1.0).abs() < 1e-4);

    // Overshoot: midway value around t=0.7 should exceed 1.0
    let t_mid = ease_out_back(0.7);
    assert!(t_mid > 1.0, "EaseOutBack should overshoot 1.0");
}

#[test]
fn test_caption_rasterization_active_yellow_and_inactive_white() {
    // REQ-CAP-002, REQ-CAP-003, REQ-CAP-004
    let style = CaptionStyle::clean_yellow();
    let renderer = CaptionRenderer::new(style.clone()).expect("Renderer init failed");

    let width = 540u32;
    let height = 960u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize]; // Black background

    let group = CaptionGroup {
        id: "test_grp".to_string(),
        timeline_in_frame: 0,
        timeline_out_frame: 30,
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
        ],
    };

    let config = CaptionsConfig {
        enabled: true,
        style_id: "clean-yellow".to_string(),
        safe_area: SafeArea::default(),
        groups: vec![group],
    };

    // Render at frame 5: "HELLO" is active (yellow #FFD60A), "WORLD" is inactive (white #FFFFFF)
    renderer.render_frame(5, &mut buffer, width, height, &config);

    // Verify presence of yellow pixels (#FFD60A: R~255, G~214, B~10)
    let mut has_yellow_pixel = false;
    let mut has_white_pixel = false;
    let mut has_outline_pixel = false;

    for chunk in buffer.chunks_exact(4) {
        let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
        // Yellow active word pixel check
        if r > 200 && g > 170 && b < 80 {
            has_yellow_pixel = true;
        }
        // White inactive word pixel check
        if r > 220 && g > 220 && b > 220 {
            has_white_pixel = true;
        }
        // Dark stroke outline check
        if r > 5 && r < 40 && g > 5 && g < 40 && b > 5 && b < 40 {
            has_outline_pixel = true;
        }
    }

    assert!(
        has_yellow_pixel,
        "Rendered frame must contain warm amber yellow pixels for active word (REQ-CAP-003)"
    );
    assert!(
        has_white_pixel,
        "Rendered frame must contain pure white pixels for inactive word (REQ-CAP-003)"
    );
    assert!(
        has_outline_pixel,
        "Rendered frame must contain dark outline stroke pixels for contrast"
    );
}
