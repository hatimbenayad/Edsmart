use crate::easing::{ease_in_cubic, ease_out_back, ease_out_cubic};
use cutline_core::project::OverlayItem;
use fontdue::Font;

/// Renders a procedural studio-grade lower-third pill (REQ-OVERLAY-001)
pub fn render_lower_third(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    frame_idx: i64,
    overlay: &OverlayItem,
    font: &Font,
) {
    let in_f = overlay.timeline_in_frame;
    let out_f = overlay.timeline_out_frame;
    if frame_idx < in_f || frame_idx >= out_f {
        return;
    }

    let rel_frame = frame_idx - in_f;
    let total_span = (out_f - in_f).max(1);
    let scale = width as f32 / 1080.0;

    // Transition animations: slide-in from left (15 frames), fade/slide-out (12 frames)
    let in_duration = 15.min(total_span / 2);
    let out_duration = 12.min(total_span / 2);

    let (slide_x, alpha_factor) = if rel_frame < in_duration {
        let t = rel_frame as f32 / in_duration as f32;
        let slide = (1.0 - ease_out_cubic(t)) * -300.0 * scale;
        (slide, ease_out_cubic(t))
    } else if rel_frame >= total_span - out_duration {
        let t = (rel_frame - (total_span - out_duration)) as f32 / out_duration as f32;
        let slide = ease_in_cubic(t) * -200.0 * scale;
        (slide, 1.0 - ease_in_cubic(t))
    } else {
        (0.0, 1.0)
    };

    let title = overlay.title.as_deref().unwrap_or("Speaker Name");
    let subtitle = overlay
        .subtitle
        .as_deref()
        .unwrap_or("Cutline Video Editor");

    let title_size = 36.0 * scale;
    let subtitle_size = 22.0 * scale;

    let mut title_width = 0.0f32;
    for ch in title.chars() {
        title_width += font.metrics(ch, title_size).advance_width;
    }
    let mut subtitle_width = 0.0f32;
    for ch in subtitle.chars() {
        subtitle_width += font.metrics(ch, subtitle_size).advance_width;
    }

    let max_text_w = title_width.max(subtitle_width);
    let pad_left = 34.0 * scale;
    let pad_right = 36.0 * scale;
    let pill_w = ((max_text_w + pad_left + pad_right).round() as i32).min((width as f32 - 160.0 * scale) as i32);
    let pill_h = (100.0 * scale).round() as i32;
    let pill_x = (90.0 * scale + slide_x).round() as i32;
    let pill_y = (height as f32 * 0.72).round() as i32; // Lower safe area
    let radius = (18.0 * scale).round() as i32;

    // Drop shadow
    let shadow_color = [0, 0, 0, (120.0 * alpha_factor * overlay.opacity) as u8];
    draw_rounded_rect(
        buffer, width, height, pill_x + 4, pill_y + 6, pill_w, pill_h, radius, shadow_color,
    );

    // Background pill fill: #18181A at 90% opacity
    let bg_color = [
        24,
        24,
        26,
        (230.0 * alpha_factor * overlay.opacity).clamp(0.0, 255.0) as u8,
    ];
    draw_rounded_rect(
        buffer, width, height, pill_x, pill_y, pill_w, pill_h, radius, bg_color,
    );

    // Left accent vertical bar: #FFD60A
    let accent_w = (8.0 * scale).max(4.0) as i32;
    let accent_color = [
        255,
        214,
        10,
        (240.0 * alpha_factor * overlay.opacity).clamp(0.0, 255.0) as u8,
    ];
    draw_rounded_rect(
        buffer,
        width,
        height,
        pill_x + (8.0 * scale) as i32,
        pill_y + (12.0 * scale) as i32,
        accent_w,
        pill_h - (24.0 * scale) as i32,
        (4.0 * scale) as i32,
        accent_color,
    );

    // Text rendering: Title (White) and Subtitle (Yellow #FFD60A)
    let text_x = pill_x + pad_left as i32;
    let title_y = pill_y + (42.0 * scale) as i32;
    let sub_y = pill_y + (76.0 * scale) as i32;

    let text_alpha = (255.0 * alpha_factor * overlay.opacity).clamp(0.0, 255.0) as u8;
    draw_text(
        buffer,
        width,
        height,
        font,
        title,
        title_size,
        text_x,
        title_y,
        [255, 255, 255, text_alpha],
    );
    draw_text(
        buffer,
        width,
        height,
        font,
        subtitle,
        subtitle_size,
        text_x,
        sub_y,
        [255, 214, 10, text_alpha],
    );
}

