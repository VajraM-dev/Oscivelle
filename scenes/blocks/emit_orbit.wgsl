// Julia orbit traps: z -> z^2 + c per pixel, c riding just inside the main cardioid of the Mandelbrot set
// (0.5 e^it - 0.25 e^2it), so the Julia set is always connected filigree. Light is trapped where the orbit
// passes near the origin and near a slowly swaying line; kicks nudge c.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let th = seed() * TAU + time() * 0.015;
    let c = (0.5 * vec2<f32>(cos(th), sin(th)) - 0.25 * vec2<f32>(cos(2.0 * th), sin(2.0 * th))) * (0.98 + 0.01 * kick());
    var z = p * EP(0);
    var dp = 1e3;
    var dl = 1e3;
    let n = select(48, 96, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        z = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
        if (dot(z, z) > 16.0) { break; }
        dp = min(dp, length(z));
        dl = min(dl, abs(z.y + 0.25 * sin(time() * 0.1)));
    }
    let i = exp(-dp * EP(1)) + 0.6 * exp(-dl * EP(1) * 2.0);
    return vec2<f32>(i * (0.6 + 0.5 * drop_env()), fract(EP(2) + 0.5 * dp));
}
