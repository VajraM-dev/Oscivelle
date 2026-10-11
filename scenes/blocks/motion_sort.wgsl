// Pixel sort: bright pixels (above a threshold that dips on kicks) smear one way (vertical or sideways
// by seed) while dark ones stay put, so the image drips and streaks like a sorted glitch photo.
fn motion_sort_luma(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let px = 1.0 / resolution();
    let dir = select(vec2<f32>(0.0, -1.0), vec2<f32>(1.0, 0.0), fract(seed() * 31.0) > 0.5);
    let l = motion_sort_luma(prev(uv - dir * px * 2.0));
    let thr = MP(0) * (1.0 - 0.3 * kick());
    let run = smoothstep(thr, thr + 0.1, l) * (MP(1) + 4.0 * drop_env());
    return uv - dir * px * run;
}

fn motion_decay() -> f32 {
    return MP(2);
}
