// Ink in water: trails are carried by a divergence-free curl-noise current and fade slowly,
// so emitted shapes stretch into fine tendrils. Never rotates or zooms in (keeps edges clean).
fn motion_ink_psi(p0: vec2<f32>, t: f32) -> f32 {
    var p = p0 + vec2<f32>(t * 0.035, -t * 0.025);
    var v = 0.0;
    var a = 0.5;
    // ponytail: 3 soft octaves; finer octaves fold the advection map into creases.
    for (var i = 0; i < 3; i++) {
        v += a * noise2(p);
        p = rot2(0.6 + 0.05 * t) * p * 2.0 + vec2<f32>(t * 0.06, 11.0);
        a *= 0.4;
    }
    return v;
}

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let t = time();
    let k = kick();
    let db = pow(drop_env(), 3.0); // fast part of the drop
    let p = to_centered(uv);
    let q = p * MP(0) + seed() * 37.0;
    let h = 0.015 * MP(0);
    let dx = motion_ink_psi(q + vec2<f32>(h, 0.0), t) - motion_ink_psi(q - vec2<f32>(h, 0.0), t);
    let dy = motion_ink_psi(q + vec2<f32>(0.0, h), t) - motion_ink_psi(q - vec2<f32>(0.0, h), t);
    let v = vec2<f32>(dy, -dx) * (1.2 / h); // ~unit-speed current
    let spd = MP(1) * (0.5 + 0.5 * energy()) * (1.0 + 1.2 * k) * (1.0 - 0.7 * tension()) + 0.006 * db;
    // Gentle outward spread; the drop blasts it out.
    let zoom = 1.0008 + 0.006 * k * k + 0.008 * db;
    return warp(to_uv(p - v * spd), zoom, 0.0, 0.0, vec2<f32>(0.0));
}

fn motion_decay() -> f32 {
    return mix(MP(2), 0.99, tension());
}
