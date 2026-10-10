// Hyperspace: the previous frame is pushed radially outward from the centre (faster near the
// edges, like flying forward) with a slow roll, leaving long after-glow streaks.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let p = to_centered(uv);
    let zoom = 1.0 + MP(0) * (0.6 + 0.4 * energy() + 0.8 * drop_env()) * (1.0 + 0.5 * length(p)) + 0.006 * kick();
    let roll = (seed() - 0.5) * 0.12 * dt();
    return to_uv(rot2(roll) * p / zoom);
}

fn motion_decay() -> f32 {
    return MP(1) * (1.0 - 0.06 * drop_env()) * (1.0 + 0.04 * tension());
}
