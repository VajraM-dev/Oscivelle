// Kaleidoscope bloom: an N-fold mandala of fine line-art petals that bloom outward forever.
// Rings of petal arcs are laid out in log-radius cells (alternate rings offset by half a wedge),
// so they grow geometrically as they flow out; the feedback zoom is locked to that flow and samples
// the previous frame through the same mirror fold, so trails stay crisp and kaleidoscopic.
// Audio: kick -> bloom shockwave + trail pump; tension -> colour drains, petals tighten, trails
// stretch; drop -> saturated full bloom. P() never multiplies time() (phases would jump).

fn kb_lum(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }

// Folded angle in [0, seg/2] (mirror symmetric wedge).
fn kb_fang(a: f32, seg: f32) -> f32 {
    let w = a - seg * floor(a / seg);
    return abs(w - 0.5 * seg);
}

fn kb_ridge(p0: vec2<f32>) -> f32 {
    var p = p0;
    var v = 0.0;
    var amp = 0.6;
    for (var i = 0; i < 3; i++) {
        let n = 1.0 - abs(noise2(p) * 2.0 - 1.0);
        v += amp * n * n;
        p = rot2(0.7) * p * 2.1 + 11.0;
        amp *= 0.5;
    }
    return v;
}

fn scene(uv: vec2<f32>) -> vec4<f32> {
    let e = energy();
    let k = kick();
    let tn = tension();
    let dr = drop_env();
    let t = time();
    let s = seed() * 50.0;
    let motion = mix(0.5, 1.0, e);

    let n = floor(P(0));
    let seg = TAU / n;
    let spacing = P(6);                 // ring spacing, log-radius units
    let flow = 0.55;                    // rings per second
    let spin = 0.045 * (select(1.0, -1.0, fract(s) > 0.5)); // rad/s, direction from seed

    let p = to_centered(uv);
    let r = length(p);
    let a = atan2(p.y, p.x) - spin * t;

    // ---- feedback through the fold, locked to the bloom flow ----
    let fa = kb_fang(a, seg);
    let qs = vec2<f32>(cos(fa + spin * t), sin(fa + spin * t)) * r;   // folded, back in screen frame
    let zoom = exp(flow * spacing * dt()) + P(1) * (1.0 + 1.5 * tn) + 0.003 * k + 0.003 * dr;
    let wuv = warp(to_uv(qs), zoom, spin * dt() + P(2) * motion, P(3), vec2<f32>(0.0));
    var col = prev(wuv);
    col = hue_rotate(col, 0.003);                       // trails drift round the hue wheel
    let l = kb_lum(col);
    col = max(vec3<f32>(l) + (col - vec3<f32>(l)) * (1.02 - 0.04 * tn), vec3<f32>(0.0));
    col *= (mix(0.8, 0.87, tn) + 0.03 * e) * (1.0 + 0.12 * k);   // kick pumps the trails

    // ---- petal rings ----
    let lr = log(max(r, 0.003));
    let u = lr / spacing - flow * t;
    let cell = floor(u);
    let hue0 = P(5) + t * 0.008;
    let tight = 1.0 - 0.3 * tn;
    var em = vec3<f32>(0.0);
    for (var j = -1; j <= 1; j++) {
        let c = cell + f32(j);
        let rc = exp((c + 0.5 + flow * t) * spacing);   // ring centre radius
        let odd = fract(c * 0.5) * 2.0;                   // 0 / 1 alternating rings
        let fo = select(fa, 0.5 * seg - fa, odd > 0.5);  // half-wedge offset on odd rings
        let q = vec2<f32>(cos(fo), sin(fo)) * r;
        let pr = rc * sin(0.5 * seg) * P(4) * tight;      // petal radius
        let d = abs(length(q - vec2<f32>(rc, 0.0)) - pr);
        let w = 0.002 + 0.0035 * rc;                      // conformal line width
        let line = w * w / (d * d + w * w);
        // Inner echo line and a jewel at the petal centre.
        let d2 = abs(length(q - vec2<f32>(rc, 0.0)) - pr * 0.55);
        let line2 = w * w / (d2 * d2 + w * w) * 0.5;
        let jd = length(q - vec2<f32>(rc + pr, 0.0)) / (w * 2.5);
        let jewel = exp(-jd * jd);
        let ch = hash21(vec2<f32>(c, s));
        let hue = hue0 + 0.31 * c + 0.1 * ch;
        let age = exp(-0.9 * rc);                         // outer rings recede
        em += (pal(hue) * line * line * 1.2 + pal(hue + 0.4) * line2 * line2 + vec3<f32>(1.0, 0.95, 0.9) * jewel * 0.8) * (0.25 + 0.75 * age);
    }
    // Filigree: sparse ridged lace inside the wedge, flowing with the rings.
    let fil = pow(clamp(kb_ridge(vec2<f32>(fa / seg * 6.0, u * 2.6) + s), 0.0, 1.0), 9.0);
    em += pal(hue0 + 0.6 + 0.2 * fract(u)) * fil * 0.35 * (0.3 + 0.7 * exp(-0.9 * r));

    // Centre: soft depth falloff so the mandala blooms out of darkness.
    let fog = smoothstep(0.015, 0.35, r);
    // Kick shockwave (seconds since kick) lights the petals it crosses.
    let kd = min(since_kick(), 0.75);
    let xs = (r - (0.04 + 3.6 * kd)) / (0.06 + 0.4 * kd);
    let shock = exp(-xs * xs) * sqrt(k);
    var gain = (0.1 + 0.08 * e) * (1.0 + 2.0 * k + 4.0 * shock) * (1.0 + 0.8 * dr);
    // Builds drain colour toward silver; the drop restores it.
    em = mix(em, vec3<f32>(kb_lum(em)) * vec3<f32>(0.85, 0.92, 1.1), 0.55 * tn * (1.0 - dr));
    col += em * fog * gain;
    // Drop: one flood of colour out of the centre, carried outward by the zoom.
    let xd = (r - 0.05) / 0.02;
    col += pal(hue0 + 0.15) * exp(-xd * xd) * 1.2 * dr;

    col *= smoothstep(0.0, 0.02, r);
    col = col / (1.0 + 0.12 * kb_lum(col));
    return vec4<f32>(col, 1.0);
}
