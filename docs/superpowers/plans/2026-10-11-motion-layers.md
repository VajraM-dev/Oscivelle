# Motion Layers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: follow `/code-implementation-build-rules` (orchestrator dispatches, subagents build; waves below). Use superpowers:subagent-driven-development for per-task dispatch. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Motion graphics in the visualiser. Feedback visuals are composited with backdrops (gradient skies, a raymarched 90s-CGI sea world, pictures), crisp overlays (HUD rings, retro grids, pixel-font text) and finishing looks (VHS, CRT, mono tint), with shot changes on the music's structure. The core engine does all of this without AI (`--layers`, `--image`, `--look`). Optional AI adds generated pictures (OpenAI image models), a storyboard and AI-written layer blocks.

**Architecture:** **The core comes first. AI is optional.** Milestone 1 builds every capability into the core engine and the block library, and exposes it through plain CLI flags. Milestone 2 only lets the AI choose and generate inputs for those same capabilities. The default output (no new flags) stays **byte-identical**, and the gate checks this. A genome gains two optional layer blocks (`backdrop`, `overlay`), an image index, text and a `Post` look. A genome that has layers also writes `<id>.layers.wgsl`: an ordinary scene whose `prev()` samples the finished feedback. The engine runs it once per output frame between accumulation and bloom (the "compose" pass). Pictures ("plates") are a `texture_2d_array` bound to every scene. Shots are extra plan boundaries the director snaps to phrases.

**Tech Stack:** Rust 2021, wgpu 30, WGSL. New crates: `image` 0.25 (png + jpeg decode and fit, M1), `base64` 0.22 (decode image-API replies, M2).

**References (the user's screenshots, 2026-10-11):** (1) a raymarched sphere and pyramid over a reflective sea under a blue gradient sky. (2) A green brain cut-out on a tilted translucent card, with HUD rings and tick marks and mono tint. (3) A saturated cloud plate slowly panning, with lowercase pixel text top-right and VHS softness. M1 alone reaches (1) and most of (3) (with `--image clouds.jpg`). M2 adds generated cut-outs for (2).

---

## Build setup (from /code-implementation-build-rules)

`T2 Standard · no web frontend (CLI + GPU shaders; design gate n/a; the screenshots above are the style reference) · review: /simplify → /review`

- **Branch:** all work on `feature/motion-layers`, created from `main` @ `5899a96` (the merge of `feature/dynamic-visuals`). Never commit to `main`. The user merges.
- **Orchestrator** plans, dispatches and joins. Subagents do the work. Independent tasks in one wave are dispatched as multiple `Agent` calls **in a single message**.
- **Model routing:** `opus` for the engine, genome, shader blocks, director, vibe and anything ambiguous. `sonnet` for spelled-out mechanical tasks (P5 `llm.rs`, docs). **No Haiku.**
- **Testing discipline:** during a wave, subagents run only `cargo build` (shader tasks: nothing, because WGSL is compiled by the gate's `engine_runs` test). They report and stop. Tests run once at each **milestone gate**. After a fix, re-run only the failing test, then one full `cargo test --release` at the gate.
- **Milestones**, as in the previous plan, with review twice:
  - **M1 (core, no AI)** = Task 0 + Phases 1–4 → gate (includes the byte-identical check) → `/simplify` → `/review` → docs → commit.
  - **M2 (AI)** = Phases 5–6 → gate → `/simplify` → `/review` → docs → commit.
- **ponytail:** minimum code. Each non-trivial piece of logic leaves ONE runnable check (a `#[test]`). Mark deliberate shortcuts with `// ponytail:` comments.
- **Commits:** one per phase, style `feat(layers): ...`, `feat(ai): ...`.

### Waves

```
M1  Wave 0 (orchestrator): Task 0 contract (types, prelude, genome fields, CONTRACT.md) → cargo build
    Wave 1 (parallel): P1 engine + finish + frame_state [opus] · P2 genome layers + font [opus]
                       · P3a backdrop blocks [opus] · P3b overlay blocks [opus]
    Wave 2: P4 core wiring: --layers/--image/--look, plates, shots, render/preview, wizard [opus]
    Gate M1 → /simplify → /review → docs [sonnet] → commit
M2  Wave 3 (parallel): P5 llm image + setup [sonnet] · P6 vibe layers/plates/storyboard + main wiring [opus]
    Gate M2 → /simplify → /review → docs [sonnet] → commit
```

File-collision check: Task 0 touches the struct literals everywhere, *before* Wave 1. In Wave 1: P1 owns `engine.rs`, `finish.wgsl`, `scenes.rs`, `director.rs` (`frame_state` + `param_mut` only) and `score.rs` (one line). P2 owns `genome.rs`, `font.rs` (new) and the single `mod font;` line in `main.rs`. P3a/P3b own only *new* files under `scenes/blocks/`. P4 (Wave 2) owns `main.rs`, `director.rs` (`plan()`), `plates.rs` (new), `render/mod.rs`, `preview.rs`, `wizard.rs` and `Cargo.toml`. In Wave 3: P5 owns `llm.rs` and `Cargo.toml`. P6 owns `vibe.rs` and `main.rs`, and calls `llm::image` by the signature fixed in Task 5.1.

---

## File map

| File | Status | Responsibility |
|---|---|---|
| `src/types.rs` | modify | `Post`, `Look`, `Shot`, `PLATE_W/H`, `MAX_PARAMS`; `SceneUniforms.p2/p3`; `FrameState.post`; `Plan.plates/post/layers` |
| `scenes/_prelude.wgsl` | modify | `U.p2/p3`, `P(0..15)`, plate bindings 3/4, `plate(i, p)` |
| `scenes/blocks/CONTRACT.md` | modify | Layer block contract, `img()`, `text_px()` |
| `src/genome.rs` | modify | Layer kinds, `layer_parts`, 14-slot scenes, `layers_wgsl`, `dress` |
| `src/font.rs` | create | 5x7 pixel font → WGSL (`text_px`, `TEXT_LEN`) |
| `src/render/engine.rs` | modify | Plate array + `set_plates`, `#layers` pipelines, compose pass, `layers` switch, post uniforms |
| `src/render/finish.wgsl` | modify | VHS wobble/bleed, tint, posterize, scanlines (all off at 0) |
| `src/scenes.rs` | modify | `layers_path`, `layers_source`, `MAX_PARAMS` limit, `pub fn hex` |
| `src/director.rs` | modify | `frame_state`: post + 16 params; `plan()`: shot boundaries |
| `src/score.rs` | modify | Judge scores the feedback only (`engine.layers = false`) |
| `scenes/blocks/back_*.{wgsl,toml}` | create | `back_sky`, `back_sea`, `back_plate`, `back_sprite` |
| `scenes/blocks/over_*.{wgsl,toml}` | create | `over_hud`, `over_grid`, `over_title` |
| `src/plates.rs` | create | Decode + centre-crop + fit images to 1536x1024 |
| `src/main.rs` | modify | `--layers`, `--image`, `--look`; dressing; shots; plan fields; gen cleanup; M2 storyboard wiring |
| `src/render/mod.rs` | modify | Upload plates; job hash covers layers + plates |
| `src/preview.rs` | modify | Upload plates |
| `src/wizard.rs` | modify | "Motion graphics?" + "Finishing look?" questions |
| `Cargo.toml` | modify | `image` (M1), `base64` (M2) |
| `src/llm.rs` | modify | `image_model`, `image()`, `send()` refactor, setup prompt |
| `src/vibe.rs` | modify | Layers/post/text in looks, plates, storyboard, `Design` |
| `docs/USAGE.md`, `docs/AI.md`, `README.md` | modify | Docs per milestone |

---

## Task 0: Contract  ·  orchestrator, before Wave 1

Everything Wave 1 builds against. The orchestrator writes it (small, cross-cutting), runs `cargo build`, and commits `feat(layers): contract`.

- [ ] **Step 1: `src/types.rs`: new types** (append after `Variation`)

```rust
/// Plate (picture) size: every image is centre-cropped and fitted to this (3:2), RGBA8 sRGB.
pub const PLATE_W: u32 = 1536;
pub const PLATE_H: u32 = 1024;
/// Scene param slots (SceneUniforms p0..p3). Hand-made scenes use ≤ 8 in practice; layered genomes 14.
pub const MAX_PARAMS: usize = 16;

/// Finishing look applied by the finish pass. All zero = the clean default (output unchanged).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct Post {
    /// Tape wobble, chroma bleed and a rolling tracking band, 0..1.
    #[serde(default)]
    pub vhs: f32,
    /// Scanline darkening, 0..1.
    #[serde(default)]
    pub scanlines: f32,
    /// Colour banding, 0 off .. 1 = 4 levels per channel.
    #[serde(default)]
    pub posterize: f32,
    /// Monochrome tint strength towards `tint`, 0..1.
    #[serde(default)]
    pub tint_amount: f32,
    /// Linear RGB.
    #[serde(default)]
    pub tint: [f32; 3],
}

impl Post {
    pub fn is_clean(&self) -> bool {
        *self == Post::default()
    }

    pub fn lerp(&self, o: &Post, f: f32) -> Post {
        let l = |a: f32, b: f32| a + (b - a) * f;
        Post {
            vhs: l(self.vhs, o.vhs),
            scanlines: l(self.scanlines, o.scanlines),
            posterize: l(self.posterize, o.posterize),
            tint_amount: l(self.tint_amount, o.tint_amount),
            tint: [0, 1, 2].map(|k| l(self.tint[k], o.tint[k])),
        }
    }
}

/// `--look` presets.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Look {
    Clean,
    Vhs,
    Crt,
    Mono,
}

impl Look {
    pub fn post(self) -> Post {
        match self {
            Look::Clean => Post::default(),
            Look::Vhs => Post { vhs: 0.7, scanlines: 0.25, ..Default::default() },
            Look::Crt => Post { scanlines: 0.7, vhs: 0.15, ..Default::default() },
            Look::Mono => Post { tint_amount: 0.85, tint: [0.15, 1.0, 0.25], posterize: 0.5, scanlines: 0.3, ..Default::default() },
        }
    }
}

/// A requested scene change inside the music (storyboard or --layers); the director snaps it to a phrase.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub time: f64,
    pub transition: Transition,
}
```

`Transition` must derive `PartialEq` for `Shot`. Add it to `Transition`'s derive list.

- [ ] **Step 2: `src/types.rs`: extend existing structs**

In `SceneUniforms`, append after `misc` (order matters; it mirrors the prelude's `struct U`):

```rust
    /// Scene params 8..11 (backdrop block), 12..15 (overlay block); unused = 0.
    pub p2: [f32; 4],
    pub p3: [f32; 4],
```

In `FrameState`, append:

```rust
    /// Finishing look (finish pass).
    pub post: Post,
```

In `Plan`, append after `segments`:

```rust
    /// Plates (pictures) the plan's looks can show: --image files, then AI-made ones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plates: Vec<PathBuf>,
    /// --look override for the whole video (set on every run; None = each look's own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<Post>,
    /// Made with --layers.
    #[serde(default)]
    pub layers: bool,
```

(`use std::path::PathBuf;` at the top if it isn't imported yet.)

- [ ] **Step 3: fix every literal the new fields break**

- `src/director.rs` `plan()` return: `Ok(Plan { ..., segments, plates: vec![], post: None, layers: false })`.
- `src/director.rs` `frame_state` `FrameState { ... }`: add `post: Post::default(),` (P1 replaces it).
- `src/score.rs` `Plan { ... }`: add `plates: vec![], post: None, layers: false,`.
- `src/render/engine.rs` test `FrameState { ... }`: add `post: Post::default()`.
- `director::PlanRequest`: add the field below, and `shots: &[],` to every `PlanRequest { ... }` literal (`main.rs`, plus any in `director.rs` tests; find them with `grep -n "PlanRequest {" src`).

```rust
    /// Shot changes inside the music (storyboard / --layers); ignored in markers mode.
    pub shots: &'a [Shot],
```

- [ ] **Step 4: `src/genome.rs`: genome fields** (P2 implements the behaviour)

Replace the `Genome` struct:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Genome {
    pub motion: String,
    pub emitter: String,
    pub colour: String,
    /// sRGB hex, 4 colours.
    pub palette: [String; 4],
    /// Tuned param values, key "<block_id>.<param_name>", in the param's own units.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f32>,
    /// Layer blocks ("" = none), drawn behind / over the feedback in a separate pass (`layers_wgsl`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub backdrop: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub overlay: String,
    /// Plan plate index `img()` shows; None = no picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plate: Option<u32>,
    /// Feedback strength over the backdrop (layer looks), 0..1.5; None = 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback: Option<f32>,
    /// Text for overlay blocks that draw it (font.rs charset; anything else becomes a space).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// Finishing look.
    #[serde(default, skip_serializing_if = "Post::is_clean")]
    pub post: crate::types::Post,
}
```

All new fields are skipped when empty, so existing genome ids (a hash of the JSON) don't change. The `empty_params_keep_the_old_id` test guards this. Add `..Default::default()` to the `Genome { ... }` literals in `genome::random`, the `genome()` test helper, `vibe::look` and the engine test.

- [ ] **Step 5: `src/scenes.rs`**: make `fn hex` public (`pub fn hex`). The vibe parses tint colours with it.

- [ ] **Step 6: `scenes/_prelude.wgsl`**

In `struct U` append after `misc`:

```wgsl
    p2: vec4<f32>,       // scene params 8..11 (generated looks: backdrop block)
    p3: vec4<f32>,       // scene params 12..15 (generated looks: overlay block)
```

After the `prev_samp` binding add:

```wgsl
@group(0) @binding(3) var plate_tex: texture_2d_array<f32>; // the plan's pictures (linear rgba)
@group(0) @binding(4) var plate_samp: sampler;               // linear, clamp
```

Replace `P(i)`:

```wgsl
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
```

After `prev()` add:

```wgsl
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
```

- [ ] **Step 7: `scenes/blocks/CONTRACT.md`**: append

````markdown
## Layer blocks (optional kinds `backdrop` and `overlay`)
A look may add one backdrop and/or one overlay. They run in a separate pass once per output frame, after
the feedback: `rgb = backdrop(p) + feedback * k`, then the overlay is alpha-blended on top. Nothing in this
pass is fed back, so backdrops and overlays stay crisp while the feedback trails over them.
- backdrop: `fn backdrop(p: vec2<f32>) -> vec3<f32>`: p = centred coords; linear HDR rgb
            (sky/world ~0..1, highlights up to ~4 to bloom).
- overlay:  `fn overlay(p: vec2<f32>) -> vec4<f32>`: rgb (HDR) + alpha coverage 0..1.
- Ids: `back_<name>` / `over_<name>`. Params: backdrop `BP(0..3)` (≤ 4), overlay `OP(0..1)` (≤ 2).
- Never call `prev()` in a layer block (in this pass it is the feedback image; the template blends it).
- No derivatives (`dpdx`, `fwidth`) in layer blocks: get the pixel size from `resolution()`.
- Raymarching: cap the steps (`select(48, 96, quality() > 0.5)`) and the distance (~40 units).
- Write `1.0 - smoothstep(a, b, x)` rather than `smoothstep(b, a, x)`.

## Helpers for any block
- `img(p)`: the look's picture in centred coords (rgba, transparent outside it or when the look has
  none). Pictures are 3:2: x -1.5..1.5, y -1..1, so scale p (e.g. `p / 1.25`) to cover the 16:9 frame.
- `text_px(q)` + `TEXT_LEN` (overlay blocks): the look's text in a 5x7 pixel font (A-Z 0-9 . , - ' ! ? : / & +).
  q in glyph pixels: x from 0 at the left edge, y from 0 at the top row; 6 px per character. Returns 0 or 1.
````

- [ ] **Step 8: build and commit**

Run: `cargo build --release`. Expected: compiles (unused-field warnings are fine).
`git add -A && git commit -m "feat(layers): contract for layers, plates and looks"`

---

## Phase 1: Engine: plates, compose pass, looks  ·  [opus]

Owns: `src/render/engine.rs`, `src/render/finish.wgsl`, `src/scenes.rs`, `src/director.rs` (`frame_state`, `param_mut` only), `src/score.rs` (one line).

### Task 1.1: `scenes.rs`: layer files

- [ ] Replace `shader_source` with the shared reader below, and raise the param limit:

```rust
/// `<id>.layers.wgsl` next to the scene: its layer pass (backdrop + feedback + overlay), run once per output frame.
pub fn layers_path(scene: &Scene) -> PathBuf {
    scene.wgsl_path.with_extension("layers.wgsl")
}

fn source(dir: &Path, path: &Path) -> Result<String> {
    let prelude = std::fs::read_to_string(prelude_path(dir)).context("reading _prelude.wgsl")?;
    let body = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(format!("{}{prelude}\n{body}", include_str!("render/common.wgsl")))
}

/// Full shader source for a scene: render/common.wgsl + prelude + scene file (re-read from disk each call, for hot reload).
pub fn shader_source(dir: &Path, scene: &Scene) -> Result<String> {
    source(dir, &scene.wgsl_path)
}

/// Full source of the scene's layer pass; None when it has none.
pub fn layers_source(dir: &Path, scene: &Scene) -> Option<Result<String>> {
    let path = layers_path(scene);
    path.exists().then(|| source(dir, &path))
}
```

In `load_all`: `if t.params.len() > crate::types::MAX_PARAMS { bail!("{id}: at most {} params", crate::types::MAX_PARAMS); }` (exported layered genomes have 14).

### Task 1.2: `engine.rs`: plates, layer pipelines, compose

- [ ] **Struct.** Add fields to `Engine`:

```rust
    scene_bgl: wgpu::BindGroupLayout,
    mirror: wgpu::Sampler,
    clamp: wgpu::Sampler,
    /// The plan's pictures (texture array); one transparent texel when there are none.
    plates: wgpu::TextureView,
    /// Uniforms of the layer pass, [a/b].
    lbuf: [wgpu::Buffer; 2],
    /// Layer pass output (sim size, HDR); bloom + finish read it instead of `accum` when used.
    comp: wgpu::TextureView,
    comp_bg: wgpu::BindGroup,
    /// [uniforms a/b], source = accum.
    layer_bg: [wgpu::BindGroup; 2],
    /// Run layer passes (the judge turns this off: it scores the feedback, not the backdrop).
    pub layers: bool,
```

and change `finish_bg: wgpu::BindGroup` to `finish_bg: [wgpu::BindGroup; 2]` (`[accum, comp]`). Add to `FinishUniforms`:

```rust
    p0: [f32; 4], // vhs, scanlines, posterize, tint amount
    p1: [f32; 4], // tint rgb (linear)
```

- [ ] **Free helpers** (module level, next to `pipeline`):

```rust
const LAYERS: &str = "#layers";

fn layers_key(id: &str) -> String {
    format!("{id}{LAYERS}")
}

/// Plate array from PLATE_W x PLATE_H RGBA8 (sRGB) layers; none = one zeroed (transparent) texel.
fn plate_array(d: &wgpu::Device, q: &wgpu::Queue, layers: &[Vec<u8>]) -> wgpu::TextureView {
    let (w, h, n) = if layers.is_empty() { (1, 1, 1) } else { (PLATE_W, PLATE_H, layers.len() as u32) };
    let t = d.create_texture(&wgpu::TextureDescriptor {
        label: Some("plates"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: n },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (i, px) in layers.iter().enumerate() {
        assert_eq!(px.len(), (4 * w * h) as usize, "plate {i} is not PLATE_W x PLATE_H RGBA8");
        q.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &t, mip_level: 0, origin: wgpu::Origin3d { x: 0, y: 0, z: i as u32 }, aspect: wgpu::TextureAspect::All },
            px,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }
    t.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() })
}

/// Scene / layer-pass bind group: uniforms, source texture (prev frame or accum), mirror sampler, plates, clamp sampler.
fn scene_bind(
    d: &wgpu::Device,
    l: &wgpu::BindGroupLayout,
    ub: &wgpu::Buffer,
    src: &wgpu::TextureView,
    mirror: &wgpu::Sampler,
    plates: &wgpu::TextureView,
    clamp: &wgpu::Sampler,
) -> wgpu::BindGroup {
    use wgpu::BindingResource as R;
    let res = [ub.as_entire_binding(), R::TextureView(src), R::Sampler(mirror), R::TextureView(plates), R::Sampler(clamp)];
    let entries: Vec<_> = res.into_iter().enumerate().map(|(i, r)| wgpu::BindGroupEntry { binding: i as u32, resource: r }).collect();
    d.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: l, entries: &entries })
}
```

- [ ] **`new()`.**
  - The texture closure `tex` stays. Add `let comp = tex(sim.0, sim.1);`.
  - Add a texture-array layout entry next to `tex_e`:

```rust
        let arr_e = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: frag,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        };
