// Ink blobs: each kick blooms a combed drop at one of four per-bar positions; two slow
// nozzles keep thin streams going between kicks.

fn emit_blobs_ridge(p: vec2<f32>) -> f32 {
    let n = 1.0 - abs(2.0 * noise2(p) - 1.0);
    return n * n * n;
}

fn emit_blobs_nozzle(t: f32, fj: f32, s: f32) -> vec2<f32> {
    return vec2<f32>(sin(t * (0.17 + 0.05 * fj) + s + fj * 2.1) * aspect() * 0.7,
                     sin(t * (0.23 - 0.04 * fj) + s * 2.0 + fj * 4.0) * 0.65);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let k = kick();
    let t = time();
    let s = seed() * 37.0;
    // Drop position: one per beat of the bar, reshuffled every 16th of a phrase.
    let bid = floor(bar_phase() * 4.0 + 0.15) + 5.0 * floor(phrase_phase() * 16.0) + s;
    let hp = vec2<f32>(hash21(vec2<f32>(bid, 1.7)), hash21(vec2<f32>(bid, 8.3)));
    let dv = p - (hp - 0.5) * vec2<f32>(1.5 * aspect(), 1.4) * (1.0 - 0.6 * tension());
    let rough = 1.0 + 0.6 * (noise2(dv * 7.0 + bid) - 0.5);
    let dd = length(dv) * rough / (EP(0) * (0.7 + 0.5 * k));
    let comb = emit_blobs_ridge(dv / max(length(dv), 1e-4) * 3.5 + vec2<f32>(bid * 1.3, dd * 1.2));
    let rq = (dd - 0.8) / 0.22;
    let blob = k * max(exp(-rq * rq), 0.5 * (1.0 - smoothstep(0.7, 1.0, dd))) * (0.35 + 0.9 * comb);

    // Streams: segment from last step's nozzle position to this one (continuous, no beads).
    var stream = 0.0;
    var shue = 0.0;
    for (var j = 0; j < 2; j++) {
        let fj = f32(j);
        let a0 = emit_blobs_nozzle(t - dt(), fj, s);
        let ba = emit_blobs_nozzle(t, fj, s) - a0;
        let pa = p - a0;
        let hh = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-8), 0.0, 1.0);
        let rd = length(pa - ba * hh) / 0.04;
        let g = EP(2) * exp(-rd * rd);
        stream += g;
        shue += g * (0.3 + 0.4 * fj);
    }
    let i = blob + stream;
    let bhue = 0.25 * floor(hash21(vec2<f32>(bid, 4.1)) * 3.0) + 0.06 * (noise2(dv * 6.0) - 0.5);
    return vec2<f32>(i, EP(1) + (blob * bhue + shue) / max(i, 1e-4));
}
