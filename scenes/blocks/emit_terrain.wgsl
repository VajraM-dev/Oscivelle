// Landscape: a raymarched heightfield. seed() picks the ground (dunes, ridged mountains, terraced mesas,
// crater field, ocean swell) and the camera (sideways glide, crane over a ridge, pan from a fixed point,
// high aerial drift). Sun + sky gradient, soft shadows, fog. Bass lifts the relief, kicks sweep a band of
// light across the ground, drops brighten. Hue from height bands and slope; the sky sits half a turn away.
fn emit_terrain_ridged(p0: vec2<f32>) -> f32 {
    var p = p0;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 4; i++) {
        let n = 1.0 - abs(noise2(p) * 2.0 - 1.0);
        v += a * n * n;
        p = rot2(0.6) * p * 2.1 + 9.0;
        a *= 0.5;
    }
    return v;
}

fn emit_terrain_h(xz: vec2<f32>) -> f32 {
    let q = xz * EP(1) * 0.35 + floor(seed() * 61.0) * 3.7;
    let style = i32(floor(fract(seed() * 7.31) * 5.0));
    var h = 0.0;
    if (style == 0) { // dunes: wind ripples bent by fbm
        let f = fbm(q * 0.6);
        let w = 1.0 - abs(sin(q.x * 1.7 + q.y * 0.6 + f * 4.0));
        h = 0.5 * w * w + 1.2 * f - 0.3;
    } else if (style == 1) { // ridged mountains
        h = 3.0 * emit_terrain_ridged(q * 0.45) - 0.8;
    } else if (style == 2) { // terraced mesas
        let k = fbm(q * 0.45) * 7.0;
        h = (floor(k) + smoothstep(0.75, 1.0, fract(k))) * 0.4 - 1.2;
    } else if (style == 3) { // crater field: one crater per cell, kept inside its cell
        let g = q * 0.8;
        let c = floor(g);
        let ctr = vec2<f32>(0.35) + 0.3 * vec2<f32>(hash21(c), hash21(c + 7.0));
        let rad = 0.12 + 0.12 * hash21(c + 3.1);
        let r = length(fract(g) - ctr) / rad;
        let bowl = -0.9 * rad * max(1.0 - r * r, 0.0) * 3.0;
        let rim = 0.8 * rad * exp(-(r - 1.0) * (r - 1.0) * 8.0);
        h = 0.7 * fbm(q * 0.7) + bowl + rim;
    } else { // ocean swell
        h = 0.22 * (sin(q.x * 2.0 + time() * 0.9) + sin(q.x * 0.7 + q.y * 1.6 - time() * 1.2))
            + 0.35 * fbm(q * 1.3 + time() * 0.05) - 0.2;
    }
    return h * (1.0 + 0.6 * bass());
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let cam = i32(floor(fract(seed() * 13.7 + 0.3) * 4.0));
    let tt = time() * EP(0);
    let yaw0 = seed() * TAU;
    let dir = vec2<f32>(sin(yaw0), cos(yaw0));
    var xz = vec2<f32>(0.0);
    var yaw = yaw0;
    var pitch = -0.1;
    var lift = 1.0;
    if (cam == 0) { // low glide across: travel along dir, look sideways
        xz = dir * tt * 1.5;
        yaw = yaw0 + 1.5708;
        lift = 0.9;
        pitch = -0.08;
    } else if (cam == 1) { // crane rising over a ridge, tilting down as it climbs
        let k = 0.5 - 0.5 * cos(tt * 0.12);
        xz = dir * tt * 0.2;
        lift = 0.5 + 3.5 * k;
        pitch = -0.03 - 0.35 * k;
    } else if (cam == 2) { // slow pan from a fixed point
        yaw = yaw0 + 0.9 * sin(tt * 0.1);
        lift = 1.6;
        pitch = -0.12;
    } else { // high aerial drift, diagonal to the view
        xz = vec2<f32>(sin(yaw0 + 0.8), cos(yaw0 + 0.8)) * tt * 0.8;
        lift = 6.0;
        pitch = -0.55;
    }
    pitch += (fract(seed() * 31.7) - 0.5) * 0.2; // horizon placement
    let ground = max(emit_terrain_h(xz), emit_terrain_h(xz + vec2<f32>(sin(yaw), cos(yaw))));
    let ro = vec3<f32>(xz.x, ground + lift, xz.y);
    let fw = vec3<f32>(cos(pitch) * sin(yaw), sin(pitch), cos(pitch) * cos(yaw));
    let rt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fw));
    let up = cross(fw, rt);
    let rd = normalize(fw * 1.5 + rt * p.x + up * p.y);

    let hoff = fract(seed() * 3.7);
    let sa = yaw0 + (fract(seed() * 5.3) - 0.5) * 2.4;
    let sun = normalize(vec3<f32>(sin(sa), EP(2), cos(sa)));
    let sd = max(dot(rd, sun), 0.0);
    let sky = (0.1 + 0.35 * pow(1.0 - max(rd.y, 0.0), 3.0) + 0.25 * pow(sd, 4.0) + 1.1 * pow(sd, 64.0))
        * (1.0 + 0.4 * drop_env());
    let sky_hue = fract(hoff + 0.5 + 0.1 * rd.y);

    // Heightfield march: relaxed steps, interpolate the crossing. Out of steps short of the far plane
    // counts as a (foggy) hit so grazing rays never punch holes to the sky.
    let far = 40.0;
    let n = select(48, 80, quality() > 0.5);
    var t = 0.05;
    var lt = t;
    var ld = 1e3;
    var hit = -1.0;
    for (var i = 0; i < n; i++) {
        let pos = ro + rd * t;
        let d = pos.y - emit_terrain_h(pos.xz);
        if (d < 0.002 * t) { hit = lt + (t - lt) * ld / max(ld - d, 1e-4); break; }
        lt = t;
        ld = d;
        t += max(d * 0.4, 0.01 + 0.004 * t);
        if (t > far) { break; }
    }
    if (hit < 0.0 && t < far) { hit = t; }
    if (hit < 0.0) { return vec2<f32>(sky, sky_hue); }

    let pos = ro + rd * hit;
    let e = 0.03 + 0.002 * hit;
    let hc = emit_terrain_h(pos.xz);
    let nrm = normalize(vec3<f32>(
        hc - emit_terrain_h(pos.xz + vec2<f32>(e, 0.0)), e, hc - emit_terrain_h(pos.xz + vec2<f32>(0.0, e))));

    var sh = 1.0;
    var st = 0.08;
    for (var i = 0; i < 16; i++) {
        let q = pos + sun * st;
        let d = q.y - emit_terrain_h(q.xz);
        sh = min(sh, 10.0 * d / st);
        st += clamp(d * 0.6, 0.08, 0.8);
        if (sh < 0.0 || st > 12.0) { break; }
    }
    sh = clamp(sh, 0.0, 1.0);

    // Kick: a band of light sweeps away from the camera across the ground.
    let ahead = dot(pos.xz - ro.xz, normalize(rd.xz + vec2<f32>(1e-4)));
    let sk = since_kick();
    let sweep = exp(-(ahead - sk * 12.0) * (ahead - sk * 12.0) * 0.6) * exp(-sk * 1.5) + 0.3 * punch();
    let lit = max(dot(nrm, sun), 0.0) * sh * 1.1 + 0.18 * (0.5 + 0.5 * nrm.y) + 0.8 * sweep;
    let fog = exp(-0.05 * hit);
    let inten = mix(sky, lit * (1.0 + 0.6 * drop_env()), fog);
    let slope = 1.0 - nrm.y;
    let hue = fract(hoff + 0.08 * floor(hc * 2.0) + 0.25 * smoothstep(0.3, 0.6, slope));
    return vec2<f32>(inten, mix(sky_hue, hue, fog));
}
