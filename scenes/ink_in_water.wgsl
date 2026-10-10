// Ink in water: pigment drops bloom into dark water and are stretched into fine tendrils by a
// turbulent divergence-free (curl-noise) current. Ink is mixed in, not added, so overlaps blend
// like paint and never wash to white; a light unsharp step each frame keeps the tendrils crisp
// instead of blurring away, and old ink sinks toward the dark water tint (depth by fading).
// Audio: kick -> a fresh drop blooms (position per beat) + the current surges;
// tension -> the current stalls and the water draws inward into a dense knot;
// drop -> the knot bursts outward with a ring of every accent colour.

// Integer lattice hash. The prelude's float hash21 can return a different value for the same
// lattice corner from neighbouring cells (FMA rounding amplified by fract), which puts a
// velocity spike on every lattice line; integer hashing is exact.
fn ink_hash(i: vec2<f32>) -> f32 {
    var h = bitcast<u32>(vec2<i32>(i).x) * 1597334677u ^ bitcast<u32>(vec2<i32>(i).y) * 3812015801u;
    h = h ^ (h >> 16u);
    h = h * 2246822519u;
    h = h ^ (h >> 13u);
    return f32(h & 0xffffffu) / 16777216.0;
}

// Quintic value noise (C2, so the current has no shear creases).
fn ink_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let w = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = ink_hash(i);
    let b = ink_hash(i + vec2<f32>(1.0, 0.0));
    let c = ink_hash(i + vec2<f32>(0.0, 1.0));
    let d = ink_hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

fn ink_psi(p0: vec2<f32>, sc: f32, t: f32, s: f32) -> f32 {
    var p = p0 * sc + vec2<f32>(t * 0.035 + s, -t * 0.025);
    var v = 0.0;
    var a = 0.5;
    // ponytail: 3 soft octaves; finer/stronger octaves fold the advection map into creases.
    for (var i = 0; i < 3; i++) {
        v += a * ink_noise(p);
        p = rot2(0.6 + 0.05 * t) * p * 2.0 + vec2<f32>(t * 0.06, 11.0);
        a *= 0.4;
    }
    return v;
}

// Nozzle j position: a slow wander, which through a build tightens into a fast small orbit.
fn ink_nozzle(t: f32, fj: f32, coil: f32, s: f32) -> vec2<f32> {
    let wander = vec2<f32>(sin(t * (0.17 + 0.05 * fj) + s + fj * 2.1) * aspect() * 0.7,
                           sin(t * (0.23 - 0.04 * fj) + s * 2.0 + fj * 4.0) * 0.65);
    let ph = t * (1.0 + 7.0 * coil) + fj * PI;
    return mix(wander, vec2<f32>(cos(ph), sin(ph)) * 0.22, coil);
}

// Ridged noise: thin bright creases, for combing filaments into injected ink.
fn ink_ridge(p: vec2<f32>) -> f32 {
    let n = 1.0 - abs(2.0 * ink_noise(p) - 1.0);
    let m = 1.0 - abs(2.0 * ink_noise(p * 2.3 + 5.0) - 1.0);
    return n * n * n * (0.5 + 0.5 * m);
}

fn ink_luma(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }

