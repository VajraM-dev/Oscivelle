// Plasma flow: the previous frame is advected along the swirl of a slowly drifting fbm field.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let t = time() * 0.1;
    let dr = drop_env();
    let p = to_centered(uv) * MP(0) + seed() * 7.0;
    let q = vec2<f32>(fbm(p + vec2<f32>(0.0, t)), fbm(p + vec2<f32>(5.2, 1.3) - t * 0.7));
    // Field offset turned 90 degrees: flow circles round the hills instead of piling up.
    let sw = vec2<f32>(-(q.y - 0.5), q.x - 0.5);
    let push = sw * MP(1) * (0.5 + 0.5 * energy() + 1.5 * dr) * (1.0 - 0.5 * tension());
    return warp(uv, 1.0 + 0.015 * kick() + 0.01 * dr, 0.0, 0.0, vec2<f32>(0.0)) + push;
}

fn motion_decay() -> f32 {
    return MP(2) * (1.0 - 0.04 * tension()) * (1.0 + 0.05 * kick());
}
