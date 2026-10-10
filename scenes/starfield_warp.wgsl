// Starfield warp: stars live on a log-polar grid (angle x log-radius), so flying forward is a
// constant scroll outward and each star is drawn analytically as a crisp radial streak that
// widens and brightens as it nears the camera. Several grid layers give depth; a light
// feedback zoom adds glowing after-trails, and a faint nebula is dragged into tunnel walls.
// Audio: kick -> streaks stretch and flare; drop -> hyperspace jump (long white streaks,
// core flash); tension -> streaks lengthen, colour drains toward the core glow.

// Integer hash (exact at any coordinate; a float hash can round differently on either side
// of a cell edge and leave visible seams).
fn sw_hash(x: i32, y: i32) -> f32 {
    var h = bitcast<u32>(x) * 1597334677u ^ bitcast<u32>(y) * 3812015801u;
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = h ^ (h >> 13u);
    return f32(h & 0xffffffu) / 16777216.0;
}

// Value noise periodic in x with period px (integer), for seamless log-polar patterns.
fn sw_pnoise(p: vec2<f32>, px: f32) -> f32 {
    let fl = floor(p);
    let f = p - fl;
    let w = f * f * (3.0 - 2.0 * f);
    let n = i32(px);
    let x0 = ((i32(fl.x) % n) + n) % n;
    let x1 = (x0 + 1) % n;
    let y0 = i32(fl.y);
    let a = sw_hash(x0, y0);
    let b = sw_hash(x1, y0);
    let c = sw_hash(x0, y0 + 1);
    let d = sw_hash(x1, y0 + 1);
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

fn sw_pfbm(p0: vec2<f32>, px0: f32) -> f32 {
    var p = p0;
    var px = px0;
    var v = 0.0;
    var amp = 0.5;
    for (var i = 0; i < 4; i++) {
        v += amp * sw_pnoise(p, px);
        p = p * 2.0 + vec2<f32>(0.0, 7.3);
        px *= 2.0;
        amp *= 0.5;
    }
    return v;
}

// One star layer. n = angular cells (integer), speed = log-radius scroll per second.
fn sw_layer(p: vec2<f32>, n: f32, speed: f32, len: f32, dens: f32, li: f32, thick: f32) -> vec3<f32> {
    let r = length(p);
    let a = atan2(p.y, p.x) / TAU + 0.5;
    let m = n / TAU * 0.16; // cells are ~6x taller (radially) than wide
    let qx = a * n;
    let col_id = floor(qx) % n; // wrap the angular seam
    let lane = sw_hash(i32(col_id), i32(li) * 131 + 17); // per-column radial offset breaks up rings
    let q = vec2<f32>(qx, log(r) * m - time() * speed * m + lane * 3.0);
    let cell = vec2<f32>(col_id, floor(q.y));
    let f = fract(q) - 0.5;
    let h = sw_hash(i32(cell.x) + i32(li) * 7919, i32(cell.y) + i32(seed() * 997.0));
    if (h > dens) { return vec3<f32>(0.0); }
    let h2 = fract(h * 71.3);
    let ox = (fract(h * 53.7) - 0.5) * 0.6;
    let head = 0.1 + 0.35 * fract(h * 91.3);
    let t0 = max(head - len * (0.5 + fract(h * 37.9)), -0.5);
    // Streak runs from head back toward the centre; tail fades to nothing so no hard cut.
    let y = clamp(f.y, t0, head);
    let along = (f.y - y) / 0.008;
    let near = smoothstep(0.04, 1.0, r); // far stars (near centre) are dim and thin
    let wdt = mix(0.0007, 0.0026, near) * (0.6 + 0.8 * h2) * thick;
    let dx = (f.x - ox) * r * TAU / n / wdt;
    let core = exp(-dx * dx - along * along);
    let tail = pow(clamp((f.y - t0) / max(head - t0, 1e-3), 0.0, 1.0), 1.5);
    let tint = pal(0.25 + 0.75 * h2);
    let b = core * tail * near * (0.4 + 1.2 * h2 * h2);
    return mix(tint, vec3<f32>(1.0), 0.5 * exp(-dx * dx * 4.0 - along * along)) * b;
}

fn scene(uv: vec2<f32>) -> vec4<f32> {
    let e = energy();
    let k = kick();
    let tn = tension();
    let dr = drop_env();
    let t = time();
    // Roll is applied analytically and matched in the feedback, so trails stay registered
    // (a feedback-only roll smears every streak sideways into a wedge).
    let w_roll = (seed() - 0.5) * 0.12;
    let p = rot2(t * w_roll) * to_centered(uv);
    let r = length(p);

    // Feedback: gentle zoom smears the streaks into after-glow.
    // Zoom stays below the stars' own outward speed so after-trails lag behind the heads.
    let zoom = 1.0 + P(0) * (0.6 + 0.4 * e + 0.8 * dr);
    var col = prev(warp(uv, zoom, w_roll * dt(), 0.0, vec2<f32>(0.0)));
    col *= P(2) * (1.0 - 0.06 * dr);

    let len = clamp(P(1) * (0.35 + 0.5 * k + 0.4 * tn) + 0.6 * dr, 0.1, 1.0);
    let ps = p * (1.0 - 0.07 * k); // kick punches the field outward
    let thick = 1.0 + 1.0 * k + 0.5 * dr;
    var st = vec3<f32>(0.0);
    st += sw_layer(ps, 23.0, 0.55, len, P(3) * 0.8, 1.0, thick) * 1.4;
    st += sw_layer(ps, 47.0, 0.40, len, P(3), 2.0, thick);
    st += sw_layer(ps, 97.0, 0.30, len, P(3), 3.0, thick) * 0.7;
    let gain = 0.45 + 0.25 * e + 1.8 * k + 1.5 * dr;
    col += st * gain;

    // Nebula tunnel walls in log-polar space, flowing outward slower than the stars.
    let ang = atan2(p.y, p.x) / TAU + 0.5;
    let nq = vec2<f32>(ang * 14.0, log(r) * 2.2 - t * 0.12 + seed() * 50.0);
    let nb = sw_pfbm(nq, 14.0);
    let wisp = 1.0 - abs(sw_pfbm(nq * vec2<f32>(2.0, 2.0) + vec2<f32>(0.0, nb * 3.0), 28.0) * 2.0 - 1.0);
    let neb = smoothstep(0.35, 0.75, nb) * pow(wisp, 8.0) * 3.0 * smoothstep(0.1, 0.8, r);
    col += pal(P(5) + nb * 0.6) * neb * P(4) * (1.0 - 0.6 * tn + 1.5 * dr);

    // Vanishing-point glow builds with tension; the drop fires a shock ring outward.
    let core = exp(-r * r / 0.004) * (0.04 * tn + 0.3 * pow(dr, 6.0));
    let since = 0.28 * since_drop(); // ring speed tuned as 0.28 units/s
    let ring = exp(-pow((r - since * 6.0) / 0.015, 2.0)) * pow(dr, 8.0) * 0.5;
    col += mix(pal(P(5) + 0.5), vec3<f32>(1.0), 0.5) * (core + ring);

    col = col / (1.0 + 0.04 * col);
    return vec4<f32>(col, 1.0);
}
