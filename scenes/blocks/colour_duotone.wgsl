fn colour(e: vec2<f32>) -> vec3<f32> {
    // Entries 1..3 sit exactly at pal(0.25/0.5/0.75); entry 0 is the dark background.
    let k = floor(clamp(CP(0), 0.0, 1.0) * 2.999);
    let a = pal(0.25 * (1.0 + k));
    let b = pal(0.25 * (1.0 + (k + 1.0) % 3.0));
    let s = 0.5 + 0.5 * sin(TAU * e.y);   // seamless across the hue wrap
    let w = CP(1);
    return mix(a, b, smoothstep(0.5 - w, 0.5 + w, s)) * max(e.x, 0.0);
}