/// Renders a high-contrast keyword callout badge in upper safe area (REQ-OVERLAY-001)
pub fn render_keyword_badge(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    frame_idx: i64,
    overlay: &OverlayItem,
    font: &Font,
) {
    let in_f = overlay.timeline_in_frame;
    let out_f = overlay.timeline_out_frame;
    if frame_idx < in_f || frame_idx >= out_f {
        return;
    }

    let rel_frame = frame_idx - in_f;
    let total_span = (out_f - in_f).max(1);
    let scale = width as f32 / 1080.0;

    // Pop-in animation with EaseOutBack (8 frames)
    let pop_duration = 8.min(total_span / 2);
    let out_duration = 8.min(total_span / 2);

    let (badge_scale, alpha_factor) = if rel_frame < pop_duration {
        let t = rel_frame as f32 / pop_duration as f32;
        (ease_out_back(t), t.clamp(0.0, 1.0))
    } else if rel_frame >= total_span - out_duration {
        let t = (rel_frame - (total_span - out_duration)) as f32 / out_duration as f32;
        (1.0 - t * 0.2, 1.0 - t)
    } else {
        (1.0, 1.0)
    };

    let text = overlay
        .title
        .as_deref()
        .or_else(|| overlay.reason.as_ref().map(|r| r.text.as_str()))
        .unwrap_or("IMPORTANT")
        .to_uppercase();

    let font_size = 30.0 * scale * badge_scale;
    let center_x = width as f32 * overlay.position.x;
    let center_y = height as f32 * overlay.position.y.clamp(0.12, 0.25);
    let fill_alpha = (245.0 * alpha_factor * overlay.opacity).clamp(0.0, 255.0) as u8;
    let shadow_alpha = (120.0 * alpha_factor * overlay.opacity).clamp(0.0, 255.0) as u8;

    // Check for segmented two-tone badge format: "PREFIX: SUFFIX"
    if let Some((p, s)) = text.split_once(": ") {
        let mut p_w = 0.0f32;
        for ch in p.chars() {
            p_w += font.metrics(ch, font_size).advance_width;
        }
        let mut s_w = 0.0f32;
        for ch in s.chars() {
            s_w += font.metrics(ch, font_size).advance_width;
        }

        let pad_h = 24.0 * scale * badge_scale;
        let pad_v = 14.0 * scale * badge_scale;
        let p_pill_w = (p_w + pad_h * 2.0).round() as i32;
        let s_pill_w = (s_w + pad_h * 2.0).round() as i32;
        let gap = (8.0 * scale).round() as i32;
        let total_w = p_pill_w + gap + s_pill_w;
        let pill_h = (font_size + pad_v * 1.8).round() as i32;
        let radius = pill_h / 2;

        let start_x = (center_x - (total_w as f32 / 2.0)).round() as i32;
        let pill_y = (center_y - (pill_h as f32 / 2.0)).round() as i32;

        // Pill 1 (Tag, e.g. "STEP 1"): Dark glass backing with white text
        draw_rounded_rect(buffer, width, height, start_x + 3, pill_y + 4, p_pill_w, pill_h, radius, [0, 0, 0, shadow_alpha]);
        draw_rounded_rect(buffer, width, height, start_x, pill_y, p_pill_w, pill_h, radius, [28, 28, 32, (235.0 * alpha_factor * overlay.opacity) as u8]);
        let tx1 = start_x + ((p_pill_w as f32 - p_w) / 2.0).round() as i32;
        draw_text(buffer, width, height, font, p, font_size, tx1, pill_y + (pill_h as f32 * 0.72).round() as i32, [255, 255, 255, fill_alpha]);

        // Pill 2 (Content, e.g. "250g CRUSHED BISCUITS"): Warm Amber yellow with dark text
        let p2_x = start_x + p_pill_w + gap;
        draw_rounded_rect(buffer, width, height, p2_x + 3, pill_y + 4, s_pill_w, pill_h, radius, [0, 0, 0, shadow_alpha]);
        draw_rounded_rect(buffer, width, height, p2_x, pill_y, s_pill_w, pill_h, radius, [255, 214, 10, fill_alpha]);
        let tx2 = p2_x + ((s_pill_w as f32 - s_w) / 2.0).round() as i32;
        draw_text(buffer, width, height, font, s, font_size, tx2, pill_y + (pill_h as f32 * 0.72).round() as i32, [17, 17, 17, fill_alpha]);
    } else {
        let mut text_width = 0.0f32;
        for ch in text.chars() {
            text_width += font.metrics(ch, font_size).advance_width;
        }

        let pad_h = 28.0 * scale * badge_scale;
        let pad_v = 14.0 * scale * badge_scale;
        let pill_w = (text_width + pad_h * 2.0).round() as i32;
        let pill_h = (font_size + pad_v * 1.8).round() as i32;
        let radius = pill_h / 2;

        let pill_x = (center_x - (pill_w as f32 / 2.0)).round() as i32;
        let pill_y = (center_y - (pill_h as f32 / 2.0)).round() as i32;

        // Badge drop shadow & fill
        draw_rounded_rect(buffer, width, height, pill_x + 3, pill_y + 4, pill_w, pill_h, radius, [0, 0, 0, shadow_alpha]);
        draw_rounded_rect(buffer, width, height, pill_x, pill_y, pill_w, pill_h, radius, [255, 214, 10, fill_alpha]);

        let text_x = pill_x + ((pill_w as f32 - text_width) / 2.0).round() as i32;
        let text_y = pill_y + (pill_h as f32 * 0.72).round() as i32;
        draw_text(buffer, width, height, font, &text, font_size, text_x, text_y, [17, 17, 17, fill_alpha]);
    }
}

