// The look's picture as a cut-out on a slowly turning, tilted card floating in dark space, with a
// faint translucent card and glowing edges (hologram). Kicks brighten it.

// v in card space: undo a rotation by b about x after a about y.
fn back_sprite_local(v: vec3<f32>, a: f32, b: f32) -> vec3<f32> {
    let uxz = rot2(a) * v.xz;
    let yz = vec2<f32>(v.y, uxz.y) * rot2(b);
    return vec3<f32>(uxz.x, yz.x, yz.y);
}

fn backdrop(p: vec2<f32>) -> vec3<f32> {
    var c = mix(pal(0.25) * 0.25, pal(0.0), smoothstep(0.0, 1.6, length(p)));
    let yaw = sin(time() * 0.1) * BP(0);
    let o = back_sprite_local(vec3<f32>(0.0, 0.0, -3.0), yaw, BP(1));
    let d = back_sprite_local(normalize(vec3<f32>(p, 2.2)), yaw, BP(1));
    if (abs(d.z) > 1e-4) {
        let t = -o.z / d.z;
        if (t > 0.0) {
            let q = (o.xy + d.xy * t - vec2<f32>(0.0, 0.05 * sin(time() * 0.7))) / BP(2);
            let edge = max(abs(q.x) - 1.5, abs(q.y) - 1.0);
            let card = pal(0.5) * (0.06 * step(edge, 0.0) + 0.5 * (1.0 - smoothstep(0.0, 0.02, abs(edge))));
            c += card * (1.0 + kick());
            let im = img(q);
            c = mix(c, im.rgb * (1.0 + 0.3 * kick()), im.a);
        }
    }
    return c;
}
