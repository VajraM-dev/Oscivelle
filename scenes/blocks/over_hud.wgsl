// Sci-fi HUD: a dashed ring turning slowly, a ring of tick marks (every third one long) and corner
// brackets, white tinted towards palette colour 2; flares on kicks.
fn over_hud_line(d: f32, w: f32) -> f32 {
    return 1.0 - smoothstep(0.0, w, d);
}

fn overlay(p: vec2<f32>) -> vec4<f32> {
    let q = p / OP(0);
    let r = length(q);
    let a = atan2(q.y, q.x);
    let px = 2.0 / resolution().y / OP(0);
    let dash = step(0.35, fract((a + time() * 0.05) * 24.0 / TAU));
    var m = over_hud_line(abs(r - 0.8), 2.5 * px) * dash;
    let k = a * 36.0 / TAU;
    let idx = i32(floor(k + 0.5));
    let tick = select(0.05, 0.1, idx % 3 == 0);
    let arc = abs(fract(k + 0.5) - 0.5) * TAU / 36.0 * r;
    m += over_hud_line(arc, 1.5 * px) * step(0.92, r) * step(r, 0.92 + tick);
    let aq = abs(q);
    m += over_hud_line(abs(aq.y - 1.15), 2.0 * px) * step(1.0, aq.x) * step(aq.x, 1.15);
    m += over_hud_line(abs(aq.x - 1.15), 2.0 * px) * step(1.0, aq.y) * step(aq.y, 1.15);
    let col = mix(vec3<f32>(0.85), pal(0.5), OP(1)) * (1.2 + 1.5 * kick());
    return vec4<f32>(col, clamp(m, 0.0, 1.0) * 0.85);
}
