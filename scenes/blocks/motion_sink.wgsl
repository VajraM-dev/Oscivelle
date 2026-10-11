// Sink: the frame drains slowly inward (zoom < 1) toward a point off the centre that jumps to a new place
// every quarter phrase, so trails gather into a drifting focus instead of a tunnel. No spin.
fn motion_sink_at() -> vec2<f32> {
    let k = floor(phrase_phase() * 4.0) + 4.0 * floor(seed() * 64.0);
    let h = vec2<f32>(hash21(vec2<f32>(k, 1.3)), hash21(vec2<f32>(k, 7.9)));
    return (h - 0.5) * vec2<f32>(1.4 * aspect(), 1.4);
}

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let zoom = 1.0 - MP(0) * (0.6 + 0.4 * energy()) - 0.004 * kick();
    return warp(uv, zoom, 0.0, 0.0, motion_sink_at());
}

fn motion_decay() -> f32 {
    return MP(1);
}