```

  - `let scene_bgl = bgl(&[uniform_e(0), tex_e(1), samp_e(2), arr_e(3), samp_e(4)]);`
  - `let lbuf = [buf(size_of::<SceneUniforms>()), buf(size_of::<SceneUniforms>())];`
  - `let plates = plate_array(d, queue, &[]);`
  - Replace the `scene_bg` construction:

```rust
        let scene_bg = [0, 1].map(|u| [0, 1].map(|f| scene_bind(d, &scene_bgl, &ubuf[u], &fb[f], &mirror, &plates, &clamp)));
        let layer_bg = [0, 1].map(|u| scene_bind(d, &scene_bgl, &lbuf[u], &accum, &mirror, &plates, &clamp));
        let comp_bg = tex_bg(&comp);
```

  - Replace `finish_bg`:

```rust
        let finish_bg = [&accum, &comp].map(|src| {
            bg(&finish_bgl, &[fbuf.as_entire_binding(), R::TextureView(src), R::TextureView(&bloom[0]), R::Sampler(&clamp)])
        });
```

  - Add the new fields to `Self { ... }`: `scene_bgl, mirror, clamp, plates, lbuf, comp, comp_bg, layer_bg, layers: true,`. The `bg` closure borrows `d`, not the samplers by move, so `mirror`/`clamp` can be moved in afterwards. If the borrow checker complains, build the struct after the last closure use.

- [ ] **Methods.**

```rust
    /// Uploads the plan's pictures (each PLATE_W x PLATE_H RGBA8 sRGB, see plates.rs).
    pub fn set_plates(&mut self, layers: &[Vec<u8>]) {
        self.plates = plate_array(&self.device, &self.queue, layers);
        let d = &self.device;
        self.scene_bg = [0, 1].map(|u| [0, 1].map(|f| scene_bind(d, &self.scene_bgl, &self.ubuf[u], &self.fb[f], &self.mirror, &self.plates, &self.clamp)));
        self.layer_bg = [0, 1].map(|u| scene_bind(d, &self.scene_bgl, &self.lbuf[u], &self.accum, &self.mirror, &self.plates, &self.clamp));
    }
```

`loaded()` must not report layer keys (the preview recompiles by scene id):

```rust
    pub fn loaded(&self) -> Vec<String> {
        self.pipelines.keys().chain(&self.failed).filter(|k| !k.ends_with(LAYERS)).cloned().collect()
    }
```

`remove_scene` also does `self.pipelines.remove(&layers_key(id));`.

Replace `compile` / `try_compile`:

```rust
    /// (Re)compiles a scene (and its layer pass, if it has one) from disk. On failure existing pipelines are kept.
    pub fn compile(&mut self, id: &str) -> Result<()> {
        match self.try_compile(id) {
            Ok((pl, layers)) => {
                self.pipelines.insert(id.to_string(), pl);
                match layers {
                    Some(l) => self.pipelines.insert(layers_key(id), l),
                    None => self.pipelines.remove(&layers_key(id)),
                };
                self.failed.remove(id);
                Ok(())
            }
            Err(e) => {
                self.failed.insert(id.to_string());
                Err(e)
            }
        }
    }

    fn try_compile(&self, id: &str) -> Result<(wgpu::RenderPipeline, Option<wgpu::RenderPipeline>)> {
        let scene = self.scenes.iter().find(|s| s.id == id).with_context(|| format!("unknown scene '{id}'"))?;
        let main = self.build(id, scenes::shader_source(&self.scenes_dir, scene)?)?;
        let layers = scenes::layers_source(&self.scenes_dir, scene).map(|src| self.build(&layers_key(id), src?)).transpose()?;
        Ok((main, layers))
    }

    fn build(&self, label: &str, src: String) -> Result<wgpu::RenderPipeline> {
        let internal = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(label), source: wgpu::ShaderSource::Wgsl(src.into()) });
        let pl = pipeline(&self.device, &module, &self.scene_layout, "fs_main", HDR, None);
        let errs = [pollster::block_on(validation.pop()), pollster::block_on(internal.pop())];
        if let Some(e) = errs.into_iter().flatten().next() {
            bail!("scene '{label}': {e}");
        }
        Ok(pl)
    }
