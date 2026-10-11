// Scatter: sparks on a jittered grid. Each kick re-rolls which cells light up and where in their cell the spark
// sits, so the frame sparkles in fresh random spots every beat; sparks fade between kicks, drops light more.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let q = p * EP(0);
    let c = floor(q);
    let kid = floor((time() - since_kick()) * 4.0);   // constant between kicks
    let h = hash21(c + vec2<f32>(kid * 7.0, floor(seed() * 131.0)));
    let o = vec2<f32>(fract(h * 37.1), fract(h * 91.7)) * 0.7 + 0.15;
    let d = length(fract(q) - o) / EP(0);
    let r = EP(1) * (0.5 + 0.8 * fract(h * 13.3));
    let on = step(1.0 - EP(2) - 0.4 * drop_env(), fract(h * 57.3));
    let spark = exp(-d * d / (r * r)) * on * (0.25 + 1.2 * exp(-since_kick() * 3.0));
    return vec2<f32>(spark, fract(h * 3.7));
}
