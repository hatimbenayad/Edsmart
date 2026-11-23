use crate::svg::render_animated_bird;
use crate::templates::{
    apply_punch_in_zoom, render_animated_progress, render_camera_viewfinder, render_keyword_badge,
    render_lower_third,
};
use cutline_core::project::OverlayItem;
use fontdue::{Font, FontSettings};

pub static INTER_BOLD: &[u8] = include_bytes!("../../cutline-render/assets/fonts/Inter-Bold.ttf");

pub struct OverlayRenderer {
    font: Font,
}

impl Default for OverlayRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to initialize OverlayRenderer font")
    }
}

impl OverlayRenderer {
    pub fn new() -> Result<Self, String> {
        let font = Font::from_bytes(INTER_BOLD, FontSettings::default())
            .map_err(|e| format!("Font parse error: {e}"))?;
        Ok(Self { font })
    }

    /// Renders all active overlays onto the RGBA frame buffer for the given timeline frame
    pub fn render_frame(
        &self,
        frame_idx: i64,
        buffer: &mut [u8],
        width: u32,
        height: u32,
        overlays: &[OverlayItem],
        total_frames: i64,
    ) {
        // 1. Process punch-in zooms first (transforms underlying pixels before drawing overlays)
        for overlay in overlays {
            if overlay.asset_id.contains("punch-in-zoom")
                && frame_idx >= overlay.timeline_in_frame
                && frame_idx < overlay.timeline_out_frame
            {
                apply_punch_in_zoom(buffer, width, height, frame_idx, overlay);
            }
        }

        // 2. Render templates and vector graphics on top of video frame
        for overlay in overlays {
            if frame_idx < overlay.timeline_in_frame || frame_idx >= overlay.timeline_out_frame {
                continue;
            }

            let asset_key = overlay.asset_id.to_lowercase();
            if asset_key.contains("viewfinder") || asset_key.contains("camera") {
                render_camera_viewfinder(buffer, width, height, frame_idx, overlay, &self.font);
            } else if asset_key.contains("lower-third") || asset_key.contains("lower_third") {
                render_lower_third(buffer, width, height, frame_idx, overlay, &self.font);
            } else if asset_key.contains("badge") || asset_key.contains("keyword") {
                render_keyword_badge(buffer, width, height, frame_idx, overlay, &self.font);
            } else if asset_key.contains("progress") {
                render_animated_progress(buffer, width, height, frame_idx, total_frames);
            } else if asset_key.contains("bird") {
                render_animated_bird(buffer, width, height, frame_idx, overlay);
            }
        }
    }
}
