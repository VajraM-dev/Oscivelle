// Cells: a Worley/Voronoi field. Bright walls (second-nearest minus nearest distance) and glowing nuclei;
// the seeds drift, and each kick lights a random few cells.
fn emit_cells_h2(c: vec2<f32>) -> vec2<f32> { return vec2<f32>(hash21(c), hash21(c + vec2<f32>(19.1, 7.3))); }

fn emit(p: vec2<f32>) -> vec2<f32> {
    let s = p * EP(0);
    let ip = floor(s);
    let fp = fract(s);
    var d1 = 8.0;
    var d2 = 8.0;
    var id = vec2<f32>(0.0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let g = vec2<f32>(f32(x), f32(y));
            let o = vec2<f32>(0.5) + 0.45 * sin(vec2<f32>(time() * 0.4) + TAU * emit_cells_h2(ip + g));
            let d = length(g + o - fp);
            if (d < d1) { d2 = d1; d1 = d; id = ip + g; } else if (d < d2) { d2 = d; }
        }
    }
    let wall = 1.0 - smoothstep(0.0, EP(1), d2 - d1);
    let nucleus = exp(-d1 * d1 * 40.0) * 0.6;
    let kid = floor((time() - since_kick()) * 4.0);
    let lit = step(0.8, hash21(id + kid)) * kick() * 0.8 * max(1.0 - d1, 0.0);
    return vec2<f32>((wall * 0.9 + nucleus + lit) * (0.8 + 0.5 * drop_env()), fract(EP(2) + 0.3 * hash21(id)));
}
