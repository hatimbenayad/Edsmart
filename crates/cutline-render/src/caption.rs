use cutline_core::project::{CaptionGroup, CaptionsConfig, WordToken};
use cutline_core::time::Fps;
use fontdue::{Font, FontSettings};
use serde::{Deserialize, Serialize};

/// Embedded open-license fonts
pub static INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
pub static MONTSERRAT_BOLD: &[u8] = include_bytes!("../assets/fonts/Montserrat-Bold.ttf");

/// Configuration and visual styling for word-by-word kinetic captions (REQ-CAP-003, REQ-DESIGN-001)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptionStyle {
    pub name: String,
    pub font_family: String,
    pub font_size: f32,            // Base size on 1080p, e.g. 76.0
    pub active_color: [u8; 4],     // RGBA: [255, 214, 10, 255] for #FFD60A
    pub inactive_color: [u8; 4],   // RGBA: [255, 255, 255, 255]
    pub outline_color: [u8; 4],    // RGBA: [0, 0, 0, 255]
    pub outline_width: f32,        // Base outline width on 1080p, e.g. 4.0
    pub shadow_color: [u8; 4],     // RGBA: [0, 0, 0, 180]
    pub shadow_offset: (f32, f32), // e.g. (3.0, 4.0)
    pub pop_scale: f32,            // e.g. 1.15
    pub uppercase: bool,
    pub position_y_ratio: f32,      // e.g. 0.68 (68% from top of canvas)
    pub max_words_per_group: usize, // 2 to 4 words
    pub min_duration_ms: u32,       // e.g. 400ms
}

impl Default for CaptionStyle {
    fn default() -> Self {
        Self::clean_yellow()
    }
}

impl CaptionStyle {
    /// Default style: clean white with warm yellow active highlight (#FFD60A)
    pub fn clean_yellow() -> Self {
        Self {
            name: "clean-yellow".to_string(),
            font_family: "Inter".to_string(),
            font_size: 76.0,
            active_color: [255, 214, 10, 255], // #FFD60A Warm Amber Yellow
            inactive_color: [255, 255, 255, 255], // #FFFFFF Pure White
            outline_color: [10, 10, 10, 255],  // Near black stroke
            outline_width: 4.0,
            shadow_color: [0, 0, 0, 180],
            shadow_offset: (3.0, 4.0),
            pop_scale: 1.15,
            uppercase: true,
            position_y_ratio: 0.68,
            max_words_per_group: 3,
            min_duration_ms: 400,
        }
    }

    /// Alternate style: warm mono with tangerine highlight (#FF9500)
    pub fn mono_warm() -> Self {
        Self {
            name: "mono-warm".to_string(),
            font_family: "Montserrat".to_string(),
            font_size: 76.0,
            active_color: [255, 149, 0, 255], // #FF9500 Warm Tangerine
            inactive_color: [245, 245, 247, 255], // #F5F5F7 Off White
            outline_color: [28, 28, 30, 255],
            outline_width: 4.0,
            shadow_color: [0, 0, 0, 160],
            shadow_offset: (3.0, 4.0),
            pop_scale: 1.12,
            uppercase: true,
            position_y_ratio: 0.68,
            max_words_per_group: 3,
            min_duration_ms: 400,
        }
    }

    /// Finds style by identifier or returns default
    pub fn from_id(id: &str) -> Self {
        match id {
            "mono-warm" => Self::mono_warm(),
            _ => Self::clean_yellow(),
        }
    }
}

/// Ease-out-back curve with standard overshoot s = 1.70158 (REQ-DESIGN-001)
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let s = 1.70158;
    let t_m1 = t - 1.0;
    1.0 + (s + 1.0) * t_m1 * t_m1 * t_m1 + s * t_m1 * t_m1
}

