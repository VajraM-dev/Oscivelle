// Hyperbolic: folds the plane into the fundamental triangle of a {p,q} tiling of the Poincare disk
// (prelude hyper_fold). A Mobius translation drifts through the tiling.
fn space(p: vec2<f32>) -> vec2<f32> {
    var q = p * 0.55;
    if (dot(q, q) > 1.0) { q = q / dot(q, q); }   // outside the disk: its mirror image inside
    let w = 0.35 * vec2<f32>(cos(SP(1) * time()), sin(SP(1) * time() * 0.7));
    return hyper_fold(q, i32(SP(0)), w).xy * 2.2;
}
