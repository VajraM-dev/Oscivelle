// Procedural world: seed() composes a small SDF scene (2-4 primitives from sphere / rounded box / torus /
// capsule / octahedron / cylinder, smooth-unioned, optional domain repetition, twist, floor) and picks a
// camera language (orbit, dolly, crane, parallax drift, low handheld). Lit with key light + soft shadow +
// AO + rim + depth fog. Hue = material id (+ a little normal), so the colour block paints each material.
var<private> emit_world_pos: array<vec3<f32>, 4>;
var<private> emit_world_rot: array<vec2<f32>, 4>;
var<private> emit_world_size: array<f32, 4>;
var<private> emit_world_kind: array<i32, 4>;
var<private> emit_world_count: i32;
var<private> emit_world_rep: i32;     // 0 none, 1 x infinite, 2 x finite, 3 xz finite
var<private> emit_world_cell: f32;
var<private> emit_world_twist: f32;
var<private> emit_world_bend: f32;
var<private> emit_world_floor: f32;   // floor height, or 99 for none
var<private> emit_world_k: f32;

fn emit_world_h(i: f32) -> f32 { return hash21(vec2<f32>(seed() * 113.7 + 3.1, i)); }

fn emit_world_setup() {
    emit_world_count = 2 + i32(emit_world_h(1.0) * 2.99);
    emit_world_rep = i32(emit_world_h(2.0) * 5.99) - 2; // < 1 (half the time) = none
    emit_world_cell = 2.6 + 1.2 * emit_world_h(3.0);
    emit_world_twist = select(0.0, (emit_world_h(4.0) - 0.5) * 1.2, emit_world_h(5.0) < 0.35);
    emit_world_bend = select(0.0, (emit_world_h(6.0) - 0.5) * 0.5, emit_world_h(7.0) < 0.25 && emit_world_rep < 1);
    emit_world_floor = select(99.0, -1.1 - 0.3 * emit_world_h(8.0), emit_world_h(9.0) < 0.65);
    emit_world_k = (0.15 + 0.5 * emit_world_h(10.0)) * EP(1);
    // Repeated scenes pack tighter so each cell's objects stay inside it.
    let spread = select(1.1, 0.35, emit_world_rep > 0);
    let pulse = 1.0 + 0.12 * punch() + 0.1 * drop_env();
    for (var i = 0; i < 4; i++) {
        let f = f32(i) * 7.0 + 20.0;
        emit_world_pos[i] = (vec3<f32>(emit_world_h(f), emit_world_h(f + 1.0), emit_world_h(f + 2.0)) - 0.5) * vec3<f32>(2.0, 1.2, 2.0) * spread;
        emit_world_rot[i] = vec2<f32>(emit_world_h(f + 3.0), emit_world_h(f + 4.0)) * TAU;
        emit_world_size[i] = (0.45 + 0.45 * emit_world_h(f + 5.0)) * select(1.0, 0.7, emit_world_rep > 0) * pulse;
        emit_world_kind[i] = i32(emit_world_h(f + 6.0) * 5.99);
    }
}

fn emit_world_prim(kind: i32, q: vec3<f32>, s: f32) -> f32 {
    switch kind {
        case 0: { return length(q) - s; }
        case 1: {
            let d = abs(q) - vec3<f32>(s * 0.8, s * 0.55, s * 0.7);
            return length(max(d, vec3<f32>(0.0))) + min(max(d.x, max(d.y, d.z)), 0.0) - 0.08 * s;
        }
        case 2: { return length(vec2<f32>(length(q.xz) - s, q.y)) - 0.28 * s; }
        case 3: {
            let y = q.y - clamp(q.y, -s * 0.7, s * 0.7);
            return length(vec3<f32>(q.x, y, q.z)) - 0.4 * s;
        }
        case 4: { return (abs(q.x) + abs(q.y) + abs(q.z) - s * 1.2) * 0.57735; }
        default: {
            let d = abs(vec2<f32>(length(q.xz), q.y)) - vec2<f32>(s * 0.55, s * 0.8);
            return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0)));
        }
    }
}

// (distance, material id 0..4; 4 = floor)
fn emit_world_map(p0: vec3<f32>) -> vec2<f32> {
    var p = p0;
    let tw = rot2(p.y * emit_world_twist) * p.xz;
    p = vec3<f32>(tw.x, p.y, tw.y);
    let bd = rot2(p.x * emit_world_bend) * p.xy;
    p = vec3<f32>(bd.x, bd.y, p.z);
    let c = emit_world_cell;
    if (emit_world_rep == 1) { p.x -= c * round(p.x / c); }
    if (emit_world_rep >= 2) { p.x -= c * clamp(round(p.x / c), -2.0, 2.0); }
    if (emit_world_rep == 3) { p.z -= c * clamp(round(p.z / c), -2.0, 2.0); }
    // bass ripples the surfaces
    let disp = 0.05 * bass() * sin(5.0 * p.x + time()) * sin(5.0 * p.y - time() * 0.7) * sin(5.0 * p.z);
    var d = 1e3;
    var m = 0.0;
    for (var i = 0; i < 4; i++) {
        if (i >= emit_world_count) { break; }
        var q = p - emit_world_pos[i];
        let a = rot2(emit_world_rot[i].x) * q.xz;
        q = vec3<f32>(a.x, q.y, a.y);
        let b = rot2(emit_world_rot[i].y) * q.xy;
        q = vec3<f32>(b.x, b.y, q.z);
        let e = emit_world_prim(emit_world_kind[i], q, emit_world_size[i]) + disp;
        let k = max(emit_world_k, 1e-3);
        let h = clamp(0.5 + 0.5 * (e - d) / k, 0.0, 1.0);
        d = mix(e, d, h) - k * h * (1.0 - h);
        m = mix(f32(i), m, h);
    }
    if (emit_world_floor < 50.0) {
        let g = p0.y - emit_world_floor - 0.06 * bass() * sin(p0.x * 2.0 + time()) * sin(p0.z * 2.0);
        if (g < d) { d = g; m = 4.0; }
    }
    return vec2<f32>(d * 0.8, m); // 0.8: twist/bend/displacement stretch the field
}

