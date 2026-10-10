// Liquid tunnel: flight down a molten tube. Each frame paints thin liquid filaments on the
// tunnel walls (perspective-mapped, domain-warped ridged noise); the feedback zoom carries the
// previous frame outward at about the walls' own speed, so filaments stay crisp and leave short
// layered trails that drift through the hue wheel as they age.
// Audio: kick -> zoom punch + filament flare; tension -> colour drains, walls tighten, push deepens;
// drop -> shock ring + full saturation. P() never multiplies time() (phases would jump).

fn lt_lum(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }

// Ridged fbm: thin bright creases where the noise crosses its midpoint.
fn lt_ridge(p0: vec2<f32>) -> f32 {
    var p = p0;
    var v = 0.0;
    var a = 0.6;
    for (var i = 0; i < 4; i++) {
        let n = 1.0 - abs(noise2(p) * 2.0 - 1.0);
        v += a * n * n;
        p = rot2(0.7) * p * 2.1 + 11.0;
        a *= 0.5;
    }
    return v;
}

fn scene(uv: vec2<f32>) -> vec4<f32> {
    let e = energy();
    let k = kick();
    let tn = tension();
    let dr = drop_env();
    let motion = mix(0.45, 1.0, e);
    let t = time();
    let s = seed() * 100.0;

    // Vanishing point wanders; it settles to the centre as a build tightens.
    let wander = P(7) * (1.0 - 0.8 * tn);
    let ctr = vec2<f32>(sin(t * 0.13 + s) * aspect(), cos(t * 0.1 + s * 1.7)) * wander * 0.4;
    let q = to_centered(uv) - ctr;
    let r = length(q);
    let a = atan2(q.y, q.x);

    // ---- feedback, locked to the walls' own motion so filaments stay sharp; the params only add
    // the drift that peels trails off them ----
    let lobes = floor(P(4));
    let flow = 0.09 * lobes;                    // wall scroll, log-polar units / s
    let twist = 0.35;
    let spin = 0.03;
    let wall_zoom = exp(flow * TAU / lobes * dt());
    let wall_rot = (twist * flow + spin) * TAU / lobes * dt();
    let zoom = wall_zoom + P(0) * (1.0 + 1.5 * tn) + 0.004 * k + 0.003 * dr;
    var wuv = warp(uv, zoom, wall_rot + P(1) * motion, P(2) * (0.6 + 0.4 * e), ctr);
    let np = q * 1.7 + vec2<f32>(t * 0.11, -t * 0.09);
    let disp = vec2<f32>(noise2(np), noise2(np + vec2<f32>(19.1, 7.3))) - 0.5;
    wuv += disp * P(3) * motion * (0.1 + 0.4 * r);

    var col = prev(wuv);
    col = hue_rotate(col, 0.0025);  // trails drift through the hue wheel as they age
    // Keep trails saturated (bilinear resampling greys them), drain a little during builds.
    let l = lt_lum(col);
    col = max(vec3<f32>(l) + (col - vec3<f32>(l)) * (1.02 - 0.03 * tn), vec3<f32>(0.0));
    col *= (mix(0.78, 0.85, tn) + 0.03 * e) * (1.0 + 0.12 * k);  // kick pumps the trails

    // ---- tunnel walls (log-polar: conformal, so detail stays sharp and matches the zoom) ----
    let lr = log(max(r, 0.002)) * lobes / TAU;  // one sector wide == one unit deep
    let v = lr - t * flow;              // scrolls outward, toward the viewer
    let tw = a / TAU * lobes + twist * lr + spin * t;  // spiral twist with depth
    let ua = abs(fract(tw) - 0.5) * 2.0;        // mirrored sectors: seamless round the tube
    let st = vec2<f32>(ua, v) * (28.0 / lobes) + s;   // same detail density for any lobe count
    let wv = vec2<f32>(fbm(st * 0.3), fbm(st * 0.3 + vec2<f32>(5.2, 1.3)));
    let thin = mix(8.0, 10.0, tn);               // builds tighten the filaments
    let fil = pow(clamp(lt_ridge(st + wv * 1.6), 0.0, 1.0), thin);
    let fine = pow(clamp(lt_ridge(st * 1.6 + wv * 2.2 + 3.0), 0.0, 1.0), 12.0);
    // Ribs: liquid hoops at fixed depth spacing, wobbling and broken into segments.
    let rv = v * 1.5 + 0.3 * wv.y;
    let rx = (fract(rv) - 0.5) / (P(6) + 0.02 * k);
    let rib = exp(-rx * rx) * smoothstep(0.35, 0.65, wv.x);

    // Depth fog: the far end is dark; walls light up as they rush past.
    let near = smoothstep(0.02, 0.6, r);
    let fog = near * near;
    let hue = P(5) + 0.5 * wv.x + 0.17 * floor(rv) + t * 0.01;
    // Slow liquid patches: whole regions of wall go dark, so the frame keeps negative space.
    let pm = smoothstep(0.38, 0.62, fbm(st * 0.18 + vec2<f32>(t * 0.04, 0.0)));
    var wall = (pal(hue) * fil + pal(hue + 0.33) * fine * 0.5) * (0.3 + 0.7 * pm) + pal(hue + 0.6) * rib * 0.8;
    wall += vec3<f32>(fil * fil * fil * 0.6 * pm);   // white-hot filament cores
    // Builds: colour drains toward an icy white; the drop restores it.
    wall = mix(wall, vec3<f32>(lt_lum(wall)) * vec3<f32>(0.8, 0.9, 1.1), 0.5 * tn * (1.0 - dr));
    // Kick shockwave: seconds since the kick drive a bright band that sweeps from the tunnel end
    // out past the frame edge (capped where the old envelope inversion clamped).
    let kd = min(since_kick(), 0.75);
    let xs = (r - (0.05 + 4.0 * kd)) / (0.08 + 0.4 * kd);
    let shock = exp(-xs * xs) * sqrt(k);
    let gain = (0.13 + 0.05 * e) * (1.0 + 2.5 * k + 3.0 * shock) * (1.0 + 0.6 * dr) * (1.0 + 0.4 * tn);
    col += wall * fog * gain + wall * shock * (0.25 + 0.25 * e) * smoothstep(0.02, 0.2, r);

    // Kick ring: born at the tunnel end on each kick, carried outward by the zoom.
    let rk = 0.05 + 0.04 * k;
    let xk = (r - rk) / 0.01;
    col += pal(hue + 0.5) * exp(-xk * xk) * (0.6 * k + 1.5 * dr);

    // Faint glow from the light at the end of the tunnel.
    col += pal(P(5) + 0.6) * 0.02 * exp(-r * 18.0) * (1.0 + 2.0 * dr);
    col *= smoothstep(0.0, 0.04, r);
    // Soft luminance cap keeps the feedback from flooding.
    col = col / (1.0 + 0.12 * lt_lum(col));
    return vec4<f32>(col, 1.0);
}
