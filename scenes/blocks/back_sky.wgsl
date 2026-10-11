// Gradient sky over a horizon: zenith = palette colour 1, horizon = pale colour 3, a soft sun that
// swells on drops, faint drifting haze; below the horizon a darker mirrored sky (calm sea or haze).
fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let horizon = BP(0);
    let h = p.y - horizon;
    let zen = pal(0.25) * 0.8;
    let low = mix(pal(0.75), vec3<f32>(1.0), 0.35);
    var c = mix(low, zen, smoothstep(0.0, 1.2, h));
    c = select(c, mix(low, pal(0.0), smoothstep(0.0, 0.6, -h)) * 0.8, h < 0.0);
    let sun = vec2<f32>(BP(1) + 0.15 * sin(time() * 0.02), horizon + BP(2));
    let d = length(p - sun);
    let disc = 1.0 - smoothstep(0.08, 0.09, d);
    c += pal(0.6) * (disc * 3.0 + 0.25 / (1.0 + d * d * 40.0)) * (1.0 + 0.8 * drop_env());
    c *= 0.92 + 0.08 * fbm(vec2<f32>(p.x * 0.8 + time() * 0.01, p.y * 4.0));
    return c;
}
