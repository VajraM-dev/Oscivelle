// Contour filaments: crisp iso-lines of a domain-warped noise plasma, brighter on its ridges.
// Kick flares the lines; tension packs them tighter.

fn emit_contours_fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 3; i++) {
        v += a * noise2(p);
        p = rot2(0.6) * p * 2.02 + vec2<f32>(3.1, 1.7);
        a *= 0.5;
    }
    return v / 0.875;
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let t = time() * 0.1;
    let q0 = p * 1.6 + seed() * 7.0;
    let q = vec2<f32>(emit_contours_fbm(q0 + vec2<f32>(0.0, t)), emit_contours_fbm(q0 + vec2<f32>(5.2, 1.3) - t * 0.7));
    let v = emit_contours_fbm(q0 + EP(1) * q + vec2<f32>(0.15 * t, 0.0));
    let x = v * EP(0) * (1.0 + 0.6 * tension());
    let d = abs(fract(x) - 0.5);
    let dpx = (0.5 - d) / max(fwidth(x), 1e-4);  // distance to the line in pixels
    let fil = 1.0 - smoothstep(0.5, 1.5, dpx);
    let halo = exp(-dpx * 0.35) * 0.3;
    let ridge = smoothstep(0.35, 0.7, v);
    let i = (fil + halo) * (0.35 + ridge) * (0.6 + 1.0 * kick() + 0.4 * drop_env());
    return vec2<f32>(i, EP(2) + v * 0.6 + (q.x - q.y) * 1.5 + t * 0.2);
}
