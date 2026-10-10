// Fractal zoom: a breathing dive onto a Misiurewicz point of the Mandelbrot set (picked by
// seed), so filaments fill the frame at every depth. Thin distance-estimate glow lines over
// dim smooth-iteration bands; stripe-average adds fine hair detail. Behind the fresh fractal
// is a feedback layer: last frame pushed outward, rolled, hue-drifted and faded, max-blended
// so echoes ripple out of the filaments into the dark gaps but can never flood the frame.
// Audio: kick -> camera zoom pulse + echo push (rings burst out of the set) + flash;
// tension -> camera dives faster, colour drains, echoes tighten; drop -> wide echo burst.

// xy = Misiurewicz-ish target, z = shallowest depth at which it fills the frame.
fn fz_target(i: i32) -> vec3<f32> {
    switch i {
        case 0: { return vec3<f32>(-0.77568377, 0.13646737, 5.2); } // seahorse double spiral
        case 1: { return vec3<f32>(0.00164372, -0.82246763, 4.0); } // swirl lace
        case 2: { return vec3<f32>(-0.1010964, 0.9562865, 4.0); }   // dendrite tip
        default: { return vec3<f32>(-0.67234469, -0.45845511, 4.5); } // pinwheel lace
    }
}

fn luma(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }

fn scene(uv: vec2<f32>) -> vec4<f32> {
    let k = kick();
    let tn = tension();
    let dr = drop_env();
    let t = time();
    let sd = seed();

    // Camera: depth breathes (period ~1.7 min) between the target's floor and ~8.6 e-folds
    // (f32 limit); tension pushes in, each kick nudges in.
    let tg = fz_target(i32(sd * 4.0) % 4);
    let tgt = tg.xy;
    let breath = 0.5 - 0.5 * cos(t * 0.06 + sd * TAU);
    let depth = min(mix(tg.z, 8.6, breath * P(0)) + 1.0 * tn + 0.06 * k, 8.8);
    let scale = 1.25 * exp(-depth);
    let ang = P(1) * sin(t * 0.05 + sd * 9.0) + t * 0.02;
    let p = to_centered(uv);
    let c = rot2(ang) * p * scale + tgt;
    let px = scale * 2.0 / resolution().y; // c-plane size of one pixel

    let iters = i32(70.0 + 30.0 * depth) + select(0, 60, quality() > 0.5);
    var z = vec2<f32>(0.0);
    var dz = vec2<f32>(0.0);
    var m = 0.0;
    var n = 0.0;
    var stripe = 0.0;
    var esc = false;
    for (var i = 0; i < iters; i++) {
        dz = 2.0 * vec2<f32>(z.x * dz.x - z.y * dz.y, z.x * dz.y + z.y * dz.x) + vec2<f32>(1.0, 0.0);
        z = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
        m = dot(z, z);
        // sin(3*arg z) without atan2: Im((z/|z|)^3)
        let w = z * inverseSqrt(max(m, 1e-20));
        stripe += 0.5 + 0.5 * (3.0 * w.x * w.x * w.y - w.y * w.y * w.y);
        if (m > 1e4) {
            esc = true;
            n = f32(i);
            break;
        }
    }

    // Fresh fractal: interior near-black, exterior dim bands + bright boundary filaments.
    var fr = pal(0.0) * 0.02;
    if (esc) {
        let sn = n + 1.0 - log2(log2(m) * 0.5);
        let stp = stripe / max(n + 1.0, 1.0);
        let de = 0.5 * sqrt(m) * log(m) * 0.5 / max(length(dz), 1e-20); // exterior distance
        let dpx = de / px;
        let glow = pow(exp(-0.6 * dpx), P(7));           // thin core line
        let halo = 1.0 / (1.0 + 0.12 * dpx * dpx);         // soft near-field
        // Exterior bands follow iteration count; the lace colour follows log-distance to the
        // dive target, so colour rings flow outward as the camera dives (reads as depth).
        let ring = 0.22 * (log(length(p) + 0.04) - depth);
        let hue = P(3) + t * 0.015 + sn * P(2);
        let lhue = P(3) + t * 0.015 + ring + 0.15 * sn * P(2);
        let band = 0.5 + 0.5 * cos(sn * P(2) * TAU * 2.0);
        let colr = pal(hue);
        let acc = pal(lhue);
        let hair = smoothstep(0.35, 0.9, stp);
        let tex = mix(1.0, 0.25 + 1.1 * hair, P(4));      // stripe texture inside dense lace
        fr = colr * (0.025 + 0.09 * band * band) + acc * halo * 0.10;
        fr += acc * glow * 0.8 * tex;
    }
    // Colour drains through the build; kick flashes the filaments.
    fr = mix(fr, vec3<f32>(luma(fr)), 0.3 * tn * (1.0 - dr));
    fr *= 0.8 + 0.6 * k + 0.5 * dr;

    // Echo layer: between kicks short and tight (fine detail stays crisp); each kick
    // throws a burst of echoes outward from the lace, the drop a long wide one.
    let db = dr * dr * dr; // fast-decaying drop burst
    let zoomf = P(6) + 0.025 * k * k + 0.04 * db;
    var echo = prev(warp(uv, zoomf, 0.002 * k + 0.006 * db, 0.0, vec2<f32>(0.0)));
    echo = hue_rotate(echo, 0.012);
    let keep = mix(P(5), 0.93, max(k * k, 0.85 * db)) * (1.0 - 0.05 * tn);
    echo *= keep;

    let col = max(fr, echo);
    return vec4<f32>(col, 1.0);
}
