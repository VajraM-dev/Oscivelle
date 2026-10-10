// Flight down a tube: zoom out of a wandering vanishing point, twisted by depth (log radius).
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let t = time();
    let s = seed() * 100.0;
    let tn = tension();
    let k = kick();
    // Vanishing point wanders; it settles to the centre as a build tightens.
    let ctr = vec2<f32>(sin(t * 0.13 + s) * aspect(), cos(t * 0.1 + s * 1.7)) * MP(2) * (1.0 - 0.8 * tn) * 0.4;
    var p = to_centered(uv) - ctr;
    let r = max(length(p), 0.002);
    let zoom = 1.0 + MP(0) * (1.0 + 1.5 * tn) + 0.004 * k + 0.003 * drop_env();
    // Twist grows with log-depth, so the walls spiral like a rifled tube.
    let ang = MP(1) * (1.0 - 0.35 * log(r)) * mix(0.5, 1.0, energy());
    p = rot2(ang) * p / zoom;
    return to_uv(p + ctr);
}

fn motion_decay() -> f32 {
    return (mix(0.8, 0.86, tension()) + 0.03 * energy()) * (1.0 + 0.1 * kick());
}
