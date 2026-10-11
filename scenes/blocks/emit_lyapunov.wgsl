// Lyapunov fractal: iterate x -> r x (1 - x) with r taking a or b in the sequence AABAB; the averaged
// log|r (1 - 2x)| is negative in stable zones (glow) and positive in chaos (dark). The window over
// (a, b) space pans slowly; the zero boundary glows on kicks.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let centre = vec2<f32>(3.4, 3.4) + 0.25 * vec2<f32>(sin(time() * 0.03), cos(time() * 0.023));
    let ab = clamp(centre + p * EP(0), vec2<f32>(2.0), vec2<f32>(4.0));
    var x = 0.5;
    var lam = 0.0;
    let n = select(40, 80, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let m = i % 5;
        let r = select(ab.x, ab.y, m == 2 || m == 4);
        if (i >= 10) { lam += log(max(abs(r * (1.0 - 2.0 * x)), 1e-6)); }
        x = r * x * (1.0 - x);
    }
    lam /= f32(n - 10);
    let stable = clamp(-lam * EP(1), 0.0, 1.5);
    let edge = exp(-abs(lam) * 8.0) * (0.5 + kick());
    return vec2<f32>(stable * 0.6 + edge * 0.5, fract(EP(2) + 0.35 * clamp(lam, -1.5, 0.5)));
}
