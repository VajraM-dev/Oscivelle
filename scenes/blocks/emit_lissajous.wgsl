// Glowing Lissajous curve (a : a+1), its phase drifting slowly so the figure rolls in 3D;
// hue runs along the curve, the kick thickens and punches it.

fn emit_lissajous_pt(s: f32, a: f32, ph: f32, sz: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(sin(a * s + ph), sin((a + 1.0) * s)) * sz;
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let k = kick();
    let a = floor(EP(0));
    let ph = time() * 0.2 + seed() * TAU;
    let r = EP(1) * (1.0 + 0.1 * k + 0.2 * drop_env());
    let sz = vec2<f32>(r * min(aspect(), 1.4), r);
    let w = 2.0 / resolution().y * (1.2 + 2.0 * k);
    let n = 48;  // ponytail: 48 segments, plenty for a <= 3; raise if the ratio range grows
    var g = 0.0;
    var gs = 0.0;
    var last = emit_lissajous_pt(0.0, a, ph, sz);
    for (var i = 1; i <= n; i++) {
        let s = f32(i) / f32(n) * TAU;
        let cur = emit_lissajous_pt(s, a, ph, sz);
        let gi = line_glow(p, last, cur, w);
        if (gi > g) { g = gi; gs = s; }
        last = cur;
    }
    return vec2<f32>(g * (0.7 + 0.8 * k), EP(2) + gs / TAU * 0.5);
}
