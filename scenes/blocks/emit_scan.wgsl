// Scan bars: three bars a third of a beat apart sweep down the frame (or across it, by seed), one full sweep
// per beat, so they lock to the tempo; the lead bar thickens on kicks, drops brighten all of them.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let across = fract(seed() * 41.0) > 0.5;
    let x = select(0.5 - 0.5 * p.y, 0.5 + 0.5 * p.x / aspect(), across);
    var i = 0.0;
    var hue = 0.0;
    for (var k = 0; k < 3; k++) {
        let fk = f32(k);
        let d0 = abs(x - fract(beat_phase() + fk / 3.0));
        let d = min(d0, 1.0 - d0);
        let w = EP(0) * select(1.0, 1.0 + 2.0 * kick(), k == 0);
        let b = exp(-d * d / (w * w));
        i += b;
        hue += b * fk / 3.0;
    }
    return vec2<f32>(i * EP(1) * (1.0 + drop_env()), fract(EP(2) + hue / max(i, 1e-4)));
}
