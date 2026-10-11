// Still: the camera holds. Nothing moves except what the emitter draws; trails fade in place, a little slower
// in tense build-ups, and kick onsets cut them short so each beat reads as a fresh frame.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    return uv;
}

fn motion_decay() -> f32 {
    return MP(0) * (1.0 - MP(1) * punch()) * (1.0 + 0.03 * tension());
}
