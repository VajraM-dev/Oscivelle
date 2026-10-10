// Stars: hashed points on three grid layers, twinkling; some only light up on the kick.
// Pair with an outward motion to streak them.

// Returns (intensity, intensity-weighted hue) of one layer; sc = cells per unit.
fn emit_stars_layer(p: vec2<f32>, sc: f32, dens: f32, li: f32) -> vec2<f32> {
    let q = p * sc;
    let c = floor(q);
    let h = hash21(c + vec2<f32>(li * 131.0, floor(seed() * 997.0)));
    let o = vec2<f32>(fract(h * 53.7), fract(h * 91.3)) * 0.7 + 0.15;
    let d = length(fract(q) - o) / sc;  // centred units
    let w = 2.0 / resolution().y * EP(1) * (1.0 + 0.8 * kick());
    let h2 = fract(h * 71.3);
    let tw = 0.6 + 0.4 * sin(time() * (2.0 + 5.0 * h2) + h * 50.0);
    let on = select(1.0, 1.5 * kick(), fract(h * 37.9) > 0.7);
    let b = exp(-d * d / (w * w)) * tw * on * step(h, dens) * (0.6 + 0.9 * h2);
    return vec2<f32>(b, b * h2);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let dens = EP(0);
    let s = emit_stars_layer(p, 5.0, dens, 1.0) + emit_stars_layer(p, 11.0, dens, 2.0) + emit_stars_layer(p, 23.0, dens, 3.0);
    return vec2<f32>(s.x * 1.5, EP(2) + 0.6 * s.y / max(s.x, 1e-4));
}
