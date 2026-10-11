// Kaleidoscope (display only): the frame is shown as 4-9 mirrored wedges (by seed) that turn slowly; the
// feedback underneath keeps its own shape, so the same simulation gets a second framing.
fn comp(uv: vec2<f32>) -> vec3<f32> {
    let seg = TAU / (4.0 + floor(seed() * 6.0));
    let p = to_centered(uv);
    var a = atan2(p.y, p.x) + 0.05 * time();
    a = a - seg * floor(a / seg);
    a = abs(a - 0.5 * seg);
    return prev(to_uv(vec2<f32>(cos(a), sin(a)) * length(p)));
}
