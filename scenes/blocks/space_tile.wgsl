// Tile: the plane repeats as a wallpaper of mirrored tiles (n per frame height); the grid is offset by seed so
// no tile is centred on the frame: the centre sits 0.2..0.3 of a tile from the nearest seam, never on a seam or a tile middle. Each tile shows the central square of the plane, so shapes and trails
// repeat across the grid, and tile edges meet without seams.
fn space(p: vec2<f32>) -> vec2<f32> {
    let u = fract(seed() * vec2<f32>(7.0, 3.0));
    let q = p * 0.5 * SP(0) + 0.2 + 0.1 * fract(u * 2.0) + 0.5 * step(vec2<f32>(0.5), u);
    // Mirrored repeat: a triangle wave 0..1..0, one ramp per tile.
    let f = abs(fract(q * 0.5) * 2.0 - 1.0);
    return f * 2.0 - 1.0;
}
