// Hyperbolic tiling (Escher's Circle Limit): fold each point into the fundamental triangle of a {p,q}
// tiling of the Poincare disk; the polygon edges are the circle mirror, so glow at distance to it.
// The disk drifts by a Mobius translation; kicks light every other tile (parity of the inversions).
fn emit_hypertile_cdiv(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x + a.y * b.y, a.y * b.x - a.x * b.y) / max(dot(b, b), 1e-6);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    var q = p / 1.05;
    let rim = dot(q, q);
    if (rim >= 1.0) { return vec2<f32>(0.0); }
    let w = 0.3 * vec2<f32>(cos(time() * 0.07), sin(time() * 0.05));
    q = emit_hypertile_cdiv(q - w, vec2<f32>(1.0 - (w.x * q.x + w.y * q.y), -(w.x * q.y - w.y * q.x)));
    let k = i32(EP(0));
    let pn = select(select(select(4.0, 5.0, k == 1), 3.0, k == 2), 6.0, k == 3);
    let qn = select(select(select(5.0, 4.0, k == 1), 7.0, k == 2), 4.0, k == 3);
    let a = PI / pn;
    let b = PI / qn;
    let d = 1.0 / sqrt(1.0 - sin(a) * sin(a) / (cos(b) * cos(b)));
    let r = d * sin(a) / cos(b);
    let c = vec2<f32>(d, 0.0);
    var n = 0;
    for (var i = 0; i < 24; i++) {
        let ang = atan2(q.y, q.x);
        var m = ang - 2.0 * a * floor(ang / (2.0 * a));
        m = select(m, 2.0 * a - m, m > a);
        q = vec2<f32>(cos(m), sin(m)) * length(q);
        let v = q - c;
        let l2 = dot(v, v);
        if (l2 >= r * r) { break; }
        q = c + v * (r * r / l2);
        n++;
    }
    let lw = max(EP(1) * (1.0 - rim), 1e-4);
    let line = 1.0 - smoothstep(0.0, lw, abs(length(q - c) - r));
    let par = f32(n % 2);
    let lit = kick() * par * 0.5 * (1.0 - rim);
    return vec2<f32>((line + lit) * (0.8 + 0.6 * drop_env()), fract(EP(2) + 0.18 * par + 0.1 * f32(n % 3)));
}
