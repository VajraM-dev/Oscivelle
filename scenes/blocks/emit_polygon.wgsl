// Neon polygons: a spinning outline, a counter-rotating inner one, and a shockwave outline
// that grows across the beat after each kick.

// Glow of the regular n-gon outline (n <= 7), radius rad, rotation ang, line width w.
fn emit_polygon_shape(p: vec2<f32>, n: f32, rad: f32, ang: f32, w: f32) -> f32 {
    let st = TAU / n;
    var g = 0.0;
    for (var i = 0; i < 7; i++) {
        if (f32(i) >= n) { break; }
        let a0 = ang + f32(i) * st;
        let a = vec2<f32>(cos(a0), sin(a0)) * rad;
        let b = vec2<f32>(cos(a0 + st), sin(a0 + st)) * rad;
        g = max(g, line_glow(p, a, b, w));  // max, not sum: no hot corners
    }
    return g;
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let t = time();
    let k = kick();
    let dr = drop_env();
    let px = 2.0 / resolution().y;
    let dir = select(-1.0, 1.0, seed() > 0.5);
    let n = floor(EP(0));
    let rad = EP(1) * (1.0 + 0.12 * k + 0.3 * dr);
    let ang = dir * t * 0.35;
    let outer = emit_polygon_shape(p, n, rad, ang, px * (1.2 + 1.5 * k)) * (0.5 + 0.8 * k);
    let inner = emit_polygon_shape(p, n, rad * 0.5, -dir * t * 0.6 + 0.3, px) * 0.6;
    let shock = emit_polygon_shape(p, n, rad * 0.5 + beat_phase() * 1.8, ang, px * 1.5) * (0.9 * k + dr);
    let i = outer + inner + shock;
    let hue = EP(2) + t * 0.02 + (0.35 * inner + 0.65 * shock) / max(i, 1e-4);
    return vec2<f32>(i, hue);
}
