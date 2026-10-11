// Engine helper passes: copy (with hardware constant blend), bloom down/up chain.
// Bindings: group0 binding0 source texture, binding1 linear clamp sampler.

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

fn s(uv: vec2<f32>) -> vec3<f32> { return textureSampleLevel(src, samp, uv, 0.0).rgb; }
fn texel() -> vec2<f32> { return 1.0 / vec2<f32>(textureDimensions(src)); }

// Used for morph blend (constant = mix) and sub-frame accumulation (constant = 1/i).
@fragment
fn fs_copy(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(s(in.uv), 1.0);
}

// Dual-filter downsample: centre x4 + 4 diagonal taps.
fn down(uv: vec2<f32>) -> vec3<f32> {
    let o = texel();
    var c = s(uv) * 4.0;
    c += s(uv + vec2<f32>(-o.x, -o.y)) + s(uv + vec2<f32>(o.x, -o.y));
    c += s(uv + vec2<f32>(-o.x, o.y)) + s(uv + o);
    return c / 8.0;
}

const THRESHOLD: f32 = 1.1;
const KNEE: f32 = 0.4;

@fragment
fn fs_down_first(in: VsOut) -> @location(0) vec4<f32> {
    let c = min(down(in.uv), vec3<f32>(16.0)); // tame fireflies
    let br = max(c.r, max(c.g, c.b));
    var soft = clamp(br - THRESHOLD + KNEE, 0.0, 2.0 * KNEE);
    soft = soft * soft / (4.0 * KNEE + 1e-4);
    let w = max(soft, br - THRESHOLD) / max(br, 1e-4);
    return vec4<f32>(c * w, 1.0);
}

@fragment
fn fs_down(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(down(in.uv), 1.0);
}

// Tent upsample, added onto the next larger level (additive blend).
@fragment
fn fs_up(in: VsOut) -> @location(0) vec4<f32> {
    let o = texel();
    let h = o * 0.5;
    var c = s(in.uv + vec2<f32>(-o.x, 0.0)) + s(in.uv + vec2<f32>(o.x, 0.0));
    c += s(in.uv + vec2<f32>(0.0, -o.y)) + s(in.uv + vec2<f32>(0.0, o.y));
    c += 2.0 * (s(in.uv + vec2<f32>(-h.x, -h.y)) + s(in.uv + vec2<f32>(h.x, -h.y)));
    c += 2.0 * (s(in.uv + vec2<f32>(-h.x, h.y)) + s(in.uv + h));
    return vec4<f32>(c / 12.0, 1.0);
}

// History ring write: the feedback frame, squeezed from HDR into 0..1 for the Rgba8 ring (prelude hist() undoes it).
@fragment
fn fs_hist(in: VsOut) -> @location(0) vec4<f32> {
    let c = s(in.uv);
    return vec4<f32>(c / (1.0 + c), 1.0);
}
