// Mandelbox: box fold (clamp-reflect at ±1) + sphere fold (min radius 0.5, fixed radius 1), then
// scale and add the start point. The scale's sign is structural: negative = spiky shells, positive =
// blocky towers. Orbited slowly; glows by its orbit trap.
fn emit_mandelbox_scale() -> f32 {
    return select(-1.0, 1.0, EP(0) > 0.0) * (1.8 + 0.8 * abs(EP(0)));
}

fn emit_mandelbox_de(p0: vec3<f32>) -> vec2<f32> {
    let s = emit_mandelbox_scale();
    var p = p0;
    var dr = 1.0;
    var trap = 1e3;
    for (var i = 0; i < 10; i++) {
        p = clamp(p, vec3<f32>(-1.0), vec3<f32>(1.0)) * 2.0 - p;
        let r2 = dot(p, p);
        if (r2 < 0.25) { p *= 4.0; dr *= 4.0; } else if (r2 < 1.0) { p /= r2; dr /= r2; }
        p = p * s + p0;
        dr = dr * abs(s) + 1.0;
        trap = min(trap, r2);
    }
    return vec2<f32>(length(p) / abs(dr), trap);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let dist = select(5.0, 9.0, emit_mandelbox_scale() > 0.0);
    let a = time() * 0.05 + seed() * TAU;
    let ro = vec3<f32>(cos(a), 0.35 + 0.2 * sin(time() * 0.04), sin(a)) * dist;
    let fw = normalize(-ro);
    let rt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fw));
    let up = cross(fw, rt);
    let rd = normalize(fw * 1.5 + rt * p.x + up * p.y);
    var t = 0.0;
    var hit = -1.0;
    var trap = 0.0;
    var steps = 0.0;
    let n = select(64, 96, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let h = emit_mandelbox_de(ro + rd * t);
        if (h.x < 0.002 * t) { hit = t; trap = h.y; break; }
        t += h.x;
        steps += 1.0;
        if (t > 20.0) { break; }
    }
    let glow = steps / f32(n);
    if (hit < 0.0) { return vec2<f32>(glow * 0.5 * EP(1), fract(EP(2) + 0.5)); }
    let shade = exp(-0.12 * hit) * EP(1) * (0.6 + 0.6 * punch() + 0.5 * drop_env());
    return vec2<f32>(shade * (0.4 + 0.6 * fract(trap * 2.0)) + glow * 0.3, fract(EP(2) + 0.2 * sqrt(trap)));
}
