// Kick-born rings: each kick fires a wobbly ring from the centre that expands and fades;
// a drop fires a wider, brighter one. Each kick gets its own hue and wobble.

fn emit(p: vec2<f32>) -> vec2<f32> {
    let r = length(p);
    let sk = since_kick();
    let kid = floor((time() - sk) * 4.0);  // constant between kicks: a per-kick seed
    let dir = p / max(r, 1e-4);
    let rr = 0.05 + sk * EP(0);
    let wob = 0.08 * rr * (noise2(dir * 1.5 + kid) - 0.5);
    let x = (r + wob - rr) / (EP(1) * (1.0 + sk));
    let ring = exp(-x * x) * exp(-sk * 1.2);
    let sd = since_drop();  // 1e3 when there was no drop
    let xd = (r - 0.05 - sd * 2.2) / (0.03 + 0.05 * sd);
    let dring = exp(-xd * xd) * exp(-sd * 0.8) * 1.5;
    let core = exp(-r * r / 0.003) * kick() * 0.5;
    let i = ring + dring + core;
    let hue = EP(2) + 0.5 * hash21(vec2<f32>(kid, 1.7)) + 0.4 * dring / max(i, 1e-4);
    return vec2<f32>(i, hue);
}
