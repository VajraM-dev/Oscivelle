// Finish pass: HDR sim texture (supersampled) + bloom -> display-referred 8-bit (sRGB-encoded values).

struct F {
    a: vec4<f32>,  // exposure, flash, saturation, grain on (0/1)
    b: vec4<f32>,  // time s, bloom strength, downsample tap offset (output px), debug overlay (0/1)
    r: vec4<f32>,  // output w, h px, output frame index
    d0: vec4<f32>, // kick, energy, tension, bass
    d1: vec4<f32>, // beat phase, bar phase, phrase phase, drop env
    p0: vec4<f32>, // vhs, scanlines, posterize, tint amount (all 0 = clean)
    p1: vec4<f32>, // tint rgb (linear)
};

@group(0) @binding(0) var<uniform> f: F;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var bloom: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

// Supersampled -> output: 4 bilinear taps around the pixel (tent-ish filter over ~2.5 source px).
fn hdr(uv: vec2<f32>) -> vec3<f32> {
    let o = f.b.z / f.r.xy;
    if (f.b.z == 0.0) { return textureSampleLevel(src, samp, uv, 0.0).rgb; }
    var c = textureSampleLevel(src, samp, uv + vec2<f32>(-o.x, -o.y), 0.0).rgb;
    c += textureSampleLevel(src, samp, uv + vec2<f32>(o.x, -o.y), 0.0).rgb;
    c += textureSampleLevel(src, samp, uv + vec2<f32>(-o.x, o.y), 0.0).rgb;
    c += textureSampleLevel(src, samp, uv + o, 0.0).rgb;
    return c * 0.25;
}

