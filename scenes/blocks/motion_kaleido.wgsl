// Kaleidoscope: the previous frame is sampled through an N-fold mirror fold that slowly rotates,
// with a gentle outward zoom, so any shape becomes a blooming mandala.
fn motion_kaleido_fold(a: f32, seg: f32) -> f32 {
    let w = a - seg * floor(a / seg);
    return abs(w - 0.5 * seg);
}

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let t = time();
    let spin = 0.045 * select(1.0, -1.0, fract(seed() * 50.0) > 0.5); // rad/s, direction from seed
    let seg = TAU / floor(MP(0));
    let p = to_centered(uv);
    let r = length(p);
    let fa = motion_kaleido_fold(atan2(p.y, p.x) - spin * t, seg) + spin * t;
    let qs = vec2<f32>(cos(fa), sin(fa)) * r;
    let zoom = 1.0 + MP(1) * (1.0 + 1.5 * tension()) + 0.003 * kick() + 0.003 * drop_env();
    return warp(to_uv(qs), zoom, spin * dt() + MP(2) * mix(0.5, 1.0, energy()), 0.0, vec2<f32>(0.0));
}

fn motion_decay() -> f32 {
    return (mix(0.8, 0.87, tension()) + 0.03 * energy()) * (1.0 + 0.1 * kick());
}
