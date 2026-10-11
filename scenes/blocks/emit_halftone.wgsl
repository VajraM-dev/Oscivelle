// Halftone: a printed dot screen at a seeded angle. Each dot's size follows a slowly drifting noise image
// sampled at the dot's centre (brighter = bigger), so shapes read as newsprint; kicks swell every dot.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let a = 0.26 + 0.5 * fract(seed() * 7.0);
    let q = rot2(a) * p * EP(0);
    let c = floor(q) + 0.5;
    let src = fbm(rot2(-a) * c / EP(0) * 1.3 + vec2<f32>(time() * 0.05, floor(seed() * 97.0)));
    let tone = smoothstep(0.3, 0.75, src);
    let r = 0.5 * sqrt(tone) * (EP(1) + 0.25 * kick());
    let i = (1.0 - smoothstep(r - 0.06, r, length(q - c))) * (0.6 + 0.4 * tone) * (1.0 + 0.5 * drop_env());
    return vec2<f32>(i, fract(EP(2) + 0.3 * tone));
}
