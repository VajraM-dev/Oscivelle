// Architecture: a raymarched grid of structures via domain repetition. seed() picks the layout
// (colonnade, monolith field, stepped ziggurats, floating cubes) and the camera (orbit, crane, side-track,
// top-down tilt). Like an equalizer, each row's heights follow bass, mid or high in turn; tops flare on
// kicks. Key light with soft shadows + AO. Hue from cell id and height.
fn emit_pillars_layout() -> i32 { return i32(floor(fract(seed() * 5.17) * 4.0)); }

// Cell c: (height or float centre, half width in cell units, present 0/1).
fn emit_pillars_cell(c: vec2<f32>) -> vec3<f32> {
    let r = hash21(c + floor(seed() * 97.0));
    let r2 = hash21(c + 11.3);
    let row = c.y - 3.0 * floor(c.y / 3.0);
    let band = clamp(select(select(high(), mid(), row < 1.5), bass(), row < 0.5), 0.0, 1.5);
    let lay = emit_pillars_layout();
    if (lay == 0) { // colonnade: aisle every 4th column
        let aisle = c.x - 4.0 * floor(c.x / 4.0);
        return vec3<f32>(1.6 + 0.3 * r + 0.9 * band, 0.17, select(1.0, 0.0, aisle < 0.5));
    } else if (lay == 1) { // monoliths at varied heights, some cells empty
        return vec3<f32>(0.4 + 2.4 * r + 1.4 * band, 0.12 + 0.18 * r2, select(1.0, 0.0, r2 < 0.3));
    } else if (lay == 2) { // stepped ziggurats tiling the floor
        let m = c - 12.0 * floor(c / 12.0) - 6.0;
        let cheb = max(abs(m.x), abs(m.y));
        return vec3<f32>(0.35 * (7.0 - cheb) + 0.5 * band, 0.4, 1.0);
    }
    // floating cubes
    return vec3<f32>(1.2 + 2.0 * r + 0.8 * band + 0.2 * sin(time() * 0.7 + r * 6.0), 0.18, select(1.0, 0.0, r2 < 0.2));
}