```

Compose pass + `finish` changes:

```rust
    /// Layer pass (backdrop + feedback + overlay) into `comp`; false when neither scene has one.
    /// A morph blends B's layer pass (or plain feedback) over A's, like `step` does.
    fn compose(&self, enc: &mut wgpu::CommandEncoder, fs: &FrameState) -> bool {
        let get = |id: &str| self.pipelines.get(&layers_key(id)).filter(|_| self.layers);
        let la = get(&fs.scene_a);
        let lb = fs.b.as_ref().and_then(|(id, _)| get(id));
        if la.is_none() && lb.is_none() {
            return false;
        }
        self.queue.write_buffer(&self.lbuf[0], 0, bytemuck::bytes_of(&fs.a));
        match la {
            Some(p) => draw(enc, &self.comp, true, p, &self.layer_bg[0], 1.0),
            None => draw(enc, &self.comp, true, &self.copy_pl, &self.accum_bg, 1.0),
        }
        if let Some((_, ub)) = &fs.b {
            self.queue.write_buffer(&self.lbuf[1], 0, bytemuck::bytes_of(ub));
            match lb {
                Some(p) => draw(enc, &self.scratch, true, p, &self.layer_bg[1], 1.0),
                None => draw(enc, &self.scratch, true, &self.copy_pl, &self.accum_bg, 1.0),
            }
            draw(enc, &self.comp, false, &self.copy_pl, &self.scratch_bg, fs.mix as f64);
        }
        true
    }
```

In `finish`, first line: `let src = self.compose(enc, fs) as usize;`. The bloom's first downsample reads `[&self.accum_bg, &self.comp_bg][src]` instead of `&self.accum_bg`. The final draw uses `&self.finish_bg[src]`. The uniforms gain:

```rust
            p0: [fs.post.vhs, fs.post.scanlines, fs.post.posterize, fs.post.tint_amount],
            p1: [fs.post.tint[0], fs.post.tint[1], fs.post.tint[2], 0.0],
```

### Task 1.3: `finish.wgsl`: looks

- [ ] Extend `struct F`:

```wgsl
    p0: vec4<f32>, // vhs, scanlines, posterize, tint amount (all 0 = clean)
    p1: vec4<f32>, // tint rgb (linear)
```

- [ ] Add before `fs_main`:

```wgsl
// VHS: slow line-wise horizontal wobble plus a tracking band rolling up the frame with per-line noise.
fn vhs_uv(uv: vec2<f32>, amt: f32, t: f32, frame: u32) -> vec2<f32> {
    let wobble = sin(uv.y * 23.0 + t * 1.7) * 0.6 + sin(uv.y * 71.0 - t * 3.1) * 0.4;
    let band = 1.0 - smoothstep(0.0, 0.04, abs(fract(uv.y + t * 0.05) - 0.5));
    let line = rnd(vec2<f32>(0.0, floor(uv.y * f.r.y)), frame, 4u) - 0.5;
    return vec2<f32>(uv.x + amt * (0.0012 * wobble + 0.012 * band * line), uv.y);
}
```

- [ ] In `fs_main`, replace the opening lines up to and including `let ca = ...` with:

```wgsl
    let frame = u32(f.r.z);
    let uv = select(in.uv, vhs_uv(in.uv, f.p0.x, f.b.x, frame), f.p0.x > 0.0);
    let p = uv - 0.5;
    // Gentle chromatic aberration, growing towards the edges; VHS adds a horizontal chroma bleed.
    let ca = p * dot(p, p) * 0.012 + vec2<f32>(f.p0.x * 0.004, 0.0);
```

Then delete the later `let frame = u32(f.r.z);`. Right after `c = srgb(tonemap(c));` insert:

```wgsl
    // Looks (finish-pass only; every term is a no-op at 0, so the clean look is unchanged).
    let lum = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = mix(c, lum * srgb(f.p1.rgb) * 1.4, f.p0.w);
    if (f.p0.z > 0.0) {
        let levels = mix(64.0, 4.0, f.p0.z);
        c = floor(c * levels + 0.5) / levels;
    }
    // Scanlines: one dark line every 2 px at 1080p (scaled with the output height).
    c *= 1.0 - f.p0.y * 0.35 * (0.5 + 0.5 * cos(in.pos.y * 540.0 / f.r.y * 6.2831853));
```

### Task 1.4: `director::frame_state`: post + 16 params

- [ ] Replace `param_mut`:

```rust
fn param_mut(u: &mut SceneUniforms, j: usize) -> &mut f32 {
    let row = match j / 4 {
        0 => &mut u.p0,
        1 => &mut u.p1,
        2 => &mut u.p2,
        _ => &mut u.p3,
    };
    &mut row[j % 4]
}
```

- [ ] In `build`: `.take(sc.params.len().min(8))` → `.take(sc.params.len().min(MAX_PARAMS))`.
- [ ] Before the final `FrameState { ... }`:

```rust
    // Finishing look: --look wins, else the segment's (blended through a morph).
    let post_of = |s: &Segment| plan.post.or_else(|| s.genome.as_ref().map(|g| g.post)).unwrap_or_default();
    let post = match in_morph(i).or_else(|| in_morph(i + 1)) {
        Some((j, m)) => post_of(&segs[j - 1]).lerp(&post_of(&segs[j]), m),
        None => segs.get(i).map(post_of).unwrap_or_default(),
    };
```

and set `post,` in the `FrameState` (replacing Task 0's placeholder).

### Task 1.5: `score.rs`: judge the feedback

- [ ] In `Judge::new`, right after creating `engine`: `engine.layers = false;` (`let mut engine`). Comment: `// Backdrops are art, not music: score the feedback only (layer passes still compile).`

### Task 1.6: engine test: every layer block compiles and finishes

- [ ] Append to `engine_runs` (after the block-combination loop; `blocks`, `gen_dir`, `dir`, `device`, `queue` are in scope):

```rust
        // Every backdrop x overlay pair (and each alone) compiles as a layer pass; finish runs it with every look on.
        let ids = |l: &[crate::genome::Block]| std::iter::once(String::new()).chain(l.iter().map(|b| b.id.clone())).collect::<Vec<_>>();
        let mut lay = Vec::new();
        for b in ids(&blocks.backdrop) {
            for o in ids(&blocks.overlay) {
                if b.is_empty() && o.is_empty() {
                    continue;
                }
                let g = crate::genome::Genome {
                    motion: blocks.motion[0].id.clone(),
                    emitter: blocks.emitter[0].id.clone(),
                    colour: blocks.colour[0].id.clone(),
                    palette: blocks.palettes[0].clone(),
                    backdrop: b.clone(),
                    overlay: o,
                    plate: Some(0),
                    text: "HELLO 123".into(),
                    ..Default::default()
                };
                lay.push(crate::genome::write(&g, &blocks, &gen_dir).unwrap());
            }
        }
        assert!(!lay.is_empty(), "no layer blocks found");
        let mut e = Engine::new(&device, &queue, &lay, dir, Quality::Draft, 64, 36, wgpu::TextureFormat::Rgba8Unorm);
        e.set_plates(&[vec![255; (PLATE_W * PLATE_H * 4) as usize]]);
        for s in &lay {
            e.compile(&s.id).unwrap_or_else(|err| panic!("{}: {err:#}", s.id));
        }
        let mut u = SceneUniforms::default();
        u.res_time = [e.sim.0 as f32, e.sim.1 as f32, 1.0, 1.0 / SIM_HZ as f32];
        let post = Post { vhs: 1.0, scanlines: 1.0, posterize: 1.0, tint_amount: 1.0, tint: [0.0, 1.0, 0.0] };
        let fs = FrameState { scene_a: lay[0].id.clone(), a: u, b: Some((lay[lay.len() - 1].id.clone(), u)), mix: 0.5, flash: 0.0, exposure: 1.0, saturation: 1.0, post };
        e.step(&fs, Some(1.0)).unwrap();
        let out = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: 64, height: 36, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let mut enc = device.create_command_encoder(&Default::default());
        e.finish(&mut enc, &out.create_view(&Default::default()), &fs, true, false);
        queue.submit([enc.finish()]);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
```

- [ ] `cargo build --release` (the test runs at the gate, once P2/P3 exist). Commit `feat(layers): engine plates, compose pass and finishing looks`.

---

## Phase 2: Genome layers + pixel font  ·  [opus]

Owns: `src/genome.rs`, `src/font.rs`, and the `mod font;` line in `src/main.rs`.

### Task 2.1: `font.rs`

- [ ] Create `src/font.rs`:

```rust
//! 5x7 pixel font for overlay text, emitted as WGSL (`text_px`, `TEXT_LEN`) into a look's layer pass.
//! Lowercase is drawn as uppercase; characters outside the set become spaces.

/// (char, 7 rows of 5; '#' = on). Index 0 must be the space.
const GLYPHS: &[(char, [&str; 7])] = &[
    (' ', [".....", ".....", ".....", ".....", ".....", ".....", "....."]),
    ('A', [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
    ('B', ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."]),
    ('C', [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."]),
    ('D', ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."]),
    ('E', ["#####", "#....", "#....", "####.", "#....", "#....", "#####"]),
    ('F', ["#####", "#....", "#....", "####.", "#....", "#....", "#...."]),
    ('G', [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####"]),
    ('H', ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
    ('I', [".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###."]),
    ('J', ["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."]),
    ('K', ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"]),
    ('L', ["#....", "#....", "#....", "#....", "#....", "#....", "#####"]),
    ('M', ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"]),
    ('N', ["#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#"]),
    ('O', [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
    ('P', ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."]),
    ('Q', [".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"]),
    ('R', ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"]),
    ('S', [".####", "#....", "#....", ".###.", "....#", "....#", "####."]),
    ('T', ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."]),
    ('U', ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
    ('V', ["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."]),
    ('W', ["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#."]),
    ('X', ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"]),
    ('Y', ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."]),
    ('Z', ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"]),
    ('0', [".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."]),
    ('1', ["..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."]),
    ('2', [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"]),
    ('3', ["#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###."]),
    ('4', ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."]),
    ('5', ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."]),
    ('6', ["..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."]),
    ('7', ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."]),
    ('8', [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."]),
    ('9', [".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."]),
    ('.', [".....", ".....", ".....", ".....", ".....", ".##..", ".##.."]),
    (',', [".....", ".....", ".....", ".....", ".##..", "..#..", ".#..."]),
    ('-', [".....", ".....", ".....", "#####", ".....", ".....", "....."]),
    ('\'', ["..#..", "..#..", ".#...", ".....", ".....", ".....", "....."]),
    ('!', ["..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#.."]),
    ('?', [".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."]),
    (':', [".....", ".##..", ".##..", ".....", ".##..", ".##..", "....."]),
    ('/', [".....", "....#", "...#.", "..#..", ".#...", "#....", "....."]),
    ('&', [".##..", "#..#.", "#.#..", ".#...", "#.#.#", "#..#.", ".##.#"]),
    ('+', [".....", "..#..", "..#..", "#####", "..#..", "..#..", "....."]),
];

/// At most this many characters are drawn.
const MAX_LEN: usize = 40;

const TEXT_PX: &str = r#"
// Coverage (0/1) of the look's text at q, in glyph pixels: x from 0 (left), y from 0 (top row).
// Glyphs are 5x7 with one pixel of spacing (6 per character).
fn text_px(q: vec2<f32>) -> f32 {
    if (q.x < 0.0 || q.y < 0.0 || q.y >= 7.0) { return 0.0; }
    let ci = i32(floor(q.x / 6.0));
    let col = i32(floor(q.x)) - ci * 6;
    if (ci >= TEXT_LEN || col >= 5) { return 0.0; }
    let row = i32(floor(q.y));
    let g = TEXT[ci];
    var bits = FONT[g * 2u] >> u32(row * 5);
    if (row >= 4) { bits = FONT[g * 2u + 1u] >> u32((row - 4) * 5); }
    return f32((bits >> u32(4 - col)) & 1u);
}
"#;

/// Glyph bits: rows 0..3 in the first word, rows 4..6 in the second, 5 bits per row (bit 4 = left column).
fn pack(rows: &[&str; 7]) -> (u32, u32) {
    let row = |r: &str| r.bytes().fold(0u32, |v, b| (v << 1) | (b == b'#') as u32);
    let (mut a, mut b) = (0, 0);
    for (i, r) in rows.iter().enumerate() {
        if i < 4 {
            a |= row(r) << (i * 5);
        } else {
            b |= row(r) << ((i - 4) * 5);
        }
    }
    (a, b)
}

/// Glyph indices for `text` (unknown characters = space), at most MAX_LEN.
fn indices(text: &str) -> Vec<u32> {
    text.chars()
        .take(MAX_LEN)
        .map(|c| GLYPHS.iter().position(|g| g.0 == c.to_ascii_uppercase()).unwrap_or(0) as u32)
        .collect()
}

/// WGSL defining FONT, TEXT, TEXT_LEN and text_px() for `text` ("" = TEXT_LEN 0).
pub fn wgsl(text: &str) -> String {
    let font: Vec<String> = GLYPHS
        .iter()
        .flat_map(|(_, rows)| {
            let (a, b) = pack(rows);
            [format!("{a}u"), format!("{b}u")]
        })
        .collect();
    let idx = indices(text);
    // WGSL arrays can't be empty: TEXT_LEN = 0 hides the placeholder.
    let shown: Vec<String> = if idx.is_empty() { vec!["0u".into()] } else { idx.iter().map(|i| format!("{i}u")).collect() };
    format!(
        "var<private> FONT: array<u32, {n}> = array<u32, {n}>({});\nvar<private> TEXT: array<u32, {m}> = array<u32, {m}>({});\nconst TEXT_LEN: i32 = {};\n{TEXT_PX}",
        font.join(", "),
        shown.join(", "),
        idx.len(),
        n = font.len(),
        m = shown.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_pack_and_text_maps() {
        assert_eq!(GLYPHS[0].0, ' ');
        for (c, rows) in GLYPHS {
            assert!(rows.iter().all(|r| r.len() == 5 && r.bytes().all(|b| b == b'.' || b == b'#')), "glyph {c:?}");
        }
        let at = |c: char| GLYPHS.iter().position(|g| g.0 == c).unwrap() as u32;
        let (a, b) = pack(&GLYPHS[at('I') as usize].1);
        assert_eq!(a & 31, 0b01110, "top row .###.");
        assert_eq!((b >> 10) & 31, 0b01110, "bottom row .###.");
        assert_eq!(indices("hi~"), vec![at('H'), at('I'), 0]);
        assert!(wgsl("").contains("const TEXT_LEN: i32 = 0;"));
    }
}
```

