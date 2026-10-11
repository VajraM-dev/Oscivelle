// Fractal cathedral: a raymarched kaleidoscopic IFS (tetrahedral mirror folds, rotate, scale) seen from a
// slowly orbiting camera. Hits glow by their orbit trap; near-misses glow by step count, so the trails
// smear real depth and parallax.
fn emit_kifs3d_de(p0: vec3<f32>) -> vec2<f32> {
    var p = p0;
    var trap = 1e3;
    let s = EP(0);
    let rot = rot2(EP(1));
    for (var i = 0; i < 8; i++) {
        if (p.x + p.y < 0.0) { p = vec3<f32>(-p.y, -p.x, p.z); }
        if (p.x + p.z < 0.0) { p = vec3<f32>(-p.z, p.y, -p.x); }
        if (p.y + p.z < 0.0) { p = vec3<f32>(p.x, -p.z, -p.y); }
        let xz = rot * p.xz;
        p = vec3<f32>(xz.x, p.y, xz.y);
        p = p * s - vec3<f32>(s - 1.0);
        trap = min(trap, dot(p, p));
    }
    return vec2<f32>(length(p) * pow(s, -8.0), trap);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let a = time() * 0.06 + seed() * TAU;
    let ro = vec3<f32>(cos(a) * 3.2, 0.6 + 0.3 * sin(time() * 0.05), sin(a) * 3.2);
    let fw = normalize(-ro);
    let rt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fw));
    let up = cross(fw, rt);
    let rd = normalize(fw * 1.6 + rt * p.x + up * p.y);
    var t = 0.0;
    var hit = -1.0;
    var trap = 0.0;
    var steps = 0.0;
    let n = select(48, 80, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let h = emit_kifs3d_de(ro + rd * t);
        if (h.x < 0.0015 * t) { hit = t; trap = h.y; break; }
        t += h.x;
        steps += 1.0;
        if (t > 12.0) { break; }
    }
    let glow = steps / f32(n);
    if (hit < 0.0) { return vec2<f32>(glow * 0.6, fract(EP(2) + 0.5)); }
    let lt = log(trap + 1.0);
    let shade = exp(-0.25 * hit) * (0.6 + 0.6 * punch() + 0.5 * drop_env());
    return vec2<f32>(shade * (0.5 + 0.5 * fract(lt)) + glow * 0.3, fract(EP(2) + 0.15 * lt));
}
