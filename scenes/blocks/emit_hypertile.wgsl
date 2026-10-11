// Hyperbolic tiling (Escher's Circle Limit): fold each point into the fundamental triangle of a {p,q}
// tiling of the Poincare disk (prelude hyper_fold); the polygon edges are the circle mirror, so glow at
// distance to it. The disk drifts by a Mobius translation; kicks light every other tile (inversion parity).
fn emit(p: vec2<f32>) -> vec2<f32> {
    let q0 = p / 1.05;
    let rim = dot(q0, q0);
    if (rim >= 1.0) { return vec2<f32>(0.0); }
    let w = 0.3 * vec2<f32>(cos(time() * 0.07), sin(time() * 0.05));
    let f = hyper_fold(q0, i32(EP(0)), w);
    let q = f.xy;
    let n = i32(f.z);
    let r = f.w;
    let c = vec2<f32>(sqrt(1.0 + r * r), 0.0);   // d^2 = 1 + r^2
    let lw = max(EP(1) * (1.0 - rim), 1e-4);
    let line = 1.0 - smoothstep(0.0, lw, abs(length(q - c) - r));
    let par = f32(n % 2);
    let lit = kick() * par * 0.5 * (1.0 - rim);
    return vec2<f32>((line + lit) * (0.8 + 0.6 * drop_env()), fract(EP(2) + 0.18 * par + 0.1 * f32(n % 3)));
}
