// Bands: the plane is cut into horizontal strips (vertical, by seed); each slides sideways by its own amount,
// re-rolled on every kick, and every other strip is mirrored, so the picture shears into offset bands.
fn space(p: vec2<f32>) -> vec2<f32> {
    let vert = fract(seed() * 23.0) > 0.5;
    var q = select(p, p.yx, vert);
    let band = floor(q.y * SP(0));
    let kid = floor((time() - since_kick()) * 4.0);   // constant between kicks
    q.x += (hash21(vec2<f32>(band, kid + floor(seed() * 977.0))) - 0.5) * SP(1);
    if (i32(band) % 2 != 0) { q.x = -q.x; }
    return select(q, q.yx, vert);
}
