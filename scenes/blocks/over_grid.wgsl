// Retro perspective floor grid scrolling towards the viewer one cell per beat (seamless: the phase
// wraps by a whole cell), fading into the horizon. Draws only below the horizon. Line widths come
// from the projection (no derivatives).
fn overlay(p: vec2<f32>) -> vec4<f32> {
    let h = OP(0) - p.y;
    if (h <= 0.0) { return vec4<f32>(0.0); }
    let z = 1.0 / h;
    let x = p.x * z;
    let pix = 2.0 / resolution().y;
    let du = abs(fract(z * 0.5 + beat_phase() + 0.5) - 0.5) * 2.0;
    let lz = 1.0 - smoothstep(0.0, 1.5 * pix / (h * h), du);
    let dx = abs(fract(x * 0.5 + 0.5) - 0.5) * 2.0;
    let lx = 1.0 - smoothstep(0.0, 1.5 * pix * z, dx);
    let m = max(lz, lx) * smoothstep(0.0, 0.25, h);
    return vec4<f32>(pal(0.5) * (1.5 + 2.0 * kick()), m * OP(1));
}
