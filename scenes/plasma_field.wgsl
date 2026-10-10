// Plasma field: a domain-warped noise plasma, lit like wet silk (normals from the field's
// screen-space slope) with crisp anti-aliased contour filaments. Only the field's ridges are
// lit, so dark valleys keep the contrast. The fresh field is blended (bounded moving average,
// so it can never flood) over the last frame advected along the warp's swirl, giving trails.
// Audio: kick -> filament flare + zoom punch; drop -> instant refresh, wider lit ridges,
// stronger swirl; tension -> ridges narrow and darken, filaments tighten, flow slows.

// Integer-hash value noise: exact at any coordinate (the prelude's float hash rounds
// differently on either side of a cell edge at large coordinates, which shows as shards).
fn pf_hash(i: vec2<i32>) -> f32 {
    var h = bitcast<u32>(i.x) * 1597334677u ^ bitcast<u32>(i.y) * 3812015801u;
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = h ^ (h >> 13u);
    return f32(h & 0xffffffu) / 16777216.0;
}

fn pf_noise(p: vec2<f32>) -> f32 {
    let fl = floor(p);
    let i = vec2<i32>(fl);
    let f = p - fl;
    let w = f * f * (3.0 - 2.0 * f);
    let a = pf_hash(i);
    let b = pf_hash(i + vec2<i32>(1, 0));
    let c = pf_hash(i + vec2<i32>(0, 1));
    let d = pf_hash(i + vec2<i32>(1, 1));
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

fn pf_fbm(p0: vec2<f32>, oct: i32) -> f32 {
    var p = p0;
    var v = 0.0;
    var a = 0.5;
    var norm = 0.0;
    for (var i = 0; i < oct; i++) {
        v += a * pf_noise(p);
        norm += a;
        p = rot2(0.6) * p * 2.02 + vec2<f32>(3.1, 1.7);
        a *= 0.5;
    }
    return v / norm;
}

fn scene(uv: vec2<f32>) -> vec4<f32> {
    let e = energy();
    let k = kick();
    let tn = tension();
    let dr = drop_env();
    let t = time() * 0.1;
    let pc = to_centered(uv);
    let p = pc * P(0) + seed() * 7.0;

    let q = vec2<f32>(pf_fbm(p + vec2<f32>(0.0, t), 3), pf_fbm(p + vec2<f32>(5.2, 1.3) - t * 0.7, 3));
    let r = vec2<f32>(pf_fbm(p + 2.5 * q + vec2<f32>(1.7, 9.2) + 0.15 * t, 3),
                      pf_fbm(p + 2.5 * q + vec2<f32>(8.3, 2.8) - 0.12 * t, 3));
    let v = pf_fbm(p + P(1) * r, 5);

    // Silk lighting from the field slope.
    let g = vec2<f32>(dpdxFine(v), dpdyFine(v)) * resolution().y * 0.25;
    let n = normalize(vec3<f32>(-g, 1.0));
    let ldir = normalize(vec3<f32>(cos(t * 2.0), sin(t * 2.0), 1.2));
    let diff = max(dot(n, ldir), 0.0);
    let spec = pow(max(dot(n, normalize(ldir + vec3<f32>(0.0, 0.0, 1.0))), 0.0), 24.0);

    let hue = P(3) + v * P(2) + (q.x - q.y) * 1.5 + t * 0.2;
    let lo = mix(0.4, 0.48, tn) - 0.06 * dr - 0.04 * k;
    let ridge = smoothstep(lo, lo + 0.3, v);
    var fresh = pal(hue) * pow(ridge, 1.6) * (0.35 + 0.9 * diff) * (0.8 + 0.4 * e + 0.5 * k);
    fresh += pal(hue + 0.5) * pow(ridge, 3.0) * spec * 0.8;

    // Crisp contour filaments, 1-2 px wide at any scale.
    let x = v * P(4) * (1.0 + 0.6 * tn);
    let d = abs(fract(x) - 0.5);
    let fw = max(fwidth(x), 1e-4);
    let dpx = (0.5 - d) / fw;
    let fil = 1.0 - smoothstep(0.5, 1.5, dpx);
    let halo = exp(-dpx * 0.35) * 0.35;
    fresh += mix(pal(hue + 0.3), vec3<f32>(1.0), 0.15 * fil) * (fil + halo) * (0.45 + ridge) * (0.6 + 1.6 * k + 0.6 * dr);

    // Fine silk threads inside the lit ridges.
    let x2 = x * 3.0;
    let d2 = (0.5 - abs(fract(x2) - 0.5)) / max(fwidth(x2), 1e-4);
    fresh += pal(hue + 0.1) * (1.0 - smoothstep(0.3, 1.2, d2)) * ridge * 0.3;

    // Feedback flows along the swirl of the warp field; kick punches a zoom.
    let sw = vec2<f32>(-(r.y - 0.5), r.x - 0.5);
    let push = sw * P(5) * (0.5 + 0.5 * e + 1.5 * dr) * (1.0 - 0.5 * tn);
    let wuv = warp(uv, 1.0 + 0.015 * k + 0.01 * dr, 0.0, 0.0, vec2<f32>(0.0)) + push;
    let old = prev(wuv);

    let blend = clamp(P(6) * (1.0 - 0.4 * tn) + 0.3 * k + 0.5 * pow(dr, 4.0), 0.0, 1.0);
    return vec4<f32>(mix(old, fresh, blend), 1.0);
}
