// Datamosh: the frame is cut into blocks; each kick re-rolls which blocks slide and in which direction,
// so the image smears in blocky P-frame chunks instead of flowing smoothly.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let n = MP(0);
    let cell = floor(uv * vec2<f32>(n * aspect(), n));
    let kid = floor((time() - since_kick()) * 4.0);   // constant between kicks
    let a = TAU * hash21(cell - kid * 3.0);
    let moving = step(1.0 - MP(1), hash21(cell + kid * 17.0));
    let speed = (1.5 + 6.0 * punch() + 4.0 * drop_env()) / resolution();
    let base = warp(uv, 1.0 + 0.002 * energy(), 0.0, 0.0, vec2<f32>(0.0));
    return base - vec2<f32>(cos(a), sin(a)) * speed * moving;
}

fn motion_decay() -> f32 {
    return MP(2) * (1.0 - 0.03 * drop_env());
}
