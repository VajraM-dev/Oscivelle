fn colour_iridescent_t(x: f32) -> f32 { return 0.25 + 0.5 * abs(fract(x) * 2.0 - 1.0); }
fn colour(e: vec2<f32>) -> vec3<f32> {
    let i = max(e.x, 0.0);
    let t = e.y + CP(0) + CP(1) * i;
    // Two offset samples blended by intensity: thin-film look, brighter where it's denser.
    let a = pal(colour_iridescent_t(t));
    let b = pal(colour_iridescent_t(t + 0.33));
    return mix(a, b, 0.5 * smoothstep(0.2, 1.2, i)) * i;
}
