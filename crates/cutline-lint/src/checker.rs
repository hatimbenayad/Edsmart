use crate::rules::{LintIssue, Severity};
use cutline_core::project::{Project, WordToken};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LintReport {
    pub valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub issues: Vec<LintIssue>,
}

pub fn lint_project(project: &Project, transcript_words: Option<&[WordToken]>) -> LintReport {
    let mut issues = Vec::new();

    // 1. LINT-001: Safe Area Caption Violation
    let safe_area = &project.captions.safe_area;
    if safe_area.left < 50 || safe_area.right < 50 || safe_area.top < 100 || safe_area.bottom < 150
    {
        issues.push(LintIssue {
            rule_id: "LINT-001".to_string(),
            severity: Severity::Error,
            message: format!(
                "Safe area margins [T:{}, B:{}, L:{}, R:{}] violate minimum platform bounds",
                safe_area.top, safe_area.bottom, safe_area.left, safe_area.right
            ),
            frame: None,
            hint: "Restore standard safe area margins (default top:200, bottom:380, left:90, right:130).".to_string(),
        });
    }

    // 2. LINT-002: Caption-Overlay Collision
    for overlay in &project.overlays {
        let is_lower_zone = overlay.anchor == "lower_third"
            || overlay.anchor == "bottom_center"
            || overlay.position.y >= 0.58;

        if is_lower_zone && project.captions.enabled {
            for group in &project.captions.groups {
                let overlap_start = overlay.timeline_in_frame.max(group.timeline_in_frame);
                let overlap_end = overlay.timeline_out_frame.min(group.timeline_out_frame);

                if overlap_start < overlap_end {
                    issues.push(LintIssue {
                        rule_id: "LINT-002".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "Overlay '{}' in lower zone collides with active caption '{}'",
                            overlay.id, group.id
                        ),
                        frame: Some(overlap_start),
                        hint: "Change overlay anchor to 'top_center' or offset its frame timing to avoid collision.".to_string(),
                    });
                    break;
                }
            }
        }
    }

    // 3. LINT-003: Cut Splits Word
    if let Some(words) = transcript_words {
        if let Some(track) = project.timeline.video_tracks.first() {
            for i in 0..track.segments.len().saturating_sub(1) {
                let cut_out = track.segments[i].source_out_frame;
                let next_in = track.segments[i + 1].source_in_frame;

                for word in words {
                    if cut_out > word.start_frame && cut_out < word.end_frame {
                        issues.push(LintIssue {
                            rule_id: "LINT-003".to_string(),
                            severity: Severity::Error,
                            message: format!(
                                "Timeline cut at source frame {} splits spoken word '{}' [{}-{}]",
                                cut_out, word.text, word.start_frame, word.end_frame
                            ),
                            frame: Some(track.segments[i].timeline_out_frame),
                            hint: "Shift cut point forward or backward into silence padding between words.".to_string(),
                        });
                    }
                    if next_in > word.start_frame && next_in < word.end_frame {
                        issues.push(LintIssue {
                            rule_id: "LINT-003".to_string(),
                            severity: Severity::Error,
                            message: format!(
                                "Timeline segment start at source frame {} splits spoken word '{}' [{}-{}]",
                                next_in, word.text, word.start_frame, word.end_frame
                            ),
                            frame: Some(track.segments[i + 1].timeline_in_frame),
                            hint: "Shift in-point into silence padding before word starts.".to_string(),
                        });
                    }
                }
            }
        }
    }

    // 4. LINT-004: Text Legibility Minimum
    // Check against standard minimum size
    let style_id = &project.captions.style_id;
    if style_id == "tiny" || style_id == "too-small" {
        issues.push(LintIssue {
            rule_id: "LINT-004".to_string(),
            severity: Severity::Warning,
            message: format!("Caption style '{}' has insufficient font size", style_id),
            frame: None,
            hint:
                "Use caption styles with font sizes >= 64px on 1080p canvas for mobile legibility."
                    .to_string(),
        });
    }

    // 5. LINT-005: Overlay Congestion
    let total_frames = project.timeline.total_duration_frames();
    if project.overlays.len() > 1 && total_frames > 0 {
        // Sample at 10-frame intervals to check simultaneous overlay count
        for f in (0..total_frames).step_by(10) {
            let active_count = project
                .overlays
                .iter()
                .filter(|o| !o.asset_id.contains("punch-in-zoom") && !o.asset_id.contains("viewfinder"))
                .filter(|o| f >= o.timeline_in_frame && f < o.timeline_out_frame)
                .count();

            if active_count > 2 {
                issues.push(LintIssue {
                    rule_id: "LINT-005".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Overlay congestion detected: {} overlays active simultaneously at frame {}",
                        active_count, f
                    ),
                    frame: Some(f),
                    hint: "Limit concurrent visual elements to maintain professional clarity.".to_string(),
                });
                break;
            }
        }
    }

    // 6. LINT-006: Caption Group Too Fast
    for group in &project.captions.groups {
        let span = group.timeline_out_frame - group.timeline_in_frame;
        if span < 10 {
            // Under 333ms at 30fps
            issues.push(LintIssue {
                rule_id: "LINT-006".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Caption group '{}' is on screen for only {} frames (< 333ms)",
                    group.id, span
                ),
                frame: Some(group.timeline_in_frame),
                hint: "Ensure caption groups remain visible for at least 400ms (12 frames) to allow reading.".to_string(),
            });
        }
    }

    // 7. LINT-007: Missing Overlay Rationale
    for overlay in &project.overlays {
        let has_rationale = overlay
            .reason
            .as_ref()
            .map(|r| !r.text.trim().is_empty())
            .unwrap_or(false);

        if !has_rationale {
            issues.push(LintIssue {
                rule_id: "LINT-007".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Overlay '{}' lacks semantic rationale explaining why it is displayed",
                    overlay.id
                ),
                frame: Some(overlay.timeline_in_frame),
                hint:
                    "Add a --reason parameter linking the visual choice to the spoken transcript."
                        .to_string(),
            });
        }
    }

    // 8. LINT-008: Non-Compliant Palette
    let approved_palettes = ["clean-yellow", "mono-warm", "ink-and-yellow", "soft-pastel"];
    if !approved_palettes.contains(&project.captions.style_id.as_str())
        && !project.captions.style_id.starts_with('{')
    {
        issues.push(LintIssue {
            rule_id: "LINT-008".to_string(),
            severity: Severity::Error,
            message: format!(
                "Caption style '{}' is not an approved brand design palette",
                project.captions.style_id
            ),
            frame: None,
            hint:
                "Select an approved palette: clean-yellow, mono-warm, ink-and-yellow, soft-pastel."
                    .to_string(),
        });
    }

    let error_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();

    LintReport {
        valid: error_count == 0,
        error_count,
        warning_count,
        issues,
    }
}
