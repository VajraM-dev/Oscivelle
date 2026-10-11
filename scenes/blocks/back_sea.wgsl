// Raymarched 90s-CGI world: a calm reflective sea (analytic plane), a pyramid standing in it and a
// marbled moon-like sphere in the sky, under a gradient sky. The camera sways slowly; kicks stir the water.

// Pyramid with a unit square base on y = 0 and height h (Inigo Quilez's exact SDF).
fn back_sea_pyr(p0: vec3<f32>, h: f32) -> f32 {
    let m2 = h * h + 0.25;
    var p = vec3<f32>(abs(p0.x), p0.y, abs(p0.z));
    if (p.z > p.x) { p = vec3<f32>(p.z, p.y, p.x); }
    p = vec3<f32>(p.x - 0.5, p.y, p.z - 0.5);
    let q = vec3<f32>(p.z, h * p.y - 0.5 * p.x, h * p.x + 0.5 * p.y);
    let s = max(-q.x, 0.0);
    let t = clamp((q.y - 0.5 * p.z) / (m2 + 0.25), 0.0, 1.0);
    let a = m2 * (q.x + s) * (q.x + s) + q.y * q.y;
    let b = m2 * (q.x + 0.5 * t) * (q.x + 0.5 * t) + (q.y - m2 * t) * (q.y - m2 * t);
    let d2 = select(min(a, b), 0.0, min(q.y, -q.x * m2 - q.y * 0.5) > 0.0);
    return sqrt((d2 + q.z * q.z) / m2) * sign(max(q.z, -p.y));
}

// x = distance, y = material (0 pyramid, 1 sphere).
fn back_sea_map(p: vec3<f32>) -> vec2<f32> {
    let s = 2.2;
    let pyr = back_sea_pyr((p - vec3<f32>(1.6, -0.05, 9.0)) / s, 1.6) * s;
    let sph = length(p - vec3<f32>(-3.2, 3.6, 14.0)) - 1.6;
    return select(vec2<f32>(sph, 1.0), vec2<f32>(pyr, 0.0), pyr < sph);
}

// (t, material) of the first hit, t = -1 for a miss.
fn back_sea_march(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> vec2<f32> {
    var t = 0.05;
    let steps = select(48, 96, quality() > 0.5);
    for (var i = 0; i < steps; i++) {
        let h = back_sea_map(ro + rd * t);
        if (h.x < 0.002 * t) { return vec2<f32>(t, h.y); }
        t += h.x;
        if (t > tmax) { break; }
    }
    return vec2<f32>(-1.0, 0.0);
}

fn back_sea_normal(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.002, 0.0);
    return normalize(vec3<f32>(
        back_sea_map(p + e.xyy).x - back_sea_map(p - e.xyy).x,
        back_sea_map(p + e.yxy).x - back_sea_map(p - e.yxy).x,
        back_sea_map(p + e.yyx).x - back_sea_map(p - e.yyx).x,
    ));
}

fn back_sea_sun() -> vec3<f32> {
    return normalize(vec3<f32>(BP(1), 0.35, 1.0));
}

fn back_sea_sky(rd: vec3<f32>) -> vec3<f32> {
    let low = mix(pal(0.75), vec3<f32>(1.0), 0.4);
    var c = mix(low, pal(0.25), smoothstep(0.0, 0.5, rd.y));
    let s = max(dot(rd, back_sea_sun()), 0.0);
    c += pal(0.6) * (pow(s, 400.0) * 3.0 + pow(s, 12.0) * 0.25) * (1.0 + 0.6 * drop_env());
    return c;
}

fn back_sea_shade(p: vec3<f32>, rd: vec3<f32>, m: f32) -> vec3<f32> {
    let n = back_sea_normal(p);
    let l = back_sea_sun();
    let dif = max(dot(n, l), 0.0);
    // Sphere: marbled grey; pyramid: a flat CGI colour from the palette.
    let marble = fbm(vec2<f32>(atan2(n.z, n.x) * 2.0, n.y * 3.0) * 2.0);
    let base = select(pal(0.5) * 0.9, mix(vec3<f32>(0.25), vec3<f32>(0.9), marble), m > 0.5);
    let spec = pow(max(dot(reflect(rd, n), l), 0.0), 40.0);
    return base * (0.15 + 0.85 * dif) + back_sea_sky(reflect(rd, n)) * 0.15 + spec * 0.6;
}

fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let yaw = sin(time() * 0.03) * 0.25 * BP(2);
    let ro = vec3<f32>(0.0, BP(0), 0.0);
    let r0 = normalize(vec3<f32>(p.x, p.y - 0.15, 1.8));
    let rd = vec3<f32>(r0.x * cos(yaw) + r0.z * sin(yaw), r0.y, -r0.x * sin(yaw) + r0.z * cos(yaw));
    let hit = back_sea_march(ro, rd, 40.0);
    // Sea plane y = 0, analytic.
    let ts = select(-1.0, -ro.y / rd.y, rd.y < 0.0);
    if (hit.x > 0.0 && (ts < 0.0 || hit.x < ts)) {
        return back_sea_shade(ro + rd * hit.x, rd, hit.y);
    }
    if (ts < 0.0) {
        return back_sea_sky(rd);
    }
    let sp = ro + rd * ts;
    let w = sp.xz;
    let g = vec2<f32>(
        sin(w.x * 1.3 + time() * 0.7) * 0.6 + sin(w.x * 3.1 + w.y * 1.7 + time() * 1.3) * 0.4,
        sin(w.y * 1.7 + time() * 0.9) * 0.6 + sin(w.y * 2.9 - w.x * 1.1 + time() * 1.1) * 0.4,
    ) * BP(3) * (1.0 + kick());
    let n = normalize(vec3<f32>(g.x, 1.0, g.y));
    let rr = reflect(rd, n);
    var c = back_sea_sky(rr);
    let rh = back_sea_march(sp + n * 0.02, rr, 40.0);
    if (rh.x > 0.0) {
        c = back_sea_shade(sp + rr * rh.x, rr, rh.y);
    }
    // Grazing views mirror more; steep views show the deep water colour.
    let fres = 0.25 + 0.75 * pow(1.0 - max(-rd.y, 0.0), 4.0);
    c = mix(pal(0.0) * 0.6 + pal(0.25) * 0.2, c, fres);
    // Hard 90s shadow of the objects on the water.
    let sh = back_sea_march(sp + vec3<f32>(0.0, 0.02, 0.0), back_sea_sun(), 30.0);
    c *= select(1.0, 0.45, sh.x > 0.0);
    return mix(c, back_sea_sky(normalize(vec3<f32>(rd.x, 0.0, rd.z))), smoothstep(10.0, 40.0, ts));
}
