// Slit-scan (display only): each row (or column, by seed) shows the frame from a different moment, up to
// ~2 s back, so moving shapes stretch into rubbery time-smears. The depth breathes with the phrase.
fn comp(uv: vec2<f32>) -> vec3<f32> {
    let along = select(uv.y, uv.x, fract(seed() * 13.0) > 0.5);
    return hist(uv, along * 63.0 * (0.6 + 0.4 * sin(TAU * phrase_phase())));
}