/// Renders a subtle 6px progress bar along top safe margin (REQ-OVERLAY-001)
pub fn render_animated_progress(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    frame_idx: i64,
    total_frames: i64,
) {
    if total_frames <= 0 {
        return;
    }

    let scale = width as f32 / 1080.0;
    let safe_left = (90.0 * scale) as i32;
    let safe_right = (130.0 * scale) as i32;
    let bar_w = (width as i32 - safe_left - safe_right).max(10);
    let bar_h = (6.0 * scale).max(3.0) as i32;
    let bar_y = (height as f32 * 0.11).round() as i32; // Just below top UI margin

    // Background track: #3A3A3C at 50% opacity
    draw_rounded_rect(
        buffer,
        width,
        height,
        safe_left,
        bar_y,
        bar_w,
        bar_h,
        bar_h / 2,
        [58, 58, 60, 130],
    );

    // Active progress fill: #FFD60A
    let progress = (frame_idx as f32 / total_frames as f32).clamp(0.0, 1.0);
    let fill_w = ((bar_w as f32) * progress).round() as i32;
    if fill_w > 0 {
        draw_rounded_rect(
            buffer,
            width,
            height,
            safe_left,
            bar_y,
            fill_w,
            bar_h,
            bar_h / 2,
            [255, 214, 10, 240],
        );
    }
}

/// Applies virtual camera punch-in zoom (1.08x) (REQ-OVERLAY-001)
pub fn apply_punch_in_zoom(
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
    let transition_frames = 6.min(total_span / 2);

    let max_zoom = 1.08f32;
    let zoom_factor = if rel_frame < transition_frames {
        let t = rel_frame as f32 / transition_frames as f32;
        1.0 + (max_zoom - 1.0) * ease_out_cubic(t)
    } else if rel_frame >= total_span - transition_frames {
        let t = (rel_frame - (total_span - transition_frames)) as f32 / transition_frames as f32;
        max_zoom - (max_zoom - 1.0) * ease_in_cubic(t)
    } else {
        max_zoom
    };

    if (zoom_factor - 1.0).abs() < 0.005 {
        return;
    }

    // Centered crop and scale back to original dimensions
    let crop_w = (width as f32 / zoom_factor).round() as u32;
    let crop_h = (height as f32 / zoom_factor).round() as u32;
    let crop_x = ((width - crop_w) / 2) as usize;
    let crop_y = ((height - crop_h) / 2) as usize;

    let temp = buffer.to_vec();

    for y in 0..height {
        let src_y = crop_y + ((y as f32 / height as f32) * crop_h as f32) as usize;
        let dst_row_idx = (y * width * 4) as usize;
        let src_row_idx = (src_y as u32 * width * 4) as usize;

        for x in 0..width {
            let src_x = crop_x + ((x as f32 / width as f32) * crop_w as f32) as usize;
            let dst_px = dst_row_idx + (x * 4) as usize;
            let src_px = src_row_idx + (src_x * 4);

            buffer[dst_px..dst_px + 4].copy_from_slice(&temp[src_px..src_px + 4]);
        }
    }
}

/// Helper: Draws a rounded rectangle onto RGBA frame buffer with alpha blending
#[allow(clippy::too_many_arguments)]
fn draw_rounded_rect(
    buffer: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    radius: i32,
    color: [u8; 4],
) {
    let r2 = radius * radius;
    let alpha_src = color[3] as u32;
    if alpha_src == 0 {
        return;
    }

    for py in y..y + h {
        if py < 0 || py >= canvas_h as i32 {
            continue;
        }

        for px in x..x + w {
            if px < 0 || px >= canvas_w as i32 {
                continue;
            }

            // Check corner rounding
            let mut inside = true;
            if px < x + radius && py < y + radius {
                let dx = x + radius - px;
                let dy = y + radius - py;
                inside = dx * dx + dy * dy <= r2;
            } else if px >= x + w - radius && py < y + radius {
                let dx = px - (x + w - radius);
                let dy = y + radius - py;
                inside = dx * dx + dy * dy <= r2;
            } else if px < x + radius && py >= y + h - radius {
                let dx = x + radius - px;
                let dy = py - (y + h - radius);
                inside = dx * dx + dy * dy <= r2;
            } else if px >= x + w - radius && py >= y + h - radius {
                let dx = px - (x + w - radius);
                let dy = py - (y + h - radius);
                inside = dx * dx + dy * dy <= r2;
            }

            if !inside {
                continue;
            }

            let idx = ((py as u32 * canvas_w + px as u32) * 4) as usize;
            let inv_a = 255 - alpha_src;
            buffer[idx] = ((color[0] as u32 * alpha_src + buffer[idx] as u32 * inv_a) / 255) as u8;
            buffer[idx + 1] =
                ((color[1] as u32 * alpha_src + buffer[idx + 1] as u32 * inv_a) / 255) as u8;
            buffer[idx + 2] =
                ((color[2] as u32 * alpha_src + buffer[idx + 2] as u32 * inv_a) / 255) as u8;
            buffer[idx + 3] = 255;
        }
    }
}