fn emit_pillars_box(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn emit_pillars_maxh() -> f32 {
    let lay = emit_pillars_layout();
    return select(select(select(5.1, 3.8, lay == 2), 5.0, lay == 1), 3.4, lay == 0);
}

fn emit_pillars_de(pos: vec3<f32>) -> f32 {
    let s = EP(1);
    let c = floor(pos.xz / s);
    let q = pos.xz - (c + 0.5) * s;
    let cl = emit_pillars_cell(c);
    let w = cl.y * s;
    let lay = emit_pillars_layout();
    var d = 1e3;
    if (lay == 0) {
        let dc = vec2<f32>(length(q) - w, abs(pos.y - cl.x * 0.5) - cl.x * 0.5);
        let col = length(max(dc, vec2<f32>(0.0))) + min(max(dc.x, dc.y), 0.0);
        let cap = emit_pillars_box(vec3<f32>(q.x, pos.y - cl.x, q.y), vec3<f32>(w * 1.4, 0.06, w * 1.4));
        d = min(col, cap);
    } else if (lay == 1) {
        d = emit_pillars_box(vec3<f32>(q.x, pos.y - cl.x * 0.5, q.y), vec3<f32>(w, cl.x * 0.5, 0.1 * s));
    } else if (lay == 2) {
        d = emit_pillars_box(vec3<f32>(q.x, pos.y - cl.x * 0.5, q.y), vec3<f32>(w, cl.x * 0.5, w));
    } else {
        d = emit_pillars_box(vec3<f32>(q.x, pos.y - cl.x, q.y), vec3<f32>(w));
    }
    d = select(1e3, d, cl.z > 0.5);
    // Neighbours sit at least (distance to our cell edge + their clearance) away, or below maxh.
    let ext = select(0.4, 0.3, lay == 1) * s;
    let gap = 0.5 * s - max(abs(q.x), abs(q.y)) + 0.5 * s - ext;
    let others = max(gap, pos.y - emit_pillars_maxh());
    return min(min(d, others), pos.y);
}

fn emit_pillars_shadow(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    var res = 1.0;
    var t = 0.05;
    for (var i = 0; i < 20; i++) {
        let h = emit_pillars_de(ro + rd * t);
        res = min(res, 8.0 * h / t);
        t += clamp(h, 0.04, 0.6);
        if (res < 0.0 || t > 10.0) { break; }
    }
    return clamp(res, 0.0, 1.0);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let s = EP(1);
    let tt = time() * EP(0);
    let cam = i32(floor(fract(seed() * 11.3 + 0.7) * 4.0));
    let off = floor(vec2<f32>(seed() * 37.0, seed() * 53.0)) * s; // cell corners stay clear of geometry
    let top = emit_pillars_maxh();
    var ro = vec3<f32>(0.0);
    var ta = vec3<f32>(0.0);
    if (cam == 0) { // orbit above the field
        let a = tt * 0.12 + seed() * TAU;
        ro = vec3<f32>(cos(a) * 9.0, top + 1.5 + 0.8 * sin(tt * 0.07), sin(a) * 9.0);
        ta = vec3<f32>(0.0, 1.0, 0.0);
    } else if (cam == 1) { // crane from the floor up over the field, from a cell corner
        let k = 0.5 - 0.5 * cos(tt * 0.12);
        ro = vec3<f32>(0.0, 0.6 + (top + 2.0) * k, 0.0);
        ta = vec3<f32>(1.5, 0.8 + 0.5 * top * (1.0 - k), 6.0);
    } else if (cam == 2) { // side-track along a row boundary, rows recede in depth
        ro = vec3<f32>(tt * 1.2, 1.4 + 0.6 * fract(seed() * 7.0), -3.0 * s);
        ta = ro + vec3<f32>(0.3, -0.35, 1.0);
    } else { // top-down, slowly tilting
        ro = vec3<f32>(tt * 0.5, top + 8.0, tt * 0.3);
        ta = ro + vec3<f32>(3.0 * sin(tt * 0.1), -10.0, 4.0 + 2.0 * cos(tt * 0.08));
    }
    ro += vec3<f32>(off.x, 0.0, off.y);
    ta += vec3<f32>(off.x, 0.0, off.y);
    let fw = normalize(ta - ro);
    let rt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fw));
    let up = cross(fw, rt);
    let rd = normalize(fw * 1.5 + rt * p.x + up * p.y);

    let hoff = fract(seed() * 3.7);
    var t = 0.02;
    var hit = -1.0;
    let n = select(48, 80, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let h = emit_pillars_de(ro + rd * t);
        if (h < 0.001 * t) { hit = t; break; }
        t += h;
        if (t > 40.0) { break; }
    }
    let bg = 0.04 + 0.08 * max(rd.y, 0.0);
    if (hit < 0.0) { return vec2<f32>(bg, fract(hoff + 0.55)); }

    let pos = ro + rd * hit;
    let e = vec2<f32>(0.002, -0.002);
    let nrm = normalize(e.xyy * emit_pillars_de(pos + e.xyy) + e.yyx * emit_pillars_de(pos + e.yyx)
        + e.yxy * emit_pillars_de(pos + e.yxy) + e.xxx * emit_pillars_de(pos + e.xxx));
    var ao = 1.0;
    for (var i = 1; i < 6; i++) {
        let h = 0.08 * f32(i);
        ao -= (h - emit_pillars_de(pos + nrm * h)) * 0.6 / f32(i);
    }
    ao = clamp(ao, 0.0, 1.0);
    let sa = seed() * TAU + 1.0;
    let sun = normalize(vec3<f32>(sin(sa), 0.9, cos(sa)));
    let sh = emit_pillars_shadow(pos + nrm * 0.01, sun);
    var lit = (max(dot(nrm, sun), 0.0) * sh * EP(2) + 0.2 * (0.6 + 0.4 * nrm.y)) * ao;

    let c = floor(pos.xz / s);
    let cl = emit_pillars_cell(c);
    var hue = fract(hoff + 0.6);
    if (pos.y > 0.01) {
        hue = fract(hoff + 0.13 * hash21(c) + 0.06 * cl.x);
        // Emissive tops: the upper face flares on kicks and glows with its row's band.
        let lay = emit_pillars_layout();
        let ytop = select(cl.x, cl.x + select(0.06, cl.y * s, lay == 3), lay == 0 || lay == 3);
        if (nrm.y > 0.5 && pos.y > ytop - 0.03) {
            lit += 0.25 + 1.1 * punch() + 0.5 * kick();
            hue = fract(hue + 0.5);
        }
    } else {
        // Floor: faint cell grid.
        let g = abs(fract(pos.xz / s) - 0.5);
        lit += 0.15 * smoothstep(0.47, 0.5, max(g.x, g.y));
    }
    let fog = exp(-0.045 * hit);
    let inten = mix(bg, lit * (1.0 + 0.6 * drop_env()), fog);
    return vec2<f32>(inten, hue);
}