// ACES fitted (Narkowicz) curve.
fn aces(x: vec3<f32>) -> vec3<f32> {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Hue-preserving roll-off: the curve runs on the peak channel and scales rgb by it, so a hot
// orange stays orange instead of per-channel ACES pushing it to flat yellow/white. Only very
// hot values (peak >> 1) drift partly towards white, like film.
fn tonemap(c: vec3<f32>) -> vec3<f32> {
    let peak = max(max(c.r, c.g), max(c.b, 1e-5));
    let hue = c * (aces(vec3<f32>(peak)).x / peak);
    return mix(hue, aces(c), 0.5 * smoothstep(1.5, 8.0, peak));
}

fn srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

// Uniform 0..1 per pixel, per frame, per stream k.
fn rnd(px: vec2<f32>, frame: u32, k: u32) -> f32 {
    let i = vec2<u32>(px);
    return f32(pcg(i.x + pcg(i.y + pcg(frame * 4u + k))) >> 8u) / 16777216.0;
}

fn box(q: vec2<f32>, lo: vec2<f32>, hi: vec2<f32>) -> bool {
    return all(q >= lo) && all(q <= hi);
}

// Debug panel, bottom-left. q = local panel units (1 unit = 1 px at 1080p).
fn overlay(c0: vec3<f32>, px: vec2<f32>) -> vec3<f32> {
    let s = f.r.y / 1080.0;
    let q = (px - vec2<f32>(20.0 * s, f.r.y - 170.0 * s)) / s;
    if (!box(q, vec2<f32>(0.0), vec2<f32>(330.0, 150.0))) { return c0; }
    var c = c0 * 0.35;
    // Beat-in-bar boxes: lit box = current beat, fades over the beat.
    let beat = i32(floor(f.d1.y * 4.0));
    for (var k = 0; k < 4; k++) {
        let x0 = 10.0 + f32(k) * 28.0;
        if (box(q, vec2<f32>(x0, 10.0), vec2<f32>(x0 + 22.0, 30.0))) {
            c = select(vec3<f32>(0.15), vec3<f32>(0.4 + 0.6 * (1.0 - f.d1.x)), k == beat);
        }
    }
    // Beat phase sweep line.
    if (box(q, vec2<f32>(130.0, 18.0), vec2<f32>(130.0 + 190.0 * f.d1.x, 22.0))) { c = vec3<f32>(0.8); }
    var vals = array<f32, 6>(f.d0.x, f.d0.y, f.d0.z, f.d0.w, f.d1.z, f.d1.w);
    var cols = array<vec3<f32>, 6>(
        vec3<f32>(1.0, 0.55, 0.1), // kick
        vec3<f32>(0.3, 0.9, 0.3),  // energy
        vec3<f32>(1.0, 0.25, 0.25),// tension
        vec3<f32>(0.3, 0.5, 1.0),  // bass
        vec3<f32>(0.6, 0.6, 0.6),  // phrase phase
        vec3<f32>(1.0, 1.0, 1.0),  // drop env
    );
    for (var i = 0; i < 6; i++) {
        let y0 = 40.0 + f32(i) * 18.0;
        if (box(q, vec2<f32>(10.0, y0), vec2<f32>(310.0, y0 + 12.0))) {
            c = select(vec3<f32>(0.12), cols[i], q.x - 10.0 < clamp(vals[i], 0.0, 1.0) * 300.0);
        }
    }
    return c;
}

// VHS: slow line-wise horizontal wobble plus a tracking band rolling up the frame with per-line noise.
fn vhs_uv(uv: vec2<f32>, amt: f32, t: f32, frame: u32) -> vec2<f32> {
    let wobble = sin(uv.y * 23.0 + t * 1.7) * 0.6 + sin(uv.y * 71.0 - t * 3.1) * 0.4;
    let band = 1.0 - smoothstep(0.0, 0.04, abs(fract(uv.y + t * 0.05) - 0.5));
    let line = rnd(vec2<f32>(0.0, floor(uv.y * f.r.y)), frame, 4u) - 0.5;
    return vec2<f32>(uv.x + amt * (0.0012 * wobble + 0.012 * band * line), uv.y);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let frame = u32(f.r.z);
    let uv = select(in.uv, vhs_uv(in.uv, f.p0.x, f.b.x, frame), f.p0.x > 0.0);
    let p = uv - 0.5;
    // Gentle chromatic aberration, growing towards the edges; VHS adds a horizontal chroma bleed.
    let ca = p * dot(p, p) * 0.012 + vec2<f32>(f.p0.x * 0.004, 0.0);
    // Full supersample filter on green only; the fringed red/blue get a single tap each.
    let r = textureSampleLevel(src, samp, uv + ca, 0.0).r;
    let b = textureSampleLevel(src, samp, uv - ca, 0.0).b;
    var c = vec3<f32>(r, hdr(uv).g, b);
    c += textureSampleLevel(bloom, samp, uv, 0.0).rgb * f.b.y;
    c *= f.a.x * (1.0 + 0.6 * f.a.y);
    let l = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = max(mix(vec3<f32>(l), c, f.a.z), vec3<f32>(0.0));
    c *= 1.0 - 0.45 * smoothstep(0.3, 0.8, length(p * vec2<f32>(1.0, 0.75)) * 1.1);
    c = srgb(tonemap(c));
    // Looks (finish-pass only; every term is a no-op at 0, so the clean look is unchanged).
    let lum = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = mix(c, lum * srgb(f.p1.rgb) * 1.4, f.p0.w);
    if (f.p0.z > 0.0) {
        let levels = mix(64.0, 4.0, f.p0.z);
        c = floor(c * levels + 0.5) / levels;
    }
    // Scanlines: one dark line every 2 px at 1080p (scaled with the output height).
    c *= 1.0 - f.p0.y * 0.35 * (0.5 + 0.5 * cos(in.pos.y * 540.0 / f.r.y * 6.2831853));

    if (f.a.w > 0.0) {
        // Light, fine, monochrome (so 4:2:0 chroma subsampling can't smear it), mostly in the midtones.
        let g = rnd(in.pos.xy, frame, 0u) + rnd(in.pos.xy, frame, 1u) - 1.0;
        let lum = dot(c, vec3<f32>(0.333));
        c += g * 0.02 * (0.3 + lum * (1.0 - lum) * 2.8);
    }
    if (f.b.w > 0.0) { c = overlay(c, in.pos.xy); }
    // Triangular dither, +-1 LSB.
    c += (rnd(in.pos.xy, frame, 2u) + rnd(in.pos.xy, frame, 3u) - 1.0) / 255.0;
    return vec4<f32>(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