/// Helper: Draws simple text string with fontdue
#[allow(clippy::too_many_arguments)]
fn draw_text(
    buffer: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    font: &Font,
    text: &str,
    font_size: f32,
    start_x: i32,
    start_y: i32,
    color: [u8; 4],
) {
    let mut cur_x = start_x as f32;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        let advance = metrics.advance_width;

        if metrics.width > 0 && metrics.height > 0 {
            let gx = (cur_x + metrics.bounds.xmin).round() as i32;
            let gy =
                (start_y as f32 - (metrics.bounds.ymin + metrics.bounds.height)).round() as i32;

            for row in 0..metrics.height as i32 {
                let py = gy + row;
                if py < 0 || py >= canvas_h as i32 {
                    continue;
                }
                for col in 0..metrics.width as i32 {
                    let px = gx + col;
                    if px < 0 || px >= canvas_w as i32 {
                        continue;
                    }
                    let alpha_mask = bitmap[row as usize * metrics.width + col as usize];
                    if alpha_mask == 0 {
                        continue;
                    }

                    let eff_a = (color[3] as u32 * alpha_mask as u32) / 255;
                    let inv_a = 255 - eff_a;
                    let idx = ((py as u32 * canvas_w + px as u32) * 4) as usize;

                    buffer[idx] =
                        ((color[0] as u32 * eff_a + buffer[idx] as u32 * inv_a) / 255) as u8;
                    buffer[idx + 1] =
                        ((color[1] as u32 * eff_a + buffer[idx + 1] as u32 * inv_a) / 255) as u8;
                    buffer[idx + 2] =
                        ((color[2] as u32 * eff_a + buffer[idx + 2] as u32 * inv_a) / 255) as u8;
                    buffer[idx + 3] = 255;
                }
            }
        }
        cur_x += advance;
    }
}

/// Helper: Draws a filled rectangle with alpha blending
#[allow(clippy::too_many_arguments)]
fn draw_rect(
    buffer: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: [u8; 4],
) {
    let alpha_src = color[3] as u32;
    if alpha_src == 0 || w <= 0 || h <= 0 {
        return;
    }

    let x_end = (x + w).min(canvas_w as i32);
    let y_end = (y + h).min(canvas_h as i32);

    for py in y.max(0)..y_end {
        for px in x.max(0)..x_end {
            let idx = ((py as u32 * canvas_w + px as u32) * 4) as usize;
            let inv_a = 255 - alpha_src;
            buffer[idx] = ((color[0] as u32 * alpha_src + buffer[idx] as u32 * inv_a) / 255) as u8;
            buffer[idx + 1] =
                ((color[1] as u32 * alpha_src + buffer[idx + 1] as u32 * inv_a) / 255) as u8;
            buffer[idx + 2] =
                ((color[2] as u32 * alpha_src + buffer[idx + 2] as u32 * inv_a) / 255) as u8;
            buffer[idx + 3] = 255;
        }
    }
}

/// Helper: Draws a filled circle with alpha blending
fn draw_circle(
    buffer: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    cx: i32,
    cy: i32,
    radius: i32,
    color: [u8; 4],
) {
    let alpha_src = color[3] as u32;
    if alpha_src == 0 || radius <= 0 {
        return;
    }
    let r2 = radius * radius;
    let y_start = (cy - radius).max(0);
    let y_end = (cy + radius).min(canvas_h as i32 - 1);
    let x_start = (cx - radius).max(0);
    let x_end = (cx + radius).min(canvas_w as i32 - 1);

    for py in y_start..=y_end {
        let dy = py - cy;
        for px in x_start..=x_end {
            let dx = px - cx;
            if dx * dx + dy * dy <= r2 {
                let idx = ((py as u32 * canvas_w + px as u32) * 4) as usize;
                let inv_a = 255 - alpha_src;
                buffer[idx] =
                    ((color[0] as u32 * alpha_src + buffer[idx] as u32 * inv_a) / 255) as u8;
                buffer[idx + 1] =
                    ((color[1] as u32 * alpha_src + buffer[idx + 1] as u32 * inv_a) / 255) as u8;
                buffer[idx + 2] =
                    ((color[2] as u32 * alpha_src + buffer[idx + 2] as u32 * inv_a) / 255) as u8;
                buffer[idx + 3] = 255;
            }
        }
    }
}

