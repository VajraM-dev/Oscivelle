// Neon geometry trails: AVS-style neon polygons drawn as crisp 1-2 px lines. The tunnel of
// echoes is drawn analytically (each echo keeps the angle and hue it was born with and flies
// outward, thickening with depth), so the lines stay sharp at any spacing; a short feedback
// zoom matched to the tunnel speed adds the glowing smear between them. A counter-rotating
// inner shape and a star polygon add layers; a shockwave outline expands on each kick.
// Audio: kick -> shape punch + shockwave; drop -> size burst + bright echoes;
// tension -> lines thin, the spin winds up and the feedback pushes in.

// Distance from p to the regular n-gon outline (radius rad, rotation ang); step = vertex skip
// (1 = polygon, 2 = star polygon).
fn ng_dist(p: vec2<f32>, n: f32, rad: f32, ang: f32, step: f32) -> f32 {
    var d = 1e9;
    let st = TAU / n;
    for (var i = 0; i < 8; i++) {
        if (f32(i) >= n) { break; }
        let a0 = ang + f32(i) * st;
        let a1 = a0 + st * step;
        let a = vec2<f32>(cos(a0), sin(a0)) * rad;
        let b = vec2<f32>(cos(a1), sin(a1)) * rad;
        let pa = p - a;
        let ba = b - a;
        let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
        d = min(d, length(pa - ba * h));
    }
    return d;
}

// Crisp neon profile: hard anti-aliased core of half-width w plus a tight glow.
fn ng_neon(d: f32, w: f32, px: f32) -> f32 {
    let core = 1.0 - smoothstep(w, w + 1.5 * px, d);
    let glow = exp(-max(d - w, 0.0) / (w * 2.0 + 2.5 * px));
    return core + 0.3 * glow;
}

const NG_ECHOES: i32 = 9;
const NG_SPEED: f32 = 0.22;  // echo flight phase per second (each echo lives 1/NG_SPEED s)
const NG_GROW: f32 = 2.4;    // log-radius growth over an echo's life

fn scene(uv: vec2<f32>) -> vec4<f32> {
    let e = energy();
    let k = kick();
    let tn = tension();
    let dr = drop_env();
    let t = time();
    let dir = select(-1.0, 1.0, seed() > 0.5);
    let p = to_centered(uv);
    let px = 2.0 / resolution().y;

    // Short feedback, zoom matched to the echo flight so the smear rides along with them.
    let zoom = exp(NG_GROW * NG_SPEED * dt()) + (P(3) - 1.0) * (0.5 + 0.5 * e) + 0.006 * tn + 0.01 * k + 0.02 * dr;
    let spin = P(1) * (1.0 + 1.5 * tn);
    var col = prev(warp(uv, zoom, spin, P(2), vec2<f32>(0.0)));
    col = max(hue_rotate(col, P(4)), vec3<f32>(0.0));
    col *= P(5);

    let n = floor(P(0));
    let rad = P(6) * (1.0 + 0.12 * k + 0.3 * dr);
    let w = px * mix(1.0, 0.6, tn);
    let rate = 0.35 * (1.0 + 0.8 * tn);
    var em = vec3<f32>(0.0);

    // Echo tunnel: echo i sits at phase s in 0..1 of its flight outward.
    for (var i = 0; i < NG_ECHOES; i++) {
        let s = fract(f32(i) / f32(NG_ECHOES) + t * NG_SPEED);
        let born = t - s / NG_SPEED;
        let r_i = rad * exp((s - 0.12) * NG_GROW);
        let fade = smoothstep(0.0, 0.08, s) * pow(1.0 - s, 1.3);
        let a_i = dir * born * 0.35 + s * P(1) * 40.0;
        let hue_i = P(7) + born * 0.02 + s * P(4) * 60.0 + f32(i % 2) * 0.3;
        let w_i = w * (1.0 + 2.5 * s);
        em += pal(hue_i + 0.25) * ng_neon(ng_dist(p, n, r_i, a_i, 1.0), w_i, px) * fade * (0.55 + 0.25 * e + 1.2 * k + 0.6 * dr);
    }

    // Live shapes: outer (kick-punched), counter-rotating inner, star polygon for n >= 5.
    let hue = P(7) + t * 0.02;
    let a_out = dir * t * 0.35;
    let r0 = rad * exp(-0.12 * NG_GROW);
    let wk = w * (1.0 + 1.2 * k);
    em += pal(hue + 0.25) * ng_neon(ng_dist(p, n, r0, a_out, 1.0), wk, px) * (0.3 + 1.5 * k);
    em += pal(hue + 0.6) * ng_neon(ng_dist(p, n, r0 * 0.5, -dir * t * rate * 1.7 + 0.3, 1.0), w, px) * (0.5 + 0.3 * e + 0.6 * k);
    if (n >= 5.0) {
        em += pal(hue + 0.45) * ng_neon(ng_dist(p, n, r0, a_out, 2.0), w * 0.8, px) * (0.3 + 0.2 * e + 0.6 * dr);
    }
    // Shockwave outline that grows across the beat while the kick envelope decays.
    let sr = r0 * 0.5 + beat_phase() * 1.8;
    em += pal(hue + 0.9) * ng_neon(ng_dist(p, n, sr, a_out, 1.0), w * 1.3, px) * (0.9 * k + 1.0 * dr);

    col += em;
    col = col / (1.0 + 0.08 * col);
    return vec4<f32>(col, 1.0);
}
