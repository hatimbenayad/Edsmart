//! Standard easing curves (REQ-DESIGN-001)

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let t_m1 = 1.0 - t;
    1.0 - t_m1 * t_m1 * t_m1
}

pub fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let s = 1.70158;
    let t_m1 = t - 1.0;
    1.0 + (s + 1.0) * t_m1 * t_m1 * t_m1 + s * t_m1 * t_m1
}
