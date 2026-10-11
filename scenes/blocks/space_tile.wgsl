// Tile: the plane repeats as a wallpaper of mirrored tiles (n per frame height); the grid is offset by seed so
// no tile is centred on the frame. Each tile shows the central square of the plane, so shapes and trails
// repeat across the grid, and tile edges meet without seams.
fn space(p: vec2<f32>) -> vec2<f32> {
    let q = p * 0.5 * SP(0) + vec2<f32>(fract(seed() * 7.0), fract(seed() * 3.0));
    // Mirrored repeat: a triangle wave 0..1..0, one ramp per tile.
    let f = abs(fract(q * 0.5) * 2.0 - 1.0);
    return f * 2.0 - 1.0;
}
