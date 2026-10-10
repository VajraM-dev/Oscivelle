# Scene blocks

A generated scene = one **motion** + one **emitter** + one **colour** block, glued by the
template in `src/genome.rs`. Each block is `<id>.wgsl` + `<id>.toml` in this folder. All prelude
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
