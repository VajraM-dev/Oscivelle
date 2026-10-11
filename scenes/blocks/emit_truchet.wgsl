// Truchet maze: quarter-circle arc tiles; some big tiles split into four small ones (two scales). The
// arcs form endless winding paths; every 4 s a few tiles flip.
fn emit_truchet_arcs(q: vec2<f32>, h: f32, w: f32) -> f32 {
    var f = fract(q) - 0.5;
    if (h > 0.5) { f.x = -f.x; }
    let d = min(abs(length(f - 0.5) - 0.5), abs(length(f + 0.5) - 0.5));
    return 1.0 - smoothstep(0.0, w, d);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let s = p * EP(0) + vec2<f32>(0.0, time() * 0.05);
    let c = floor(s);
    let split = hash21(c + 3.1) > 0.6;
    let flip = select(0.0, floor(time() * 0.25), hash21(c * 0.37) > 0.8);
    var i: f32;
    if (split) {
        let q = s * 2.0;
        i = emit_truchet_arcs(q, hash21(floor(q) + flip), EP(1) * 2.0);
    } else {
        i = emit_truchet_arcs(s, hash21(c + flip), EP(1));
    }
    i *= 0.7 + 0.5 * punch() + 0.5 * drop_env();
    return vec2<f32>(i, fract(EP(2) + 0.25 * hash21(c) + select(0.0, 0.33, split)));
}