- [ ] Add `mod font;` to `src/main.rs` next to `mod genome;`.

### Task 2.2: `genome.rs`: layer kinds

- [ ] Kinds and slots:

```rust
/// Block kinds; `SLOTS` and the `Blocks` lists follow this order. The last two (layers) are optional.
pub const KINDS: [&str; 5] = ["motion", "emitter", "colour", "backdrop", "overlay"];

/// Param slots per kind: 8 for the feedback (scene params 0..7), then 4 + 2 for the layers (8..13).
const SLOTS: [usize; 5] = [3, 3, 2, 4, 2];
```

- [ ] `Genome`: `block_mut` gains `3 => &mut self.backdrop, _ => &mut self.overlay` (`2 => &mut self.colour`). Add:

```rust
    /// Has a backdrop or overlay (so it also has a layer pass).
    pub fn has_layers(&self) -> bool {
        !self.backdrop.is_empty() || !self.overlay.is_empty()
    }
```

and replace `prune`:

```rust
    /// Keeps only finite tuned values whose key names a param of this genome's blocks.
    pub fn prune(&mut self, blocks: &Blocks) {
        let (Ok(parts), Ok(layers)) = (blocks.parts(self), blocks.layer_parts(self)) else { return self.params.clear() };
        let keys: Vec<String> = parts
            .into_iter()
            .chain(layers.into_iter().flatten())
            .flat_map(|b| b.params.iter().filter(|p| !p.name.is_empty()).map(|p| param_key(&b.id, &p.name)))
            .collect();
        self.params.retain(|k, v| keys.contains(k) && v.is_finite());
    }
```

- [ ] `Blocks`: add `pub backdrop: Vec<Block>, pub overlay: Vec<Block>,` (doc: `/// Layer blocks (optional kinds).`). In `load`, the literal gains `backdrop: vec![], overlay: vec![]` (the non-empty check stays on the first three kinds). `list` and `list_mut` both become a five-way match:

```rust
    pub fn list(&self, kind: &str) -> Result<&Vec<Block>> {
        Ok(match kind_index(kind)? {
            0 => &self.motion,
            1 => &self.emitter,
            2 => &self.colour,
            3 => &self.backdrop,
            _ => &self.overlay,
        })
    }
```

(`list_mut` is the same with `&mut`.) `subset` adds `backdrop: pick(&self.backdrop, |g| &g.backdrop), overlay: pick(&self.overlay, |g| &g.overlay),`. `param` chains all five lists:

```rust
        let all = [&self.motion, &self.emitter, &self.colour, &self.backdrop, &self.overlay].into_iter().flatten();
```

Add:

```rust
    fn opt<'a>(list: &'a [Block], id: &str) -> Result<Option<&'a Block>> {
        if id.is_empty() { Ok(None) } else { Self::get(list, id).map(Some) }
    }

    /// The layer blocks ("" = none), or an error naming an unknown id (AI output goes through this).
    pub fn layer_parts(&self, g: &Genome) -> Result<[Option<&Block>; 2]> {
        Ok([Self::opt(&self.backdrop, &g.backdrop)?, Self::opt(&self.overlay, &g.overlay)?])
    }
```

### Task 2.3: `genome.rs`: templates, scene, write, dress

- [ ] Feedback template: add `{IMG}` on its own line after the `CP` line of `TEMPLATE`. Add the layer template and stubs:

```rust
/// A look's layer pass: an ordinary scene whose prev() is the finished feedback (engine compose pass).
const LAYERS: &str = r#"
fn BP(i: i32) -> f32 { return P(8 + i); }
fn OP(i: i32) -> f32 { return P(12 + i); }
{IMG}
{FONT}
{BACKDROP}
{OVERLAY}
fn scene(uv: vec2<f32>) -> vec4<f32> {
    let p = to_centered(uv);
    var c = backdrop(p) + prev(uv) * {FEEDBACK};
    let o = overlay(p);
    c = mix(c, o.rgb, clamp(o.a, 0.0, 1.0));
    return vec4<f32>(c, 1.0);
}
"#;
const NO_BACKDROP: &str = "fn backdrop(p: vec2<f32>) -> vec3<f32> { return vec3<f32>(0.0); }";
const NO_OVERLAY: &str = "fn overlay(p: vec2<f32>) -> vec4<f32> { return vec4<f32>(0.0); }";

fn img_wgsl(g: &Genome) -> String {
    let i = g.plate.map_or(-1, |i| i as i64);
    format!("// The look's picture (prelude plate()); transparent when it has none.\nfn img(p: vec2<f32>) -> vec4<f32> {{ return plate({i}, p); }}")
}
```

- [ ] `wgsl()` adds `.replace("{IMG}", &img_wgsl(g))`. Add:

```rust
/// Layer pass source (None when the genome has no layer blocks).
pub fn layers_wgsl(g: &Genome, blocks: &Blocks) -> Result<Option<String>> {
    if !g.has_layers() {
        return Ok(None);
    }
    let [bd, ov] = blocks.layer_parts(g)?;
    let fb = g.feedback.unwrap_or(1.0).clamp(0.0, 1.5);
    Ok(Some(
        LAYERS
            .replace("{IMG}", &img_wgsl(g))
            .replace("{FONT}", &crate::font::wgsl(&g.text))
            .replace("{BACKDROP}", bd.map_or(NO_BACKDROP, |b| &b.wgsl))
            .replace("{OVERLAY}", ov.map_or(NO_OVERLAY, |b| &b.wgsl))
            .replace("{FEEDBACK}", &format!("{fb:?}")),
    ))
}
```

- [ ] Replace the param loop in `to_scene`:

```rust
    let [m, e, c] = blocks.parts(g)?;
    let [bd, ov] = blocks.layer_parts(g)?;
    let mut slots = vec![(Some(m), SLOTS[0]), (Some(e), SLOTS[1]), (Some(c), SLOTS[2])];
    if g.has_layers() {
        slots.extend([(bd, SLOTS[3]), (ov, SLOTS[4])]);
    }
    let mut params = Vec::with_capacity(crate::types::MAX_PARAMS);
    for (b, n) in slots {
        let own = b.map_or(0, |b| b.params.len());
        for p in b.iter().flat_map(|b| &b.params) {
            let key = b.filter(|_| !p.name.is_empty()).map(|b| param_key(&b.id, &p.name));
            params.push(match key.and_then(|k| g.params.get(&k)) {
                Some(&v) => narrow(p, v),
                None => p.clone(),
            });
        }
        params.extend(std::iter::repeat_n(pad.clone(), n - own));
    }
```

- [ ] `write()` also writes or removes the layer file:

```rust
pub fn write(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let sc = to_scene(g, blocks, gen_dir)?;
    std::fs::create_dir_all(gen_dir)?;
    std::fs::write(&sc.wgsl_path, wgsl(g, blocks)?)?;
    let lp = crate::scenes::layers_path(&sc);
    match layers_wgsl(g, blocks)? {
        Some(src) => std::fs::write(lp, src)?,
        None => {
            let _ = std::fs::remove_file(lp);
        }
    }
    Ok(sc)
}
```

- [ ] `dress` (the core `--layers`; no AI):

```rust
/// Core --layers: gives a look a backdrop (one showing a picture only when there are plates), an overlay
/// half the time (text overlays only when there is text), a plate and the text. `r` = 64-bit hash.
pub fn dress(g: &Genome, blocks: &Blocks, r: u64, plates: usize, text: &str) -> Genome {
    // ponytail: needs are sniffed from the block source; a toml flag if a block ever calls them indirectly.
    let backs: Vec<&Block> = blocks.backdrop.iter().filter(|b| plates > 0 || !b.wgsl.contains("img(")).collect();
    let overs: Vec<&Block> = blocks.overlay.iter().filter(|b| !text.is_empty() || !b.wgsl.contains("text_px(")).collect();
    let piece = |k: u32| ((r >> (16 * k)) & 0xffff) as usize;
    let mut out = g.clone();
    if let Some(b) = backs.get(piece(0) % backs.len().max(1)) {
        out.backdrop = b.id.clone();
    }
    if piece(1) % 2 == 0 {
        if let Some(b) = overs.get(piece(2) % overs.len().max(1)) {
            out.overlay = b.id.clone();
        }
    }
    out.plate = (plates > 0).then(|| (piece(3) % plates) as u32);
    out.text = text.into();
    out.feedback = Some(0.7);
    out
}
```

`random` and `mutate` stay as they are: the core's generated scenes only get layers through `dress`. This keeps default renders unchanged.

### Task 2.4: test

- [ ] Add to `genome::tests`:

```rust
    #[test]
    fn layers_add_slots_and_a_layer_pass() {
        let b = blocks();
        let plain = genome();
        assert!(layers_wgsl(&plain, &b).unwrap().is_none());
        assert_eq!(to_scene(&plain, &b, Path::new(".")).unwrap().params.len(), 8);
        let g = Genome { backdrop: "back_sky".into(), overlay: "over_hud".into(), plate: Some(1), feedback: Some(0.6), ..genome() };
        assert_eq!(to_scene(&g, &b, Path::new(".")).unwrap().params.len(), 14);
        let src = layers_wgsl(&g, &b).unwrap().unwrap();
        assert!(src.contains("fn backdrop") && src.contains("fn overlay") && src.contains("plate(1, p)") && src.contains("prev(uv) * 0.6"));
        assert!(b.layer_parts(&Genome { backdrop: "nope".into(), ..genome() }).is_err());
        // Without plates, dress never picks a backdrop that needs a picture.
        for r in 0..16u64 {
            let d = dress(&plain, &b, r * 0x0001_0001_0001_0001, 0, "");
            assert!(d.plate.is_none() && !b.backdrop.iter().find(|x| x.id == d.backdrop).unwrap().wgsl.contains("img("));
        }
    }
```

- [ ] `cargo build --release`, commit `feat(layers): genome layer blocks and pixel font`.

---

## Phase 3: Layer block library  ·  [opus ×2, parallel]

Two agents: **P3a** writes the four backdrops, **P3b** the three overlays. Each block is a `.wgsl` + `.toml` in `scenes/blocks/`, following `CONTRACT.md` (Task 0). The code below is the reference implementation: copy it, then improve it if you see a clear visual win that stays within the contract. Don't run anything. The gate's `engine_runs` test compiles every block. Helpers are prefixed with the block id.

### Task 3a: backdrops

- [ ] `back_sky.toml`

```toml
kind = "backdrop"
description = "Gradient sky over a horizon with a soft sun, haze bands and a mirrored lower half (calm, open, 90s CGI)"
[[param]]
name = "horizon"
min = -0.6
max = 0.1
[[param]]
name = "sun_x"
min = -1.2
max = 1.2
[[param]]
name = "sun_height"
min = 0.1
max = 0.8
```

- [ ] `back_sky.wgsl`

