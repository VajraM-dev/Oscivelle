// Mirror: the plane folds across one (or two) axes that slowly turn, so trails and shapes become symmetric.
fn space(p: vec2<f32>) -> vec2<f32> {
    var q = rot2(SP(1) * time()) * p;
    q.x = abs(q.x);
    if (SP(0) > 1.5) { q.y = abs(q.y); }
    return q;
}
