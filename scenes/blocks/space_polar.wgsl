// Turbine: N rotated copies around the centre (no mirror, so motion spins like a turbine) and rings that
// repeat outward (every ring maps onto the second one).
fn space(p: vec2<f32>) -> vec2<f32> {
    let seg = TAU / floor(SP(0));
    let a = atan2(p.y, p.x);
    let a2 = a - seg * floor(a / seg) - 0.5 * seg;
    let ring = 1.0 / SP(1);
    let r = length(p);
    let rr = r - ring * floor(r / ring) + ring;
    return vec2<f32>(cos(a2), sin(a2)) * rr;
}
