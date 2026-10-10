// Palette position from the emitter hue, folded so it never lands on the dark background entry.
fn colour_hue_t(x: f32) -> f32 { return 0.25 + 0.5 * abs(fract(x) * 2.0 - 1.0); }
fn colour(e: vec2<f32>) -> vec3<f32> {
    return pal(colour_hue_t(e.y + CP(0))) * max(e.x, 0.0) * CP(1);
}
