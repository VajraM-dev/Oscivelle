// Metaballs: 5-8 blobs (count, orbits and bounce by seed) smooth-unioned in 3D. Kicks pull every orbit in
// so the blobs melt together; bass swells their radii. Glossy shading (diffuse + specular + fresnel rim),
// orbiting or static camera, optional floor that fakes a reflection by marching the blobs once more.
var<private> emit_metaballs_c: array<vec4<f32>, 8>; // xyz centre, w radius
var<private> emit_metaballs_n: i32;

fn emit_metaballs_h(i: f32) -> f32 { return hash21(vec2<f32>(seed() * 71.3 + 9.7, i)); }

fn emit_metaballs_setup() {
    emit_metaballs_n = 5 + i32(emit_metaballs_h(1.0) * 3.99);
    let t = time() * EP(0);
    let pull = 1.0 - 0.55 * kick() - 0.25 * drop_env();
    let swell = 1.0 + 0.4 * bass();
    let bounce = emit_metaballs_h(2.0);
    for (var i = 0; i < 8; i++) {
        let f = f32(i) * 5.0 + 10.0;
        let w = 0.3 + 0.5 * emit_metaballs_h(f);
        let ph = emit_metaballs_h(f + 1.0) * TAU;
        let r = 0.9 + 0.8 * emit_metaballs_h(f + 2.0);
        let tilt = (emit_metaballs_h(f + 3.0) - 0.5) * 1.6;
        let a = t * w + ph;
        var c = vec3<f32>(cos(a) * r, sin(a * 1.3 + ph) * r * tilt, sin(a) * r) * pull;
        // some seeds bounce: |sin| hops instead of smooth orbits
        c.y += bounce * (abs(sin(t * (1.1 + w) + ph)) * 1.2 - 0.5);
        emit_metaballs_c[i] = vec4<f32>(c, (0.32 + 0.22 * emit_metaballs_h(f + 4.0)) * swell);
    }
}

// (distance, blended blob index)
fn emit_metaballs_map(p: vec3<f32>) -> vec2<f32> {
    var d = 1e3;
    var m = 0.0;
    let k = EP(1);
    for (var i = 0; i < 8; i++) {
        if (i >= emit_metaballs_n) { break; }
        let e = length(p - emit_metaballs_c[i].xyz) - emit_metaballs_c[i].w;
        let h = clamp(0.5 + 0.5 * (e - d) / k, 0.0, 1.0);
        d = mix(e, d, h) - k * h * (1.0 - h);
        m = mix(f32(i), m, h);
    }
    return vec2<f32>(d, m);
}

fn emit_metaballs_normal(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.002, -0.002);
    return normalize(e.xyy * emit_metaballs_map(p + e.xyy).x + e.yyx * emit_metaballs_map(p + e.yyx).x
        + e.yxy * emit_metaballs_map(p + e.yxy).x + e.xxx * emit_metaballs_map(p + e.xxx).x);
}

// Glossy shade of a blob hit: (intensity, hue shift)
fn emit_metaballs_shade(pos: vec3<f32>, rd: vec3<f32>, m: f32) -> vec2<f32> {
    let n = emit_metaballs_normal(pos);
    let la = emit_metaballs_h(3.0) * TAU;
    let l = normalize(vec3<f32>(cos(la), 0.9, sin(la)));
    let dif = max(dot(n, l), 0.0);
    let spec = pow(max(dot(reflect(rd, n), l), 0.0), 40.0);
    let fres = pow(1.0 - max(dot(n, -rd), 0.0), 4.0);
    let inten = 0.15 + 0.6 * dif + EP(2) * (1.2 * spec + 0.8 * fres * (1.0 + punch()));
    return vec2<f32>(inten, 0.11 * m + 0.08 * fres);
}

fn emit_metaballs_march(ro: vec3<f32>, rd: vec3<f32>, n_steps: i32) -> vec2<f32> {
    var t = 0.0;
    for (var i = 0; i < n_steps; i++) {
        let h = emit_metaballs_map(ro + rd * t);
        if (h.x < 0.001 * (1.0 + t)) { return vec2<f32>(t, h.y); }
        t += h.x;
        if (t > 20.0) { break; }
    }
    return vec2<f32>(-1.0, 0.0);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    emit_metaballs_setup();
    let a = emit_metaballs_h(4.0) * TAU + select(0.0, time() * 0.1, emit_metaballs_h(5.0) < 0.6);
    let ro = vec3<f32>(cos(a) * 5.5, 1.2 + 0.6 * emit_metaballs_h(6.0), sin(a) * 5.5);
    let fw = normalize(vec3<f32>(0.0, 0.1, 0.0) - ro);
    let rt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fw));
    let up = cross(fw, rt);
    let rd = normalize(fw * 1.8 + rt * p.x + up * p.y);
    let hue0 = emit_metaballs_h(7.0);

    let hit = emit_metaballs_march(ro, rd, select(48, 72, quality() > 0.5));
    let floor_y = select(99.0, -2.1, emit_metaballs_h(8.0) < 0.6);
    let tf = select(-1.0, (floor_y - ro.y) / rd.y, rd.y < 0.0 && floor_y < 50.0);
    if (hit.x > 0.0 && (tf < 0.0 || hit.x < tf)) {
        let s = emit_metaballs_shade(ro + rd * hit.x, rd, hit.y);
        return vec2<f32>(s.x * exp(-0.04 * hit.x) * (1.0 + 0.4 * drop_env()), fract(hue0 + s.y));
    }
    if (tf > 0.0 && tf < 25.0) {
        // floor: a dim grid-free sheen plus the blobs reflected in it
        let fp = ro + rd * tf;
        let rr = reflect(rd, vec3<f32>(0.0, 1.0, 0.0));
        let rh = emit_metaballs_march(fp + rr * 0.01, rr, 24);
        var v = 0.05 * exp(-0.15 * length(fp.xz));
        var hs = 0.5;
        if (rh.x > 0.0) {
            let s = emit_metaballs_shade(fp + rr * rh.x, rr, rh.y);
            v += 0.4 * s.x * exp(-0.3 * rh.x);
            hs = s.y;
        }
        return vec2<f32>(v * exp(-0.05 * tf), fract(hue0 + hs));
    }
    return vec2<f32>(0.02, fract(hue0 + 0.5));
}