```wgsl
// Gradient sky over a horizon: zenith = palette colour 1, horizon = pale colour 3, a soft sun that
// swells on drops, faint drifting haze; below the horizon a darker mirrored sky (calm sea or haze).
fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let horizon = BP(0);
    let h = p.y - horizon;
    let zen = pal(0.25) * 0.8;
    let low = mix(pal(0.75), vec3<f32>(1.0), 0.35);
    var c = mix(low, zen, smoothstep(0.0, 1.2, h));
    c = select(c, mix(low, pal(0.0), smoothstep(0.0, 0.6, -h)) * 0.8, h < 0.0);
    let sun = vec2<f32>(BP(1) + 0.15 * sin(time() * 0.02), horizon + BP(2));
    let d = length(p - sun);
    let disc = 1.0 - smoothstep(0.08, 0.09, d);
    c += pal(0.6) * (disc * 3.0 + 0.25 / (1.0 + d * d * 40.0)) * (1.0 + 0.8 * drop_env());
    c *= 0.92 + 0.08 * fbm(vec2<f32>(p.x * 0.8 + time() * 0.01, p.y * 4.0));
    return c;
}
```

- [ ] `back_sea.toml`

```toml
kind = "backdrop"
description = "Raymarched 90s-CGI world: calm reflective sea, a pyramid standing in it, a marbled moon sphere, gradient sky, long shadow"
[[param]]
name = "eye_height"
min = 0.6
max = 2.0
[[param]]
name = "sun_x"
min = -1.5
max = 1.5
[[param]]
name = "sway"
min = 0.0
max = 1.0
[[param]]
name = "waves"
min = 0.02
max = 0.12
```

- [ ] `back_sea.wgsl`

```wgsl
// Raymarched 90s-CGI world: a calm reflective sea (analytic plane), a pyramid standing in it and a
// marbled moon-like sphere in the sky, under a gradient sky. The camera sways slowly; kicks stir the water.

// Pyramid with a unit square base on y = 0 and height h (Inigo Quilez's exact SDF).
fn back_sea_pyr(p0: vec3<f32>, h: f32) -> f32 {
    let m2 = h * h + 0.25;
    var p = vec3<f32>(abs(p0.x), p0.y, abs(p0.z));
    if (p.z > p.x) { p = vec3<f32>(p.z, p.y, p.x); }
    p = vec3<f32>(p.x - 0.5, p.y, p.z - 0.5);
    let q = vec3<f32>(p.z, h * p.y - 0.5 * p.x, h * p.x + 0.5 * p.y);
    let s = max(-q.x, 0.0);
    let t = clamp((q.y - 0.5 * p.z) / (m2 + 0.25), 0.0, 1.0);
    let a = m2 * (q.x + s) * (q.x + s) + q.y * q.y;
    let b = m2 * (q.x + 0.5 * t) * (q.x + 0.5 * t) + (q.y - m2 * t) * (q.y - m2 * t);
    let d2 = select(min(a, b), 0.0, min(q.y, -q.x * m2 - q.y * 0.5) > 0.0);
    return sqrt((d2 + q.z * q.z) / m2) * sign(max(q.z, -p.y));
}

// x = distance, y = material (0 pyramid, 1 sphere).
fn back_sea_map(p: vec3<f32>) -> vec2<f32> {
    let s = 2.2;
    let pyr = back_sea_pyr((p - vec3<f32>(1.6, -0.05, 9.0)) / s, 1.6) * s;
    let sph = length(p - vec3<f32>(-3.2, 3.6, 14.0)) - 1.6;
    return select(vec2<f32>(sph, 1.0), vec2<f32>(pyr, 0.0), pyr < sph);
}

// (t, material) of the first hit, t = -1 for a miss.
fn back_sea_march(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> vec2<f32> {
    var t = 0.05;
    let steps = select(48, 96, quality() > 0.5);
    for (var i = 0; i < steps; i++) {
        let h = back_sea_map(ro + rd * t);
        if (h.x < 0.002 * t) { return vec2<f32>(t, h.y); }
        t += h.x;
        if (t > tmax) { break; }
    }
    return vec2<f32>(-1.0, 0.0);
}

fn back_sea_normal(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.002, 0.0);
    return normalize(vec3<f32>(
        back_sea_map(p + e.xyy).x - back_sea_map(p - e.xyy).x,
        back_sea_map(p + e.yxy).x - back_sea_map(p - e.yxy).x,
        back_sea_map(p + e.yyx).x - back_sea_map(p - e.yyx).x,
    ));
}

fn back_sea_sun() -> vec3<f32> {
    return normalize(vec3<f32>(BP(1), 0.35, 1.0));
}

fn back_sea_sky(rd: vec3<f32>) -> vec3<f32> {
    let low = mix(pal(0.75), vec3<f32>(1.0), 0.4);
    var c = mix(low, pal(0.25), smoothstep(0.0, 0.5, rd.y));
    let s = max(dot(rd, back_sea_sun()), 0.0);
    c += pal(0.6) * (pow(s, 400.0) * 3.0 + pow(s, 12.0) * 0.25) * (1.0 + 0.6 * drop_env());
    return c;
}

fn back_sea_shade(p: vec3<f32>, rd: vec3<f32>, m: f32) -> vec3<f32> {
    let n = back_sea_normal(p);
    let l = back_sea_sun();
    let dif = max(dot(n, l), 0.0);
    // Sphere: marbled grey; pyramid: a flat CGI colour from the palette.
    let marble = fbm(vec2<f32>(atan2(n.z, n.x) * 2.0, n.y * 3.0) * 2.0);
    let base = select(pal(0.5) * 0.9, mix(vec3<f32>(0.25), vec3<f32>(0.9), marble), m > 0.5);
    let spec = pow(max(dot(reflect(rd, n), l), 0.0), 40.0);
    return base * (0.15 + 0.85 * dif) + back_sea_sky(reflect(rd, n)) * 0.15 + spec * 0.6;
}

fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let yaw = sin(time() * 0.03) * 0.25 * BP(2);
    let ro = vec3<f32>(0.0, BP(0), 0.0);
    let r0 = normalize(vec3<f32>(p.x, p.y - 0.15, 1.8));
    let rd = vec3<f32>(r0.x * cos(yaw) + r0.z * sin(yaw), r0.y, -r0.x * sin(yaw) + r0.z * cos(yaw));
    let hit = back_sea_march(ro, rd, 40.0);
    // Sea plane y = 0, analytic.
    let ts = select(-1.0, -ro.y / rd.y, rd.y < 0.0);
    if (hit.x > 0.0 && (ts < 0.0 || hit.x < ts)) {
        return back_sea_shade(ro + rd * hit.x, rd, hit.y);
    }
    if (ts < 0.0) {
        return back_sea_sky(rd);
    }
    let sp = ro + rd * ts;
    let w = sp.xz;
    let g = vec2<f32>(
        sin(w.x * 1.3 + time() * 0.7) * 0.6 + sin(w.x * 3.1 + w.y * 1.7 + time() * 1.3) * 0.4,
        sin(w.y * 1.7 + time() * 0.9) * 0.6 + sin(w.y * 2.9 - w.x * 1.1 + time() * 1.1) * 0.4,
    ) * BP(3) * (1.0 + kick());
    let n = normalize(vec3<f32>(g.x, 1.0, g.y));
    let rr = reflect(rd, n);
    var c = back_sea_sky(rr);
    let rh = back_sea_march(sp + n * 0.02, rr, 40.0);
    if (rh.x > 0.0) {
        c = back_sea_shade(sp + rr * rh.x, rr, rh.y);
    }
    // Grazing views mirror more; steep views show the deep water colour.
    let fres = 0.25 + 0.75 * pow(1.0 - max(-rd.y, 0.0), 4.0);
    c = mix(pal(0.0) * 0.6 + pal(0.25) * 0.2, c, fres);
    // Hard 90s shadow of the objects on the water.
    let sh = back_sea_march(sp + vec3<f32>(0.0, 0.02, 0.0), back_sea_sun(), 30.0);
    c *= select(1.0, 0.45, sh.x > 0.0);
    return mix(c, back_sea_sky(normalize(vec3<f32>(rd.x, 0.0, rd.z))), smoothstep(10.0, 40.0, ts));
}
```

- [ ] `back_plate.toml`

```toml
kind = "backdrop"
description = "The look's picture filling the frame with a slow Ken Burns drift (pan + zoom) and a small punch-in on kicks"
[[param]]
name = "zoom"
min = 1.0
max = 1.3
[[param]]
name = "pan"
min = 0.0
max = 0.08
[[param]]
name = "brightness"
min = 0.6
max = 1.4
```

- [ ] `back_plate.wgsl`

```wgsl
// The look's picture filling the frame: slow Ken Burns drift (pan + zoom), a small punch-in on kicks.
// The 1.25 cover factor plus pan <= 0.08 keeps the 3:2 picture's edges outside the 16:9 frame.
// Without a picture: a dark palette gradient.
fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let zoom = BP(0) * (1.0 + 0.04 * sin(time() * 0.05)) * (1.0 + 0.015 * kick());
    let pan = vec2<f32>(sin(time() * 0.031), cos(time() * 0.023)) * BP(1);
    let im = img(p / (1.25 * zoom) + pan);
    let fallback = mix(pal(0.0), pal(0.25) * 0.5, 0.5 + 0.5 * p.y);
    return mix(fallback, im.rgb * BP(2), im.a);
}
```

- [ ] `back_sprite.toml`

```toml
kind = "backdrop"
description = "The look's picture as a cut-out on a slowly turning, tilted translucent card floating in dark space (hologram); best with transparent images"
[[param]]
name = "turn"
min = 0.0
max = 0.6
[[param]]
name = "tilt"
min = -0.5
max = 0.5
[[param]]
name = "size"
min = 0.5
max = 1.0
```

- [ ] `back_sprite.wgsl`

```wgsl
// The look's picture as a cut-out on a slowly turning, tilted card floating in dark space, with a
// faint translucent card and glowing edges (hologram). Kicks brighten it.

// v in card space: undo a rotation by b about x after a about y.
fn back_sprite_local(v: vec3<f32>, a: f32, b: f32) -> vec3<f32> {
    let ca = cos(a);
    let sa = sin(a);
    let u = vec3<f32>(ca * v.x - sa * v.z, v.y, sa * v.x + ca * v.z);
    let cb = cos(b);
    let sb = sin(b);
    return vec3<f32>(u.x, cb * u.y + sb * u.z, -sb * u.y + cb * u.z);
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
```

### Task 3b: overlays

- [ ] `over_hud.toml`

```toml
kind = "overlay"
description = "Sci-fi HUD: a turning dashed ring, a ring of tick marks (every third long), corner brackets; flares on kicks"
[[param]]
name = "scale"
min = 0.6
max = 1.0
[[param]]
name = "tint"
min = 0.0
max = 1.0
```

- [ ] `over_hud.wgsl`

```wgsl
// Sci-fi HUD: a dashed ring turning slowly, a ring of tick marks (every third one long) and corner
// brackets, white tinted towards palette colour 2; flares on kicks.
fn over_hud_line(d: f32, w: f32) -> f32 {
    return 1.0 - smoothstep(0.0, w, d);
}

fn overlay(p: vec2<f32>) -> vec4<f32> {
    let q = p / OP(0);
    let r = length(q);
    let a = atan2(q.y, q.x);
    let px = 2.0 / resolution().y / OP(0);
    let dash = step(0.35, fract((a + time() * 0.05) * 24.0 / TAU));
    var m = over_hud_line(abs(r - 0.8), 2.5 * px) * dash;
    let k = a * 36.0 / TAU;
    let idx = i32(floor(k + 0.5));
    let len = select(0.05, 0.1, idx % 3 == 0);
    let arc = abs(fract(k + 0.5) - 0.5) * TAU / 36.0 * r;
    m += over_hud_line(arc, 1.5 * px) * step(0.92, r) * step(r, 0.92 + len);
    let aq = abs(q);
    m += over_hud_line(abs(aq.y - 1.15), 2.0 * px) * step(1.0, aq.x) * step(aq.x, 1.15);
    m += over_hud_line(abs(aq.x - 1.15), 2.0 * px) * step(1.0, aq.y) * step(aq.y, 1.15);
    let col = mix(vec3<f32>(0.85), pal(0.5), OP(1)) * (1.2 + 1.5 * kick());
    return vec4<f32>(col, clamp(m, 0.0, 1.0) * 0.85);
}
```

- [ ] `over_grid.toml`

```toml
kind = "overlay"
description = "Retro perspective floor grid (90s CGI / synthwave) scrolling towards the viewer one cell per beat, fading into the horizon"
[[param]]
name = "horizon"
min = -0.4
max = 0.0
[[param]]
name = "opacity"
min = 0.3
max = 1.0
```

- [ ] `over_grid.wgsl`

