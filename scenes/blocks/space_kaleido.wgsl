// Kaleidoscope: N mirrored wedges around the centre; the pattern turns slowly and breathes on kicks.
fn space(p: vec2<f32>) -> vec2<f32> {
    let seg = TAU / floor(SP(0));
    var a = atan2(p.y, p.x) + SP(1) * time();
    a = a - seg * floor(a / seg);
    a = abs(a - 0.5 * seg);
    return vec2<f32>(cos(a), sin(a)) * length(p) * (1.0 - 0.04 * kick());
}
