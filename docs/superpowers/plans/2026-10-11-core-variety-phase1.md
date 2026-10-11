# Core Variety Phase 1 Implementation Plan ("Every seed is a different piece")

> **For agentic workers:** REQUIRED SUB-SKILL: follow `/code-implementation-build-rules` (orchestrator dispatches, subagents build; waves below). Use superpowers:subagent-driven-development for per-task dispatch. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Phase 1 of `docs/research/2026-10-11-crazy-visuals.md`, non-AI rows only (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12). Without AI, every `--seed` retry should give a structurally different look: space folds × motion × emitter × blend × mood × palette. Drops should hit and the visuals should lock to onsets.

**Architecture:** Everything is opt-in behind **`--engine 2`**, saved in the plan as `Plan.engine`. Engine 1, the default, stays **byte-identical**. Its block library and the director's scene pool are pinned to frozen id lists, so new blocks (and the AI blocks or `--keep` scenes already in the folders) never change old seeds. A genome gains three optional genes (`space`, `blend`, `regime`), each skipped when empty, so engine-1 genome ids don't change. Genomes with genes compile through a second template, `TEMPLATE_V2`: `prev(motion(space(uv)))`, `blend(col, emit(space(p)))`, regime decay/drive. Space blocks use the free param slots 14–15, so no uniform change is needed for them. One new uniform vec4 `hits` (snare env, punch, s since snare, drop anticipation) feeds onset-locked blocks. Engine-2 evolution uses the judge only as a gate (regime thresholds), picks parents by seed and spreads picks over "look space" (MAP-Elites-lite descriptors). The finish pass gains strobe, RGB split, tear and invert, gated on downbeats and snares (flashes ≤ 1 per bar, under WCAG's 3 Hz limit).

**Tech Stack:** Rust 2021, wgpu 30, WGSL. No new crates.

**Not in this plan:** AI rows (A0–A14), Phase 2/3 rows, row 20 (param tuning in mutation), a second emitter (row 11 "optional"), and a wizard question for `--engine` (the flag works from the wizard's printed command line; add the question when the user asks).

---

## Build setup (from /code-implementation-build-rules)

`T2 Standard · no web frontend (CLI + GPU shaders; design gate n/a) · review: /simplify → /review`

- **Branch:** `feature/core-variety`, created from `main` @ `676e3ab`. Never commit to `main`; the user merges.
- **Untracked files:** `scenes/blocks/back_runner_tunnel.*`, `back_trippy_fog.*` and `over_runner_silhouette.*` are AI-written (vibe) and untracked. **Leave them alone** (don't commit, don't delete). Engine 1 pinning excludes them by design.
- **Orchestrator** plans, dispatches and joins. Subagents do the work. Independent tasks in one wave go out as multiple `Agent` calls **in a single message**.
- **Model routing:** `opus` for genome, evolve, director, engine and every shader block. `sonnet` for docs. Haiku is allowed for purely mechanical edits (none planned).
- **Testing discipline:** during a wave, subagents run `cargo build --release` plus their own task's named tests. Shader-block tasks run nothing, because WGSL compiles in the gate's `engine_runs`. Full `cargo test --release` runs once at the gate. After a fix, re-run only the failing test, then one full run.
- **ponytail:** minimum code. Each non-trivial piece of logic leaves ONE runnable `#[test]`. Mark deliberate shortcuts with `// ponytail:`.
- **Commits:** one per phase: `feat(core): ...`, `feat(blocks): ...`, `docs: ...`.

### Waves

```
Wave 0 (orchestrator, sequential): Task 0 goldens → Task 1 contract + pinning → cargo test --release engine1 library_pins regime
Wave 1 (parallel, 7 agents):
  P2 judge descriptors [opus]       · P3 genome v2 template/genes/palettes [opus]
  P4 music: snares, hits, anticipation, finish glitch [opus]
  P5a space blocks [opus] · P5b feedback-operator motion blocks [opus]
  P5c 2D emitters [opus]  · P5d raymarched emitters [opus]
Wave 2: P6 evolve v2 + main wiring + engine test [opus]
Gate → /simplify → /review → docs [sonnet] → commit
```

**File ownership (no collisions):**
- **Task 1** owns: `types.rs`, `regime.rs` (new), `scenes.rs`, `_prelude.wgsl`, `CONTRACT.md`, the contract edits in `genome.rs` (fields, kind, pinning), `director.rs` (`plan()`, `PlanRequest`, test literals), `score.rs` (`Thresholds`/`passes` only), and one-line compile fixes in `vibe.rs`, `main.rs` and `analysis.rs`.
- **Wave 1:**
  - P2 owns `score.rs`.
  - P3 owns `genome.rs`.
  - P4 owns `analysis.rs`, `director.rs` (`frame_state`, `mods`, the two `mods()` call sites in `plan()`, new tests), `finish.wgsl` and `engine.rs` (`FinishUniforms` + the test's `Post` literal).
  - P5a–d each own only *new* files under `scenes/blocks/`.
- **Wave 2:** P6 owns `evolve.rs`, `main.rs` and the `engine.rs` test.

---

## File map

| File | Status | Responsibility |
|---|---|---|
| `src/types.rs` | modify | `Plan.engine`, `same_inputs(.., engine)`, `SceneUniforms.hits`, `Analysis.snares`, `ANALYSIS_VERSION`, `Post` glitch fields, `ModSource::{Snare, Punch}` |
| `src/regime.rs` | create | Six moods: judge thresholds, decay, drive, finish look |
| `scenes/_prelude.wgsl` | modify | `U.hits`; `snare()`, `punch()`, `since_snare()`, `anticipation()` |
| `scenes/blocks/CONTRACT.md` | modify | `space` kind, `SP()`, new helpers, prev() taps rule |
| `src/scenes.rs` | modify | `BUILTIN` scene ids |
| `src/genome.rs` | modify | genes, `space` kind, `pinned()`, `TEMPLATE_V2`, blends, `random_v2`, `mutate_v2`, `cosine_palette` |
| `src/score.rs` | modify | `Thresholds`, `passes`, symmetry/density/colourfulness, `measure`, `descriptor`, `cell` |
| `src/analysis.rs` | modify | `detect_onsets` (kicks + snares), version 2 |
| `src/director.rs` | modify | pinned pick pool, `engine`, `hits`, anticipation, glitch gating, v2 mod sources |
| `src/render/finish.wgsl`, `src/render/engine.rs` | modify | strobe / RGB split / tear / invert (`F.p2`) |
| `src/evolve.rs` | modify | `best_v2`, `pick_v2` |
| `src/main.rs` | modify | `--engine`, pinned evolution pool, re-analysis, novelty avoid list |
| `scenes/blocks/space_*` | create | mirror, kaleido, kifs, droste, polar, hyper |
| `scenes/blocks/motion_*` | create | mosh, sort, etch, erode |
| `scenes/blocks/emit_*` | create | warpfbm, cells, truchet, hypertile, lyapunov, orbit, kifs3d, mandelbox, sdftunnel |

---

## Task 0: Goldens (orchestrator, before any change)

Characterization tests. They capture engine-1 behaviour **now** and must stay green through every later task.

**Files:** `src/genome.rs` (tests), `src/director.rs` (tests)

- [ ] **Step 1: genome golden.** Append to `mod tests` in `src/genome.rs`:

```rust
    /// Engine 1 must never change: fingerprints of generated sources and picks (captured before engine 2 work).
    #[test]
    fn engine1_genomes_are_frozen() {
        let mut b = blocks();
        // Only the committed layer blocks (the folder may hold untracked AI-written ones).
        b.backdrop.retain(|x| ["back_plate", "back_sea", "back_sky", "back_sprite"].contains(&x.id.as_str()));
        b.overlay.retain(|x| ["over_grid", "over_hud", "over_title"].contains(&x.id.as_str()));
        let sig = |s: String| format!("{:016x}", fnv1a(s.as_bytes()));
        let layered = Genome { backdrop: "back_sky".into(), overlay: "over_hud".into(), plate: Some(1), feedback: Some(0.6), text: "HI".into(), ..genome() };
        let picks: String = (0..16u64)
            .map(|r| {
                let h = crate::types::hash(1, r);
                let g = random(&b, h);
                format!("{} {} {}\n", g.id(), mutate(&g, &b, h ^ 7).id(), dress(&g, &b, h ^ 9, 2, "HI").id())
            })
            .collect();
        assert_eq!(sig(wgsl(&genome(), &b).unwrap()), "CAPTURE");
        assert_eq!(sig(layers_wgsl(&layered, &b).unwrap().unwrap()), "CAPTURE");
        assert_eq!(sig(picks), "CAPTURE");
    }
```

- [ ] **Step 2: director golden.** Append to `mod tests` in `src/director.rs`:

```rust
    /// Engine 1 must never change: plan JSON and per-frame uniforms (captured before engine 2 work).
    #[test]
    fn engine1_plan_and_frames_are_frozen() {
        let a = test_analysis();
        let scenes = test_scenes();
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Auto, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 0.0, vibe: None, genome_for: None, blocks: None, gen_dir: Path::new("."), shots: &[] };
        let p = plan(&req).unwrap();
        let mut s = serde_json::to_string(&p).unwrap();
        for t in [0.3, 10.0, 63.9, 64.05, 96.0, 300.0] {
            let f = frame_state(&p, &a, &scenes, t, 64, 64, Quality::Draft);
            let u = &f.a;
            s += &format!(
                "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?} {} {} {} {}\n",
                u.res_time, u.beat, u.bands, u.shape, u.p0, u.p1, u.p2, u.p3, u.pal, u.misc, f.mix, f.flash, f.exposure, f.saturation
            );
        }
        assert_eq!(format!("{:016x}", fnv1a(s.as_bytes())), "CAPTURE");
    }
```

- [ ] **Step 3: capture.** Run `cargo test --release engine1 -- --nocapture`. Each `assert_eq!` fails and prints `left: "…"`. Paste each left value over its `"CAPTURE"`, in order: the genome test fails at the first assert, so fix and re-run three times. Then re-run until both tests pass.
- [ ] **Step 4:** Commit `test(core): engine-1 goldens`.

---

## Task 1: Contract + pinning (orchestrator)

Every Wave-1 task codes against this. It changes no engine-1 behaviour; the goldens prove it.

### 1.1 `src/types.rs`

- [ ] In `Plan`, after `layers`:

```rust
    /// Visual engine (--engine): 1 = the original looks, 2 = space folds, blends, moods, new blocks, onset hits.
    #[serde(default = "engine_v1", skip_serializing_if = "is_engine_v1")]
    pub engine: u32,
```

  and at module level:

```rust
fn engine_v1() -> u32 {
    1
}
fn is_engine_v1(e: &u32) -> bool {
    *e <= 1
}
```

- [ ] `same_inputs` gains `engine`:

```rust
    pub fn same_inputs(&self, seed: u64, mode: Mode, generated: f32, vibe: Option<&str>, engine: u32) -> bool {
        self.seed == seed && self.mode == mode && self.generated == generated && self.vibe.as_deref() == vibe && self.engine == engine
    }
```

- [ ] In `Analysis`, after `track_changes`:

```rust
    /// Snare/clap onsets (broadband-dominated); empty in analyses older than ANALYSIS_VERSION 2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snares: Vec<Kick>,
```

  and near `SIM_HZ`:

```rust
/// Analysis format version; engine 2 re-analyses older files (they lack snares).
pub const ANALYSIS_VERSION: u32 = 2;
```

- [ ] `ModSource`: append `Snare,` and `Punch,` after `Drop` (serde names; old plans never contain them).
- [ ] `Post`: append four fields, each kept out of the JSON at 0 so existing genome ids and plan JSON stay identical:

```rust
    /// White flash on downbeats, 0..1 (the director gates it: ≤ 1 per bar).
    #[serde(skip_serializing_if = "is_zero")]
    pub strobe: f32,
    /// Red/blue split on snares, 0..1.
    #[serde(skip_serializing_if = "is_zero")]
    pub rgb_split: f32,
    /// Horizontal tear bands on snares, 0..1.
    #[serde(skip_serializing_if = "is_zero")]
    pub tear: f32,
    /// Colour inversion on downbeats, 0..1.
    #[serde(skip_serializing_if = "is_zero")]
    pub invert: f32,
```

  plus `fn is_zero(x: &f32) -> bool { *x == 0.0 }`. Extend `Post::lerp` with the four `lerp(self.x, o.x, f)` lines.
- [ ] `SceneUniforms`: append after `p3`:

```rust
    /// Snare envelope 0..1, punch (fast kick onset) 0..1, s since snare (1e3 if none), drop anticipation 0..1.
    pub hits: [f32; 4],
```

### 1.2 `scenes/_prelude.wgsl`

- [ ] In `struct U` after `p3`: `hits: vec4<f32>,     // snare env, punch (fast kick onset), s since snare (1e3 if none), drop anticipation 0..1`
- [ ] After `fn since_drop()`:

```wgsl
fn snare() -> f32 { return u.hits.x; }
fn punch() -> f32 { return u.hits.y; }
fn since_snare() -> f32 { return u.hits.z; }
fn anticipation() -> f32 { return u.hits.w; }
```

### 1.3 `src/regime.rs` (new) + `src/score.rs` thresholds

- [ ] `src/score.rs`: replace the body of `verdict` and add the threshold type:

```rust
/// Judge acceptance ranges (engine 2: one set per regime).
#[derive(Clone, Copy, Debug)]
pub struct Thresholds {
    pub mean: (f32, f32),
    pub min_contrast: f32,
    pub motion: (f32, f32),
    pub max_clipped: f32,
    pub max_grain: f32,
}

pub const V1: Thresholds = Thresholds { mean: MEAN, min_contrast: MIN_CONTRAST, motion: MOTION, max_clipped: MAX_CLIPPED, max_grain: MAX_GRAIN };

/// Not black, static, flooded or noisy by `th`.
pub fn passes(m: &Metrics, th: &Thresholds) -> bool {
    (th.mean.0..=th.mean.1).contains(&m.mean)
        && m.contrast >= th.min_contrast
        && (th.motion.0..=th.motion.1).contains(&m.motion)
        && m.clipped <= th.max_clipped
        && m.grain <= th.max_grain
}

/// None = rejected; else higher is better (contrast + music lock + some motion).
pub fn verdict(m: &Metrics) -> Option<f32> {
    passes(m, &V1).then(|| m.contrast + 2.0 * m.kick_sync.max(0.0) + 4.0 * m.motion.min(0.05))
}
```

- [ ] `src/regime.rs`:

```rust
//! Aesthetic regimes (engine 2): a segment's mood. Each sets the judge's acceptance ranges, how long trails
//! last, how hard the music drives the emitter, and a finishing look. The seed picks one per segment.
use crate::score::{Thresholds, V1};
use crate::types::Post;

pub struct Regime {
    pub name: &'static str,
    pub th: Thresholds,
    /// Multiplies the motion block's decay (the template caps the product at 0.995).
    pub decay: f32,
    /// Multiplies the emitter's music drive.
    pub drive: f32,
    pub post: Post,
}

const CLEAN: Post = Post { vhs: 0.0, scanlines: 0.0, posterize: 0.0, tint_amount: 0.0, tint: [1.0; 3], strobe: 0.0, rgb_split: 0.0, tear: 0.0, invert: 0.0 };

pub const REGIMES: [Regime; 6] = [
    Regime { name: "calm", th: Thresholds { mean: (0.03, 0.6), min_contrast: 0.03, motion: (0.0005, 0.03), max_clipped: 0.2, max_grain: 0.15 }, decay: 1.03, drive: 0.55, post: CLEAN },
    Regime { name: "dark", th: Thresholds { mean: (0.012, 0.18), min_contrast: 0.02, motion: (0.001, 0.15), max_clipped: 0.1, max_grain: 0.2 }, decay: 0.97, drive: 0.8, post: CLEAN },
    // Tint colour is set per genome from its palette (genome::random_v2).
    Regime { name: "mono", th: V1, decay: 1.0, drive: 1.0, post: Post { tint_amount: 0.85, ..CLEAN } },
    Regime { name: "sparse", th: Thresholds { mean: (0.01, 0.25), min_contrast: 0.02, motion: (0.001, 0.2), max_clipped: 0.15, max_grain: 0.2 }, decay: 0.93, drive: 1.1, post: CLEAN },
    Regime { name: "glitch", th: Thresholds { motion: (0.002, 0.35), max_grain: 0.35, ..V1 }, decay: 0.95, drive: 1.2, post: Post { rgb_split: 0.6, tear: 0.5, invert: 0.25, scanlines: 0.2, ..CLEAN } },
    Regime { name: "maximal", th: Thresholds { mean: (0.08, 0.8), min_contrast: 0.06, max_clipped: 0.35, ..V1 }, decay: 1.0, drive: 1.4, post: Post { strobe: 0.5, ..CLEAN } },
];

pub fn get(name: &str) -> Option<&'static Regime> {
    REGIMES.iter().find(|r| r.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regimes_are_unique_and_sane() {
        for (i, r) in REGIMES.iter().enumerate() {
            assert!(REGIMES[..i].iter().all(|o| o.name != r.name), "{}", r.name);
            assert!(r.th.mean.0 < r.th.mean.1 && r.th.motion.0 < r.th.motion.1, "{}", r.name);
            assert!((0.5..=1.5).contains(&r.decay) && (0.3..=2.0).contains(&r.drive), "{}", r.name);
        }
        assert!(get("mono").is_some() && get("nope").is_none());
    }
}
```

- [ ] `main.rs`: add `mod regime;` (alphabetical, after `mod preview;`).

  Note: `Post` derives `Default`, which gives `tint: [0.0; 3]`. `CLEAN` uses `[1.0; 3]` on purpose: tint is ignored while `tint_amount` is 0.

### 1.4 `src/genome.rs`: genes, space kind, pinning

- [ ] `Genome`: append after `post`:

```rust
    /// Engine-2 genes ("" = none, the engine-1 recipe). Space block id, blend mode (genome::BLENDS) and
    /// regime name (regime::REGIMES). Empty genes are left out of the JSON, so engine-1 ids are unchanged.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub space: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub blend: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub regime: String,
```

  and the method:

```rust
    /// Uses the engine-2 template (has any engine-2 gene).
    pub fn is_v2(&self) -> bool {
        !self.space.is_empty() || !self.blend.is_empty() || !self.regime.is_empty()
    }
```

- [ ] Kinds: `pub const KINDS: [&str; 6] = ["motion", "emitter", "colour", "backdrop", "overlay", "space"];` and `const SLOTS: [usize; 6] = [3, 3, 2, 4, 2, 2];`. Update the doc comments: "space (engine 2) uses slots 14..15".
- [ ] `Blocks`: add `pub space: Vec<Block>,` after `overlay`. Every match/array over kinds gets a 6th arm:
  - `Genome::block_mut`: `4 => &mut self.overlay, _ => &mut self.space`.
  - `Blocks::list`: `[&self.motion, &self.emitter, &self.colour, &self.backdrop, &self.overlay, &self.space][kind_index(kind)?]`.
  - `list_mut`: `4 => &mut self.overlay, _ => &mut self.space`.
  - `load`: `space: vec![]` in the literal.
  - `subset`: `space: pick(&self.space, |g| &g.space),`.
  - `param()`: add `&self.space` to the array.
- [ ] Space accessor:

```rust
    /// The genome's space block ("" = none), or an error naming an unknown id.
    pub fn space_part(&self, g: &Genome) -> Result<Option<&Block>> {
        Self::opt(&self.space, &g.space)
    }
```

- [ ] `prune`: include the space block. Replace the first line with:

```rust
        let (Ok(parts), Ok(layers), Ok(space)) = (blocks.parts(self), blocks.layer_parts(self), blocks.space_part(self)) else { return self.params.clear() };
```

  and chain `.chain(space)` after `.chain(layers.into_iter().flatten())`.
- [ ] Pinning (above `impl Blocks`):

```rust
/// Library v1: the only blocks engine 1 picks from, frozen so new (or AI-written) blocks never change old seeds.
const V1_BLOCKS: &[&str] = &[
    "back_plate", "back_sea", "back_sky", "back_sprite",
    "colour_duotone", "colour_hue", "colour_intensity", "colour_iridescent", "colour_whitehot",
    "emit_blobs", "emit_clouds", "emit_contours", "emit_filaments", "emit_lissajous", "emit_polygon", "emit_rings", "emit_stars",
    "motion_deepzoom", "motion_ink", "motion_kaleido", "motion_push", "motion_swirl", "motion_tunnel",
    "over_grid", "over_hud", "over_title",
];
/// palettes.toml entries engine 1 picks from (the first N).
const V1_PALETTES: usize = 30;
/// Added for engine 2 (V1_BLOCKS + these).
const V2_BLOCKS: &[&str] = &[
    "space_droste", "space_hyper", "space_kaleido", "space_kifs", "space_mirror", "space_polar",
    "motion_erode", "motion_etch", "motion_mosh", "motion_sort",
    "emit_cells", "emit_hypertile", "emit_kifs3d", "emit_lyapunov", "emit_mandelbox", "emit_orbit", "emit_sdftunnel", "emit_truchet", "emit_warpfbm",
];
```

  and in `impl Blocks`:

```rust
    /// The frozen library of `engine` (1 or 2): only its block ids, in load (sorted) order, and its palettes.
    pub fn pinned(&self, engine: u32) -> Blocks {
        let ok = |id: &str| V1_BLOCKS.contains(&id) || (engine >= 2 && V2_BLOCKS.contains(&id));
        let keep = |l: &[Block]| l.iter().filter(|b| ok(&b.id)).cloned().collect();
        Blocks {
            motion: keep(&self.motion),
            emitter: keep(&self.emitter),
            colour: keep(&self.colour),
            backdrop: keep(&self.backdrop),
            overlay: keep(&self.overlay),
            space: keep(&self.space),
            palettes: self.palettes[..V1_PALETTES.min(self.palettes.len())].to_vec(),
        }
    }
```

- [ ] Test:

```rust
    #[test]
    fn library_pins() {
        let b = blocks();
        let v1 = b.pinned(1);
        let ids = |l: &[Block]| l.iter().map(|x| x.id.as_str()).collect::<Vec<_>>().join(" ");
        assert_eq!(ids(&v1.motion), "motion_deepzoom motion_ink motion_kaleido motion_push motion_swirl motion_tunnel");
        assert_eq!(ids(&v1.backdrop), "back_plate back_sea back_sky back_sprite");
        assert_eq!((v1.emitter.len(), v1.colour.len(), v1.overlay.len(), v1.space.len(), v1.palettes.len()), (8, 5, 3, 0, 30));
        let v2 = b.pinned(2);
        assert!(v1.motion.iter().all(|m| v2.motion.iter().any(|x| x.id == m.id)));
    }
```

- [ ] In `engine1_genomes_are_frozen`, replace the three `let mut b = blocks(); b.backdrop.retain…; b.overlay.retain…;` lines with `let b = blocks().pinned(1);`. The values must not change.

### 1.5 `src/scenes.rs` + `src/director.rs`

- [ ] `scenes.rs`:

```rust
/// The shipped scenes: the only ones the director picks on its own (pinned, like genome V1_BLOCKS).
/// --keep / --author scenes stay reachable through --scene and markers.
pub const BUILTIN: [&str; 7] = ["fractal_zoom", "ink_in_water", "kaleidoscope_bloom", "liquid_tunnel", "neon_geometry", "plasma_field", "starfield_warp"];
```

- [ ] `director.rs`:
  - `PlanRequest` gains `/// Visual engine (Plan.engine). pub engine: u32,`.
  - `same_plan` passes `req.engine`.
  - At the top of `plan()` after `bail!` add:

```rust
    // Auto picks come from the shipped scenes only; a library without them (tests) uses all.
    let builtin: Vec<Scene> = req.scenes.iter().filter(|s| crate::scenes::BUILTIN.contains(&s.id.as_str())).cloned().collect();
    let pool: &[Scene] = if builtin.is_empty() { req.scenes } else { &builtin };
```

  - Change both `pick(req.scenes, …)` calls to `pick(pool, …)`.
  - The `Plan { … }` literal adds `engine: req.engine`.
  - Every `PlanRequest { … }` literal in the tests adds `engine: 1`; the `Blocks { … }` literal adds `space: vec![]`; `test_analysis` adds `snares: vec![]`.

### 1.6 Compile fixes (no behaviour change)

- [ ] `score.rs` `Plan { … }` literal: add `engine: 1`.
- [ ] `analysis.rs` `Analysis { … }` literal: add `snares: Vec::new()` (P4 fills it).
- [ ] `main.rs`:
  - `same_inputs(cli.seed, mode, generated, cli.vibe.as_deref())` → append `, 1`.
  - Add `engine: 1,` to the `director::PlanRequest` literal. P6 replaces both with the flag.
- [ ] `vibe.rs`:
  - `for kind in genome::KINDS {` (menu, ~line 117) → `for kind in &genome::KINDS[..5] {`, with the comment `// space is an engine-2 core gene; not offered to the AI`.
  - In `look()`, add `..Default::default()` as the last field of both the `Genome { … }` and the `Post { … }` literals.
- [ ] `engine.rs` test `Post { … }` literal: add `..Default::default()` (P4 sets the new fields).

### 1.7 `scenes/blocks/CONTRACT.md`

- [ ] Append:

````markdown
## Engine 2 (`--engine 2`)
Generated looks may add a **space** block (kind `space`, ids `space_<name>`): it folds the plane before
both the motion and the emitter see it, so trails and shapes inherit its symmetry.
- space: `fn space(p: vec2<f32>) -> vec2<f32>`: centred coords in, centred coords out. Params `SP(0..1)` (≤ 2).
  Keep the output roughly inside -2..2 (prev() mirrors outside the frame).
- Engine-2 template: `p = space(to_centered(uv))`, `col = prev(motion(to_uv(p))) * decay`,
  `col = blend(col, colour(emit(p)) * drive)`. Blocks never define `space`/`blend` unless they are that kind.
- New helpers (all blocks): `snare()` (snare/clap envelope), `punch()` (fast kick onset, sharper than
  `kick()`), `since_snare()`, `anticipation()` (0..1 ramp over the half bar before a drop; 0 after).
- Motion blocks may tap `prev()` a few times (≤ 5) to react to the image (edges, brightness); get the pixel
  size from `resolution()`.
````

- [ ] **Verify:** `cargo build --release`, then `cargo test --release engine1 library_pins regime`. All green, with golden values unchanged.
- [ ] Commit `feat(core): engine-2 contract, pinned engine-1 library`.

---

## Phase P2: Judge descriptors · [opus] · owns `src/score.rs`

### Task P2.1

- [ ] `Metrics`: add fields:

```rust
    pub symmetry: f32,      // 1 = left-right mirror image, 0 = unrelated halves
    pub density: f32,       // share of pixels with luma > 0.1
    pub colourfulness: f32, // Hasler–Süsstrunk, set by the judge from rgb (0 in `metrics`)
```

- [ ] In `metrics()`, before the struct literal:

```rust
    let symmetry = frames
        .iter()
        .map(|f| {
            let d = f.chunks(w).map(|r| (0..w / 2).map(|x| (r[x] - r[w - 1 - x]).abs()).sum::<f32>()).sum::<f32>() / (px / 2.0);
            (1.0 - d / (2.0 * mean_of(f) + 1e-3)).clamp(0.0, 1.0)
        })
        .sum::<f32>()
        / n;
    let density = frames.iter().map(|f| f.iter().filter(|&&v| v > 0.1).count() as f32 / px).sum::<f32>() / n;
```

  and add `symmetry, density, colourfulness: 0.0` to the literal.
- [ ] Colourfulness + descriptors:

```rust
/// Hasler–Süsstrunk colourfulness of linear-ish 0..1 rgb pixels (0 = grey; ~0.3+ = vivid).
pub fn colourfulness(px: &[[f32; 3]]) -> f32 {
    let n = px.len().max(1) as f32;
    let (rg, yb): (Vec<f32>, Vec<f32>) = px.iter().map(|c| (c[0] - c[1], 0.5 * (c[0] + c[1]) - c[2])).unzip();
    let stat = |v: &[f32]| {
        let m = v.iter().sum::<f32>() / n;
        (m, v.iter().map(|x| (x - m).powi(2)).sum::<f32>() / n)
    };
    let ((mr, vr), (my, vy)) = (stat(&rg), stat(&yb));
    (vr + vy).sqrt() + 0.3 * (mr * mr + my * my).sqrt()
}

/// Where a look sits in "look space", each 0..1: brightness, motion, symmetry, colourfulness, density, fine detail.
pub fn descriptor(m: &Metrics) -> [f32; 6] {
    [(m.mean / 0.5).min(1.0), (m.motion / 0.05).min(1.0), m.symmetry, (m.colourfulness / 0.4).min(1.0), m.density, (m.grain / 0.2).min(1.0)]
}

/// Quality-diversity cell (0..27): brightness × motion × colourfulness, 3 bins each.
pub fn cell(d: &[f32; 6]) -> usize {
    let b = |x: f32| ((x * 3.0) as usize).min(2);
    b(d[0]) + 3 * b(d[1]) + 9 * b(d[3])
}
```

- [ ] `Judge`: `judge` delegates to a new `measure`, and `score` returns `Metrics`:

```rust
    pub fn judge(&mut self, scene: Scene, analysis: &Analysis, t0: f64, secs: f64) -> Result<Option<f32>> {
        Ok(verdict(&self.measure(scene, analysis, t0, secs)?))
    }

    /// The raw metrics of `secs` seconds from `t0` (engine 2 gates by regime and picks by descriptor).
    /// Like `judge`, the scene is dropped from the judge afterwards.
    pub fn measure(&mut self, scene: Scene, analysis: &Analysis, t0: f64, secs: f64) -> Result<Metrics> {
        let id = scene.id.clone();
        let r = self.score(scene, analysis, t0, secs);
        self.engine.remove_scene(&id);
        r
    }
```

  In `score`:
  - Change the return type to `Result<Metrics>`.
  - Add `let mut cols = Vec::new();` next to `frames`/`kicks`.
  - In the readback block, build `let mut rgb = Vec::with_capacity((W * H) as usize);` alongside `luma`, pushing `[p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]` per pixel, then `cols.push(colourfulness(&rgb));`.
  - End with:

```rust
        let mut m = metrics(&frames, W as usize, &kicks);
        m.colourfulness = cols.iter().sum::<f32>() / cols.len().max(1) as f32;
        Ok(m)
```

- [ ] Test:

```rust
    #[test]
    fn descriptors_see_symmetry_density_colour_and_cells() {
        let w = 16;
        let sym: Vec<f32> = (0..w * 9).map(|p| 0.1 + 0.05 * ((p % w) as f32 - 7.5).abs()).collect();
        let ramp: Vec<f32> = (0..w * 9).map(|p| (p % w) as f32 / w as f32).collect();
        let kicks = vec![0.0; 4];
        let (ms, mr) = (metrics(&vec![sym; 4], w, &kicks), metrics(&vec![ramp; 4], w, &kicks));
        assert!(ms.symmetry > 0.99 && mr.symmetry < 0.6, "{} {}", ms.symmetry, mr.symmetry);
        assert!(ms.density > 0.99 && (0.8..0.9).contains(&mr.density), "{}", mr.density);
        assert!(colourfulness(&[[0.5; 3]; 8]) < 1e-6);
        assert!(colourfulness(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].repeat(4)) > 0.3);
        let d = descriptor(&ms);
        assert!(d.iter().all(|x| (0.0..=1.0).contains(x)) && cell(&d) < 27);
    }
```

- [ ] `cargo test --release descriptors verdict` → green. The existing `calibrate` test still compiles: it calls `judge`. Commit `feat(core): judge descriptors for look-space picks`.

---

## Phase P3: Genome engine 2 · [opus] · owns `src/genome.rs`

### Task P3.1: template, blends, slots

- [ ] Add below `NO_OVERLAY`:

```rust
/// Engine 2: the plane is folded (space) before motion and emitter see it; the emitter blends over the
/// feedback by mode; the regime scales trail length and music drive; onsets punch, anticipation squeezes.
const TEMPLATE_V2: &str = r#"
fn MP(i: i32) -> f32 { return P(i); }
fn EP(i: i32) -> f32 { return P(3 + i); }
fn CP(i: i32) -> f32 { return P(6 + i); }
fn SP(i: i32) -> f32 { return P(14 + i); }
{IMG}
{SPACE}
{MOTION}
{EMITTER}
{COLOUR}
{BLEND}
fn scene(uv: vec2<f32>) -> vec4<f32> {
    let p = space(to_centered(uv));
    var col = prev(motion(to_uv(p))) * min(motion_decay() * {DECAY}, 0.995);
    let drive = {DRIVE} * (0.5 + 0.9 * punch() + 0.3 * snare() + 0.6 * drop_env()) * (1.0 - 0.6 * anticipation());
    col = blend(col, colour(emit(p)) * drive);
    col = col / (1.0 + 0.12 * dot(col, vec3<f32>(0.2126, 0.7152, 0.0722)));
    return vec4<f32>(col, 1.0);
}
"#;
const NO_SPACE: &str = "fn space(p: vec2<f32>) -> vec2<f32> { return p; }";

/// Engine-2 blend modes of the emitter over the feedback; "" = add (the engine-1 recipe).
pub const BLENDS: [&str; 4] = ["screen", "difference", "multiply", "lighten"];

fn blend_wgsl(name: &str) -> &'static str {
    match name {
        "screen" => "fn blend(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> { return a + b * max(vec3<f32>(1.0) - a, vec3<f32>(0.0)); }",
        "difference" => "fn blend(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> { return abs(a - b); }",
        // A pure multiply goes black in a feedback loop: the emitter reveals the feedback instead.
        "multiply" => "fn blend(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> { return a * (vec3<f32>(0.4) + 1.6 * b) + 0.05 * b; }",
        "lighten" => "fn blend(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> { return max(a, b); }",
        _ => "fn blend(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> { return a + b; }",
    }
}
```

- [ ] `wgsl()` dispatch (the engine-1 branch is byte-for-byte the old body):

```rust
pub fn wgsl(g: &Genome, blocks: &Blocks) -> Result<String> {
    let [m, e, c] = blocks.parts(g)?;
    if !g.is_v2() {
        return Ok(TEMPLATE.replace("{IMG}", &img_wgsl(g)).replace("{MOTION}", &m.wgsl).replace("{EMITTER}", &e.wgsl).replace("{COLOUR}", &c.wgsl));
    }
    let r = crate::regime::get(&g.regime);
    Ok(TEMPLATE_V2
        .replace("{IMG}", &img_wgsl(g))
        .replace("{SPACE}", blocks.space_part(g)?.map_or(NO_SPACE, |b| &b.wgsl))
        .replace("{MOTION}", &m.wgsl)
        .replace("{EMITTER}", &e.wgsl)
        .replace("{COLOUR}", &c.wgsl)
        .replace("{BLEND}", blend_wgsl(&g.blend))
        .replace("{DECAY}", &format!("{:?}", r.map_or(1.0, |r| r.decay)))
        .replace("{DRIVE}", &format!("{:?}", r.map_or(1.0, |r| r.drive))))
}
```

- [ ] `to_scene`: change `let mut slots = vec![(Some(m), SLOTS[0]), …]` to type `Vec<(Option<&Block>, usize)>` (already inferred). After the `has_layers` extend:

```rust
    // Space params live in slots 14..15 (after the layers' 8..13, padded when there are none).
    if let Some(sp) = blocks.space_part(g)? {
        if !g.has_layers() {
            slots.extend([(None, SLOTS[3]), (None, SLOTS[4])]);
        }
        slots.push((Some(sp), SLOTS[5]));
    }
```

  Update the doc comment: "8 param slots, 14 with layers, 16 with a space block".

### Task P3.2: cosine palettes, random_v2, mutate_v2

- [ ] Add:

```rust
/// Seed-made palette from Inigo Quilez's cosine formula a + b·cos(2π(c·t + d)) (c picks the banding:
/// 0.5/1/2/3 cycles), sampled at t = 0.25/0.5/0.75; entry 0 is the t = 0 colour darkened to near black
/// (blocks treat entry 0 as the background). `r` = 64-bit hash.
// ponytail: sampled into the 4-colour bank format, so banding only shows through 3 samples; a uniform-driven pal() if that's too coarse.
pub fn cosine_palette(r: u64) -> [String; 4] {
    let u = |k: u64| (crate::types::mix64(r.wrapping_add(k)) >> 40) as f32 / (1u64 << 24) as f32;
    let freq = [0.5, 1.0, 2.0, 3.0][(crate::types::mix64(r) % 4) as usize];
    let ch = |k: u64| (0.5 + 0.1 * (u(k) - 0.5), 0.35 + 0.15 * u(k + 3), freq * (0.9 + 0.2 * u(k + 6)), u(k + 9));
    let abcd = [ch(1), ch(2), ch(3)];
    let at = |t: f32| abcd.map(|(a, b, c, d)| (a + b * (std::f32::consts::TAU * (c * t + d)).cos()).clamp(0.0, 1.0));
    let hex = |c: [f32; 3]| format!("#{:02x}{:02x}{:02x}", (c[0] * 255.0).round() as u8, (c[1] * 255.0).round() as u8, (c[2] * 255.0).round() as u8);
    [hex(at(0.0).map(|x| x * 0.08)), hex(at(0.25)), hex(at(0.5)), hex(at(0.75))]
}

fn pick_blend(h: u64) -> String {
    if h % 2 == 0 { String::new() } else { BLENDS[((h >> 8) % BLENDS.len() as u64) as usize].into() }
}

/// Engine-2 random genome: `random`'s blocks plus a space fold (none a third of the time), a blend mode
/// (add half the time), a cosine palette half the time, and the segment's `regime` with its finishing look
/// (mono tints with palette colour 2). `r` = 64-bit hash.
pub fn random_v2(blocks: &Blocks, r: u64, regime: &crate::regime::Regime) -> Genome {
    let h = |k: u64| crate::types::hash(r, k);
    let mut g = random(blocks, r);
    if !blocks.space.is_empty() && h(1) % 3 != 0 {
        g.space = nth_id(&blocks.space, h(2) as usize);
    }
    g.blend = pick_blend(h(3));
    if h(4) % 2 == 0 {
        g.palette = cosine_palette(h(5));
    }
    g.regime = regime.name.into();
    g.post = regime.post;
    if g.post.tint_amount > 0.0 {
        g.post.tint = crate::scenes::hex(&g.palette[2]).unwrap_or([1.0; 3]);
    }
    g
}

/// Engine-2 mutation: a third of the time swap the space fold (or drop it), a third the blend mode, else
/// `mutate` (blocks / palette / params). The regime never changes: it belongs to the segment.
pub fn mutate_v2(g: &Genome, blocks: &Blocks, r: u64) -> Genome {
    let h = |k: u64| crate::types::hash(r, k);
    let mut out = match h(1) % 3 {
        0 if !blocks.space.is_empty() => Genome { space: if h(2) % 4 == 0 { String::new() } else { nth_id(&blocks.space, h(3) as usize) }, ..g.clone() },
        1 => Genome { blend: pick_blend(h(4)), ..g.clone() },
        _ => mutate(g, blocks, r),
    };
    out.prune(blocks);
    out
}
```

### Task P3.3: tests

- [ ] Append to `mod tests`:

```rust
    fn space_block(id: &str, wgsl: &str) -> Block {
        let pr = |name: &str, min, max, fixed| ParamRange { name: name.into(), min, max, fixed };
        Block { id: id.into(), params: vec![pr("n", 3.0, 6.0, true), pr("s", 0.1, 0.2, false)], wgsl: wgsl.into(), toml: String::new() }
    }

    #[test]
    fn v2_template_space_slots_and_blend() {
        let mut b = blocks().pinned(1);
        b.add("space", space_block("space_test", "fn space(p: vec2<f32>) -> vec2<f32> { return abs(p) * SP(0); }")).unwrap();
        let g = Genome { space: "space_test".into(), blend: "difference".into(), regime: "dark".into(), ..genome() };
        let src = wgsl(&g, &b).unwrap();
        assert!(src.contains("fn SP(") && src.contains("abs(p) * SP(0)") && src.contains("abs(a - b)") && src.contains("* 0.97, 0.995)"));
        let sc = to_scene(&g, &b, Path::new(".")).unwrap();
        assert_eq!(sc.params.len(), 16);
        assert_eq!((sc.params[14].min, sc.params[15].max), (3.0, 0.2));
        assert!(sc.params[8..14].iter().all(|p| p.fixed && p.max == 0.0));
        assert!(!wgsl(&genome(), &b).unwrap().contains("fn space"), "no genes: engine-1 recipe");
        let calm = wgsl(&Genome { regime: "calm".into(), ..genome() }, &b).unwrap();
        assert!(calm.contains(NO_SPACE) && calm.contains("return a + b;"));
    }

    #[test]
    fn random_v2_varies_every_gene_deterministically() {
        let mut b = blocks().pinned(1);
        for k in 0..3 {
            b.add("space", space_block(&format!("space_t{k}"), "")).unwrap();
        }
        let reg = &crate::regime::REGIMES[1];
        let gs: Vec<Genome> = (0..48).map(|r| random_v2(&b, crate::types::hash(9, r), reg)).collect();
        assert_eq!(gs[3], random_v2(&b, crate::types::hash(9, 3), reg));
        let distinct = |f: &dyn Fn(&Genome) -> String| gs.iter().map(f).collect::<std::collections::HashSet<_>>().len();
        assert_eq!(distinct(&|g| g.space.clone()), 4, "none + 3 folds");
        assert_eq!(distinct(&|g| g.blend.clone()), 5, "add + 4 modes");
        assert!(distinct(&|g| g.palette.join("")) > 30, "bank + cosine palettes");
        for (i, g) in gs.iter().enumerate() {
            assert!(crate::scenes::palette(&g.palette).is_ok());
            assert_eq!(mutate_v2(g, &b, crate::types::hash(4, i as u64)).regime, reg.name);
        }
        let mono = random_v2(&b, 5, crate::regime::get("mono").unwrap());
        assert_eq!(mono.post.tint, crate::scenes::hex(&mono.palette[2]).unwrap());
    }

    #[test]
    fn cosine_palettes_are_dark_first_and_varied() {
        let ps: std::collections::HashSet<[String; 4]> = (0..32u64).map(|r| cosine_palette(crate::types::hash(3, r))).collect();
        assert!(ps.len() >= 30);
        for p in &ps {
            let lin = crate::scenes::palette(p).unwrap();
            assert!(lin[0].iter().all(|&c| c < 0.01), "entry 0 must be near black: {p:?}");
        }
    }

    /// Engine-2 blocks prefix every helper with their id, so any combination links.
    #[test]
    fn engine2_block_helpers_are_prefixed() {
        let b = blocks().pinned(2);
        let contract = ["motion", "motion_decay", "emit", "colour", "space"];
        for blk in [&b.motion, &b.emitter, &b.space].into_iter().flatten().filter(|x| V2_BLOCKS.contains(&x.id.as_str())) {
            for line in blk.wgsl.lines().filter(|l| l.starts_with("fn ") || l.starts_with("const ")) {
                let name = line.split([' ', '(', ':']).nth(1).unwrap_or_default();
                assert!(contract.contains(&name) || name.starts_with(&blk.id), "{}: helper {name} needs the prefix {}", blk.id, blk.id);
            }
        }
    }
```

  The "add + 4 modes" assert needs `pick_blend` to cover all 4 modes within 48 draws (~24 odd draws). If it flakes on this exact hash set, raise 48 → 96. Don't weaken the assert.
- [ ] `cargo test --release v2_template random_v2 cosine engine1 library_pins` → green. (`engine2_block_helpers_are_prefixed` passes trivially until P5 lands; it runs at the gate.) Commit `feat(core): engine-2 genome (space, blend, regime, cosine palettes)`.

---

## Phase P4: Music lock · [opus] · owns `analysis.rs`, `director.rs` (frame_state/mods), `finish.wgsl`, `engine.rs` (FinishUniforms)

### Task P4.1: snares in analysis

- [ ] Rename `detect_kicks` to a general detector. The kick path must stay exactly the same (`env = kflux`, `other = hflux`, `ratio = 1.5`):

```rust
/// Onset peaks of `env`, skipping frames where `env < ratio * other` (the other band dominates).
/// Kicks: (kflux, hflux, 1.5). Snares/claps: (hflux, kflux, 1.0).
fn detect_onsets(fr: &[Frame], fps: f64, env: fn(&Frame) -> f32, other: fn(&Frame) -> f32, ratio: f32) -> Vec<Kick> {
    let e: Vec<f32> = fr.iter().map(env).collect();
    // … body of detect_kicks with `env` → `e` and the dominance test replaced by:
    //     if env(&fr[i]) < ratio * other(&fr[i]) { continue; }
}
```

  Do it mechanically: copy the old body, rename the local `env` to `e`, and replace only the `if fr[i].kflux < 1.5 * fr[i].hflux` line and its comment (`// the other band dominates: not this kind of onset`).
- [ ] In `analyze()`:
  - Use `let kicks = detect_onsets(fr, fps, |f| f.kflux, |f| f.hflux, 1.5);` and `let snares = detect_onsets(fr, fps, |f| f.hflux, |f| f.kflux, 1.0);`.
  - In the literal, set `version: crate::types::ANALYSIS_VERSION` and `snares`.
- [ ] Test (in `analysis.rs` tests):

```rust
    #[test]
    fn onsets_split_kicks_from_snares() {
        let fps = 100.0;
        let mut fr: Vec<Frame> = (0..1000).map(|_| Frame { kflux: 0.1, hflux: 0.1, ..Default::default() }).collect();
        for i in (50..1000).step_by(100) {
            fr[i].kflux = 5.0;
        }
        for i in (100..1000).step_by(100) {
            fr[i].hflux = 5.0;
        }
        let kicks = detect_onsets(&fr, fps, |f| f.kflux, |f| f.hflux, 1.5);
        let snares = detect_onsets(&fr, fps, |f| f.hflux, |f| f.kflux, 1.0);
        assert_eq!((kicks.len(), snares.len()), (10, 9));
        assert!((snares[0].time - (1.0 + ONSET_LAG)).abs() < 0.011);
    }
```

### Task P4.2: hits, anticipation, glitch gating, v2 mods (`director.rs`)

- [ ] Helpers (below `sample`):

```rust
/// Strongest envelope of the last 4 onsets at `t`: 10 ms attack, exponential decay `decay` s.
fn onset_env(xs: &[Kick], t: f64, decay: f64) -> f64 {
    let k = xs.partition_point(|k| k.time <= t + 0.01);
    xs[k.saturating_sub(4)..k]
        .iter()
        .map(|kk| {
            let dt = t - kk.time;
            let env = if dt < 0.0 { 1.0 + dt / 0.01 } else { (-dt / decay).exp() };
            kk.strength as f64 * env
        })
        .fold(0.0, f64::max)
}

/// Seconds since the last onset at or before `t` (1e3 if none).
fn since(xs: &[Kick], t: f64) -> f64 {
    xs[..xs.partition_point(|k| k.time <= t)].last().map_or(1e3, |k| t - k.time)
}
```

- [ ] `frame_state`:
  - Replace the `kick` computation with `let kick = onset_env(&a.kicks, t, 0.12);` and `since_kick` with `let since_kick = since(&a.kicks, t);`. Both are identical arithmetic; the golden proves it.
  - After `drop_env`/`drop_dt`:

```rust
    // Half a bar before each drop: 0 → drop strength (engine 2 darkens and drains towards it; shaders squeeze).
    let antic = a.drops.get(d).map_or(0.0, |dr| {
        let pre = dr.time - t;
        if pre > 0.0 && pre < bar / 2.0 { smoothstep((1.0 - pre / (bar / 2.0)) as f32) * dr.strength } else { 0.0 }
    });
    let snare = onset_env(&a.snares, t, 0.09);
```

  - In `base`, add: `hits: [snare as f32, onset_env(&a.kicks, t, 0.045) as f32, since(&a.snares, t).min(1e3) as f32, antic],`
  - In `build`'s `src` closure, add the arms `ModSource::Snare => base.hits[0], ModSource::Punch => base.hits[1],`.
  - Replace the `drop_flash` line and the `FrameState` exposure/saturation/flash/post with:

```rust
    let v2 = plan.engine >= 2;
    let drop_flash = if drop_dt < beat_len { if v2 { 0.8 } else { 0.4 } * drop_env } else { 0.0 };
    // … post_of / post as before, then:
    let mut post = post;
    if post.strobe + post.invert + post.rgb_split + post.tear > 0.0 {
        // Flashes (strobe, invert) only on downbeats: ≤ 1 per bar, under the 3 Hz flash limit (WCAG 2.3.1)
        // below 720 BPM. Displacement glitches (split, tear) ride the snares.
        let since_down = a.downbeats[..a.downbeats.partition_point(|&x| x <= t)].last().map_or(1e3, |&x| t - x);
        let down = (-since_down / 0.08).exp() as f32;
        post.strobe *= down;
        post.invert *= down;
        post.rgb_split *= snare as f32;
        post.tear *= snare as f32;
    }
    let squeeze = if v2 { 1.0 - antic } else { 1.0 };
    FrameState {
        scene_a,
        a: ua,
        b,
        mix,
        flash: (0.06 * kick + trans_flash + drop_flash).clamp(0.0, 1.0) as f32,
        exposure: (1.0 + 0.1 * energy) * (1.0 - 0.85 * (1.0 - squeeze)),
        saturation: (1.0 - 0.45 * tension + 0.15 * drop_env as f32).clamp(0.4, 1.2) * (1.0 - 0.8 * (1.0 - squeeze)),
        post,
    }
```

  At `squeeze = 1.0` both factors are exactly `1.0 - 0.0 = 1.0`, so engine-1 floats don't change; the golden proves it. `ua` and `b` already carry `base.hits` because `build` copies `base`.
- [ ] Mods:

```rust
/// Engine 2 adds onset sources (appended, so engine-1 picks are unchanged).
const SOURCES_V2: [ModSource; 9] = [ModSource::Kick, ModSource::Bass, ModSource::Mid, ModSource::High, ModSource::Energy, ModSource::Tension, ModSource::Drop, ModSource::Snare, ModSource::Punch];
```

  `fn mods(sc: &Scene, seed: u64, engine: u32)`: add `let sources: &[ModSource] = if engine >= 2 { &SOURCES_V2 } else { &SOURCES };` and use `sources` in place of `SOURCES`. The two call sites in `plan()` pass `req.engine`.
- [ ] Test:

```rust
    #[test]
    fn engine2_hits_anticipation_and_gated_glitch() {
        let mut a = test_analysis();
        a.snares = vec![Kick { time: 31.0, strength: 1.0 }];
        let scenes = test_scenes();
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Auto, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 0.0, vibe: None, genome_for: None, blocks: None, gen_dir: Path::new("."), shots: &[], engine: 2 };
        let mut p = plan(&req).unwrap();
        let f = frame_state(&p, &a, &scenes, 31.02, 64, 64, Quality::Draft);
        assert!(f.a.hits[0] > 0.7 && (f.a.hits[2] - 0.02).abs() < 1e-3, "{:?}", f.a.hits);
        // Drop at 64 s, bar 2 s: 0.25 s before it, anticipation = smoothstep(0.75) ≈ 0.84 and the frame darkens
        // to ≈ 0.30 exposure (engine 2 only).
        let pre = frame_state(&p, &a, &scenes, 63.75, 64, 64, Quality::Draft);
        assert!(pre.a.hits[3] > 0.8 && pre.exposure < 0.6, "{:?} {}", pre.a.hits, pre.exposure);
        p.engine = 1;
        assert!(frame_state(&p, &a, &scenes, 63.75, 64, 64, Quality::Draft).exposure > 1.0);
        p.engine = 2;
        p.segments[0].genome = Some(Genome { post: Post { strobe: 1.0, tear: 1.0, ..Default::default() }, ..Default::default() });
        let on = frame_state(&p, &a, &scenes, 32.0, 64, 64, Quality::Draft);
        let off = frame_state(&p, &a, &scenes, 33.0, 64, 64, Quality::Draft);
        assert!(on.post.strobe > 0.99 && off.post.strobe < 0.01, "strobe only on downbeats");
        assert!(frame_state(&p, &a, &scenes, 31.02, 64, 64, Quality::Draft).post.tear > 0.7 && off.post.tear < 0.01, "tear rides the snare");
    }
```

### Task P4.3: finish glitches (`finish.wgsl`, `engine.rs`)

- [ ] `finish.wgsl`, `struct F`: after `p1` add `p2: vec4<f32>, // strobe, rgb split, tear, invert (0 = off; the director gates them on onsets)`.
- [ ] In `fs_main`:
  - After the `vhs_uv` line: `if (f.p2.z > 0.0) { uv = tear_uv(uv, f.p2.z, frame); }`
  - After `let ca = …;`, make `ca` a `var` and add `if (f.p2.y > 0.0) { ca += vec2<f32>(f.p2.y * 0.015, 0.0); }`
  - After the posterize block:

```wgsl
    if (f.p2.w > 0.0) { c = mix(c, vec3<f32>(1.0) - c, f.p2.w); }
    if (f.p2.x > 0.0) { c = mix(c, vec3<f32>(1.0), 0.7 * f.p2.x); }
```

  - Above `fs_main`:

```wgsl
// Tear: a few of 24 horizontal bands jump sideways (re-rolled every frame while the snare rings).
fn tear_uv(uv: vec2<f32>, amt: f32, frame: u32) -> vec2<f32> {
    let band = floor(uv.y * 24.0);
    let on = step(0.7, rnd(vec2<f32>(1.0, band), frame, 6u));
    return vec2<f32>(uv.x + amt * 0.08 * (rnd(vec2<f32>(0.0, band), frame, 5u) - 0.5) * on, uv.y);
}
```

- [ ] `engine.rs`:
  - `FinishUniforms` gets `p2: [f32; 4], // strobe, rgb split, tear, invert`.
  - In `finish()`, set `p2: [fs.post.strobe, fs.post.rgb_split, fs.post.tear, fs.post.invert],`.
  - In the test's layered `Post { … }`, set `strobe: 1.0, rgb_split: 1.0, tear: 1.0, invert: 1.0`.
- [ ] `cargo test --release onsets engine2_hits engine1 mods` → green. Commit `feat(core): onset hits, drop anticipation, gated glitch looks`.

---

## Phase P5: Engine-2 blocks · [opus ×4, parallel] · each owns only its new files

Rules (CONTRACT.md):
- Prefix every helper with the block id.
- Params `MP/EP/SP` never multiply `time()` unless `fixed = true`.
- Cap raymarch steps with `select(…, …, quality() > 0.5)`.
- No derivatives; no texture sampling other than `prev()`.
- Write `1.0 - smoothstep(a, b, x)`, never reversed edges.

Each block is `<id>.wgsl` + `<id>.toml`; ids must match `V2_BLOCKS`. Nothing to run: the gate's `engine_runs` compiles each block.

### P5a: space blocks (`kind = "space"`, ≤ 2 params)

- [ ] `space_mirror.toml`

```toml
kind = "space"
description = "Mirror: the frame folds across one or two slowly turning axes, so every look becomes symmetric"
[[param]]
name = "axes"       # 1 or 2 mirror axes
min = 1.0
max = 2.99
fixed = true
[[param]]
name = "spin"       # rad/s
min = -0.05
max = 0.05
fixed = true
```

  `space_mirror.wgsl`

```wgsl
// Mirror: the plane folds across one (or two) axes that slowly turn, so trails and shapes become symmetric.
fn space(p: vec2<f32>) -> vec2<f32> {
    var q = rot2(SP(1) * time()) * p;
    q.x = abs(q.x);
    if (SP(0) > 1.5) { q.y = abs(q.y); }
    return q;
}
```

- [ ] `space_kaleido.toml`

```toml
kind = "space"
description = "Kaleidoscope: N mirrored wedges around the centre; the wedge pattern turns slowly and breathes on kicks"
[[param]]
name = "segments"
min = 3.0
max = 12.0
fixed = true
[[param]]
name = "spin"       # rad/s
min = -0.04
max = 0.04
fixed = true
```

  `space_kaleido.wgsl`

```wgsl
// Kaleidoscope: N mirrored wedges around the centre; the pattern turns slowly and breathes on kicks.
fn space(p: vec2<f32>) -> vec2<f32> {
    let seg = TAU / floor(SP(0));
    var a = atan2(p.y, p.x) + SP(1) * time();
    a = a - seg * floor(a / seg);
    a = abs(a - 0.5 * seg);
    return vec2<f32>(cos(a), sin(a)) * length(p) * (1.0 - 0.04 * kick());
}
```

- [ ] `space_kifs.toml`

```toml
kind = "space"
description = "Crystal: a 2D kaleidoscopic IFS; rounds of mirror-fold, rotate, scale and shift grow a branching crystal lattice"
[[param]]
name = "folds"
min = 2.0
max = 5.99
fixed = true
[[param]]
name = "scale"
min = 1.3
max = 1.9
```

  `space_kifs.wgsl`

```wgsl
// Crystal: a 2D kaleidoscopic IFS. Each round mirror-folds, rotates, scales and shifts the plane, so the
// image repeats as a branching crystal lattice; the fold angle drifts with the phrase.
fn space(p: vec2<f32>) -> vec2<f32> {
    let k = i32(SP(0));
    let s = SP(1);
    let ang = 0.55 + 0.2 * sin(TAU * phrase_phase());
    let off = vec2<f32>(0.6, 0.25);
    var q = p;
    for (var i = 0; i < k; i++) {
        q = rot2(ang) * abs(q);
        q = q * s - off * (s - 1.0);
    }
    return q / pow(s, f32(k));
}
```

- [ ] `space_droste.toml`

```toml
kind = "space"
description = "Droste: log-polar spiral; the frame repeats inside itself in an endlessly zooming, twisting tunnel (Escher's Print Gallery)"
[[param]]
name = "twist"      # turns per copy (multiplies time via the zoom)
min = -1.5
max = 1.5
fixed = true
[[param]]
name = "zoom_speed" # log-scale units per s
min = 0.05
max = 0.25
fixed = true
```

  `space_droste.wgsl`

```wgsl
// Droste: log-polar repeat. Each ring of the plane (ratio 2.4) maps onto the same band, so the frame
// repeats inside itself; the band slides with time (endless zoom) and the angle twists with depth.
fn space(p: vec2<f32>) -> vec2<f32> {
    let lr = log(2.4);
    let lz = log(max(length(p), 1e-4)) - SP(1) * time();
    let band = lz - lr * floor(lz / lr);
    let a = atan2(p.y, p.x) + SP(0) * lz;
    return vec2<f32>(cos(a), sin(a)) * exp(band) * 0.42;
}
```

- [ ] `space_polar.toml`

```toml
kind = "space"
description = "Turbine: the plane is cut into N rotated (unmirrored) copies around the centre and into repeating rings"
[[param]]
name = "copies"
min = 3.0
max = 9.0
fixed = true
[[param]]
name = "rings"      # rings per unit radius
min = 1.0
max = 4.0
fixed = true
```

  `space_polar.wgsl`

```wgsl
// Turbine: N rotated copies around the centre (no mirror, so motion spins like a turbine) and rings that
// repeat outward (every ring maps onto the second one).
fn space(p: vec2<f32>) -> vec2<f32> {
    let seg = TAU / floor(SP(0));
    let a = atan2(p.y, p.x);
    let a2 = a - seg * floor(a / seg) - 0.5 * seg;
    let ring = 1.0 / SP(1);
    let r = length(p);
    let rr = r - ring * floor(r / ring) + ring;
    return vec2<f32>(cos(a2), sin(a2)) * rr;
}
```

- [ ] `space_hyper.toml`

```toml
kind = "space"
description = "Hyperbolic: the plane folds into a {p,q} tiling of the Poincare disk (Escher's Circle Limit), drifting slowly"
[[param]]
name = "tiling"     # 0 {4,5}, 1 {5,4}, 2 {3,7}, 3 {6,4}
min = 0.0
max = 3.99
fixed = true
[[param]]
name = "drift"      # rad/s of the Mobius drift
min = 0.05
max = 0.2
fixed = true
```

  `space_hyper.wgsl`

```wgsl
// Hyperbolic: folds the plane into the fundamental triangle of a {p,q} tiling of the Poincare disk.
// Mirrors: the x-axis, a line at angle pi/p, and a circle orthogonal to the unit circle (centre d, radius r
// with d^2 = 1 + r^2 and cos(pi/q) = d sin(pi/p) / r). A Mobius translation drifts through the tiling.
fn space_hyper_cdiv(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x + a.y * b.y, a.y * b.x - a.x * b.y) / max(dot(b, b), 1e-6);
}

fn space(p: vec2<f32>) -> vec2<f32> {
    var q = p * 0.55;
    if (dot(q, q) > 1.0) { q = q / dot(q, q); }   // outside the disk: its mirror image inside
    let w = 0.35 * vec2<f32>(cos(SP(1) * time()), sin(SP(1) * time() * 0.7));
    q = space_hyper_cdiv(q - w, vec2<f32>(1.0 - (w.x * q.x + w.y * q.y), -(w.x * q.y - w.y * q.x)));
    let k = i32(SP(0));
    let pn = select(select(select(4.0, 5.0, k == 1), 3.0, k == 2), 6.0, k == 3);
    let qn = select(select(select(5.0, 4.0, k == 1), 7.0, k == 2), 4.0, k == 3);
    let a = PI / pn;
    let b = PI / qn;
    let d = 1.0 / sqrt(1.0 - sin(a) * sin(a) / (cos(b) * cos(b)));
    let r = d * sin(a) / cos(b);
    let c = vec2<f32>(d, 0.0);
    for (var i = 0; i < 24; i++) {
        let ang = atan2(q.y, q.x);
        var m = ang - 2.0 * a * floor(ang / (2.0 * a));
        m = select(m, 2.0 * a - m, m > a);
        q = vec2<f32>(cos(m), sin(m)) * length(q);
        let v = q - c;
        let l2 = dot(v, v);
        if (l2 >= r * r) { break; }
        q = c + v * (r * r / l2);
    }
    return q * 2.2;
}
```

  The Möbius denominator is `1 − conj(w)·q`, with `conj(w)·q = (w.x q.x + w.y q.y) + i(w.x q.y − w.y q.x)`.

### P5b: feedback-operator motion blocks (`kind = "motion"`)

- [ ] `motion_mosh.toml`

```toml
kind = "motion"
description = "Datamosh: the frame is cut into blocks; each kick re-rolls which blocks slide and where, smearing in P-frame chunks"
[[param]]
name = "blocks"     # blocks across the height
min = 6.0
max = 24.0
fixed = true
[[param]]
name = "share"      # share of blocks that slide
min = 0.2
max = 0.7
[[param]]
name = "persistence"
min = 0.94
max = 0.985
```

  `motion_mosh.wgsl`

```wgsl
// Datamosh: the frame is cut into blocks; each kick re-rolls which blocks slide and in which direction,
// so the image smears in blocky P-frame chunks instead of flowing smoothly.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let n = MP(0);
    let cell = floor(uv * vec2<f32>(n * aspect(), n));
    let kid = floor((time() - since_kick()) * 4.0);   // constant between kicks
    let a = TAU * hash21(cell - kid * 3.0);
    let moving = step(1.0 - MP(1), hash21(cell + kid * 17.0));
    let speed = (1.5 + 6.0 * punch() + 4.0 * drop_env()) / resolution();
    let base = warp(uv, 1.0 + 0.002 * energy(), 0.0, 0.0, vec2<f32>(0.0));
    return base - vec2<f32>(cos(a), sin(a)) * speed * moving;
}

fn motion_decay() -> f32 {
    return MP(2) * (1.0 - 0.03 * drop_env());
}
```

- [ ] `motion_sort.toml`

```toml
kind = "motion"
description = "Pixel sort: pixels brighter than a threshold smear along one direction, dark ones stay put; the image drips in streaks"
[[param]]
name = "threshold"
min = 0.15
max = 0.5
[[param]]
name = "run"        # px per step
min = 1.0
max = 4.0
[[param]]
name = "persistence"
min = 0.95
max = 0.985
```

  `motion_sort.wgsl`

```wgsl
// Pixel sort: bright pixels (above a threshold that dips on kicks) smear one way (vertical or sideways
// by seed) while dark ones stay put, so the image drips and streaks like a sorted glitch photo.
fn motion_sort_luma(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let px = 1.0 / resolution();
    let dir = select(vec2<f32>(0.0, -1.0), vec2<f32>(1.0, 0.0), fract(seed() * 31.0) > 0.5);
    let l = motion_sort_luma(prev(uv - dir * px * 2.0));
    let thr = MP(0) * (1.0 - 0.3 * kick());
    let run = smoothstep(thr, thr + 0.1, l) * (MP(1) + 4.0 * drop_env());
    return uv - dir * px * run;
}

fn motion_decay() -> f32 {
    return MP(2);
}
```

- [ ] `motion_etch.toml`

```toml
kind = "motion"
description = "Etch: the image flows along its own brightness contours, engraving rippling lines; kicks push it across the edges"
[[param]]
name = "flow"       # px per step along contours
min = 0.8
max = 3.0
[[param]]
name = "push"       # px per step across edges on kicks
min = 1.0
max = 5.0
[[param]]
name = "persistence"
min = 0.93
max = 0.98
```

  `motion_etch.wgsl`

```wgsl
// Etch: the image flows along its own brightness contours (perpendicular to the luma gradient), so
// shapes get engraved into rippling lines; kicks and drops push it across the edges, crystallising it.
fn motion_etch_l(uv: vec2<f32>) -> f32 { return dot(prev(uv), vec3<f32>(0.2126, 0.7152, 0.0722)); }

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let e = 2.0 / resolution();
    let g = vec2<f32>(
        motion_etch_l(uv + vec2<f32>(e.x, 0.0)) - motion_etch_l(uv - vec2<f32>(e.x, 0.0)),
        motion_etch_l(uv + vec2<f32>(0.0, e.y)) - motion_etch_l(uv - vec2<f32>(0.0, e.y)),
    );
    let gn = g / (length(g) + 0.05);
    let along = vec2<f32>(-gn.y, gn.x) * MP(0);
    let across = gn * MP(1) * (punch() + drop_env());
    let base = warp(uv, 1.0 + 0.001 * energy(), 0.0, 0.0, vec2<f32>(0.0));
    return base + (along + across) / resolution();
}

fn motion_decay() -> f32 {
    return MP(2);
}
```

- [ ] `motion_erode.toml`

```toml
kind = "motion"
description = "Erode: each pixel copies its darkest neighbour, so darkness eats shapes from the edges; on drops it dilates instead"
[[param]]
name = "radius"     # px
min = 1.0
max = 3.0
[[param]]
name = "zoom"
min = 0.0
max = 0.004
[[param]]
name = "persistence"
min = 0.9
max = 0.97
```

  `motion_erode.wgsl`

```wgsl
// Erode: each pixel copies its darkest of 4 rotating neighbours, so darkness eats into bright shapes and
// only fresh emission survives; during a drop it takes the brightest instead and everything swells.
fn motion_erode_l(uv: vec2<f32>) -> f32 { return dot(prev(uv), vec3<f32>(0.2126, 0.7152, 0.0722)); }

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let r = MP(0) / resolution();
    let grow = drop_env() > 0.3;
    var best = uv;
    var bl = motion_erode_l(uv);
    for (var i = 0; i < 4; i++) {
        let a = TAU * (f32(i) + 0.5) / 4.0 + time() * 0.3;   // rotating taps avoid grid artefacts
        let q = uv + vec2<f32>(cos(a), sin(a)) * r;
        let l = motion_erode_l(q);
        if ((l < bl) != grow) { best = q; bl = l; }
    }
    return warp(best, 1.0 + MP(1), 0.0, 0.0, vec2<f32>(0.0));
}

fn motion_decay() -> f32 {
    return MP(2);
}
```

### P5c: 2D emitters (`kind = "emitter"`)

- [ ] `emit_warpfbm.toml`

```toml
kind = "emitter"
description = "Marble: domain-warped fbm (noise fed through noise twice) folds into marbled liquid veins; kicks deepen the warp"
[[param]]
name = "scale"
min = 1.0
max = 3.0
[[param]]
name = "warp"
min = 2.0
max = 5.0
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_warpfbm.wgsl`

```wgsl
// Marble: Inigo Quilez's domain warping, fbm(p + k·fbm(p + k·fbm(p))), so the noise folds into marbled
// liquid veins. Kicks and drops deepen the warp; the hue follows the warp field.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let t = time() * 0.05;
    let s = p * EP(0);
    let q = vec2<f32>(fbm(s + vec2<f32>(0.0, t)), fbm(s + vec2<f32>(5.2, 1.3 - t)));
    let k = EP(1) * (1.0 + 0.6 * punch() + 0.8 * drop_env());
    let r = vec2<f32>(fbm(s + k * q + vec2<f32>(1.7, 9.2) + 0.15 * t), fbm(s + k * q + vec2<f32>(8.3, 2.8) - 0.12 * t));
    let f = fbm(s + k * r);
    let i = smoothstep(0.45, 0.85, f) * (0.5 + 0.8 * length(q));
    return vec2<f32>(i * 0.9, fract(EP(2) + 0.6 * r.x + 0.2 * q.y));
}
```

- [ ] `emit_cells.toml`

```toml
kind = "emitter"
description = "Cells: a drifting Voronoi field with glowing walls and nuclei; each kick lights a random few cells"
[[param]]
name = "density"    # cells per unit
min = 2.0
max = 6.0
[[param]]
name = "wall"       # wall width
min = 0.03
max = 0.12
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_cells.wgsl`

```wgsl
// Cells: a Worley/Voronoi field. Bright walls (second-nearest minus nearest distance) and glowing nuclei;
// the seeds drift, and each kick lights a random few cells.
fn emit_cells_h2(c: vec2<f32>) -> vec2<f32> { return vec2<f32>(hash21(c), hash21(c + vec2<f32>(19.1, 7.3))); }

fn emit(p: vec2<f32>) -> vec2<f32> {
    let s = p * EP(0);
    let ip = floor(s);
    let fp = fract(s);
    var d1 = 8.0;
    var d2 = 8.0;
    var id = vec2<f32>(0.0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let g = vec2<f32>(f32(x), f32(y));
            let o = vec2<f32>(0.5) + 0.45 * sin(vec2<f32>(time() * 0.4) + TAU * emit_cells_h2(ip + g));
            let d = length(g + o - fp);
            if (d < d1) { d2 = d1; d1 = d; id = ip + g; } else if (d < d2) { d2 = d; }
        }
    }
    let wall = 1.0 - smoothstep(0.0, EP(1), d2 - d1);
    let nucleus = exp(-d1 * d1 * 40.0) * 0.6;
    let kid = floor((time() - since_kick()) * 4.0);
    let lit = step(0.8, hash21(id + kid)) * kick() * 0.8 * (1.0 - d1);
    return vec2<f32>((wall * 0.9 + nucleus + lit) * (0.8 + 0.5 * drop_env()), fract(EP(2) + 0.3 * hash21(id)));
}
```

- [ ] `emit_truchet.toml`

```toml
kind = "emitter"
description = "Truchet maze: quarter-circle tiles at two scales draw glowing winding paths; a few tiles re-roll every few seconds"
[[param]]
name = "scale"      # tiles per unit
min = 2.0
max = 6.0
[[param]]
name = "width"
min = 0.03
max = 0.1
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_truchet.wgsl`

```wgsl
// Truchet maze: quarter-circle arc tiles; some big tiles split into four small ones (two scales). The
// arcs form endless winding paths; every 4 s a few tiles flip.
fn emit_truchet_arcs(q: vec2<f32>, h: f32, w: f32) -> f32 {
    var f = fract(q) - 0.5;
    if (h > 0.5) { f.x = -f.x; }
    let d = min(abs(length(f - 0.5) - 0.5), abs(length(f + 0.5) - 0.5));
    return 1.0 - smoothstep(0.0, w, d);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let s = p * EP(0) + vec2<f32>(0.0, time() * 0.05);
    let c = floor(s);
    let split = hash21(c + 3.1) > 0.6;
    let flip = select(0.0, floor(time() * 0.25), hash21(c * 0.37) > 0.8);
    var i: f32;
    if (split) {
        let q = s * 2.0;
        i = emit_truchet_arcs(q, hash21(floor(q) + flip), EP(1) * 2.0);
    } else {
        i = emit_truchet_arcs(s, hash21(c + flip), EP(1));
    }
    i *= 0.7 + 0.5 * punch() + 0.5 * drop_env();
    return vec2<f32>(i, fract(EP(2) + 0.25 * hash21(c) + select(0.0, 0.33, split)));
}
```

- [ ] `emit_hypertile.toml`

```toml
kind = "emitter"
description = "Hyperbolic tiling: glowing polygon edges of a {p,q} Poincare-disk tiling shrinking to the rim; kicks light alternate tiles"
[[param]]
name = "tiling"     # 0 {4,5}, 1 {5,4}, 2 {3,7}, 3 {6,4}
min = 0.0
max = 3.99
fixed = true
[[param]]
name = "line"       # edge width at the centre
min = 0.01
max = 0.04
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_hypertile.wgsl`

```wgsl
// Hyperbolic tiling (Escher's Circle Limit): fold each point into the fundamental triangle of a {p,q}
// tiling of the Poincare disk; the polygon edges are the circle mirror, so glow at distance to it.
// The disk drifts by a Mobius translation; kicks light every other tile (parity of the inversions).
fn emit_hypertile_cdiv(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x + a.y * b.y, a.y * b.x - a.x * b.y) / max(dot(b, b), 1e-6);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    var q = p / 1.05;
    let rim = dot(q, q);
    if (rim >= 1.0) { return vec2<f32>(0.0); }
    let w = 0.3 * vec2<f32>(cos(time() * 0.07), sin(time() * 0.05));
    q = emit_hypertile_cdiv(q - w, vec2<f32>(1.0 - (w.x * q.x + w.y * q.y), -(w.x * q.y - w.y * q.x)));
    let k = i32(EP(0));
    let pn = select(select(select(4.0, 5.0, k == 1), 3.0, k == 2), 6.0, k == 3);
    let qn = select(select(select(5.0, 4.0, k == 1), 7.0, k == 2), 4.0, k == 3);
    let a = PI / pn;
    let b = PI / qn;
    let d = 1.0 / sqrt(1.0 - sin(a) * sin(a) / (cos(b) * cos(b)));
    let r = d * sin(a) / cos(b);
    let c = vec2<f32>(d, 0.0);
    var n = 0;
    for (var i = 0; i < 24; i++) {
        let ang = atan2(q.y, q.x);
        var m = ang - 2.0 * a * floor(ang / (2.0 * a));
        m = select(m, 2.0 * a - m, m > a);
        q = vec2<f32>(cos(m), sin(m)) * length(q);
        let v = q - c;
        let l2 = dot(v, v);
        if (l2 >= r * r) { break; }
        q = c + v * (r * r / l2);
        n++;
    }
    let lw = max(EP(1) * (1.0 - rim), 1e-4);
    let line = 1.0 - smoothstep(0.0, lw, abs(length(q - c) - r));
    let par = f32(n % 2);
    let lit = kick() * par * 0.5 * (1.0 - rim);
    return vec2<f32>((line + lit) * (0.8 + 0.6 * drop_env()), fract(EP(2) + 0.18 * par + 0.1 * f32(n % 3)));
}
```

- [ ] `emit_lyapunov.toml`

```toml
kind = "emitter"
description = "Lyapunov fractal: logistic map with r alternating AABAB; stable zones glow, chaos stays dark; the window pans slowly"
[[param]]
name = "zoom"       # (a, b) units per centred unit
min = 0.15
max = 0.5
[[param]]
name = "gain"
min = 0.8
max = 2.5
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_lyapunov.wgsl`

```wgsl
// Lyapunov fractal: iterate x -> r x (1 - x) with r taking a or b in the sequence AABAB; the averaged
// log|r (1 - 2x)| is negative in stable zones (glow) and positive in chaos (dark). The window over
// (a, b) space pans slowly; the zero boundary glows on kicks.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let centre = vec2<f32>(3.4, 3.4) + 0.25 * vec2<f32>(sin(time() * 0.03), cos(time() * 0.023));
    let ab = clamp(centre + p * EP(0), vec2<f32>(2.0), vec2<f32>(4.0));
    var x = 0.5;
    var lam = 0.0;
    let n = select(40, 80, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let m = i % 5;
        let r = select(ab.x, ab.y, m == 2 || m == 4);
        if (i >= 10) { lam += log(max(abs(r * (1.0 - 2.0 * x)), 1e-6)); }
        x = r * x * (1.0 - x);
    }
    lam /= f32(n - 10);
    let stable = clamp(-lam * EP(1), 0.0, 1.5);
    let edge = exp(-abs(lam) * 8.0) * (0.5 + kick());
    return vec2<f32>(stable * 0.6 + edge * 0.5, fract(EP(2) + 0.35 * clamp(lam, -1.5, 0.5)));
}
```

- [ ] `emit_orbit.toml`

```toml
kind = "emitter"
description = "Julia orbit traps: z -> z^2 + c with c riding the Mandelbrot cardioid; orbits passing a point and a line draw filigree"
[[param]]
name = "zoom"
min = 0.7
max = 1.4
[[param]]
name = "sharpness"
min = 6.0
max = 20.0
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_orbit.wgsl`

```wgsl
// Julia orbit traps: z -> z^2 + c per pixel, c riding just inside the main cardioid of the Mandelbrot set
// (0.5 e^it - 0.25 e^2it), so the Julia set is always connected filigree. Light is trapped where the orbit
// passes near the origin and near a slowly swaying line; kicks nudge c.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let th = seed() * TAU + time() * 0.015;
    let c = (0.5 * vec2<f32>(cos(th), sin(th)) - 0.25 * vec2<f32>(cos(2.0 * th), sin(2.0 * th))) * (0.98 + 0.01 * kick());
    var z = p * EP(0);
    var dp = 1e3;
    var dl = 1e3;
    let n = select(48, 96, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        z = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
        if (dot(z, z) > 16.0) { break; }
        dp = min(dp, length(z));
        dl = min(dl, abs(z.y + 0.25 * sin(time() * 0.1)));
    }
    let i = exp(-dp * EP(1)) + 0.6 * exp(-dl * EP(1) * 2.0);
    return vec2<f32>(i * (0.6 + 0.5 * drop_env()), fract(EP(2) + 0.5 * dp));
}
```

### P5d: raymarched emitters (`kind = "emitter"`)

- [ ] `emit_kifs3d.toml`

```toml
kind = "emitter"
description = "Fractal cathedral: a raymarched 3D tetrahedral IFS seen from an orbiting camera, glowing by its fold orbit"
[[param]]
name = "scale"
min = 1.8
max = 2.3
[[param]]
name = "twist"      # rotation per fold
min = 0.0
max = 0.6
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_kifs3d.wgsl`

```wgsl
// Fractal cathedral: a raymarched kaleidoscopic IFS (tetrahedral mirror folds, rotate, scale) seen from a
// slowly orbiting camera. Hits glow by their orbit trap; near-misses glow by step count, so the trails
// smear real depth and parallax.
fn emit_kifs3d_de(p0: vec3<f32>) -> vec2<f32> {
    var p = p0;
    var trap = 1e3;
    let s = EP(0);
    let rot = rot2(EP(1));
    for (var i = 0; i < 8; i++) {
        if (p.x + p.y < 0.0) { p = vec3<f32>(-p.y, -p.x, p.z); }
        if (p.x + p.z < 0.0) { p = vec3<f32>(-p.z, p.y, -p.x); }
        if (p.y + p.z < 0.0) { p = vec3<f32>(p.x, -p.z, -p.y); }
        let xz = rot * p.xz;
        p = vec3<f32>(xz.x, p.y, xz.y);
        p = p * s - vec3<f32>(s - 1.0);
        trap = min(trap, dot(p, p));
    }
    return vec2<f32>(length(p) * pow(s, -8.0), trap);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let a = time() * 0.06 + seed() * TAU;
    let ro = vec3<f32>(cos(a) * 3.2, 0.6 + 0.3 * sin(time() * 0.05), sin(a) * 3.2);
    let fw = normalize(-ro);
    let rt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fw));
    let up = cross(fw, rt);
    let rd = normalize(fw * 1.6 + rt * p.x + up * p.y);
    var t = 0.0;
    var hit = -1.0;
    var trap = 0.0;
    var steps = 0.0;
    let n = select(48, 80, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let h = emit_kifs3d_de(ro + rd * t);
        if (h.x < 0.0015 * t) { hit = t; trap = h.y; break; }
        t += h.x;
        steps += 1.0;
        if (t > 12.0) { break; }
    }
    let glow = steps / f32(n);
    if (hit < 0.0) { return vec2<f32>(glow * 0.6, fract(EP(2) + 0.5)); }
    let lt = log(trap + 1.0);
    let shade = exp(-0.25 * hit) * (0.6 + 0.6 * punch() + 0.5 * drop_env());
    return vec2<f32>(shade * (0.5 + 0.5 * fract(lt)) + glow * 0.3, fract(EP(2) + 0.15 * lt));
}
```

- [ ] `emit_mandelbox.toml`

```toml
kind = "emitter"
description = "Mandelbox: raymarched box-fold + sphere-fold fractal; negative scale = spiky shells, positive = blocky towers"
[[param]]
name = "scale_gene" # sign picks the family, size the scale (|s| = 1.8..2.6)
min = -1.0
max = 1.0
fixed = true
[[param]]
name = "glow"
min = 0.6
max = 1.4
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_mandelbox.wgsl`

```wgsl
// Mandelbox: box fold (clamp-reflect at ±1) + sphere fold (min radius 0.5, fixed radius 1), then
// scale and add the start point. The scale's sign is structural: negative = spiky shells, positive =
// blocky towers. Orbited slowly; glows by its orbit trap.
fn emit_mandelbox_scale() -> f32 {
    return select(-1.0, 1.0, EP(0) > 0.0) * (1.8 + 0.8 * abs(EP(0)));
}

fn emit_mandelbox_de(p0: vec3<f32>) -> vec2<f32> {
    let s = emit_mandelbox_scale();
    var p = p0;
    var dr = 1.0;
    var trap = 1e3;
    for (var i = 0; i < 10; i++) {
        p = clamp(p, vec3<f32>(-1.0), vec3<f32>(1.0)) * 2.0 - p;
        let r2 = dot(p, p);
        if (r2 < 0.25) { p *= 4.0; dr *= 4.0; } else if (r2 < 1.0) { p /= r2; dr /= r2; }
        p = p * s + p0;
        dr = dr * abs(s) + 1.0;
        trap = min(trap, r2);
    }
    return vec2<f32>(length(p) / abs(dr), trap);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let dist = select(5.0, 9.0, emit_mandelbox_scale() > 0.0);
    let a = time() * 0.05 + seed() * TAU;
    let ro = vec3<f32>(cos(a), 0.35 + 0.2 * sin(time() * 0.04), sin(a)) * dist;
    let fw = normalize(-ro);
    let rt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fw));
    let up = cross(fw, rt);
    let rd = normalize(fw * 1.5 + rt * p.x + up * p.y);
    var t = 0.0;
    var hit = -1.0;
    var trap = 0.0;
    var steps = 0.0;
    let n = select(64, 96, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let h = emit_mandelbox_de(ro + rd * t);
        if (h.x < 0.002 * t) { hit = t; trap = h.y; break; }
        t += h.x;
        steps += 1.0;
        if (t > 20.0) { break; }
    }
    let glow = steps / f32(n);
    if (hit < 0.0) { return vec2<f32>(glow * 0.5 * EP(1), fract(EP(2) + 0.5)); }
    let shade = exp(-0.12 * hit) * EP(1) * (0.6 + 0.6 * punch() + 0.5 * drop_env());
    return vec2<f32>(shade * (0.4 + 0.6 * fract(trap * 2.0)) + glow * 0.3, fract(EP(2) + 0.2 * sqrt(trap)));
}
```

- [ ] `emit_sdftunnel.toml`

```toml
kind = "emitter"
description = "SDF tunnel: flying down a raymarched tube carved with repeating rings and studs; kicks pulse the rings"
[[param]]
name = "spacing"    # ring spacing along the tube
min = 0.8
max = 2.0
[[param]]
name = "radius"
min = 0.9
max = 1.6
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_sdftunnel.wgsl`

```wgsl
// SDF tunnel: fly down a raymarched tube of radius EP(1); its wall carries rings (torus slices) every
// EP(0) units and 8 studs around each ring. Kicks fatten the rings; the hue steps per ring; fog by depth.
fn emit_sdftunnel_de(p: vec3<f32>) -> f32 {
    let r = length(p.xy);
    let tube = EP(1) - r;
    let zz = p.z - EP(0) * floor(p.z / EP(0)) - 0.5 * EP(0);
    let ring = length(vec2<f32>(r - EP(1) * 0.85, zz)) - 0.06 * (1.0 + kick());
    let seg = TAU / 8.0;
    let a = atan2(p.y, p.x);
    let aa = a - seg * floor(a / seg) - 0.5 * seg;
    let q = vec3<f32>(r * cos(aa) - EP(1) * 0.9, r * sin(aa), zz);
    let stud = length(max(abs(q) - vec3<f32>(0.08, 0.08, 0.3), vec3<f32>(0.0)));
    return min(tube, min(ring, stud));
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let ro = vec3<f32>(0.15 * sin(time() * 0.3), 0.15 * cos(time() * 0.23), time() * 1.6);
    let d0 = normalize(vec3<f32>(p * 0.9, 1.0));
    let rd = vec3<f32>(rot2(time() * 0.05) * d0.xy, d0.z);
    var t = 0.0;
    var hit = -1.0;
    let n = select(48, 80, quality() > 0.5);
    for (var i = 0; i < n; i++) {
        let h = emit_sdftunnel_de(ro + rd * t);
        if (h < 0.002 * t) { hit = t; break; }
        t += h;
        if (t > 30.0) { break; }
    }
    if (hit < 0.0) { return vec2<f32>(0.0); }
    let z = ro.z + rd.z * hit;
    let fog = exp(-0.08 * hit);
    let ringlit = 1.0 - smoothstep(0.0, 0.25, abs(z - EP(0) * floor(z / EP(0)) - 0.5 * EP(0)));
    let i = fog * (0.35 + 0.9 * ringlit * (0.6 + 0.8 * punch())) * (1.0 + 0.5 * drop_env());
    return vec2<f32>(i, fract(EP(2) + 0.1 * floor(z / EP(0))));
}
```

- [ ] Each P5 agent: commit `feat(blocks): engine-2 <space|motion|2d emitter|raymarch emitter> blocks`.

---

## Phase P6: Evolution v2 + wiring · [opus] · owns `evolve.rs`, `main.rs`, `engine.rs` test

### Task P6.1: `evolve.rs`

- [ ] Add:

```rust
/// Engine 2, same budget as `best`. The verdict only gates (by the segment's seed-chosen regime), the
/// mutation parents are picked by seed, not score, and the final pick spreads over look space: the seed
/// picks an occupied cell (score::cell), then the candidate there farthest from `avoid` (the hand-made
/// scenes and this plan's earlier picks). Returns the genome and its descriptor.
#[allow(clippy::too_many_arguments)]
pub fn best_v2(
    judge: &mut Judge,
    blocks: &Blocks,
    gen_dir: &std::path::Path,
    a: &Analysis,
    start: f64,
    end: f64,
    seed: u64,
    budget: usize,
    avoid: &[[f32; 6]],
) -> Option<(Genome, [f32; 6])> {
    use crate::{regime::REGIMES, score};
    let t0 = (start + 30.0).min((start + end) / 2.0);
    let regime = &REGIMES[(hash(seed, 0x4E61) % REGIMES.len() as u64) as usize];
    let mut seen = std::collections::HashMap::new();
    let mut measure = |g: &Genome| -> Option<[f32; 6]> {
        *seen.entry(g.id()).or_insert_with(|| {
            let sc = genome::write(g, blocks, gen_dir).ok()?;
            let m = judge.measure(sc, a, t0, 4.0).ok()?;
            score::passes(&m, &regime.th).then(|| score::descriptor(&m))
        })
    };
    let first = (budget * 2 / 5).max(2);
    let mut pass: Vec<(Genome, [f32; 6])> = Vec::new();
    let mut k = 0u64;
    let mut fresh = |pass: &mut Vec<(Genome, [f32; 6])>, k: &mut u64, measure: &mut dyn FnMut(&Genome) -> Option<[f32; 6]>| {
        let g = genome::random_v2(blocks, hash(seed, *k), regime);
        *k += 1;
        if let Some(d) = measure(&g) {
            pass.push((g, d));
        }
    };
    for _ in 0..first {
        fresh(&mut pass, &mut k, &mut measure);
    }
    let parents: Vec<Genome> = (0..2.min(pass.len())).map(|j| pass[(hash(seed, 0xA0 + j as u64) % pass.len() as u64) as usize].0.clone()).collect();
    let rest = budget.saturating_sub(first);
    let per = rest.div_ceil(2) / parents.len().max(1);
    for (pi, p) in parents.iter().enumerate() {
        for j in 0..per {
            let child = genome::mutate_v2(p, blocks, hash(seed, 1000 + (pi * 100 + j) as u64));
            if let Some(d) = measure(&child) {
                pass.push((child, d));
            }
        }
    }
    for _ in 0..rest - per * parents.len() {
        fresh(&mut pass, &mut k, &mut measure);
    }
    pick_v2(pass, seed, avoid)
}

/// Seed-chosen occupied cell, then the candidate in it farthest from everything in `avoid`.
fn pick_v2(pass: Vec<(Genome, [f32; 6])>, seed: u64, avoid: &[[f32; 6]]) -> Option<(Genome, [f32; 6])> {
    use crate::score::cell;
    let mut ids = std::collections::HashSet::new();
    let pass: Vec<_> = pass.into_iter().filter(|(g, _)| ids.insert(g.id())).collect();
    let mut cells: Vec<usize> = pass.iter().map(|(_, d)| cell(d)).collect();
    cells.sort_unstable();
    cells.dedup();
    let c = *cells.get((hash(seed, 0xCE11) % cells.len().max(1) as u64) as usize)?;
    let dist = |d: &[f32; 6]| avoid.iter().map(|a| a.iter().zip(d).map(|(x, y)| (x - y).powi(2)).sum::<f32>()).fold(f32::MAX, f32::min);
    pass.into_iter().filter(|(_, d)| cell(d) == c).max_by(|x, y| dist(&x.1).total_cmp(&dist(&y.1)))
}
```

  If the `fresh` closure fights the borrow checker (it borrows `measure` mutably), inline its 5 lines at both call sites instead. Don't restructure anything else.
- [ ] Test:

```rust
    #[test]
    fn pick_v2_spreads_over_cells_and_avoids_seen_looks() {
        let g = |m: &str| Genome { motion: m.into(), ..Default::default() };
        let dark = [0.1, 0.1, 0.5, 0.1, 0.5, 0.5];
        let bright = [0.9, 0.9, 0.5, 0.9, 0.5, 0.5];
        let bright2 = [0.95, 0.95, 0.9, 0.95, 0.9, 0.9];
        let pool = vec![(g("d"), dark), (g("b"), bright), (g("b2"), bright2)];
        let picks: std::collections::HashSet<String> = (0..32).map(|s| pick_v2(pool.clone(), s, &[]).unwrap().0.motion).collect();
        assert!(picks.contains("d") && picks.len() >= 2, "{picks:?}");
        // In the bright cell, the look farthest from an already-chosen bright look wins.
        let s = (0..64).find(|&s| pick_v2(pool.clone(), s, &[]).unwrap().0.motion != "d").unwrap();
        assert_eq!(pick_v2(pool.clone(), s, &[bright2]).unwrap().0.motion, "b");
        assert_eq!(pick_v2(pool.clone(), 3, &[dark]).map(|x| x.0.id()), pick_v2(pool, 3, &[dark]).map(|x| x.0.id()));
        assert!(pick_v2(vec![], 1, &[]).is_none());
    }
```

### Task P6.2: `main.rs`

- [ ] `Cli`, after `look`:

```rust
    /// Visual engine: 1 = the original looks (default), 2 = space folds, blend modes, moods, new blocks, onset hits
    /// and drop build-ups (every seed a different piece). Default: the saved plan's, else 1.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=2))]
    engine: Option<u32>,
```

- [ ] In `run`, move `let plan_path = …;` and `let previous: Option<Plan> = …;` up to just before `load_or_analyse`. Then:

```rust
    let engine = cli.engine.or(previous.as_ref().map(|p| p.engine)).unwrap_or(1);
    // Engine 2 needs snares: older analyses are redone once.
    let analysis = load_or_analyse(&audio, &data, cli.reanalyse, if engine >= 2 { ANALYSIS_VERSION } else { 0 })?;
```

  `load_or_analyse` gains `min_version: u32` and its cache check becomes `.filter(|a: &Analysis| a.version >= min_version)` before `return Ok(a)`.
- [ ] Replace the Task-1 `1` placeholders: `same_inputs(…, engine)` and `engine,` in `PlanRequest`.
- [ ] Evolution pool, after `blocks` is loaded:

```rust
    // Evolution and dressing pick from the frozen library of this engine. A vibe also uses the AI's own
    // blocks. The full `blocks` still builds any genome a saved plan names.
    let core = blocks.as_ref().map(|b| if cli.vibe.is_some() { b.clone() } else { b.pinned(engine) });
```

  - In `evolved`, replace `let blocks = blocks.as_ref().filter(|_| generated > 0.0)?;` with `let blocks = core.as_ref().filter(|_| generated > 0.0)?;`.
  - In `dressed`, change `match blocks.as_ref()` to `match core.as_ref()`.
- [ ] Engine-2 branch at the top of `evolved` after the judge is obtained (declare `let handmade = OnceCell::new(); let picks = RefCell::new(Vec::<[f32; 6]>::new());` next to `gen`):

```rust
                if engine >= 2 && seeds.is_empty() {
                    let mut j = judge.borrow_mut();
                    // Distance from the hand-made scenes keeps engine 2 off their look (measured once per run).
                    let hm: &Vec<[f32; 6]> = handmade.get_or_init(|| {
                        let t = 60.0f64.min(analysis.duration / 2.0);
                        scenes.iter().filter(|s| scenes::BUILTIN.contains(&s.id.as_str())).filter_map(|s| j.measure(s.clone(), &analysis, t, 4.0).ok()).map(|m| score::descriptor(&m)).collect()
                    });
                    let avoid: Vec<[f32; 6]> = hm.iter().chain(picks.borrow().iter()).copied().collect();
                    let (g, d) = evolve::best_v2(&mut j, blocks, &gen_dir, &analysis, start, end, r, budget, &avoid)?;
                    picks.borrow_mut().push(d);
                    return Some(g);
                }
```

  With vibe seeds, the old path runs: `--vibe` keeps engine-1 looks (AI rows come later). Before planning, add `if engine >= 2 && cli.vibe.is_some() { eprintln!("note: --vibe uses engine 1 looks"); }`.
- [ ] `cargo build --release`.

### Task P6.3: `engine.rs` test

- [ ] In `engine_runs`, change the full motion×emitter×colour product to iterate `blocks.pinned(1)` (bind `let v1 = blocks.pinned(1);` and loop over `v1.motion` etc.) so its cost stays fixed. After the layer section, append:

```rust
        // Engine 2: every new block, space fold, blend mode and regime compiles and steps at least once
        // (round-robin partners; engine2_block_helpers_are_prefixed guarantees any pairing links).
        let b2 = blocks.pinned(2);
        let (ns, nb, nr) = (b2.space.len() + 1, crate::genome::BLENDS.len() + 1, crate::regime::REGIMES.len());
        let n = [b2.motion.len(), b2.emitter.len(), b2.colour.len(), ns, nb, nr].into_iter().max().unwrap();
        let mut v2 = Vec::new();
        for i in 0..n {
            let g = crate::genome::Genome {
                motion: b2.motion[i % b2.motion.len()].id.clone(),
                emitter: b2.emitter[i % b2.emitter.len()].id.clone(),
                colour: b2.colour[i % b2.colour.len()].id.clone(),
                palette: b2.palettes[0].clone(),
                space: if i % ns == 0 { String::new() } else { b2.space[i % ns - 1].id.clone() },
                blend: if i % nb == 0 { String::new() } else { crate::genome::BLENDS[i % nb - 1].into() },
                regime: crate::regime::REGIMES[i % nr].name.into(),
                ..Default::default()
            };
            v2.push(crate::genome::write(&g, &b2, &gen_dir).unwrap());
        }
        let mut e = Engine::new(&device, &queue, &v2, dir, Quality::Draft, 64, 36, wgpu::TextureFormat::Rgba8Unorm, &[]);
        for s in &v2 {
            e.compile(&s.id).unwrap_or_else(|err| panic!("{}: {err:#}", s.id));
            let mut u = SceneUniforms::default();
            u.res_time = [e.sim.0 as f32, e.sim.1 as f32, 1.0, 1.0 / SIM_HZ as f32];
            u.p0 = [1.0; 4];
            let fs = FrameState { scene_a: s.id.clone(), a: u, b: None, mix: 0.0, flash: 0.0, exposure: 1.0, saturation: 1.0, post: Post::default() };
            e.step(&fs, Some(1.0)).unwrap();
        }
        assert_eq!(b2.space.len(), 6, "all space blocks present");
```

- [ ] Commit `feat(core): engine-2 evolution (gate-only, seed parents, look-space picks) and --engine`.

---

## Gate

- [ ] `cargo test --release`: everything green, including:
  - `engine1_genomes_are_frozen`, `engine1_plan_and_frames_are_frozen`, `library_pins`
  - `engine2_block_helpers_are_prefixed`
  - `engine_runs`, which compiles and steps all 19 new blocks
  - `v2_template_space_slots_and_blend`, `random_v2_varies_every_gene_deterministically`, `cosine_palettes_are_dark_first_and_varied`
  - `descriptors_see_symmetry_density_colour_and_cells`, `onsets_split_kicks_from_snares`, `engine2_hits_anticipation_and_gated_glitch`, `pick_v2_spreads_over_cells_and_avoids_seen_looks`, `regimes_are_unique_and_sane`

  Fix a failing block in its own `.wgsl`, then re-run only `cargo test --release engine_runs`.
- [ ] **Default output is byte-identical.** Copy `dnb8.wav` from `C:\Prathamesh\Ableton\Music\dnb5` into `testdata/` if it's missing. Build `main` in a separate git worktree, then run on `main` and on this branch:
  `visualiser testdata/dnb8.wav --replan --preview 1:00 --quality draft`, then
  `ffmpeg -i testdata/dnb8.preview-60s.mp4 -f framemd5 -an <main|branch>.md5`.
  The two files must be equal. Repeat with `--layers`, but for the `main` run first move the 6 untracked `scenes/blocks/*runner*`/`*trippy*` files into the scratchpad and put them back afterwards (main still loads them; the branch pins them out). If anything differs, find the change that touched the default path and fix it. Usual suspects: an engine-1 golden that was edited, or `frame_state` arithmetic. Don't loosen the check.
- [ ] Smoke renders for the user to watch (draft, 60 s each, `--generated 1` so every segment is generated): `visualiser testdata/dnb8.wav --engine 2 --generated 1 --replan --preview 1:00 --quality draft --seed N` for N = 1, 2, 3, 4. Each seed should differ structurally: a different fold, mood or blend. Also check drop anticipation (darkens half a bar before the drop, explodes on it) and that no flashes come faster than once per bar.
- [ ] `/simplify` → `/review` over `git diff main...HEAD`. Fix the findings, then `cargo test --release` once.
- [ ] Docs [sonnet]:
  - `docs/USAGE.md`: section "Engine 2: --engine 2". Cover what it adds (space folds, blend modes, six moods, 19 new blocks, onset hits, drop build-ups, glitch looks on mood "glitch"/"maximal"), that it's saved in the plan, that engine 1 output is unchanged, that the first engine-2 run re-analyses the track once, the flash limit, and that `--vibe` keeps engine-1 looks.
  - `README.md`: one feature line.
  - `docs/MANUAL-TESTS.md`: the four seed smokes plus the framemd5 check.
- [ ] Commit `docs: engine 2 (core variety phase 1)`.

---

## Self-review notes

- **Spec coverage:**

  | Row | Where |
  |---|---|
  | 0 | Task 1 (`pinned`, `BUILTIN`, `Plan.engine` in the job hash via the plan JSON) |
  | 1 | P6 `best_v2` (gate only, seed parents) |
  | 2 | Task 1 `hits`, P4 snares and punch, v2 mod sources, the template's drive |
  | 3 | P4 anticipation and the bigger drop flash |
  | 4 | P3 `cosine_palette` |
  | 5 | Task 1 `regime.rs`, P3 template decay/drive/post, P6 per-segment regime |
  | 6 | P5b |
  | 7 | P5c |
  | 8 | Task 1 space kind, P3 template and slots, P5a |
  | 9 | P5d |
  | 10 | P2 descriptors, P6 `pick_v2` and the hand-made/novelty avoid list |
  | 11 | P3 `BLENDS` |
  | 12 | Task 1 `Post` fields, P4 gating and finish |

- **Byte-identical engine 1:**
  - New genes and `Post` fields are skipped in JSON when empty or 0, and `Plan.engine` is skipped at 1.
  - `wgsl()`'s engine-1 branch is the old body.
  - `frame_state` changes are exact no-ops at engine 1. `hits` is computed but no engine-1 shader reads it.
  - `SOURCES` is untouched (`SOURCES_V2` is separate).
  - Finish additions sit behind `if > 0`.
  - The pinned library equals the committed library.
  - The goldens and the framemd5 gate both check this.
- **Names used across tasks:** `pinned`, `space_part`, `is_v2`, `BLENDS`, `random_v2`, `mutate_v2`, `cosine_palette`, `regime::{REGIMES, get, Regime}`, `score::{Thresholds, V1, passes, measure, descriptor, cell, colourfulness}`, `detect_onsets`, `onset_env`, `since`, `SOURCES_V2`, `best_v2`, `pick_v2`, `ANALYSIS_VERSION`, `BUILTIN`. These are consistent.
- **Cost:** engine-2 plans measure the 7 hand-made scenes once (7 × 4 s at 160×90). The raymarched emitters are heavy at final quality, which is fine offline. The live `--dev` preview will be slow on them, and a half-res preview option is Phase 2.