/// Renders a cinematic camera viewfinder overlay with recording indicators (REQ-OVERLAY-001)
pub fn render_camera_viewfinder(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    frame_idx: i64,
    overlay: &OverlayItem,
    font: &Font,
) {
    let in_f = overlay.timeline_in_frame;
    let out_f = overlay.timeline_out_frame;
    if frame_idx < in_f || frame_idx >= out_f {
        return;
    }

    let rel_frame = frame_idx - in_f;
    let total_span = (out_f - in_f).max(1);
    let scale = width as f32 / 1080.0;

    let in_dur = 10.min(total_span / 2);
    let out_dur = 10.min(total_span / 2);
    let alpha_factor = if rel_frame < in_dur {
        rel_frame as f32 / in_dur as f32
    } else if rel_frame >= total_span - out_dur {
        (total_span - rel_frame) as f32 / out_dur as f32
    } else {
        1.0
    } * overlay.opacity;

    let white_alpha = (235.0 * alpha_factor).clamp(0.0, 255.0) as u8;
    let white_color = [255, 255, 255, white_alpha];
    let bracket_color = [255, 255, 255, (210.0 * alpha_factor).clamp(0.0, 255.0) as u8];

    // 1. Four Viewfinder Corner Brackets
    let mx = (60.0 * scale).round() as i32;
    let my = (90.0 * scale).round() as i32;
    let blen = (56.0 * scale).round() as i32;
    let bthick = (4.0 * scale).max(2.0).round() as i32;

    // Top-left
    draw_rect(buffer, width, height, mx, my, blen, bthick, bracket_color);
    draw_rect(buffer, width, height, mx, my, bthick, blen, bracket_color);

    // Top-right
    draw_rect(buffer, width, height, width as i32 - mx - blen, my, blen, bthick, bracket_color);
    draw_rect(buffer, width, height, width as i32 - mx - bthick, my, bthick, blen, bracket_color);

    // Bottom-left
    draw_rect(buffer, width, height, mx, height as i32 - my - bthick, blen, bthick, bracket_color);
    draw_rect(buffer, width, height, mx, height as i32 - my - blen, bthick, blen, bracket_color);

    // Bottom-right
    draw_rect(buffer, width, height, width as i32 - mx - blen, height as i32 - my - bthick, blen, bthick, bracket_color);
    draw_rect(buffer, width, height, width as i32 - mx - bthick, height as i32 - my - blen, bthick, blen, bracket_color);

    // 2. Battery Indicator (Top-Left)
    let bat_x = mx + (24.0 * scale).round() as i32;
    let bat_y = my + (18.0 * scale).round() as i32;
    let bat_w = (44.0 * scale).round() as i32;
    let bat_h = (22.0 * scale).round() as i32;
    let bat_border = (2.0 * scale).max(1.0).round() as i32;

    draw_rect(buffer, width, height, bat_x, bat_y, bat_w, bat_border, white_color);
    draw_rect(buffer, width, height, bat_x, bat_y + bat_h - bat_border, bat_w, bat_border, white_color);
    draw_rect(buffer, width, height, bat_x, bat_y, bat_border, bat_h, white_color);
    draw_rect(buffer, width, height, bat_x + bat_w - bat_border, bat_y, bat_border, bat_h, white_color);
    draw_rect(buffer, width, height, bat_x + bat_w, bat_y + (bat_h / 4), (4.0 * scale).max(2.0) as i32, bat_h / 2, white_color);

    let fill_w = ((bat_w - bat_border * 4) as f32 * 0.85).round() as i32;
    let fill_color = [76, 217, 100, white_alpha];
    draw_rect(buffer, width, height, bat_x + bat_border * 2, bat_y + bat_border * 2, fill_w, bat_h - bat_border * 4, fill_color);

    // 3. REC Blinker (Top-Right)
    let rec_x = width as i32 - mx - (140.0 * scale).round() as i32;
    let rec_y = my + (28.0 * scale).round() as i32;
    let pulse = ((rel_frame as f32 * 0.12).sin().abs() * 155.0 + 100.0) as u8;
    let rec_dot_alpha = ((pulse as f32 / 255.0) * white_alpha as f32) as u8;
    draw_circle(buffer, width, height, rec_x, rec_y, (8.0 * scale).round() as i32, [255, 59, 48, rec_dot_alpha]);
    draw_text(buffer, width, height, font, "REC", 26.0 * scale, rec_x + (16.0 * scale).round() as i32, rec_y + (10.0 * scale).round() as i32, white_color);

    // 4. Center Focus Reticle [   ] + Crosshair (only shown when no title card is active to avoid clutter)
    let cx = width as i32 / 2;
    let cy = height as i32 / 2;
    let has_title_card = overlay.title.as_ref().map(|t| !t.is_empty()).unwrap_or(false);
    if !has_title_card {
        let f_half = (120.0 * scale).round() as i32;
        let f_len = (28.0 * scale).round() as i32;
        let f_thick = (2.0 * scale).max(1.0).round() as i32;
        let f_color = [255, 255, 255, (160.0 * alpha_factor).clamp(0.0, 255.0) as u8];

        draw_rect(buffer, width, height, cx - f_half, cy - f_half, f_len, f_thick, f_color);
        draw_rect(buffer, width, height, cx - f_half, cy - f_half, f_thick, f_len, f_color);
        draw_rect(buffer, width, height, cx + f_half - f_len, cy - f_half, f_len, f_thick, f_color);
        draw_rect(buffer, width, height, cx + f_half - f_thick, cy - f_half, f_thick, f_len, f_color);
        draw_rect(buffer, width, height, cx - f_half, cy + f_half - f_thick, f_len, f_thick, f_color);
        draw_rect(buffer, width, height, cx - f_half, cy + f_half - f_len, f_thick, f_len, f_color);
        draw_rect(buffer, width, height, cx + f_half - f_len, cy + f_half - f_thick, f_len, f_thick, f_color);
        draw_rect(buffer, width, height, cx + f_half - f_thick, cy + f_half - f_len, f_thick, f_len, f_color);

        let ch_len = (12.0 * scale).round() as i32;
        draw_rect(buffer, width, height, cx - ch_len, cy, ch_len * 2, f_thick, f_color);
        draw_rect(buffer, width, height, cx, cy - ch_len, f_thick, ch_len * 2, f_color);
    }

    // 5. Technical Spec Metadata (Bottom)
    draw_text(buffer, width, height, font, "4K 60FPS", 24.0 * scale, mx + (20.0 * scale).round() as i32, height as i32 - my - (18.0 * scale).round() as i32, white_color);

    let hd_x = width as i32 - mx - (80.0 * scale).round() as i32;
    let hd_y = height as i32 - my - (36.0 * scale).round() as i32;
    let hd_w = (60.0 * scale).round() as i32;
    let hd_h = (28.0 * scale).round() as i32;
    draw_rect(buffer, width, height, hd_x, hd_y, hd_w, bat_border, white_color);
    draw_rect(buffer, width, height, hd_x, hd_y + hd_h - bat_border, hd_w, bat_border, white_color);
    draw_rect(buffer, width, height, hd_x, hd_y, bat_border, hd_h, white_color);
    draw_rect(buffer, width, height, hd_x + hd_w - bat_border, hd_y, bat_border, hd_h, white_color);
    draw_text(buffer, width, height, font, "HD", 20.0 * scale, hd_x + (14.0 * scale).round() as i32, hd_y + (21.0 * scale).round() as i32, white_color);

    // 6. Centered Title Card (Torn paper stickers with gold stars as requested)
    if let Some(title) = &overlay.title {
        let title_lower = title.to_lowercase();
        if title_lower.contains("cheesecake") || title_lower.contains("chocolate") {
            render_torn_paper_stickers(
                buffer, width, height, frame_idx, 0.38, alpha_factor, "Chocolate", "cheesecake", font,
            );
        } else if title_lower.contains("result") {
            render_torn_paper_stickers(
                buffer, width, height, frame_idx, 0.44, alpha_factor, "The result", "🤎", font,
            );
        } else {
            let title_upper = title.to_uppercase();
            let font_size = 46.0 * scale;
            let mut text_w = 0.0f32;
            for ch in title_upper.chars() {
                text_w += font.metrics(ch, font_size).advance_width;
            }

            let card_w = (text_w + 64.0 * scale).round() as i32;
            let card_h = (font_size + 36.0 * scale).round() as i32;
            let card_x = cx - card_w / 2;
            let card_y = (height as f32 * overlay.position.y).round() as i32 - card_h / 2;
            let radius = (16.0 * scale).round() as i32;

            draw_rounded_rect(buffer, width, height, card_x + 4, card_y + 6, card_w, card_h, radius, [0, 0, 0, (140.0 * alpha_factor) as u8]);
            draw_rounded_rect(buffer, width, height, card_x, card_y, card_w, card_h, radius, [20, 20, 24, (220.0 * alpha_factor) as u8]);

            let tx = card_x + ((card_w as f32 - text_w) / 2.0).round() as i32;
            let ty = card_y + (card_h as f32 * 0.72).round() as i32;
            draw_text(buffer, width, height, font, &title_upper, font_size, tx, ty, [255, 255, 255, white_alpha]);
        }
    }
}

