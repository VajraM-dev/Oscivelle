# Scene blocks

A generated scene = one **motion** + one **emitter** + one **colour** block, glued by the
template in `src/genome.rs`. Blocks may also be written by the AI from `--vibe`; they follow this same contract. Each block is `<id>.wgsl` + `<id>.toml` in this folder. All prelude
helpers (`prev`, `warp`, `pal`, `kick()`, `fbm`, `line_glow`, ...) are available.

## <id>.toml
```toml
kind = "motion"            # motion | emitter | colour
description = "Flight down a tube: zoom + twist, trails stream outward"   # one line, shown to the AI
[[param]]                  # motion ≤ 3, emitter ≤ 3, colour ≤ 2 params
name = "zoom"
min = 1.002
max = 1.02
fixed = false              # optional, same meaning as in scene tomls
```

## Functions each block must define (exactly one block of each kind is included)
- motion:  `fn motion(uv: vec2<f32>) -> vec2<f32>`: uv to sample the previous frame at.
           `fn motion_decay() -> f32`: trail multiplier per step, 0.75..0.99.
- emitter: `fn emit(p: vec2<f32>) -> vec2<f32>`: p = centred coords (`to_centered(uv)`);
           returns (intensity ≥ 0, roughly 0..1.5; hue 0..1).
- colour:  `fn colour(e: vec2<f32>) -> vec3<f32>`: (intensity, hue) → linear HDR rgb, via `pal()`.

## Rules
- Params: motion uses `MP(0..2)`, emitter `EP(0..2)`, colour `CP(0..1)`. Never call `P()` directly.
- Helper functions and consts are prefixed with the block id (`tunnel_ridge`), so blocks never collide.
- React to the kick and drops (`kick()`, `drop_env()`, `since_kick()`), not to every band.
- `MP/EP/CP` never multiply `time()` (phase jumps when params change). Mark such params `fixed = true`.
- No `textureSample` other than through `prev()`; no derivatives (`dpdx`, `fwidth`) in motion blocks.

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

## Engine 2 (`--engine 2`)
Generated looks may add a **space** block (kind `space`, ids `space_<name>`): it folds the plane before
both the motion and the emitter see it, so trails and shapes inherit its symmetry.
- space: `fn space(p: vec2<f32>) -> vec2<f32>`: centred coords in, centred coords out. Params `SP(0..1)` (≤ 2).
  Keep the output roughly inside -2..2 (prev() mirrors outside the frame).
- Engine-2 template: `p = space(to_centered(uv))`, `col = prev(motion(to_uv(p))) * decay`,
  `col = blend(col, colour(emit(p)) * drive)`. Blocks never define `space`/`blend` unless they are that kind.
- New helpers (all blocks): `snare()` (snare/clap envelope), `punch()` (fast kick onset, sharper than
  `kick()`), `since_snare()`, `anticipation()` (0..1 ramp over the half bar before a drop; 0 after).
- `hyper_fold(q, k, w) -> vec4` (|q|<1): Mobius-drift by w, fold into the {p,q} Poincare tiling k (0..3); returns (q, inversions, mirror-circle r; centre (sqrt(1+r^2), 0)).
- Motion blocks may tap `prev()` a few times (≤ 5) to react to the image (edges, brightness); get the pixel
  size from `resolution()`.

## Engine 2: comp blocks, history, recipes
**Comp blocks** (kind `comp`, ids `comp_<name>`, **no params**) reframe the finished feedback for display
only, in the look's layer pass: `rgb = backdrop(p) + comp(uv) * feedback`, then the overlay. Nothing in this
pass is fed back, so the simulation underneath keeps its own shape.
- comp: `fn comp(uv: vec2<f32>) -> vec3<f32>`: the colour to show at `uv`. Read the feedback with `prev(uv)`
  (in this pass `prev()` is the finished feedback; comp is the one layer kind that may call it) or the
  display history with `hist(uv, k)`.
- `hist(uv, k)`: the feedback `k` output frames ago (0..63, about 2 s at 30 fps), HDR. It is black until
  the ring has filled. The engine keeps the ring only for looks whose comp block calls `hist(`.
- No params: vary with `seed()` (fixed per segment) and the music helpers.

**Recipes** (`Genome::template`): `""` is the feedback template above. Two others exist:
- `"stateless"`: `col = colour(emit(space(p))) * drive`, with no `prev()`; the motion block is unused.
  It suits dense and raymarched emitters.
- `"dual"`: a second emitter (`Genome::emitter_b`, ≠ `emitter`). The brighter of the two wins each pixel.
  The second is shown half a hue turn away and swells on snares.
  - Its source is included with `fn emit(` renamed to `fn emit_b(` and `EP(` to `EP2(`, which reads params
    `P(8..10)`. So emitters must define `emit` once, never call it themselves, and read params only
    through `EP(`.
  - Dual looks never get a backdrop or overlay, because they use those slots.

## Engine 2: grammars
Each generated look has a **grammar** (`Genome::grammar`: radial, flow, grid, scroll, still) drawn before
its blocks. It picks only from blocks tagged with that grammar in `BLOCK_TAGS` (`src/genome.rs`), so a new
engine-2 motion, emitter, space or comp block needs an entry there (and in `V2_BLOCKS`), or no grammar uses it.
- **radial**: the classic library: tunnels, folds, spins. Anything centred goes here only.
- **flow**: advected fields with no centre and no spin. **grid**: tiles, mosaics, sequencers, blocky jumps.
  **scroll**: the picture slides one way. **still**: the camera holds; motion is decay only (or kick jumps).
- Non-radial blocks: no time-driven rotation, no zoom about the frame centre, nothing centred. A zoom must be
  inward (< 1) and about an off-centre point.
- Motions that slide the whole frame may wrap with `fract(uv)` instead of mirroring off the edge.
