// Marble: Inigo Quilez's domain warping, fbm(p + k·fbm(p + k·fbm(p))), so the noise folds into marbled
// liquid veins. Kicks and drops deepen the warp; the hue follows the warp field.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let t = time() * 0.05;
    let s = p * EP(0);
    let q = vec2<f32>(fbm(s + vec2<f32>(0.0, t)), fbm(s + vec2<f32>(5.2, 1.3 - t)));
    let k = EP(1) * (1.0 + 0.6 * punch() + 0.8 * drop_env());
    let r = vec2<f32>(fbm(s + k * q + vec2<f32>(1.7, 9.2) + 0.15 * t), fbm(s + k * q + vec2<f32>(8.3, 2.8) - 0.12 * t));
    let f = fbm(s + k * r);
    let i = smoothstep(0.45, 0.85, f) * (0.5 + 0.8 * length(q));
    return vec2<f32>(i * 0.9, fract(EP(2) + 0.6 * r.x + 0.2 * q.y));
}
