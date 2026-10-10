// Deep zoom: a steady dive into a slowly wandering point with a gentle roll sway; each kick
// throws a burst of echoes outward, the drop a long wide one.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let t = time();
    let sd = seed();
    let k = kick();
    let tn = tension();
    let db = pow(drop_env(), 3.0);
    let ctr = vec2<f32>(sin(t * 0.07 + sd * TAU) * aspect(), sin(t * 0.053 + sd * 17.0)) * MP(2) * (1.0 - 0.7 * tn);
    let zoom = 1.0 + MP(0) * (1.0 + tn) + 0.025 * k * k + 0.04 * db;
    let ang = MP(1) * sin(t * 0.05 + sd * 9.0) + 0.002 * k + 0.006 * db;
    return warp(uv, zoom, ang, 0.0, ctr);
}

fn motion_decay() -> f32 {
    let k = kick();
    return mix(0.86, 0.93, max(k * k, 0.85 * pow(drop_env(), 3.0))) * (1.0 - 0.05 * tension());
}
