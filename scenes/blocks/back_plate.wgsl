// The look's picture filling the frame: slow Ken Burns drift (pan + zoom), a small punch-in on kicks.
// Zoom only breathes inward (>= BP(0) >= 1), so the 1.3 cover factor plus pan <= 0.08 keeps the 3:2
// picture's edges outside the 16:9 frame (1.778 / 1.3 + 0.08 < 1.5).
// Without a picture: a dark palette gradient.
fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let zoom = BP(0) * (1.02 + 0.02 * sin(time() * 0.05)) * (1.0 + 0.015 * kick());
    let pan = vec2<f32>(sin(time() * 0.031), cos(time() * 0.023)) * BP(1);
    let im = img(p / (1.3 * zoom) + pan);
    let fallback = mix(pal(0.0), pal(0.25) * 0.5, 0.5 + 0.5 * p.y);
    return mix(fallback, im.rgb * BP(2), im.a);
}
