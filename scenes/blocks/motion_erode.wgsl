// Erode: each pixel copies its darkest of 4 rotating neighbours, so darkness eats into bright shapes and
// only fresh emission survives; during a drop it takes the brightest instead and everything swells.
fn motion_erode_l(uv: vec2<f32>) -> f32 { return dot(prev(uv), vec3<f32>(0.2126, 0.7152, 0.0722)); }

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let r = MP(0) / resolution();
    let grow = drop_env() > 0.3;
    var best = uv;
    var bl = motion_erode_l(uv);
    for (var i = 0; i < 4; i++) {
        let a = TAU * (f32(i) + 0.5) / 4.0 + time() * 0.3;   // rotating taps avoid grid artefacts
        let q = uv + vec2<f32>(cos(a), sin(a)) * r;
        let l = motion_erode_l(q);
        if ((l < bl) != grow) { best = q; bl = l; }
    }
    return warp(best, 1.0 + MP(1), 0.0, 0.0, vec2<f32>(0.0));
}

fn motion_decay() -> f32 {
    return MP(2);
}
