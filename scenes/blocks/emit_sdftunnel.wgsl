// SDF tunnel: fly down a raymarched tube of radius EP(1); its wall carries rings (torus slices) every
// EP(0) units and 8 studs around each ring. Kicks fatten the rings; the hue steps per ring; fog by depth.
fn emit_sdftunnel_de(p: vec3<f32>) -> f32 {
    let r = length(p.xy);
    let tube = EP(1) - r;
    let zz = p.z - EP(0) * floor(p.z / EP(0)) - 0.5 * EP(0);
    let ring = length(vec2<f32>(r - EP(1) * 0.85, zz)) - 0.06 * (1.0 + kick());
    let seg = TAU / 8.0;
    let a = atan2(p.y, p.x);
    let aa = a - seg * floor(a / seg) - 0.5 * seg;
    let q = vec3<f32>(r * cos(aa) - EP(1) * 0.9, r * sin(aa), zz);
    let stud = length(max(abs(q) - vec3<f32>(0.08, 0.08, 0.3), vec3<f32>(0.0)));
    return min(tube, min(ring, stud));
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let ro = vec3<f32>(0.15 * sin(time() * 0.3), 0.15 * cos(time() * 0.23), time() * 1.6);
    let d0 = normalize(vec3<f32>(p * 0.9, 1.0));
    let rd = vec3<f32>(rot2(time() * 0.05) * d0.xy, d0.z);
    var t = 0.0;
    var hit = -1.0;
    let n = select(48, 80, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let h = emit_sdftunnel_de(ro + rd * t);
        if (h < 0.002 * t) { hit = t; break; }
        t += h;
        if (t > 30.0) { break; }
    }
    if (hit < 0.0) { return vec2<f32>(0.0); }
    let z = ro.z + rd.z * hit;
    let fog = exp(-0.08 * hit);
    let ringlit = 1.0 - smoothstep(0.0, 0.25, abs(z - EP(0) * floor(z / EP(0)) - 0.5 * EP(0)));
    let inten = fog * (0.35 + 0.9 * ringlit * (0.6 + 0.8 * punch())) * (1.0 + 0.5 * drop_env());
    return vec2<f32>(inten, fract(EP(2) + 0.1 * floor(z / EP(0))));
}
