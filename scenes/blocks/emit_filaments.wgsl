// Thin liquid filaments: domain-warped ridged noise, lit in slow patches so the frame keeps
// negative space. Kick flares them; tension tightens them.

fn emit_filaments_ridge(p0: vec2<f32>) -> f32 {
    var p = p0;
    var v = 0.0;
    var a = 0.6;
    for (var i = 0; i < 4; i++) {
        let n = 1.0 - abs(noise2(p) * 2.0 - 1.0);
        v += a * n * n;
        p = rot2(0.7) * p * 2.1 + 11.0;
        a *= 0.5;
    }
    return v;
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let t = time();
    let q = p * EP(0) + seed() * 100.0;
    let wv = vec2<f32>(noise2(q * 0.5 + vec2<f32>(t * 0.05, 0.0)), noise2(q * 0.5 + vec2<f32>(5.2, 1.3 - t * 0.04)));
    let thin = EP(1) * (1.0 + 0.25 * tension());
    let fil = pow(clamp(emit_filaments_ridge(q + wv * 1.6 + vec2<f32>(0.0, t * 0.08)), 0.0, 1.0), thin);
    let patches = smoothstep(0.35, 0.65, noise2(q * 0.25 + vec2<f32>(t * 0.03, 0.0)));
    let i = fil * (0.3 + 0.7 * patches) * (0.6 + 0.8 * kick() + 0.4 * drop_env());
    return vec2<f32>(i, EP(2) + 0.5 * wv.x + t * 0.01);
}
