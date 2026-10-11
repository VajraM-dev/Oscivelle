fn emit_clouds_bank(q: vec2<f32>, center: f32, n: f32, fine: f32) -> vec2<f32> {
    let top = center + 0.27 + (n - 0.5) * 0.44 + sin(q.x * 6.0 + center * 3.0) * 0.025;
    let bottom = center - 0.42 + (n - 0.5) * 0.25;
    let upper = 1.0 - smoothstep(top - 0.09, top + 0.10, q.y);
    let lower = smoothstep(bottom - 0.10, bottom + 0.11, q.y);
    let texture = 0.72 + 0.36 * fine;
    let density = upper * lower * texture;
    let wisps = 0.55 + 0.45 * smoothstep(0.3, 0.85, fine);
    let edge_top = exp(-abs(q.y - top) * 22.0) * wisps;
    let edge_bottom = exp(-abs(q.y - bottom) * 18.0) * (0.35 + 0.65 * fine);
    return vec2<f32>(density, edge_top + edge_bottom);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let t = time();
    let q = vec2<f32>(p.x / aspect(), p.y);
    let drift = vec2<f32>(sin(t * 0.035) * 0.22, cos(t * 0.027) * 0.07);
    let scale = EP(0);
    let n1 = fbm((q + drift) * vec2<f32>(3.2, 2.7) * scale);
    let n2 = fbm((q - drift * 0.7 + vec2<f32>(2.7, -1.3)) * vec2<f32>(3.8, 3.0) * scale);
    let fine = noise2((q + drift * 1.4) * vec2<f32>(11.0, 8.0) * scale);
    let a = emit_clouds_bank(q, -0.12, n1, fine);
    let b = emit_clouds_bank(q, 0.48, n2, noise2((q - drift) * vec2<f32>(9.0, 7.0) * scale + 5.4));
    let density = min(a.x + b.x * 0.82, 1.35);
    let edges = min(a.y + b.y * 0.72, 1.4);
    let bloom = 1.0 + 0.14 * kick() + 0.12 * drop_env();
    let intensity = EP(1) * bloom * (0.72 * density + 0.12 * edges);
    let hue = fract(EP(2) + 0.08 * (n1 - n2) + 0.025 * (fine - 0.5));
    return vec2<f32>(max(intensity, 0.0), hue);
}
