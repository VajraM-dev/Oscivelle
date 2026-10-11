fn colour_whitehot_t(x: f32) -> f32 { return 0.25 + 0.5 * abs(fract(x) * 2.0 - 1.0); }
fn colour(e: vec2<f32>) -> vec3<f32> {
    let i = max(e.x, 0.0);
    // Cubic white term: invisible on faint detail, dominates the brightest cores (from liquid_tunnel).
    return pal(colour_whitehot_t(e.y + CP(0))) * i + vec3<f32>(CP(1) * i * i * i);
}
