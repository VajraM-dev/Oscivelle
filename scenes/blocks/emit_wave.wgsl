// Waveform: an oscilloscope trace across the frame at a seeded height. Its wiggle is built from the four
// bands (sub = slow swell, highs = fine jitter); kick onsets and drops punch the amplitude.
fn emit_wave_y(x: f32) -> f32 {
    let t = time();
    return 0.35 * sub() * sin(x * 1.5 + t * 0.7) + 0.25 * bass() * sin(x * 4.0 - t * 2.1)
        + 0.12 * mid() * sin(x * 11.0 + t * 5.3) + 0.06 * high() * sin(x * 29.0 - t * 9.0);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let y0 = fract(seed() * 13.0) - 0.5;
    let amp = EP(0) * (1.0 + 1.5 * punch() + drop_env());
    let d = abs(p.y - y0 - amp * emit_wave_y(p.x));
    let w = EP(1) * (1.0 + kick());
    return vec2<f32>(w * w / (d * d + w * w), fract(EP(2) + 0.1 * p.x));
}