fn emit_world_normal(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.002, -0.002);
    return normalize(e.xyy * emit_world_map(p + e.xyy).x + e.yyx * emit_world_map(p + e.yyx).x
        + e.yxy * emit_world_map(p + e.yxy).x + e.xxx * emit_world_map(p + e.xxx).x);
}

fn emit_world_shadow(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    var s = 1.0;
    var t = 0.03;
    for (var i = 0; i < 24; i++) {
        let h = emit_world_map(ro + rd * t).x;
        s = min(s, 8.0 * h / t);
        t += clamp(h, 0.03, 0.5);
        if (s < 0.01 || t > 8.0) { break; }
    }
    return clamp(s, 0.0, 1.0);
}

fn emit_world_ao(p: vec3<f32>, n: vec3<f32>) -> f32 {
    var o = 0.0;
    for (var i = 1; i <= 5; i++) {
        let h = 0.06 * f32(i);
        o += (h - emit_world_map(p + n * h).x) / f32(i);
    }
    return clamp(1.0 - 2.0 * o, 0.0, 1.0);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    emit_world_setup();
    let t = time() * EP(0);
    let cam = i32(emit_world_h(11.0) * 4.99);
    let a0 = emit_world_h(12.0) * TAU;
    let dist = select(5.0, select(6.5, 9.0, emit_world_rep == 3), emit_world_rep > 0);
    var ro = vec3<f32>(cos(a0) * dist, 1.0, sin(a0) * dist);
    var ta = vec3<f32>(0.0);
    switch cam {
        case 0: { // slow orbit
            let a = a0 + t * 0.12;
            ro = vec3<f32>(cos(a) * dist, 1.0 + 0.3 * sin(t * 0.07), sin(a) * dist);
        }
        case 1: { ro *= 0.8 + 0.3 * sin(t * 0.11); } // dolly in/out
        case 2: { ro.y = 0.2 + 2.6 * (0.5 + 0.5 * sin(t * 0.09)); } // crane
        case 3: { // static, parallax drift sideways
            let side = normalize(vec3<f32>(-ro.z, 0.0, ro.x));
            ro += side * 1.2 * sin(t * 0.1) + vec3<f32>(0.0, 0.25 * sin(t * 0.13), 0.0);
        }
        default: { // low handheld
            ro.y = select(-0.3, emit_world_floor + 0.5, emit_world_floor < 50.0);
            ro *= 0.85;
            ta = vec3<f32>(0.08 * sin(t * 1.3) + 0.05 * sin(t * 2.9), 0.3 + 0.06 * sin(t * 1.7), 0.07 * sin(t * 2.3));
        }
    }
    // Repeated rows fill the ground plane: keep the camera above them (the tallest part sits below ~1.2).
    if (emit_world_rep > 0) { ro.y = max(ro.y, 1.5); }
    let fw = normalize(ta - ro);
    let rt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fw));
    let up = cross(fw, rt);
    let rd = normalize(fw * 1.7 + rt * p.x + up * p.y);

    var d = 0.0;
    var hit = -1.0;
    var mat = 0.0;
    var near = 1e3;
    let n_steps = select(48, 80, quality() > 0.5);
    for (var i = 0; i < n_steps; i++) {
        let h = emit_world_map(ro + rd * d);
        near = min(near, h.x);
        if (h.x < 0.001 * (1.0 + d)) { hit = d; mat = h.y; break; }
        d += h.x;
        if (d > 30.0) { break; }
    }
    let hue0 = emit_world_h(13.0);
    let glow = exp(-6.0 * max(near, 0.0)) * 0.25 * (1.0 + punch());
    if (hit < 0.0) { return vec2<f32>(0.03 + 0.04 * max(rd.y, 0.0) + glow, fract(hue0 + 0.5)); }

    let pos = ro + rd * hit;
    let n = emit_world_normal(pos);
    let la = emit_world_h(14.0) * TAU;
    let l = normalize(vec3<f32>(cos(la), 0.5 + 0.7 * emit_world_h(15.0), sin(la)));
    let flick = 1.0 - 0.5 * snare() * step(0.5, hash21(vec2<f32>(floor(time() * 24.0), seed())));
    let li = EP(2) * flick;
    let dif = max(dot(n, l), 0.0) * emit_world_shadow(pos + n * 0.01, l);
    let ao = emit_world_ao(pos, n);
    let spec = pow(max(dot(reflect(rd, n), l), 0.0), 24.0);
    let rim = pow(1.0 - max(dot(n, -rd), 0.0), 3.0);
    // one material (by seed) glows on the kick
    let emis = select(0.0, 0.8 * kick(), abs(mat - floor(emit_world_h(16.0) * f32(emit_world_count))) < 0.5);
    var lit = li * (0.95 * dif + 0.4 * spec) + 0.12 * ao + rim * 0.35 * (1.0 + punch()) + emis;
    lit *= exp(-0.07 * hit) * (1.0 + 0.4 * drop_env());
    return vec2<f32>(lit + glow * 0.3, fract(hue0 + 0.19 * mat + 0.05 * n.y));
}
