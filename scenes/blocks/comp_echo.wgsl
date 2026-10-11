// Time echo (display only): the frame plus three fading copies from 6, 12 and 18 frames ago, each a little
// larger and hue-shifted, so moving shapes leave stepped ghost trails; kicks brighten the echoes.
fn comp_echo_at(uv: vec2<f32>, s: f32) -> vec2<f32> { return (uv - 0.5) / s + 0.5; }

fn comp(uv: vec2<f32>) -> vec3<f32> {
    var c = prev(uv);
    for (var i = 1; i < 4; i++) {
        let fi = f32(i);
        let echo = hist(comp_echo_at(uv, 1.0 + 0.04 * fi), 6.0 * fi);
        c += hue_rotate(echo, 0.08 * fi) * (0.5 - 0.12 * fi) * (1.0 + 0.5 * kick());
    }
    return c;
}
