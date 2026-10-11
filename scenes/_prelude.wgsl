// Shared prelude, prepended to every scene file before compiling.
// A scene file defines exactly one entry:
//     fn scene(uv: vec2<f32>) -> vec4<f32>
// uv is 0..1 (0,0 = top-left). Return the NEW feedback colour for this pixel:
// linear HDR rgb (may exceed 1; the finish pass tonemaps), alpha ignored.
// The scene is responsible for sampling the previous frame (prev / warp), fading it,
// and drawing new shapes on top. Called once per simulation step (60 Hz).

struct U {
    res_time: vec4<f32>, // xy = resolution px, z = time s, w = dt s
    beat: vec4<f32>,     // kick env, beat phase, bar phase, phrase phase
    bands: vec4<f32>,    // sub, bass, mid, high
    shape: vec4<f32>,    // energy, tension, drop env, seed 0..1
    p0: vec4<f32>,       // scene params 0..3 (order = scene.toml)
    p1: vec4<f32>,       // scene params 4..7
    pal: array<vec4<f32>, 4>,
    misc: vec4<f32>,     // hue shift turns, quality (0 draft / 1 final), s since kick, s since drop (1e3 if none)
    p2: vec4<f32>,       // scene params 8..11 (generated looks: backdrop block)
    p3: vec4<f32>,       // scene params 12..15 (generated looks: overlay block)
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var prev_tex: texture_2d<f32>;
@group(0) @binding(2) var prev_samp: sampler; // linear, mirror-repeat
@group(0) @binding(3) var plate_tex: texture_2d_array<f32>; // the plan's pictures (linear rgba)
@group(0) @binding(4) var plate_samp: sampler;               // linear, clamp

const PI: f32 = 3.14159265;
const TAU: f32 = 6.28318531;

fn resolution() -> vec2<f32> { return u.res_time.xy; }
fn aspect() -> f32 { return u.res_time.x / u.res_time.y; }
fn time() -> f32 { return u.res_time.z; }
fn dt() -> f32 { return u.res_time.w; }
fn kick() -> f32 { return u.beat.x; }
fn beat_phase() -> f32 { return u.beat.y; }
fn bar_phase() -> f32 { return u.beat.z; }
fn phrase_phase() -> f32 { return u.beat.w; }
fn sub() -> f32 { return u.bands.x; }
fn bass() -> f32 { return u.bands.y; }
fn mid() -> f32 { return u.bands.z; }
fn high() -> f32 { return u.bands.w; }
fn energy() -> f32 { return u.shape.x; }
fn tension() -> f32 { return u.shape.y; }
fn drop_env() -> f32 { return u.shape.z; }
fn seed() -> f32 { return u.shape.w; }
fn quality() -> f32 { return u.misc.y; }
fn since_kick() -> f32 { return u.misc.z; }
fn since_drop() -> f32 { return u.misc.w; }

// Scene param i (0..15), order as declared in the scene's .toml.
fn P(i: i32) -> f32 {
    switch i {
        case 0: { return u.p0.x; } case 1: { return u.p0.y; }
        case 2: { return u.p0.z; } case 3: { return u.p0.w; }
        case 4: { return u.p1.x; } case 5: { return u.p1.y; }
        case 6: { return u.p1.z; } case 7: { return u.p1.w; }
        case 8: { return u.p2.x; } case 9: { return u.p2.y; }
        case 10: { return u.p2.z; } case 11: { return u.p2.w; }
        case 12: { return u.p3.x; } case 13: { return u.p3.y; }
        case 14: { return u.p3.z; } default: { return u.p3.w; }
    }
}

// Previous frame (HDR).
fn prev(uv: vec2<f32>) -> vec3<f32> { return textureSampleLevel(prev_tex, prev_samp, uv, 0.0).rgb; }

// Plate i (one of the plan's 3:2 pictures) placed in centred coords: x -1.5..1.5, y -1..1
// (the frame is x -aspect..aspect). Linear rgba with the picture's alpha; 0 outside it or when
// there is no plate i.
fn plate(i: i32, p: vec2<f32>) -> vec4<f32> {
    let uv = vec2<f32>(p.x / 3.0 + 0.5, 0.5 - p.y * 0.5);
    if (i < 0 || i >= i32(textureNumLayers(plate_tex)) || any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return vec4<f32>(0.0);
    }
    return textureSampleLevel(plate_tex, plate_samp, uv, i, 0.0);
}

// uv (0..1) <-> centred coords: y in -1..1, x in -aspect..aspect.
fn to_centered(uv: vec2<f32>) -> vec2<f32> { return vec2<f32>((uv.x * 2.0 - 1.0) * aspect(), 1.0 - uv.y * 2.0); }
fn to_uv(p: vec2<f32>) -> vec2<f32> { return vec2<f32>((p.x / aspect() + 1.0) * 0.5, (1.0 - p.y) * 0.5); }

fn rot2(a: f32) -> mat2x2<f32> { let c = cos(a); let s = sin(a); return mat2x2<f32>(c, s, -s, c); }

// MilkDrop-style per-pixel warp of the previous frame lookup.
// zoom > 1 pushes the image outward (sample closer to centre), angle rad/step,
// swirl rad scaled by distance from centre. Returns the uv to sample prev at.
fn warp(uv: vec2<f32>, zoom: f32, angle: f32, swirl: f32, center: vec2<f32>) -> vec2<f32> {
    var p = to_centered(uv) - center;
    let r = length(p);
    p = rot2(angle + swirl * r) * p / zoom;
    return to_uv(p + center);
}

// pcg(), VsOut and vs_main come from src/render/common.wgsl (prepended before this file).

// Integer hash of the exact float bits: no precision seams at large coordinates
// (a float fract/sin hash shards fbm once coords grow).
fn hash21(p: vec2<f32>) -> f32 {
    let b = bitcast<vec2<u32>>(p + 0.0);
    return f32(pcg(b.x ^ pcg(b.y)) >> 8u) / 16777216.0;
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

fn fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 5; i++) {
        v += a * noise2(p);
        p = rot2(0.5) * p * 2.03 + 17.0;
        a *= 0.5;
    }
    return v;
}

fn hue_rotate(c: vec3<f32>, turns: f32) -> vec3<f32> {
    // Rotate around the grey axis (preserves luminance-ish).
    let k = vec3<f32>(0.57735);
    let a = turns * TAU;
    let ca = cos(a);
    return c * ca + cross(k, c) * sin(a) + k * dot(k, c) * (1.0 - ca);
}

// Cyclic palette lookup over the 4 scene colours, with the phrase hue shift applied.
fn pal(t: f32) -> vec3<f32> {
    let x = fract(t) * 4.0;
    // fract() of a tiny negative can round to 1.0 (x == 4.0); wrap so u.pal[i] stays in bounds.
    let i = i32(floor(x)) % 4;
    let f = smoothstep(0.0, 1.0, fract(x));
    let c = mix(u.pal[i].rgb, u.pal[(i + 1) % 4].rgb, f);
    return max(hue_rotate(c, u.misc.x), vec3<f32>(0.0));
}

// Soft glow of a segment a-b in centred coords, width w.
fn line_glow(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, w: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    let d = length(pa - ba * h);
    return w * w / (d * d + w * w);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = scene(in.uv).rgb;
    // Guard against NaN/inf poisoning the feedback loop forever.
    let safe = select(vec3<f32>(0.0), clamp(c, vec3<f32>(0.0), vec3<f32>(64.0)), c == c);
    return vec4<f32>(safe, 1.0);
}
