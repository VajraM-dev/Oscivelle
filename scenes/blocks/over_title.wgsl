// The look's text in a 5x7 pixel font in a top corner (corner < 0.5 = left, else right),
// typed in over the first eighth of every phrase; white, brightening on kicks.
fn overlay(p: vec2<f32>) -> vec4<f32> {
    if (TEXT_LEN == 0) { return vec4<f32>(0.0); }
    let cell = OP(0);
    let w = f32(TEXT_LEN) * 6.0 - 1.0;
    let margin = 0.08;
    let x0 = select(-aspect() + margin, aspect() - margin - w * cell, OP(1) > 0.5);
    let q = vec2<f32>((p.x - x0) / cell, (1.0 - margin - p.y) / cell);
    let shown = floor(f32(TEXT_LEN) * clamp(phrase_phase() * 8.0, 0.0, 1.0) + 0.999);
    let on = text_px(q) * step(q.x, shown * 6.0);
    return vec4<f32>(vec3<f32>(1.0 + 0.5 * kick()), on);
}
