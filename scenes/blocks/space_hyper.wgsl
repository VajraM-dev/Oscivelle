// Hyperbolic: folds the plane into the fundamental triangle of a {p,q} tiling of the Poincare disk.
// Mirrors: the x-axis, a line at angle pi/p, and a circle orthogonal to the unit circle (centre d, radius r
// with d^2 = 1 + r^2 and cos(pi/q) = d sin(pi/p) / r). A Mobius translation drifts through the tiling.
fn space_hyper_cdiv(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x + a.y * b.y, a.y * b.x - a.x * b.y) / max(dot(b, b), 1e-6);
}

fn space(p: vec2<f32>) -> vec2<f32> {
    var q = p * 0.55;
    if (dot(q, q) > 1.0) { q = q / dot(q, q); }   // outside the disk: its mirror image inside
    let w = 0.35 * vec2<f32>(cos(SP(1) * time()), sin(SP(1) * time() * 0.7));
    q = space_hyper_cdiv(q - w, vec2<f32>(1.0 - (w.x * q.x + w.y * q.y), -(w.x * q.y - w.y * q.x)));
    let k = i32(SP(0));
    let pn = select(select(select(4.0, 5.0, k == 1), 3.0, k == 2), 6.0, k == 3);
    let qn = select(select(select(5.0, 4.0, k == 1), 7.0, k == 2), 4.0, k == 3);
    let a = PI / pn;
    let b = PI / qn;
    let d = 1.0 / sqrt(1.0 - sin(a) * sin(a) / (cos(b) * cos(b)));
    let r = d * sin(a) / cos(b);
    let c = vec2<f32>(d, 0.0);
    for (var i = 0; i < 24; i++) {
        let ang = atan2(q.y, q.x);
        var m = ang - 2.0 * a * floor(ang / (2.0 * a));
        m = select(m, 2.0 * a - m, m > a);
        q = vec2<f32>(cos(m), sin(m)) * length(q);
        let v = q - c;
        let l2 = dot(v, v);
        if (l2 >= r * r) { break; }
        q = c + v * (r * r / l2);
    }
    return q * 2.2;
}