/// Helper: Draws a procedurally torn paper deckled rectangle with alpha blending
fn draw_torn_paper_rect(
    buffer: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    seed: u32,
    color: [u8; 4],
) {
    let alpha_src = color[3] as u32;
    if alpha_src == 0 || w <= 0 || h <= 0 {
        return;
    }

    let seed_f = seed as f32;
    let s_inv = 255 - alpha_src;

    let y_start = (y - 12).max(0);
    let y_end = (y + h + 12).min(canvas_h as i32);

    for py in y_start..y_end {
        let rel_y = (py - y) as f32;
        // Natural torn washi tape edge on sides (subtle rip, not sawtooth)
        let left_offset =
            ((rel_y * 0.08 + seed_f).sin() * 3.2 + (rel_y * 0.22 + seed_f * 1.5).cos() * 1.6)
                .round() as i32;
        let right_offset =
            ((rel_y * 0.07 + seed_f * 2.1).cos() * 3.2 + (rel_y * 0.25 + seed_f * 0.7).sin() * 1.6)
                .round() as i32;

        let start_x = (x + left_offset).max(0);
        let end_x = (x + w + right_offset).min(canvas_w as i32);

        for px in start_x..end_x {
            let rel_x = (px - x) as f32;
            // Natural wavy rip along top and bottom with subtle micro-tears
            let top_offset = ((rel_x * 0.032 + seed_f).sin() * 3.2
                + (rel_x * 0.085 + seed_f * 1.8).cos() * 2.0
                + ((rel_x * 0.21 + seed_f * 3.0).sin() * 1.0))
                .round() as i32;
            let bot_offset = ((rel_x * 0.038 + seed_f * 2.2).cos() * 3.2
                + (rel_x * 0.092 + seed_f * 0.6).sin() * 2.0
                + ((rel_x * 0.23 + seed_f * 2.0).cos() * 1.0))
                .round() as i32;

            if py < y + top_offset || py > y + h + bot_offset {
                continue;
            }

            let idx = ((py as u32 * canvas_w + px as u32) * 4) as usize;
            buffer[idx] = ((color[0] as u32 * alpha_src + buffer[idx] as u32 * s_inv) / 255) as u8;
            buffer[idx + 1] =
                ((color[1] as u32 * alpha_src + buffer[idx + 1] as u32 * s_inv) / 255) as u8;
            buffer[idx + 2] =
                ((color[2] as u32 * alpha_src + buffer[idx + 2] as u32 * s_inv) / 255) as u8;
            buffer[idx + 3] = 255;
        }
    }
}