fn scene(uv: vec2<f32>) -> vec4<f32> {
    let e = energy();
    let k = kick();
    let tn = tension();
    let dr = drop_env();
    let db = dr * dr * dr; // fast part of the drop
    let t = time();
    let s = seed() * 37.0;
    let p = to_centered(uv);

    // Curl of a noise stream function -> swirling, incompressible current.
    let sc = P(0);
    let h = 0.015;
    let dx = ink_psi(p + vec2<f32>(h, 0.0), sc, t, s) - ink_psi(p - vec2<f32>(h, 0.0), sc, t, s);
    let dy = ink_psi(p + vec2<f32>(0.0, h), sc, t, s) - ink_psi(p - vec2<f32>(0.0, h), sc, t, s);
    let v = vec2<f32>(dy, -dx) * (1.2 / h); // ~unit-speed current
    let spd = P(1) * (0.5 + 0.5 * e) * (1.0 + 1.2 * k) * (1.0 - 0.7 * tn) + 0.006 * db;
    let src = p - v * spd;
    // Gentle outward spread; the drop blasts it out. Never zoom in or rotate: that would drag
    // the frame edge (a straight line) into the water.
    let zoom = P(2) + 0.006 * k * k + 0.008 * db;
    let wuv = warp(to_uv(src), zoom, 0.0, 0.0, vec2<f32>(0.0));

    // Advected ink with an unsharp step so tendrils stay crisp; clamped to the neighbourhood
    // range so it can never ring or build up white crusts.
    let px = 1.0 / resolution();
    let c0 = prev(wuv);
    let n1 = prev(wuv + vec2<f32>(px.x, 0.0));
    let n2 = prev(wuv - vec2<f32>(px.x, 0.0));
    let n3 = prev(wuv + vec2<f32>(0.0, px.y));
    let n4 = prev(wuv - vec2<f32>(0.0, px.y));
    let lo = min(min(min(n1, n2), min(n3, n4)), c0);
    let hi = max(max(max(n1, n2), max(n3, n4)), c0);
    var col = clamp(c0 + P(7) * (c0 - 0.25 * (n1 + n2 + n3 + n4)), lo, hi);

    // Fade toward dark water; under tension ink lingers.
    let water = pal(0.0) * 0.06 + vec3<f32>(0.004, 0.006, 0.01);
    let persist = mix(P(4), 0.994, tn);
    // Multiplicative fade plus a small subtractive one: dim haze clears to clean water while
    // dense ink lingers (contrast stays high).
    col = max(mix(water, col, persist) - vec3<f32>(0.0015), water);
    
    // Kick drop: one position per beat of the bar, reshuffled every 16th of a phrase.
    let bid = floor(bar_phase() * 4.0 + 0.15) + 5.0 * floor(phrase_phase() * 16.0) + s;
    let hp = vec2<f32>(hash21(vec2<f32>(bid, 1.7)), hash21(vec2<f32>(bid, 8.3)));
    let dc = (hp - 0.5) * vec2<f32>(1.5 * aspect(), 1.4) * (1.0 - 0.6 * tn);
    let dv = p - dc;
    let rough = 1.0 + 0.6 * (ink_noise(dv * 7.0 + bid) - 0.5);
    let rad = P(5) * (0.7 + 0.5 * k);
    let dd = length(dv) * rough / rad;
    let ci = floor(hash21(vec2<f32>(bid, 4.1)) * 3.0);
    // Accent colours only (palette slots 1..3; slot 0 is the water).
    let ink = pal(0.25 + 0.25 * ci + P(3) + 0.06 * (ink_noise(dv * 6.0) - 0.5)) * (0.9 + 0.6 * k);
    // Crisp-edged core with a denser vortex-ring rim, combed by radial ridged-noise filaments
    // that the current then stretches into hair-thin tendrils.
    let dir = dv / max(length(dv), 1e-4);
    let comb = ink_ridge(dir * 3.5 + vec2<f32>(bid * 1.3, dd * 1.2));
    let rq = (dd - 0.8) / 0.22;
    let rim = exp(-rq * rq);
    let core = 1.0 - smoothstep(0.7, 1.0, dd);
    col = mix(col, ink, clamp(k * 0.5 * max(rim, 0.5 * core) * (0.35 + 0.9 * comb), 0.0, 1.0));

    // Two nozzles keep thin streams going so breakdowns still have tendrils. Through a build
    // their orbits tighten and quicken into a coiling knot at the centre (anticipation).
    let coil = tn * (1.0 - dr);
    for (var j = 0; j < 2; j++) {
        let fj = f32(j);
        // Segment from last step's nozzle position to this one: a continuous stream, no beads.
        let a0 = ink_nozzle(t - dt(), fj, coil, s);
        let a1 = ink_nozzle(t, fj, coil, s);
        let pa = p - a0;
        let ba = a1 - a0;
        let hh = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-8), 0.0, 1.0);
        let rd = length(pa - ba * hh) / (0.045 - 0.022 * coil);
        let nc = pal(0.3 + 0.4 * fj + 0.12 * sin(t * 0.07 + fj)) * 1.2;
        col = mix(col, nc, clamp((P(6) * (2.0 - e) + 0.35 * coil) * exp(-rd * rd), 0.0, 1.0));
    }

    // Drop: a combed ring of all three accents racing out from the centre.
    let r = length(p);
    let grow = 1.0 - db;
    let rr = 0.08 + 1.4 * sqrt(grow);
    let rx = (r - rr) / (0.04 + 0.1 * grow);
    let pdir = p / max(r, 1e-4);
    let burst = pal(0.25 + 0.5 * fract(0.5 + 0.5 * sin(atan2(p.y, p.x) * 3.0 + s) + 0.3 * ink_noise(p * 3.0))) * 1.4;
    let bcomb = ink_ridge(pdir * 5.0 + vec2<f32>(s, 0.0));
    col = mix(col, burst, clamp(db * 0.4 * exp(-rx * rx) * (0.1 + 1.0 * bcomb), 0.0, 1.0));

    return vec4<f32>(col, 1.0);
}