/// Smart caption word grouping algorithm (REQ-CAP-001)
/// Groups 2-4 words per group, splitting on punctuation or pauses > 200ms,
/// enforcing minimum on-screen duration (400ms) without overlapping adjacent groups.
pub fn group_words_into_captions(
    words: &[WordToken],
    fps: Fps,
    style: &CaptionStyle,
) -> Vec<CaptionGroup> {
    if words.is_empty() {
        return Vec::new();
    }

    let min_duration_frames =
        ((style.min_duration_ms as f64 / 1000.0) * fps.as_f64()).ceil() as i64;
    let pause_threshold_frames = (0.200 * fps.as_f64()).ceil() as i64; // 200ms pause

    let mut groups: Vec<CaptionGroup> = Vec::new();
    let mut current_words: Vec<WordToken> = Vec::new();
    let mut group_idx = 1;

    for i in 0..words.len() {
        let word = &words[i];
        current_words.push(word.clone());

        let clean_text = word.text.trim();
        let ends_with_punct = clean_text.ends_with('.')
            || clean_text.ends_with(',')
            || clean_text.ends_with('?')
            || clean_text.ends_with('!')
            || clean_text.ends_with(';')
            || clean_text.ends_with(':');

        let next_word = words.get(i + 1);
        let has_pause = if let Some(nw) = next_word {
            nw.start_frame - word.end_frame >= pause_threshold_frames
        } else {
            false
        };

        let reached_max_words = current_words.len() >= style.max_words_per_group;
        let is_last = i == words.len() - 1;

        if ends_with_punct || has_pause || reached_max_words || is_last {
            let in_frame = current_words.first().unwrap().start_frame;
            let natural_out = current_words.last().unwrap().end_frame;

            let group = CaptionGroup {
                id: format!("group_{}", group_idx),
                timeline_in_frame: in_frame,
                timeline_out_frame: natural_out,
                words: current_words.clone(),
            };
            groups.push(group);
            group_idx += 1;
            current_words.clear();
        }
    }

    // Adjust group boundaries to satisfy minimum duration constraint without overlapping next group
    for i in 0..groups.len() {
        let in_frame = groups[i].timeline_in_frame;
        let current_out = groups[i].timeline_out_frame;
        let desired_out = in_frame + min_duration_frames;

        let max_allowed_out = if i + 1 < groups.len() {
            groups[i + 1].timeline_in_frame
        } else {
            desired_out.max(current_out)
        };

        let extended_out = desired_out.min(max_allowed_out);
        groups[i].timeline_out_frame = current_out.max(extended_out);
        if groups[i].timeline_out_frame <= groups[i].timeline_in_frame {
            groups[i].timeline_out_frame = groups[i].timeline_in_frame + 1;
        }
    }

    groups
}

/// Cached font handles for pure-Rust rasterization
pub struct FontManager {
    pub inter: Font,
    pub montserrat: Font,
}

impl FontManager {
    pub fn new() -> Result<Self, String> {
        let inter = Font::from_bytes(INTER_BOLD, FontSettings::default())
            .map_err(|e| format!("Failed to parse Inter-Bold font: {e}"))?;
        let montserrat = Font::from_bytes(MONTSERRAT_BOLD, FontSettings::default())
            .map_err(|e| format!("Failed to parse Montserrat-Bold font: {e}"))?;
        Ok(Self { inter, montserrat })
    }

    pub fn get_font(&self, family: &str) -> &Font {
        if family.eq_ignore_ascii_case("montserrat") {
            &self.montserrat
        } else {
            &self.inter
        }
    }
}

/// Word layout box for positioning
#[derive(Debug, Clone)]
struct LayoutWord {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub is_active: bool,
    pub pop_scale: f32,
}

/// Pure-Rust kinetic caption renderer (REQ-CAP-002, REQ-CAP-003, REQ-CAP-004)
pub struct CaptionRenderer {
    fonts: FontManager,
    style: CaptionStyle,
}

impl CaptionRenderer {
    pub fn new(style: CaptionStyle) -> Result<Self, String> {
        let fonts = FontManager::new()?;
        Ok(Self { fonts, style })
    }