/// Helper: Draws a 5-point star with raycasting point-in-polygon fill
fn draw_star_5point(
    buffer: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    cx: i32,
    cy: i32,
    outer_r: f32,
    inner_r: f32,
    angle_offset: f32,
    color: [u8; 4],
) {
    let alpha_src = color[3] as u32;
    if alpha_src == 0 || outer_r <= 0.0 {
        return;
    }

    let mut poly = Vec::with_capacity(10);
    for i in 0..10 {
        let angle = angle_offset + (i as f32 * std::f32::consts::PI / 5.0)
            - std::f32::consts::FRAC_PI_2;
        let r = if i % 2 == 0 { outer_r } else { inner_r };
        let x = cx as f32 + angle.cos() * r;
        let y = cy as f32 + angle.sin() * r;
        poly.push((x, y));
    }

    let max_r = outer_r.ceil() as i32;
    let s_inv = 255 - alpha_src;

    let y_start = (cy - max_r).max(0);
    let y_end = (cy + max_r).min(canvas_h as i32 - 1);
    let x_start = (cx - max_r).max(0);
    let x_end = (cx + max_r).min(canvas_w as i32 - 1);

    for py in y_start..=y_end {
        let y_f = py as f32 + 0.5;
        for px in x_start..=x_end {
            let x_f = px as f32 + 0.5;

            let mut inside = false;
            let mut j = poly.len() - 1;
            for i in 0..poly.len() {
                if (poly[i].1 > y_f) != (poly[j].1 > y_f)
                    && (x_f < (poly[j].0 - poly[i].0) * (y_f - poly[i].1) / (poly[j].1 - poly[i].1)
                        + poly[i].0)
                {
                    inside = !inside;
                }
                j = i;
            }

            if inside {
                let idx = ((py as u32 * canvas_w + px as u32) * 4) as usize;
                buffer[idx] =
                    ((color[0] as u32 * alpha_src + buffer[idx] as u32 * s_inv) / 255) as u8;
                buffer[idx + 1] =
                    ((color[1] as u32 * alpha_src + buffer[idx + 1] as u32 * s_inv) / 255) as u8;
                buffer[idx + 2] =
                    ((color[2] as u32 * alpha_src + buffer[idx + 2] as u32 * s_inv) / 255) as u8;
                buffer[idx + 3] = 255;
            }
        }
    }
}

/// Helper: Draws a 4-point sparkle star
fn draw_sparkle_4point(
    buffer: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    cx: i32,
    cy: i32,
    outer_r: f32,
    inner_r: f32,
    color: [u8; 4],
) {
    let alpha_src = color[3] as u32;
    if alpha_src == 0 || outer_r <= 0.0 {
        return;
    }

    let mut poly = Vec::with_capacity(8);
    for i in 0..8 {
        let angle =
            (i as f32 * std::f32::consts::FRAC_PI_4) - std::f32::consts::FRAC_PI_2;
        let r = if i % 2 == 0 { outer_r } else { inner_r };
        let x = cx as f32 + angle.cos() * r;
        let y = cy as f32 + angle.sin() * r;
        poly.push((x, y));
    }

    let max_r = outer_r.ceil() as i32;
    let s_inv = 255 - alpha_src;

    let y_start = (cy - max_r).max(0);
    let y_end = (cy + max_r).min(canvas_h as i32 - 1);
    let x_start = (cx - max_r).max(0);
    let x_end = (cx + max_r).min(canvas_w as i32 - 1);

    for py in y_start..=y_end {
        let y_f = py as f32 + 0.5;
        for px in x_start..=x_end {
            let x_f = px as f32 + 0.5;

            let mut inside = false;
            let mut j = poly.len() - 1;
            for i in 0..poly.len() {
                if (poly[i].1 > y_f) != (poly[j].1 > y_f)
                    && (x_f < (poly[j].0 - poly[i].0) * (y_f - poly[i].1) / (poly[j].1 - poly[i].1)
                        + poly[i].0)
                {
                    inside = !inside;
                }
                j = i;
            }

            if inside {
                let idx = ((py as u32 * canvas_w + px as u32) * 4) as usize;
                buffer[idx] =
                    ((color[0] as u32 * alpha_src + buffer[idx] as u32 * s_inv) / 255) as u8;
                buffer[idx + 1] =
                    ((color[1] as u32 * alpha_src + buffer[idx + 1] as u32 * s_inv) / 255) as u8;
                buffer[idx + 2] =
                    ((color[2] as u32 * alpha_src + buffer[idx + 2] as u32 * s_inv) / 255) as u8;
                buffer[idx + 3] = 255;
            }
        }
    }
}

