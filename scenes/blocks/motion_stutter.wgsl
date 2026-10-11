// Stutter: the frame holds still between kicks; on the one step a kick lands it jumps by a whole tile (1/n of
// the height) in a direction re-rolled per kick, wrapping round, like a glitching freeze-frame. Drops jump two.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let kid = floor((time() - since_kick()) * 4.0);   // constant between kicks
    let a = floor(hash21(vec2<f32>(kid, floor(seed() * 97.0))) * 4.0) * (TAU / 4.0);
    let on = select(0.0, 1.0, since_kick() < dt());
    let tile = vec2<f32>(1.0 / (MP(0) * aspect()), 1.0 / MP(0));
    return fract(uv - vec2<f32>(cos(a), sin(a)) * tile * on * (1.0 + step(0.5, drop_env())));
}

fn motion_decay() -> f32 {
    return MP(1);
}
