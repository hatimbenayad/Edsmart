use crate::easing::ease_out_back;
use cutline_core::project::OverlayItem;
use tiny_skia::{PixmapMut, Transform};
use usvg::{Options, Tree};

pub static BIRD_WING_UP_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 80" width="120" height="80">
  <path d="M 20 50 C 35 20, 60 10, 80 35 C 95 20, 110 25, 115 35 C 100 45, 75 55, 55 52 C 40 55, 25 55, 20 50 Z" fill="#FFD60A" stroke="#111111" stroke-width="3" stroke-linejoin="round"/>
  <circle cx="105" cy="32" r="3" fill="#111111"/>
  <path d="M 50 35 C 45 10, 65 5, 75 15 C 65 25, 55 30, 50 35 Z" fill="#FFE600" stroke="#111111" stroke-width="2.5" stroke-linejoin="round"/>
</svg>"##;

pub static BIRD_WING_FLAT_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 80" width="120" height="80">
  <path d="M 20 45 C 35 30, 60 25, 80 40 C 95 30, 110 32, 115 40 C 100 50, 75 58, 55 54 C 40 56, 25 52, 20 45 Z" fill="#FFD60A" stroke="#111111" stroke-width="3" stroke-linejoin="round"/>
  <circle cx="105" cy="38" r="3" fill="#111111"/>
  <path d="M 45 42 C 30 38, 50 30, 75 38 C 60 42, 50 44, 45 42 Z" fill="#FFE600" stroke="#111111" stroke-width="2.5" stroke-linejoin="round"/>
</svg>"##;

pub static BIRD_WING_DOWN_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 80" width="120" height="80">
  <path d="M 20 40 C 35 35, 60 35, 80 42 C 95 35, 110 38, 115 45 C 100 55, 75 60, 55 55 C 40 58, 25 50, 20 40 Z" fill="#FFD60A" stroke="#111111" stroke-width="3" stroke-linejoin="round"/>
  <circle cx="105" cy="42" r="3" fill="#111111"/>
  <path d="M 45 45 C 40 68, 60 70, 70 50 C 60 48, 50 46, 45 45 Z" fill="#FFE600" stroke="#111111" stroke-width="2.5" stroke-linejoin="round"/>
</svg>"##;

/// Renders a dynamic vector animated flying bird (REQ-OVERLAY-002)
pub fn render_animated_bird(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    frame_idx: i64,
    overlay: &OverlayItem,
) {
    let in_f = overlay.timeline_in_frame;
    let out_f = overlay.timeline_out_frame;
    if frame_idx < in_f || frame_idx >= out_f {
        return;
    }

    let rel_frame = frame_idx - in_f;
    let total_span = (out_f - in_f).max(1);
    let scale_res = width as f32 / 1080.0;

    // Wing flapping cycle: 4 frames per wing position (16 frames per full cycle)
    let wing_state = ((rel_frame / 4) % 4) as usize;
    let svg_xml = match wing_state {
        0 => BIRD_WING_UP_SVG,
        1 => BIRD_WING_FLAT_SVG,
        2 => BIRD_WING_DOWN_SVG,
        _ => BIRD_WING_FLAT_SVG,
    };

    let opt = Options::default();
    let tree = match Tree::from_str(svg_xml, &opt) {
        Ok(t) => t,
        Err(_) => return,
    };

    let in_duration = 10.min(total_span / 2);
    let out_duration = 10.min(total_span / 2);

    let anim_scale = if rel_frame < in_duration {
        let t = rel_frame as f32 / in_duration as f32;
        ease_out_back(t)
    } else if rel_frame >= total_span - out_duration {
        let t = (rel_frame - (total_span - out_duration)) as f32 / out_duration as f32;
        1.0 - t * 0.3
    } else {
        1.0
    };

    let base_scale = overlay.scale * scale_res * anim_scale * 1.5;
    let bobbing = (rel_frame as f32 * 0.25).sin() * 12.0 * scale_res;
    let gliding_x = (rel_frame as f32 - (total_span as f32 / 2.0)) * 2.5 * scale_res;

    let center_x = width as f32 * overlay.position.x + gliding_x;
    let center_y = height as f32 * overlay.position.y.clamp(0.12, 0.40) + bobbing;

    let bird_w = 120.0 * base_scale;
    let bird_h = 80.0 * base_scale;
    let bird_x = center_x - (bird_w / 2.0);
    let bird_y = center_y - (bird_h / 2.0);

    let transform = Transform::from_translate(bird_x, bird_y).post_scale(base_scale, base_scale);

    if let Some(mut pixmap) = PixmapMut::from_bytes(buffer, width, height) {
        resvg::render(&tree, transform, &mut pixmap);
    }
}