/// Renders two stacked torn paper tape sticker banners with gold sparkle stars (as in creator reference)
#[allow(clippy::too_many_arguments)]
pub fn render_torn_paper_stickers(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    frame_idx: i64,
    center_y_ratio: f32,
    alpha_factor: f32,
    line1: &str,
    line2: &str,
    font: &Font,
) {
    let scale = width as f32 / 1080.0;
    let cx = (width as f32 / 2.0).round() as i32;
    let cy = (height as f32 * center_y_ratio).round() as i32;

    // Load serif font (Georgia Bold) if available on Windows, fallback to bundled font
    let georgia_font = std::fs::read("C:\\Windows\\Fonts\\georgiab.ttf")
        .ok()
        .and_then(|bytes| fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok());
    let active_font = georgia_font.as_ref().unwrap_or(font);

    let font_size = 72.0 * scale;
    let pad_h = 42.0 * scale;
    let pad_v = 15.0 * scale;

    // Measure line 1
    let mut w1 = 0.0f32;
    for ch in line1.chars() {
        w1 += active_font.metrics(ch, font_size).advance_width;
    }
    let banner1_w = (w1 + pad_h * 2.0).round() as i32;
    let banner1_h = (font_size + pad_v * 1.8).round() as i32;

    // Measure line 2
    let mut w2 = 0.0f32;
    for ch in line2.chars() {
        w2 += active_font.metrics(ch, font_size).advance_width;
    }
    let banner2_w = (w2 + pad_h * 2.0).round() as i32;
    let banner2_h = (font_size + pad_v * 1.8).round() as i32;

    let spacing = (14.0 * scale).round() as i32;
    let total_h = banner1_h + spacing + banner2_h;

    let b1_y = cy - (total_h / 2);
    let b1_x = cx - (banner1_w / 2);

    let b2_y = b1_y + banner1_h + spacing;
    let b2_x = cx - (banner2_w / 2);

    // Multi-pass soft diffused drop shadows for BOTH banners first
    // Outer soft shadow
    draw_torn_paper_rect(
        buffer, width, height, b1_x + 3, b1_y + 8, banner1_w, banner1_h, 42,
        [0, 0, 0, (50.0 * alpha_factor).clamp(0.0, 255.0) as u8],
    );
    draw_torn_paper_rect(
        buffer, width, height, b2_x + 3, b2_y + 8, banner2_w, banner2_h, 99,
        [0, 0, 0, (50.0 * alpha_factor).clamp(0.0, 255.0) as u8],
    );
    // Inner closer shadow
    draw_torn_paper_rect(
        buffer, width, height, b1_x + 2, b1_y + 4, banner1_w, banner1_h, 42,
        [0, 0, 0, (110.0 * alpha_factor).clamp(0.0, 255.0) as u8],
    );
    draw_torn_paper_rect(
        buffer, width, height, b2_x + 2, b2_y + 4, banner2_w, banner2_h, 99,
        [0, 0, 0, (110.0 * alpha_factor).clamp(0.0, 255.0) as u8],
    );

    // Creamy parchment paper tape banners
    let paper_color = [252, 246, 228, (255.0 * alpha_factor).clamp(0.0, 255.0) as u8];
    draw_torn_paper_rect(
        buffer, width, height, b1_x, b1_y, banner1_w, banner1_h, 42, paper_color,
    );
    draw_torn_paper_rect(
        buffer, width, height, b2_x, b2_y, banner2_w, banner2_h, 99, paper_color,
    );

    // Deep espresso serif text
    let text_color = [24, 18, 14, (255.0 * alpha_factor).clamp(0.0, 255.0) as u8];
    let tx1 = b1_x + ((banner1_w as f32 - w1) / 2.0).round() as i32;
    let ty1 = b1_y + (banner1_h as f32 * 0.74).round() as i32;
    draw_text(
        buffer, width, height, active_font, line1, font_size, tx1, ty1, text_color,
    );

    let tx2 = b2_x + ((banner2_w as f32 - w2) / 2.0).round() as i32;
    let ty2 = b2_y + (banner2_h as f32 * 0.74).round() as i32;
    draw_text(
        buffer, width, height, active_font, line2, font_size, tx2, ty2, text_color,
    );

    // Gold Stars & Sparkles matching reference image
    let gold_star_color = [255, 206, 22, (255.0 * alpha_factor).clamp(0.0, 255.0) as u8];
    let star_pulse = 1.0 + (frame_idx as f32 * 0.12).sin() * 0.10;

    // 1. Five-point star to the top-left of line 1 (matching user image)
    let star1_x = b1_x - (26.0 * scale).round() as i32;
    let star1_y = b1_y + (10.0 * scale).round() as i32;
    draw_star_5point(
        buffer,
        width,
        height,
        star1_x,
        star1_y,
        24.0 * scale * star_pulse,
        10.0 * scale * star_pulse,
        -0.28,
        gold_star_color,
    );

    // 2. Five-point star to the bottom-right of line 2 (matching user image)
    let star2_x = b2_x + banner2_w + (24.0 * scale).round() as i32;
    let star2_y = b2_y + banner2_h - (12.0 * scale).round() as i32;
    draw_star_5point(
        buffer,
        width,
        height,
        star2_x,
        star2_y,
        22.0 * scale * star_pulse,
        9.0 * scale * star_pulse,
        0.22,
        gold_star_color,
    );

    // 3. Small twinkle sparkle accent near top right
    let star3_x = b1_x + banner1_w + (14.0 * scale).round() as i32;
    let star3_y = b1_y - (6.0 * scale).round() as i32;
    draw_sparkle_4point(
        buffer,
        width,
        height,
        star3_x,
        star3_y,
        14.0 * scale,
        4.0 * scale,
        gold_star_color,
    );
}
