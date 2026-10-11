// Drift: the whole frame slides one way (one of 8 directions, by seed) and wraps around the edges, so trails
// stream across like a scrolling landscape; kick onsets and drops shove it further. No zoom, no spin.
fn motion_drift_dir() -> vec2<f32> {
    let a = TAU * floor(seed() * 8.0) / 8.0;
    return vec2<f32>(cos(a), sin(a));
}

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let speed = MP(0) * (0.5 + 0.5 * energy()) + MP(1) * punch() + 3.0 * drop_env();
    return fract(uv - motion_drift_dir() * speed / resolution());
}

fn motion_decay() -> f32 {
    return MP(2);
}
