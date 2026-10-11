// Step sequencer: 16 columns of rounded pads by 4-8 rows. A seeded pattern switches some pads on; the playhead
// column walks one step per 16th of the bar, lighting its pads (the switched-on ones flash on kicks).
fn emit(p: vec2<f32>) -> vec2<f32> {
    let rows = EP(0);
    let q = vec2<f32>((p.x / aspect() + 1.0) * 8.0, (p.y + 1.0) * 0.5 * rows);
    let c = floor(q);
    let f = fract(q) - 0.5;
    let on = step(1.0 - EP(1), hash21(c + vec2<f32>(floor(seed() * 977.0), 3.0)));
    let lit = select(0.0, 1.0, c.x == floor(bar_phase() * 16.0));
    let d = length(max(abs(f) - vec2<f32>(0.28), vec2<f32>(0.0))) - 0.12;
    let pad = 1.0 - smoothstep(0.0, 0.03, d);
    let i = pad * (0.08 + 0.15 * lit + on * (0.35 + lit * (0.8 + 1.2 * kick()))) * (1.0 + 0.5 * drop_env());
    return vec2<f32>(i, fract(EP(2) + 0.5 * c.y / rows + 0.25 * lit));
}
