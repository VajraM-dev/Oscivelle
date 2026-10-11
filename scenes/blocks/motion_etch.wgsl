// Etch: the image flows along its own brightness contours (perpendicular to the luma gradient), so
// shapes get engraved into rippling lines; kicks and drops push it across the edges, crystallising it.
fn motion_etch_l(uv: vec2<f32>) -> f32 { return dot(prev(uv), vec3<f32>(0.2126, 0.7152, 0.0722)); }

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let e = 2.0 / resolution();
    let g = vec2<f32>(
        motion_etch_l(uv + vec2<f32>(e.x, 0.0)) - motion_etch_l(uv - vec2<f32>(e.x, 0.0)),
        motion_etch_l(uv + vec2<f32>(0.0, e.y)) - motion_etch_l(uv - vec2<f32>(0.0, e.y))
    );
    let gn = g / (length(g) + 0.05);
    let along = vec2<f32>(-gn.y, gn.x) * MP(0);
    let across = gn * MP(1) * (punch() + drop_env());
    let base = warp(uv, 1.0 + 0.001 * energy(), 0.0, 0.0, vec2<f32>(0.0));
    return base + (along + across) / resolution();
}

fn motion_decay() -> f32 {
    return MP(2);
}