```wgsl
// Retro perspective floor grid scrolling towards the viewer one cell per beat (seamless: the phase
// wraps by a whole cell), fading into the horizon. Draws only below the horizon. Line widths come
// from the projection (no derivatives).
fn overlay(p: vec2<f32>) -> vec4<f32> {
    let h = OP(0) - p.y;
    if (h <= 0.0) { return vec4<f32>(0.0); }
    let z = 1.0 / h;
    let x = p.x * z;
    let pix = 2.0 / resolution().y;
    let du = abs(fract(z * 0.5 + beat_phase() + 0.5) - 0.5) * 2.0;
    let lz = 1.0 - smoothstep(0.0, 1.5 * pix / (h * h), du);
    let dx = abs(fract(x * 0.5 + 0.5) - 0.5) * 2.0;
    let lx = 1.0 - smoothstep(0.0, 1.5 * pix * z, dx);
    let m = max(lz, lx) * smoothstep(0.0, 0.25, h);
    return vec4<f32>(pal(0.5) * (1.5 + 2.0 * kick()), m * OP(1));
}
```

- [ ] `over_title.toml`

```toml
kind = "overlay"
description = "The look's text in a pixel font in a top corner, typed in over the start of every phrase"
[[param]]
name = "size"
min = 0.005
max = 0.012
[[param]]
name = "corner"
min = 0.0
max = 1.0
fixed = true
```

- [ ] `over_title.wgsl`

```wgsl
// The look's text in a 5x7 pixel font in a top corner (corner < 0.5 = left, else right),
// typed in over the first eighth of every phrase; white, brightening on kicks.
fn overlay(p: vec2<f32>) -> vec4<f32> {
    if (TEXT_LEN == 0) { return vec4<f32>(0.0); }
    let cell = OP(0);
    let w = f32(TEXT_LEN) * 6.0 - 1.0;
    let margin = 0.08;
    let x0 = select(-aspect() + margin, aspect() - margin - w * cell, OP(1) > 0.5);
    let q = vec2<f32>((p.x - x0) / cell, (1.0 - margin - p.y) / cell);
    let shown = floor(f32(TEXT_LEN) * clamp(phrase_phase() * 8.0, 0.0, 1.0) + 0.999);
    let on = text_px(q) * step(q.x, shown * 6.0);
    return vec4<f32>(vec3<f32>(1.0 + 0.5 * kick()), on);
}
```

- [ ] Commit (each agent): `feat(layers): backdrop blocks` / `feat(layers): overlay blocks`.

---

## Phase 4: Core wiring  ·  [opus]

Owns: `src/main.rs`, `src/director.rs` (`plan()`), `src/plates.rs` (new), `src/render/mod.rs`, `src/preview.rs`, `src/wizard.rs`, `Cargo.toml`.

### Task 4.1: `plates.rs` + dependency

- [ ] `Cargo.toml`: `image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }`.
- [ ] Create `src/plates.rs` (+ `mod plates;` in `main.rs`):

```rust
//! Pictures ("plates") a look can show: --image files and AI-made ones. Each is centre-cropped and
//! resized to PLATE_W x PLATE_H (3:2), RGBA8 sRGB, ready for Engine::set_plates.
use crate::types::{PLATE_H, PLATE_W};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Plates per plan (texture array layers).
pub const MAX: usize = 8;

fn fit(img: image::DynamicImage) -> Vec<u8> {
    img.resize_to_fill(PLATE_W, PLATE_H, image::imageops::FilterType::Lanczos3).to_rgba8().into_raw()
}

pub fn load(path: &Path) -> Result<Vec<u8>> {
    Ok(fit(image::open(path).with_context(|| format!("reading image {}", path.display()))?))
}

/// The plan's plates; a missing or broken file becomes a blank (transparent) plate, with a warning.
pub fn load_all(paths: &[PathBuf]) -> Vec<Vec<u8>> {
    paths
        .iter()
        .take(MAX)
        .map(|p| {
            load(p).unwrap_or_else(|e| {
                eprintln!("warning: {e:#}; that picture stays blank");
                vec![0; (PLATE_W * PLATE_H * 4) as usize]
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_any_image_to_the_plate_size() {
        let red = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(30, 10, image::Rgb([255, 0, 0])));
        let px = fit(red);
        assert_eq!(px.len(), (PLATE_W * PLATE_H * 4) as usize);
        assert_eq!(&px[..4], &[255, 0, 0, 255]);
        assert_eq!(load_all(&[PathBuf::from("no/such.png")])[0].len(), px.len());
    }
}
```

### Task 4.2: director: shot boundaries

- [ ] In `plan()`, right after the `match req.mode { ... }` that builds `bounds` (before the locked-segment block):

```rust
    // Shot changes inside the music (storyboard / --layers). Markers mode keeps the user's own cuts.
    if req.mode != Mode::Markers {
        for s in req.shots {
            let t = snap(a, s.time, bar);
            if t < 8.0 * bar || a.duration - t < 8.0 * bar || bounds.iter().any(|b| (b.start - t).abs() < 8.0 * bar) {
                continue;
            }
            bounds.push(Boundary { start: t, title: None, scene: None, transition: s.transition });
        }
        bounds.sort_by(|x, y| x.start.total_cmp(&y.start));
    }
```

- [ ] Test in `director::tests` (`test_analysis()`: 120 BPM, bar = 2 s, phrases every 16 s, drop at 64 s):

```rust
    #[test]
    fn shots_split_a_single_segment() {
        let a = test_analysis();
        let scenes = test_scenes();
        let flash = Transition { kind: TransitionKind::Flash, duration: 0.0 };
        // 64 s is kept; 70 s is within 8 bars of it and 2 s is within 8 bars of the start: both dropped.
        let shots = [Shot { time: 64.0, transition: flash }, Shot { time: 70.0, transition: flash }, Shot { time: 2.0, transition: flash }];
        let req = PlanRequest {
            analysis: &a,
            scenes: &scenes,
            mode: Mode::Single,
            markers: &[],
            seed: 7,
            previous: None,
            reroll: &[],
            force_scene: None,
            generated: 0.0,
            vibe: None,
            genome_for: None,
            blocks: None,
            gen_dir: Path::new("."),
            shots: &shots,
        };
        let p = plan(&req).unwrap();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.segments[1].start, 64.0);
        assert_eq!(p.segments[1].transition_in.kind, TransitionKind::Flash);
        // Markers mode ignores shots.
        assert_eq!(plan(&PlanRequest { mode: Mode::Markers, ..req }).unwrap().segments.len(), 1);
    }
```

### Task 4.3: `main.rs`: flags, dressing, plan fields

- [ ] `Cli` gains (after `vibe`):

```rust
    /// Motion graphics on generated scenes: a backdrop (sky, raymarched sea world, your --image), sometimes a HUD/grid/text overlay, and a new shot at each drop.
    #[arg(long)]
    layers: bool,
    /// A picture the looks can show (with --layers or --vibe); repeatable, up to 8.
    #[arg(long)]
    image: Vec<PathBuf>,
    /// Finishing look for the whole video: clean, vhs, crt or mono.
    #[arg(long, value_enum)]
    look: Option<Look>,
```

- [ ] Before `let reuse = ...`:

```rust
    let plates: Vec<PathBuf> = cli.image.iter().take(plates::MAX).cloned().collect();
    if !plates.is_empty() && !cli.layers && cli.vibe.is_none() {
        eprintln!("warning: --image is only shown with --layers or --vibe");
    }
```

  - `generated`: `let generated = if (cli.vibe.is_some() || cli.layers) && generated == 0.0 { 1.0 } else { generated };` (comment: "A vibe or --layers only steers generated scenes, so they imply some.").
  - `reuse` gains: `&& previous.as_ref().is_some_and(|p| p.layers == cli.layers && p.plates.starts_with(&plates))` (`// ponytail: AI plates follow the user's (M2), so a prefix match keeps them.`).
- [ ] In the plan-making branch, after `evolved`:

```rust
            let text = audio.file_stem().map(|s| s.to_string_lossy().replace(['_', '-'], " ")).unwrap_or_default();
            // --layers dresses the evolved look (unless it already has layers, e.g. from the vibe).
            let dressed = |start: f64, end: f64, r: u64| {
                let g = evolved(start, end, r)?;
                match blocks.as_ref() {
                    Some(b) if cli.layers && !g.has_layers() => Some(genome::dress(&g, b, hash(r, 7), plates.len(), &text)),
                    _ => Some(g),
                }
            };
            // --layers: a morph into each build-up and a flash on each drop.
            let shots: Vec<Shot> = if cli.layers {
                analysis
                    .drops
                    .iter()
                    .flat_map(|d| {
                        [
                            Shot { time: d.build_start, transition: Transition { kind: TransitionKind::Morph, duration: 4.0 } },
                            Shot { time: d.time, transition: Transition { kind: TransitionKind::Flash, duration: 0.0 } },
                        ]
                    })
                    .collect()
            } else {
                Vec::new()
            };
```

  (`audio` = the audio path variable already in `run`. Use its real name.) Pass `genome_for: Some(&dressed)` and `shots: &shots`. Make the result `let mut plan = director::plan(...)?;` and set `plan.plates = plates.clone(); plan.layers = cli.layers;` before it's written.
- [ ] `let plan = match previous {` → `let mut plan = match previous {`, and right after the match: `plan.post = cli.look.map(Look::post);` (comment: `// A render setting like --no-grain: set on every run, never reused.`).
- [ ] Gen-dir cleanup keeps layer files:

```rust
        let used: Vec<String> = plan
            .segments
            .iter()
            .filter_map(|s| s.genome.as_ref())
            .flat_map(|g| [format!("{}.wgsl", g.id()), format!("{}.layers.wgsl", g.id())])
            .collect();
```

### Task 4.4: render, preview

- [ ] `render/mod.rs` `render_video`: after `let mut eng = Engine::new(...)`: `eng.set_plates(&crate::plates::load_all(&job.plan.plates));`.
- [ ] `job_hash`: inside the used-scenes loop add `if let Some(src) = scenes::layers_source(&job.scenes_dir, s) { data.extend(src?.bytes()); }`, and after it:

```rust
    // Plate files by content (an edited --image must re-render).
    for p in &job.plan.plates {
        data.extend(std::fs::read(p).unwrap_or_default());
    }
```

- [ ] `preview.rs`: after each of the two `Engine::new(...)` calls: `engine.set_plates(&crate::plates::load_all(&self.plan.plates));` (adapt to the binding name: `l.engine` at the resize site). `// ponytail: layer files aren't hot-reloaded; restart --dev after editing a back_/over_ block.`

### Task 4.5: wizard

- [ ] `Answers` gains `pub layers: bool, pub look: Option<Look>,` (import `Look`). `to_args` gains:

```rust
    if a.layers {
        v.push("--layers".into());
    }
    if let Some(l) = &a.look {
        v.extend(["--look".into(), val(l)]);
    }
```

- [ ] In `ask()`, inside `if a.generated != Some(0.0) { ... }` after the vibe part:

```rust
        a.layers = Confirm::new("Add motion graphics (backdrops, HUD, text, a new shot at each drop)?").with_default(false).prompt()?;
```

  then, before the "What do you want to make?" question:

```rust
    let looks = vec!["Clean (recommended)", "VHS tape", "CRT screen", "Mono (green hologram)"];
    a.look = match Select::new("Finishing look?", looks).raw_prompt()?.index {
        1 => Some(Look::Vhs),
        2 => Some(Look::Crt),
        3 => Some(Look::Mono),
        _ => None,
    };
```

- [ ] If `wizard.rs` has a `to_args` test with a fully populated `Answers`, add `layers: true, look: Some(Look::Vhs)` and the expected `"--layers", "--look", "vhs"`.
- [ ] `cargo build --release`, commit `feat(layers): --layers, --image, --look and shots`.

---

## Gate M1

