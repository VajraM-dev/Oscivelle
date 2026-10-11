// Mirror (display only): the finished frame folds into a left-right mirror image, or a four-way one (by
// seed). The simulation underneath runs unfolded, so this is only a framing.
fn comp(uv: vec2<f32>) -> vec3<f32> {
    var q = uv;
    q.x = 0.5 - abs(q.x - 0.5);
    if (fract(seed() * 7.0) > 0.5) { q.y = 0.5 - abs(q.y - 0.5); }
    return prev(q);
}
