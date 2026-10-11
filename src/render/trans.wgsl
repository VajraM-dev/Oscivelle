// Styled morph: the incoming look B (scratch) laid over A (already in the target) through a mask that grows
// with progress. Alpha blending, so out = B * m + A * (1 - m). Every style is 0 at progress 0 and 1 at 1, and
// each pixel switches once (no flashing).

struct T {
    a: vec4<f32>, // progress 0..1, style (1 wipe, 2 iris, 3 melt, 4 cells), aspect, unused
};

@group(0) @binding(0) var<uniform> t: T;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

fn trans_hash(c: vec2<f32>) -> f32 {
    let b = bitcast<vec2<u32>>(c + 0.0);
    return f32(pcg(b.x ^ pcg(b.y)) >> 8u) / 16777216.0;
}

@fragment
fn fs_trans(in: VsOut) -> @location(0) vec4<f32> {
    let k = t.a.x;
    let aspect = t.a.z;
    var uv = in.uv;
    var m = k;
    switch i32(t.a.y) {
        case 1: {
            // Luma wipe: B's brightest parts arrive first.
            let c = textureSampleLevel(src, samp, uv, 0.0).rgb;
            let l = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
            let key = 1.0 - l / (1.0 + l);
            m = smoothstep(key - 0.15, key, k * 1.15);
        }
        case 2: {
            // Iris zoom-through: B zooms in from the centre inside a growing circle.
            let p = (in.uv - 0.5) * vec2<f32>(aspect, 1.0);
            let r = k * (0.5 * length(vec2<f32>(aspect, 1.0)) + 0.06);
            m = 1.0 - smoothstep(r - 0.04, r, length(p));
            uv = (in.uv - 0.5) * mix(0.35, 1.0, k) + 0.5;
        }
        case 3: {
            // Melt: B drips in from the top, each column at its own speed, sliding down as it lands.
            let col = trans_hash(vec2<f32>(floor(in.uv.x * 48.0), 3.0));
            let front = k * (1.6 + 0.8 * col) - 0.3;
            m = 1.0 - smoothstep(front - 0.08, front, in.uv.y);
            uv.y = in.uv.y - (1.0 - k) * 0.15 * col;
        }
        default: {
            // Cells: a grid of tiles flips over to B in random order.
            let cell = floor(in.uv * vec2<f32>(round(9.0 * aspect), 9.0));
            m = step(trans_hash(cell), k);
        }
    }
    return vec4<f32>(textureSampleLevel(src, samp, uv, 0.0).rgb, clamp(m, 0.0, 1.0));
}