    /// Renders active captions for the given timeline frame onto the RGBA frame buffer
    pub fn render_frame(
        &self,
        frame_idx: i64,
        buffer: &mut [u8],
        width: u32,
        height: u32,
        config: &CaptionsConfig,
    ) {
        if !config.enabled {
            return;
        }

        // 1. Locate active CaptionGroup for frame_idx
        let active_group = config
            .groups
            .iter()
            .find(|g| frame_idx >= g.timeline_in_frame && frame_idx < g.timeline_out_frame);

        let Some(group) = active_group else {
            return;
        };

        // 2. Select font
        let font = self.fonts.get_font(&self.style.font_family);

        // 3. Compute resolution scaling factor (relative to 1080p canvas)
        let scale = width as f32 / 1080.0;
        let font_size = self.style.font_size * scale;
        let outline_width = (self.style.outline_width * scale).max(1.0);
        let shadow_offset = (
            self.style.shadow_offset.0 * scale,
            self.style.shadow_offset.1 * scale,
        );

        // 4. Safe area calculation (clamped inside platform safe zone)
        let safe_left = (config.safe_area.left as f32 * scale).max(20.0);
        let safe_right = (config.safe_area.right as f32 * scale).max(20.0);
        let max_text_width = (width as f32 - safe_left - safe_right).max(100.0);

        // 5. Layout words and measure dimensions
        let space_metrics = font.metrics(' ', font_size);
        let space_advance = space_metrics.advance_width;

        let mut words_meta = Vec::new();
        for word_token in &group.words {
            let display_text = if self.style.uppercase {
                word_token.text.to_uppercase()
            } else {
                word_token.text.clone()
            };

            let mut word_width = 0.0f32;
            for ch in display_text.chars() {
                let m = font.metrics(ch, font_size);
                word_width += m.advance_width;
            }

            let is_active =
                frame_idx >= word_token.start_frame && frame_idx <= word_token.end_frame;
            let pop_scale = if is_active {
                // Kinetic pop calculation: scale up to 1.15x with EaseOutBack on word attack
                let word_span = (word_token.end_frame - word_token.start_frame).max(1) as f32;
                let word_t =
                    ((frame_idx - word_token.start_frame) as f32 / word_span).clamp(0.0, 1.0);
                if word_t <= 0.4 {
                    let pop_t = word_t / 0.4;
                    1.0 + (self.style.pop_scale - 1.0) * ease_out_back(pop_t)
                } else {
                    let settle_t = (word_t - 0.4) / 0.6;
                    self.style.pop_scale - (self.style.pop_scale - 1.0) * settle_t
                }
            } else {
                1.0
            };

            words_meta.push((display_text, word_width, is_active, pop_scale));
        }

        // 6. Multi-line wrapping (max 2 lines)
        let mut lines: Vec<Vec<(String, f32, bool, f32)>> = Vec::new();
        let mut current_line: Vec<(String, f32, bool, f32)> = Vec::new();
        let mut current_line_width = 0.0f32;

        for (text, w, is_active, pop_scale) in words_meta {
            let word_w_with_space = if current_line.is_empty() {
                w
            } else {
                w + space_advance
            };

            if current_line_width + word_w_with_space > max_text_width
                && !current_line.is_empty()
                && lines.is_empty()
            {
                lines.push(current_line);
                current_line = Vec::new();
                current_line_width = 0.0;
            }

            if !current_line.is_empty() {
                current_line_width += space_advance;
            }
            current_line_width += w;
            current_line.push((text, w, is_active, pop_scale));
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // 7. Calculate vertical placement centered around position_y_ratio
        let line_height = font_size * 1.25;
        let total_block_height = lines.len() as f32 * line_height;
        let center_y = height as f32 * self.style.position_y_ratio;
        let start_y = center_y - (total_block_height / 2.0);

        // 8. Generate positioned word boxes
        let mut layout_words = Vec::new();
        for (line_idx, line) in lines.iter().enumerate() {
            let total_line_w: f32 = line.iter().map(|item| item.1).sum::<f32>()
                + (line.len().saturating_sub(1) as f32 * space_advance);

            let mut cur_x = safe_left + (max_text_width - total_line_w) / 2.0;
            let cur_y = start_y + (line_idx as f32 * line_height);

            for (text, w, is_active, pop_scale) in line {
                layout_words.push(LayoutWord {
                    text: text.clone(),
                    x: cur_x,
                    y: cur_y,
                    width: *w,
                    height: line_height,
                    is_active: *is_active,
                    pop_scale: *pop_scale,
                });
                cur_x += w + space_advance;
            }
        }

        // 9. Rasterize & Composite each word
        for word in &layout_words {
            let text_color = if word.is_active {
                self.style.active_color
            } else {
                self.style.inactive_color
            };

            let center_word_x = word.x + (word.width / 2.0);
            let center_word_y = word.y + (word.height / 2.0);
            let word_font_size = font_size * word.pop_scale;

            let mut char_x = word.x;
            for ch in word.text.chars() {
                let (metrics, bitmap) = font.rasterize(ch, word_font_size);
                let advance = metrics.advance_width;

                if metrics.width > 0 && metrics.height > 0 {
                    // Apply pop scaling offset relative to word center
                    let base_x = center_word_x + (char_x - center_word_x) * word.pop_scale;
                    let base_y = center_word_y + (word.y - center_word_y) * word.pop_scale;

                    let glyph_x = base_x + metrics.bounds.xmin;
                    // fontdue ymin is from baseline upwards
                    let glyph_y =
                        base_y + (font_size * 0.8) - (metrics.bounds.ymin + metrics.bounds.height);

                    // Pass 1: Drop Shadow
                    self.blend_glyph(
                        buffer,
                        width,
                        height,
                        glyph_x + shadow_offset.0,
                        glyph_y + shadow_offset.1,
                        &metrics,
                        &bitmap,
                        self.style.shadow_color,
                    );

                    // Pass 2: 4px Dark Outline (Multi-directional circular stroke)
                    let rad = outline_width;
                    let offsets = [
                        (-rad, 0.0),
                        (rad, 0.0),
                        (0.0, -rad),
                        (0.0, rad),
                        (-rad * 0.7, -rad * 0.7),
                        (rad * 0.7, -rad * 0.7),
                        (-rad * 0.7, rad * 0.7),
                        (rad * 0.7, rad * 0.7),
                    ];
                    for (ox, oy) in offsets {
                        self.blend_glyph(
                            buffer,
                            width,
                            height,
                            glyph_x + ox,
                            glyph_y + oy,
                            &metrics,
                            &bitmap,
                            self.style.outline_color,
                        );
                    }

                    // Pass 3: Main Glyph Text Fill
                    self.blend_glyph(
                        buffer, width, height, glyph_x, glyph_y, &metrics, &bitmap, text_color,
                    );
                }

                char_x += advance;
            }
        }
    }

    /// Blends a single rasterized glyph mask onto the RGBA frame buffer
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn blend_glyph(
        &self,
        buffer: &mut [u8],
        canvas_width: u32,
        canvas_height: u32,
        gx: f32,
        gy: f32,
        metrics: &fontdue::Metrics,
        bitmap: &[u8],
        color: [u8; 4],
    ) {
        let ix = gx.round() as i32;
        let iy = gy.round() as i32;

        let gw = metrics.width as i32;
        let gh = metrics.height as i32;

        for row in 0..gh {
            let py = iy + row;
            if py < 0 || py >= canvas_height as i32 {
                continue;
            }

            for col in 0..gw {
                let px = ix + col;
                if px < 0 || px >= canvas_width as i32 {
                    continue;
                }

                let alpha_mask = bitmap[(row * gw + col) as usize];
                if alpha_mask == 0 {
                    continue;
                }

                // Compute effective blending alpha
                let eff_alpha = (color[3] as u32 * alpha_mask as u32) / 255;
                if eff_alpha == 0 {
                    continue;
                }

                let inv_alpha = 255 - eff_alpha;
                let buf_idx = ((py as u32 * canvas_width + px as u32) * 4) as usize;

                let dst_r = buffer[buf_idx] as u32;
                let dst_g = buffer[buf_idx + 1] as u32;
                let dst_b = buffer[buf_idx + 2] as u32;

                buffer[buf_idx] = ((color[0] as u32 * eff_alpha + dst_r * inv_alpha) / 255) as u8;
                buffer[buf_idx + 1] =
                    ((color[1] as u32 * eff_alpha + dst_g * inv_alpha) / 255) as u8;
                buffer[buf_idx + 2] =
                    ((color[2] as u32 * eff_alpha + dst_b * inv_alpha) / 255) as u8;
                buffer[buf_idx + 3] = 255;
            }
        }
    }
}
