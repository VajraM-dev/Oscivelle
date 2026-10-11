// Droste: log-polar repeat. Each ring of the plane (ratio 2.4) maps onto the same band, so the frame
// repeats inside itself; the band slides with time (endless zoom) and the angle twists with depth.
fn space(p: vec2<f32>) -> vec2<f32> {
    let lr = log(2.4);
    let lz = log(max(length(p), 1e-4)) - SP(1) * time();
    let band = lz - lr * floor(lz / lr);
    let a = atan2(p.y, p.x) + SP(0) * lz;
    return vec2<f32>(cos(a), sin(a)) * exp(band) * 0.42;
}
