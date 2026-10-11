// Pixel: the plane snaps to a grid of square cells, so trails and shapes become a blocky mosaic. The cell size
// re-rolls eight times a phrase (half to one and a half times the param) and the cells shrink on drops.
fn space(p: vec2<f32>) -> vec2<f32> {
    let k = floor(phrase_phase() * 8.0);
    let n = SP(0) * mix(0.5, 1.5, hash21(vec2<f32>(k, floor(seed() * 977.0)))) * (1.0 + drop_env());
    return (floor(p * n) + 0.5) / n;
}