- [ ] `cargo test --release`: every test green, including `engine_runs` (all layer blocks compile and finish), `layers_add_slots_and_a_layer_pass`, `glyphs_pack_and_text_maps`, `fits_any_image_to_the_plate_size` and `shots_split_a_single_segment`. Fix a failing block in its own `.wgsl`, then re-run only `cargo test --release engine_runs`.
- [ ] **Default output is byte-identical** (the core-first promise). Copy one test track (see memory: the user's DnB files, copied into `testdata/`). On `main` and on this branch, run:
  `visualiser testdata/dnb8.wav --replan --preview 1:00 --quality draft`, then
  `ffmpeg -i testdata/dnb8.preview-60s.mp4 -f framemd5 -an <main|branch>.md5`. The two `.md5` files must be equal. If they differ, find which change touched the default path (the finish looks must be exact no-ops at 0; `random`/`mutate` must not see layer blocks) and fix it.
- [ ] Smoke renders, watched by the user (draft, 60 s each):
  1. `--replan --layers --preview 1:00 --quality draft` (sky/sea backdrops, shots at drops)
  2. `--replan --layers --image <a cloud photo> --look vhs --preview 1:00 --quality draft` (reference 3)
  3. `--replan --layers --look mono --preview 1:00 --quality draft`
- [ ] `/simplify` → `/review` over the M1 diff (`git diff main...HEAD`). Fix the findings, then `cargo test --release` once.
- [ ] Docs [sonnet]: `docs/USAGE.md` section "Motion graphics: --layers, --image, --look" (what each does, the three smoke commands, that default renders are unchanged, plates are 3:2 and centre-cropped, `--keep` exports layered looks too); `README.md` one feature line; `docs/MANUAL-TESTS.md` add the three smokes + the framemd5 check.
- [ ] Commit `docs: motion layers (M1)`.

---

## Phase 5: AI pictures: `llm.rs`  ·  [sonnet]

Owns: `src/llm.rs`, `Cargo.toml`.

### Task 5.1: image model setting + `image()`

- [ ] `Cargo.toml`: `base64 = "0.22"`.
- [ ] `AiConfig`: derive `Default` and add

```rust
    /// Image model id for AI pictures ("" = none; OpenAI gpt-image models).
    #[serde(default)]
    pub image_model: String,
```

  `load()`: after the config is resolved, apply `VISUALISER_IMAGE_MODEL` if set (same pattern as the model override). For the env-only path, `image_model: env("VISUALISER_IMAGE_MODEL").unwrap_or_default()`. Fix `AiConfig { .. }` literals (tests) with `..Default::default()`.
- [ ] Extract the request into `send` and make `post` use it:

```rust
/// One POST to `url`; the JSON body on 200, else an `ApiError` with the service's message (never the key).
fn send(c: &AiConfig, url: &str, body: &Value) -> Result<Value> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(600))) // --author asks for up to 8k tokens; local CPU models are slow
        .http_status_as_error(false)
        .build()
        .into();
    let mut req = agent.post(url);
    if !c.key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", c.key.trim()));
    }
    let mut resp = req.send_json(body).map_err(|e| anyhow!("AI request failed: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().unwrap_or_default();
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status == 200 {
        return Ok(v);
    }
    // Non-JSON errors (e.g. a plain 404 from a base URL missing /v1) are shown raw.
    let raw: String = text.trim().chars().take(200).collect();
    let msg = v.pointer("/error/message").or(v.pointer("/message")).or(v.pointer("/error")).and_then(Value::as_str).unwrap_or(&raw);
    let msg = if msg.is_empty() { "no details" } else { msg };
    Err(ApiError { status, msg: msg.into() }.into())
}

/// POST a chat-completions request; returns the JSON body or an `ApiError`.
fn post(c: &AiConfig, body: &Value) -> Result<Value> {
    let mut body = body.clone();
    if COMPLETION_TOKENS.load(Relaxed) {
        completion_tokens(&mut body);
    }
    match send(c, &endpoint(c), &body) {
        // OpenAI reasoning models reject `max_tokens`; local servers only know `max_tokens`.
        // Remembered, so only the first request to such a model is wasted.
        Err(e) if e.downcast_ref::<ApiError>().is_some_and(|a| a.status == 400 && a.msg.contains("max_completion_tokens")) && completion_tokens(&mut body) => {
            COMPLETION_TOKENS.store(true, Relaxed);
            send(c, &endpoint(c), &body)
        }
        r => r,
    }
}
```

- [ ] Add (the signature is the contract P6 calls):

```rust
/// The image-generation URL of the same service (".../v1/images/generations").
fn images_endpoint(c: &AiConfig) -> String {
    let base = c.base_url.trim().trim_end_matches('/');
    format!("{}/images/generations", base.strip_suffix("/chat/completions").unwrap_or(base))
}

/// One 1536x1024 picture from the image model (OpenAI gpt-image models); PNG bytes.
/// `transparent`: a cut-out on a transparent background.
pub fn image(c: &AiConfig, prompt: &str, transparent: bool) -> Result<Vec<u8>> {
    use base64::Engine as _;
    let body = json!({
        "model": c.image_model.trim(),
        "prompt": prompt,
        "size": "1536x1024",
        "quality": "medium",
        "background": if transparent { "transparent" } else { "opaque" },
        "output_format": "png",
        "n": 1
    });
    let v = send(c, &images_endpoint(c), &body)?;
    let b64 = v.pointer("/data/0/b64_json").and_then(Value::as_str).context("the image reply had no picture")?;
    Ok(base64::engine::general_purpose::STANDARD.decode(b64)?)
}
```

- [ ] `setup()`: after the model prompt:

```rust
    let image_model = Text::new("Image model id for AI pictures (Enter to skip, e.g. gpt-image-1):")
        .with_help_message("OpenAI image models only. Each picture costs a few cents; pictures are cached per prompt.")
        .prompt()?
        .trim()
        .to_string();
```

  Include it in the `AiConfig { .. }`. Don't test-call it (it costs money). After "works." print `if !c.image_model.is_empty() { println!("Image model {} saved (not test-called: it costs money; the first --vibe that asks for a picture tests it).", c.image_model); }`.
- [ ] Test, appended to `endpoints_and_tool_parsing`:

```rust
        assert_eq!(images_endpoint(&c("https://api.openai.com/v1/")), "https://api.openai.com/v1/images/generations");
        assert_eq!(images_endpoint(&c("https://api.openai.com/v1/chat/completions")), "https://api.openai.com/v1/images/generations");
```

- [ ] `cargo build --release`, commit `feat(ai): image model setting and image()`.

---

## Phase 6: AI looks with layers, pictures, storyboard  ·  [opus]

Owns: `src/vibe.rs`, `src/main.rs`.

### Task 6.1: `vibe.rs`

- [ ] **Types.** Replace `Cache` and add `Design`:

```rust
/// What the vibe made: looks (genome seeds), the plan's plates and an optional storyboard.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Design {
    pub genomes: Vec<Genome>,
    /// --image files first, then the AI's pictures (cached in <data>/plates/).
    #[serde(default)]
    pub plates: Vec<PathBuf>,
    /// Shot changes; `.1` indexes `genomes`. Empty = one evolving look (evolution as before).
    #[serde(default)]
    pub shots: Vec<(Shot, usize)>,
}

#[derive(Serialize, Deserialize)]
struct Cache {
    vibe: String,
    note: String,
    #[serde(default)]
    images: Vec<PathBuf>,
    // Flattened: an old cache ({vibe, note, genomes}) still reads, with no plates or shots.
    #[serde(flatten)]
    design: Design,
}
```

- [ ] **Signature:** `pub fn run(c: &AiConfig, judge: &mut Judge, blocks_dir: &Path, gen_dir: &Path, blocks: &Blocks, a: &Analysis, vibe: &str, images: &[PathBuf], cache: &Path) -> Result<Design>`. Plates are cached in `cache.with_file_name("plates")` (the data folder).
- [ ] **Cache hit:** `hit.vibe == vibe && hit.images == images`, non-empty genomes, and every genome survives `keep_one(g, blocks, hit.design.plates.len())` unchanged. Then print the note and return `hit.design`.
- [ ] **Prompt text.** Append to the existing system prompt, after `{STANCE}`:

```rust
const LAYERS: &str = "## Layers (motion graphics)\n\
Besides the feedback (motion + emitter + colour), a look may add a backdrop block (behind: a sky, a raymarched world, a picture) \
and an overlay block (on top, not smeared: HUD rings, grids, text); \"\" = none. feedback (0..1.5) is how strongly the feedback \
glows over the backdrop (lower for busy backdrops). post is the finishing look: vhs, scanlines, posterize 0..1; tint_amount 0..1 \
towards tint (#rrggbb) for monochrome looks; all 0 = clean. text is shown by overlays that draw text: short, A-Z 0-9 . , - ' ! ? : / & +, or \"\".\n\
Make looks that work as scenes: the backdrop sets the place, the feedback is the music, the overlay frames it.";

const STORYBOARD: &str = "## Storyboard\n\
Shots cut the music into scenes: pick moments (by index, from the list in the request) where the look changes, each with one of \
your looks (index) and a transition: cut, flash (on a drop) or morph (gradual). The first shot must be moment 0. A few meaningful \
changes beat many: follow the energy (build-ups, drops, breakdowns). Up to 8 shots; [] = one look that evolves.";

/// The pictures part of the prompt.
fn images_text(user: usize, can_paint: bool) -> String {
    let mut s = String::from("## Pictures\nLooks show a picture (plate) through img(), e.g. the back_plate and back_sprite backdrops. plate = index, -1 = none.\n");
    if user > 0 {
        s += &format!("The user gave {user} picture(s): plates 0..{} (content unknown; make them the heroes).\n", user - 1);
    }
    if can_paint {
        s += &format!("You may request up to 4 new pictures (plates: prompt + sprite); they become plates {user} onwards, in order. \
sprite = true for one object on a transparent background (floating cut-outs, back_sprite); false for a full-frame picture \
(sky, landscape, texture; back_plate). Describe one strong image, no text in it.\n");
    } else {
        s += "Picture generation is not available: request no plates.\n";
    }
    s
}
```

  System prompt = `format!("...{STANCE}\n\n{LAYERS}\n\n{}\n\n{STORYBOARD}", images_text(images.len(), !c.image_model.trim().is_empty()))`. The menu loop already lists all five kinds (it iterates `genome::KINDS`).
- [ ] **Moments** (appended to the user message as `"Moments:\n" + lines joined by "\n"`, each line `"{i}: {label}"`):

```rust
/// Candidate shot moments: the start, each drop's build-up and drop, then phrase starts (time, label).
fn moments(a: &Analysis) -> Vec<(f64, String)> {
    let c = &a.curves;
    let at = |t: f64, v: &[f32]| v.get(((t * c.rate) as usize).min(v.len().saturating_sub(1))).copied().unwrap_or(0.0);
    let mut m: Vec<(f64, &str)> = vec![(0.0, "start")];
    for d in &a.drops {
        m.push((d.build_start, "build-up starts"));
        m.push((d.time, "DROP"));
    }
    m.extend(a.phrases.iter().filter(|p| p.time > 1.0).map(|p| (p.time, "phrase")));
    // Stable sort, then drop near-duplicates keeping the first: a drop wins over a phrase at the same time.
    m.sort_by(|x, y| x.0.total_cmp(&y.0));
    m.dedup_by(|x, y| (x.0 - y.0).abs() < 1.0);
    m.into_iter()
        .take(60)
        .map(|(t, k)| (t, format!("{} {k}, energy {:.2}, tension {:.2}", hms(t as u64, false), at(t, &c.energy), at(t, &c.tension))))
        .collect()
}
```

- [ ] **Schema.** `looks.items.properties` gains:

```rust
                "backdrop": {"type": "string", "description": "backdrop block id or \"\""},
                "overlay": {"type": "string", "description": "overlay block id or \"\""},
                "plate": {"type": "integer", "description": "plate index, -1 = none"},
                "feedback": {"type": "number"},
                "text": {"type": "string"},
                "post": {"type": "object", "properties": {
                    "vhs": {"type": "number"}, "scanlines": {"type": "number"}, "posterize": {"type": "number"},
                    "tint_amount": {"type": "number"}, "tint": {"type": "string", "description": "#rrggbb"}
                }, "required": ["vhs", "scanlines", "posterize", "tint_amount", "tint"]}
```

  and its `required` lists them too. Top-level `properties` gains:

```rust
            "plates": {"type": "array", "items": {"type": "object", "properties": {
                "prompt": {"type": "string"}, "sprite": {"type": "boolean"}
            }, "required": ["prompt", "sprite"]}},
            "shots": {"type": "array", "items": {"type": "object", "properties": {
                "moment": {"type": "integer"}, "look": {"type": "integer"},
                "transition": {"type": "string", "enum": ["cut", "flash", "morph"]}
            }, "required": ["moment", "look", "transition"]}}
```

  with `"plates", "shots"` added to the top-level `required`. In `new_blocks`, `kind.enum` becomes `["motion", "emitter", "colour", "backdrop", "overlay"]` and the id description becomes `^(motion|emit|colour|back|over)_[a-z0-9_]{2,30}$, not an existing block id`. `valid_id` prefixes gain `"back_"` and `"over_"`.
- [ ] **`look()`** returns the layer fields too:

```rust
fn look(v: &Value) -> Option<Genome> {
    let s = |k: &str| v[k].as_str().map(String::from);
    let pal: Vec<String> = v["palette"].as_array()?.iter().filter_map(|c| c.as_str().map(String::from)).collect();
    let params = v["params"].as_array().into_iter().flatten().filter_map(|p| Some((p["key"].as_str()?.to_string(), p["value"].as_f64()? as f32))).collect();
    let post = &v["post"];
    let f = |k: &str| post[k].as_f64().unwrap_or(0.0).clamp(0.0, 1.0) as f32;
    Some(Genome {
        motion: s("motion")?,
        emitter: s("emitter")?,
        colour: s("colour")?,
        palette: pal.try_into().ok()?,
        params,
        backdrop: s("backdrop").unwrap_or_default(),
        overlay: s("overlay").unwrap_or_default(),
        plate: v["plate"].as_i64().filter(|&i| i >= 0).map(|i| i as u32),
        feedback: v["feedback"].as_f64().map(|x| x.clamp(0.0, 1.5) as f32),
        text: s("text").unwrap_or_default().chars().take(40).collect(),
        post: Post {
            vhs: f("vhs"),
            scanlines: f("scanlines"),
            posterize: f("posterize"),
            tint_amount: f("tint_amount"),
            tint: post["tint"].as_str().and_then(|h| crate::scenes::hex(h).ok()).unwrap_or([1.0; 3]),
        },
    })
}
```

- [ ] **`keep_one` / `keep`:**

```rust
/// A valid look (known blocks incl. layers, parseable palette, plate in range), unknown or non-finite params dropped.
fn keep_one(mut g: Genome, blocks: &Blocks, plates: usize) -> Option<Genome> {
    blocks.parts(&g).ok()?;
    blocks.layer_parts(&g).ok()?;
    crate::scenes::palette(&g.palette).ok()?;
    if g.plate.is_some_and(|i| i as usize >= plates) {
        g.plate = None;
    }
    g.prune(blocks);
    Some(g)
}

fn keep(looks: Vec<Genome>, blocks: &Blocks, plates: usize) -> Vec<Genome> {
    looks.into_iter().filter_map(|g| keep_one(g, blocks, plates)).collect()
}
```

- [ ] **Pictures:**

```rust
/// Paints (or reuses, cached by prompt) the requested pictures; None where painting failed.
fn paint(c: &AiConfig, req: &[Value], dir: &Path) -> Vec<Option<PathBuf>> {
    req.iter()
        .take(4)
        .map(|p| {
            let prompt = p["prompt"].as_str().unwrap_or_default();
            let sprite = p["sprite"].as_bool().unwrap_or(false);
            let path = dir.join(format!("{:016x}.png", fnv1a(format!("{sprite} {prompt}").as_bytes())));
            if path.exists() {
                return Some(path);
            }
            eprintln!("vibe: painting {} ...", if sprite { "a cut-out" } else { "a picture" });
            let made = llm::image(c, prompt, sprite).and_then(|png| {
                std::fs::create_dir_all(dir)?;
                std::fs::write(&path, png)?;
                Ok(path)
            });
            made.map_err(|e| eprintln!("warning: vibe: picture failed ({e:#}); looks using it go without")).ok()
        })
        .collect()
}
```

- [ ] **`run` body changes** (after the design call `d`):
  1. `let raw: Vec<Option<Genome>> = d["looks"].as_array().into_iter().flatten().take(6).map(look).collect();`. Pass `&raw.iter().flatten().cloned().collect::<Vec<_>>()` where the new-block loop needs `looks`.
  2. Pictures: `let painted = if c.image_model.trim().is_empty() { vec![] } else { paint(c, d["plates"].as_array().map_or(&[][..], |v| v), &cache.with_file_name("plates")) };`. Then build the plate list and the index remap from the AI's numbering:

```rust
    let mut plates: Vec<PathBuf> = images.to_vec();
    let mut remap: Vec<Option<u32>> = (0..images.len() as u32).map(Some).collect();
    for p in painted {
        remap.push(p.map(|p| {
            plates.push(p);
            plates.len() as u32 - 1
        }));
    }
    let raw: Vec<Option<Genome>> = raw
        .into_iter()
        .map(|g| g.map(|mut g| {
            g.plate = g.plate.and_then(|i| remap.get(i as usize).copied().flatten());
            g
        }))
        .collect();
```

  3. After the new blocks are written (`all` is final): `let kept: Vec<Option<Genome>> = raw.into_iter().map(|g| g.and_then(|g| keep_one(g, &all, plates.len()))).collect();`. Then `genomes = kept.iter().flatten().cloned().collect()`. Map each raw index to its position in `genomes`: `let pos: Vec<Option<usize>> = kept.iter().scan(0, |n, g| Some(g.as_ref().map(|_| { *n += 1; *n - 1 }))).collect();`.
  4. Shots:

```rust
    let ms = moments(a);
    let mut shots: Vec<(Shot, usize)> = d["shots"]
        .as_array()
        .into_iter()
        .flatten()
        .take(8)
        .filter_map(|s| {
            let time = ms.get(s["moment"].as_u64()? as usize)?.0;
            let look = (*pos.get(s["look"].as_u64()? as usize)?)?;
            let kind = match s["transition"].as_str()? {
                "flash" => TransitionKind::Flash,
                "morph" => TransitionKind::Morph,
                _ => TransitionKind::Cut,
            };
            let duration = if kind == TransitionKind::Morph { 4.0 } else { 0.0 };
            Some((Shot { time, transition: Transition { kind, duration } }, look))
        })
        .collect();
    shots.sort_by(|x, y| x.0.time.total_cmp(&y.0.time));
```

  5. Write the cache with `Cache { vibe, note, images: images.to_vec(), design: design.clone() }` and return `design` (`Design { genomes, plates, shots }`). If `genomes.is_empty()`, bail as before.
- [ ] **`write_block`** example choice: `let ex = all.list(kind)?.iter().max_by_key(|b| b.wgsl.len()).context("no example block")?;` (`// The fullest block of the kind teaches the most (back_sea for backdrops).`). For a layer kind with no blocks yet the error explains itself.
- [ ] **Tests.** Update `keeps_valid_looks_and_known_params` to call `keep(looks, &blocks, 0)`. Add:

```rust
    #[test]
    fn layered_looks_and_moments() {
        let blocks = Blocks::load(Path::new("scenes/blocks")).unwrap();
        let (m, e, c) = (&blocks.motion[0].id, &blocks.emitter[0].id, &blocks.colour[0].id);
        let v = json!({"motion": m, "emitter": e, "colour": c, "palette": ["#000000", "#ff0000", "#00ff00", "#0000ff"], "params": [],
            "backdrop": "back_sky", "overlay": "", "plate": 3, "feedback": 9.0, "text": "neo.",
            "post": {"vhs": 2.0, "scanlines": 0.5, "posterize": 0.0, "tint_amount": 0.0, "tint": "#00ff00"}});
        let g = look(&v).unwrap();
        assert_eq!((g.backdrop.as_str(), g.overlay.as_str(), g.plate, g.feedback), ("back_sky", "", Some(3), Some(1.5)));
        assert_eq!((g.post.vhs, g.post.scanlines, g.post.tint), (1.0, 0.5, [0.0, 1.0, 0.0]));
        assert_eq!(keep_one(g.clone(), &blocks, 2).unwrap().plate, None, "plate out of range");
        assert!(keep_one(Genome { overlay: "over_nope".into(), ..g }, &blocks, 4).is_none());
        let mut a: Analysis = serde_json::from_str(r#"{"version":1,"source":"x","duration":200,"bpm":174,"beats":[],"downbeats":[],"phrases":[],"kicks":[],"curves":{"rate":1,"sub":[],"bass":[],"mid":[],"high":[],"energy":[0.5],"tension":[0.1]},"drops":[],"track_changes":[]}"#).unwrap();
        a.phrases = vec![crate::types::Phrase { time: 64.0, bars: 16 }];
        a.drops = vec![crate::types::Drop { time: 64.0, build_start: 40.0, strength: 1.0 }];
        let ms = moments(&a);
        assert_eq!(ms.iter().map(|m| m.0).collect::<Vec<_>>(), [0.0, 40.0, 64.0]);
        assert!(ms[2].1.contains("DROP"), "a drop wins over a phrase at the same time");
    }
```

(If `Analysis` JSON field names differ, build it with the same fields as `types.rs` declares. The assertions are the spec.)

### Task 6.2: `main.rs` wiring

- [ ] `vibe_seeds` returns `(Option<score::Judge>, vibe::Design)` (`Design::default()` where it returned `Vec::new()`) and passes `&cli.image` and the cache path to `vibe::run`.
- [ ] At the call site: `let (judge, design) = ...; let seeds = design.genomes.clone();`. When the vibe ran, `plates` becomes `design.plates.clone()` if that is non-empty. Write `let plates = if design.plates.is_empty() { plates } else { design.plates.clone() };` right after, before `reuse` is used to make the plan. The `reuse` check must still compare against the user's `--image` list. Keep the original under a different name, e.g. `user_plates`, and use that in `reuse`.
- [ ] Storyboard genome choice:

```rust
            // A storyboard picks each shot's look as designed (no evolution): the AI is directing.
            let bar = 240.0 / if analysis.bpm > 0.0 { analysis.bpm } else { 120.0 };
            let storyboard = |start: f64, _end: f64, _r: u64| {
                blocks.as_ref()?;
                let (_, look) = design.shots.iter().rev().find(|(s, _)| s.time <= start + 2.0 * bar).or(design.shots.first())?;
                design.genomes.get(*look).cloned()
            };
            let gf: &dyn Fn(f64, f64, u64) -> Option<genome::Genome> = if design.shots.is_empty() { &dressed } else { &storyboard };
            let story: Vec<Shot> = design.shots.iter().map(|(s, _)| *s).collect();
            let shots = if story.is_empty() { shots } else { story };
```

  Pass `genome_for: Some(gf)` and `shots: &shots`.
- [ ] `cargo build --release`, commit `feat(ai): vibe looks with layers, pictures and a storyboard`.

---

## Gate M2

- [ ] `cargo test --release`: all green.
- [ ] Live smoke with the user's key, run by the user (it costs money). The orchestrator prints the exact commands:
  1. `visualiser --setup-ai` → enter `gpt-image-1` (or their image model) at the new prompt.
  2. `visualiser testdata/dnb8.wav --replan --vibe "90s CGI, chrome pyramid on a mirror sea, VHS" --preview 1:00 --quality draft`
  3. `visualiser testdata/dnb8.wav --replan --vibe "green hologram brain, HUD rings, mono" --preview 1:00 --quality draft` (expect a cut-out plate + `back_sprite` + `over_hud`)
  4. Run 3 again: no new pictures painted (cache hit).
  5. Without an image model (`VISUALISER_IMAGE_MODEL=` unset, stored empty): the vibe still works, with no plates requested.
- [ ] `/simplify` → `/review` over the M2 diff. Fix the findings, then `cargo test --release` once.
- [ ] Docs [sonnet]: `docs/AI.md` sections "Pictures (image model)", "Storyboard", "Layers in looks", plus cost notes (a few cents per picture, cached per prompt in `<data>/plates/`) and troubleshooting (non-OpenAI image endpoints are unsupported; a picture failure only drops that picture). `docs/USAGE.md` `--setup-ai` table gains the image-model row.
- [ ] Commit `docs: AI pictures and storyboard (M2)`.

---

## Later (deliberately not in this plan)

- **Vision critic:** render draft frames and ask a multimodal model "does this match the vibe?", feeding the answer back into params. Add it once M2 looks are judged by eye and found wanting.
- **`emit_plate`** (the picture's edges as feedback ink): left out because adding an emitter changes `random()`'s picks, so default renders would no longer be byte-identical. Add it behind the layer path, or let the AI write it (the contract documents `img()`).
- **Hot reload of `.layers.wgsl`** in `--dev`.
- **Video generation models:** expensive, a few seconds long, not synced to audio.

## Self-review notes

- Spec coverage: layered compositor (P1), backdrops/overlays/text matching the three references (P3, font P2), finishing looks (P1, Look presets Task 0), user pictures (P4), non-AI shots (P4), AI pictures (P5), AI layers + storyboard (P6), core-first byte-identical gate (Gate M1).
- Names are consistent across tasks: `layers_key`, `layers_path`, `layers_source`, `layer_parts`, `layers_wgsl`, `has_layers`, `dress`, `set_plates`, `plates::load_all`, `Design`, `keep_one`, `moments`, `paint`, `llm::image(c, prompt, transparent)`.
- Default path untouched: no flags → no genome has layers (`random`/`mutate` never add them, `dress` only runs with `--layers`), `compose` returns false, `Post` is zero, and finish's look terms are exact no-ops. The Gate M1 framemd5 check proves it.
