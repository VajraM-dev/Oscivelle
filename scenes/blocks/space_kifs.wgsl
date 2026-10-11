// Crystal: a 2D kaleidoscopic IFS. Each round mirror-folds, rotates, scales and shifts the plane, so the
// image repeats as a branching crystal lattice; the fold angle drifts with the phrase.
fn space(p: vec2<f32>) -> vec2<f32> {
    let k = i32(SP(0));
    let s = SP(1);
    let ang = 0.55 + 0.2 * sin(TAU * phrase_phase());
    let off = vec2<f32>(0.6, 0.25);
    var q = p;
    for (var i = 0; i < k; i++) {
        q = rot2(ang) * abs(q);
        q = q * s - off * (s - 1.0);
    }
    return q / pow(s, f32(k));
}
