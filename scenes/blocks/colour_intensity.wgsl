fn colour_intensity_t(x: f32) -> f32 { return 0.25 + 0.5 * abs(fract(x) * 2.0 - 1.0); }
fn colour(e: vec2<f32>) -> vec3<f32> {
    let i = max(e.x, 0.0);
    // A touch of the emitter hue keeps some variation inside one intensity level.
    return pal(colour_intensity_t(CP(0) + CP(1) * i + 0.1 * e.y)) * i;
}
