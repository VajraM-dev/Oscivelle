// Shear: each row (or column, by seed) slides sideways by a wave travelling along the frame, on top of a slow
// one-way drift, so trails ripple into flags and ribbons; kicks swell the wave, drops whip it.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let cols = fract(seed() * 17.0) > 0.5;
    let along = select(uv.y, uv.x, cols);
    let wave = sin(TAU * (along * MP(0) - 0.25 * time()));
    let amt = MP(1) * (0.5 + wave) * (1.0 + 2.0 * kick()) + 4.0 * drop_env() * wave;
    let d = select(vec2<f32>(amt, 0.0), vec2<f32>(0.0, amt), cols) / resolution();
    return fract(uv - d);
}

fn motion_decay() -> f32 {
    return MP(2);
}
