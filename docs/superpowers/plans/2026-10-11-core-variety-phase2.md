# Core Variety Phase 2 Implementation Plan ("Shows, not loops")

> **For agentic workers:** REQUIRED SUB-SKILL: follow `/code-implementation-build-rules` (orchestrator dispatches, subagents build; waves below). Use superpowers:subagent-driven-development for per-task dispatch. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Phase 2 of `docs/research/2026-10-11-crazy-visuals.md`, non-AI rows only (13, 14, 15, 16, 17, 19, 21). A generated take should play like a show: new worlds on the drops (black, then rebuild), a look that develops every 16/32 bars, designed transitions, display-only framings including slit-scan time smears, recipes other than feedback, and no hand-made scenes in new engine-2 takes.

**Architecture:** Everything rides on **engine 2** (no engine 3). Rule (user decision, also for Phase 3): a feature that extends the engine-2 pipeline with new genes, stages, blocks or director behaviour stays engine 2; only an entirely different render grammar or pipeline gets engine 3. Saved engine-2 plans stay reproducible by one mechanism: every new genome gene (`template`, `emitter_b`, `comp`) and the new `Segment.variant` flag are `skip_serializing_if` empty/false, so saved genome ids, plan JSON and generated WGSL are unchanged; the new director and main behaviour runs only while planning (new plans, `--replan`, `--reroll`), and a saved plan is reused verbatim (`same_inputs`); the per-frame and engine paths are exact no-ops for old data (`Morph` keeps `wipe = 0` and the constant-blend copy, `reset` fires only on the new `Reset` kind, the history ring exists only when a comp block calls `hist()`). Two Task-0 goldens (`engine2_saved_*`) and a framemd5 gate against the base commit prove it.

Per-row verdict:

| Row | Verdict | Why |
|---|---|---|
| 13 comp stage | engine 2 | A new optional block kind in the existing layer pass (`LAYERS`): `comp(uv)` replaces `prev(uv)` only when the gene is set. |
| 14 hard cuts + reset | engine 2 | Director/main planning behaviour plus one clear of the feedback buffer on a new `TransitionKind::Reset`. |
| 15 phrase variants | engine 2 | Director post-pass that splits generated segments into variant segments; same segments, scenes and renderer. |
| 16 transition shaders | engine 2 | The morph's constant-blend copy gains a masked alternative (`trans.wgsl`) selected by new `TransitionKind`s; `Morph` is untouched. |
| 17 slit-scan | engine 2 | One extra ring texture, written per output frame and read only by comp blocks through `hist()`; allocated only when a plan uses it. |
| 19 all-generated default | engine 2 | Changes only the default of `--generated` for new engine-2 projects; saved plans keep theirs. |
| 21 recipe variants | engine 2 | A different grammar but the same pipeline: a genome gene picks another `scene()` body (stateless, dual emitter) in the same scene slot, step and finish. It would be engine 3 only if it needed a new pass structure (as Phase 3's simulation layer will). |

How it works:
- **Comp (13):** display-only blocks (`kind = "comp"`, no params) reframe the finished feedback in the look's layer pass (mirror, kaleido, slit-scan, time echo).
- **Slit-scan (17):** the engine keeps a 64-frame, half-output-size Rgba8 history ring. It is written once per output frame, and the chunk pre-roll fills it exactly as a continuous render would.
- **Drops (14):** each drop becomes a `Reset` shot: a new judged genome and a cleared feedback buffer.
- **Phrases (15):** each 16/32-bar phrase inside a generated segment becomes a **phrase variant**, an unjudged structural change via `genome::vary`: another fold, comp or palette.
- **Transitions (16):** morphs (track blends, `--layers` build-ups, variants) get seed-chosen styles: luma wipe, iris zoom-through, melt and tiles.
- **Recipes (21):** `stateless` and `dual` get their own judge-gate widening (`regime::gate`).
- **Default (19):** new engine-2 projects default to `--generated 1`.

**Tech Stack:** Rust 2021, wgpu 30, WGSL. No new crates.

**Not in this plan:**
- AI rows (A0–A14) and Phase 3 rows 22–28. Phase 3 follows the same engine rule: the simulation layer is a new pass structure, so decide there whether it is engine 3.
- Rows 18 (extra finish looks) and 20 (param tuning in mutation).
- A third "raymarch-first, feedback as post" recipe: Phase 1's raymarched emitters already run inside feedback, and `stateless` covers the crisp 3D case.
- Judging phrase variants: `vary` only makes safe changes; judge them if one ever renders black.
- Styled crossfades of the *layer* pass: a comp-to-comp change inside a morph still crossfades.
- Phase-1 deferred items, untouched here:
  - `--keep` export loses the look's `Post`.
  - Reroll novelty ignores saved looks.
  - No cache for the hand-made descriptors.
  - `best`/`best_v2` don't share a driver.
  - Double drop darkening (anticipation plus exposure). The new `Reset` adds no further darkening.
- A wizard question for transition or recipe styles: the seed picks them.

---

## Build setup (from /code-implementation-build-rules)

`T2 Standard · no web frontend (CLI + GPU shaders; design gate n/a) · review: /simplify → /review`

- **Branch:** `feature/core-variety-2`, created from `main` @ `7a6d6e4` (Phase 1 merged; the saved-engine-2 framemd5 base is still `231afa2`). Never commit to `main`; the user merges.
- **Untracked AI blocks:** `scenes/blocks/*runner*`, `*trippy*`, `colour_cube_mono.*` and `emit_beat_cube.*` (10 files) are AI-written and untracked. **Never commit or delete them.** Stage files by explicit path only; never `git add -A` or `git add .`. Engine pinning already excludes them.
- **Orchestrator** plans, dispatches, joins and commits. Subagents do the work and **don't commit** (parallel commits in one worktree race on the index lock). Independent tasks in one wave go out as multiple `Agent` calls **in a single message**.
- **Model routing:** `opus` for engine, render, genome, director, main/evolve wiring and every shader block. `sonnet` for docs. No Haiku.
- **Testing discipline:** during the wave, subagents run `cargo build --release` plus their own task's named tests. The shader-block task runs nothing, because WGSL compiles in the gate's `engine_runs`. Full `cargo test --release` runs once at the gate. After a fix, re-run only the failing test, then one full run.
- **ponytail:** minimum code. Each non-trivial piece of logic leaves ONE runnable `#[test]`. Mark deliberate shortcuts with `// ponytail:`, naming the ceiling.
- **Commits** (orchestrator, after each wave joins, by explicit paths): `test(core): …`, `feat(core): …`, `feat(blocks): …`, `docs: …`.

### Waves

```
Wave 0 (orchestrator, sequential): Task 0 engine-2 goldens → Task 1 contract
                                   → cargo test --release engine1 engine2_saved library_pins vary gate
Wave 1 (parallel, 5 agents):
  P2 engine: history ring, styled morphs, reset, render/preview wiring [opus]
  P3 genome: stateless/dual recipes, comp layer pass, random_v2 [opus]
  P4 director: drop resets, styled morphs, phrase variants, frame_state [opus]
  P5 comp blocks [opus]
  P6 main/evolve/wizard: drop shots, generated default, recipe gate [opus]
Gate → /simplify → /review → docs [sonnet] → commit
```

**File ownership (no collisions):**
- **Task 0** (orchestrator) owns the new golden tests in `genome.rs` and `director.rs`.
- **Task 1** (orchestrator) owns:
  - `types.rs`, `regime.rs` (`gate`) and `CONTRACT.md`.
  - The contract edits in `genome.rs`: fields, the `comp` kind, accessors, `prune`, `V2_BLOCKS`, `TEMPLATES`, `vary`, and test-literal fixes.
  - Compile fixes only in `director.rs` (the `Segment`/`FrameState` literals and the test `Blocks` literal), `score.rs` (the `Segment` literal) and `engine.rs` (the test `FrameState` literals).
- **Wave 1:**
  - **P2** owns `src/render/engine.rs`, `src/render/post.wgsl`, `src/render/trans.wgsl` (new), `scenes/_prelude.wgsl`, `src/render/mod.rs`, `src/preview.rs`, and the one `Engine::new` line in `src/score.rs`.
  - **P3** owns `src/genome.rs` (everything outside Task 1's contract edits).
  - **P4** owns `src/director.rs`.
  - **P5** owns only *new* files `scenes/blocks/comp_*`.
  - **P6** owns `src/main.rs`, `src/evolve.rs` and `src/wizard.rs`.
- **Docs** ([sonnet], after the gate) own `docs/USAGE.md`, `README.md` and `docs/MANUAL-TESTS.md`.

---

## File map

| File | Status | Responsibility |
|---|---|---|
| `src/types.rs` | modify | `TransitionKind::{Reset, Wipe, Iris, Melt, Cells}`, `is_morph`, `style`, `MORPHS`, `FrameState.{wipe, reset}`, `Segment.variant` |
| `src/regime.rs` | modify | `gate(regime, template)`: per-recipe judge ranges |
| `scenes/blocks/CONTRACT.md` | modify | comp kind, `hist()`, recipes, dual renaming rules |
| `src/genome.rs` | modify | genes `template`/`emitter_b`/`comp`; `comp` kind; `comp_part`/`emitter_b_part`; `TEMPLATES`; `vary`; stateless/dual templates; comp layer pass; dual slots; `random_v2` recipes/comps; `dress` skips dual |
| `src/render/engine.rs` | modify | `Engine::new(.., history)`, history ring + `push_history`, `hist_layer`, `Reset` clear, styled-morph pipeline, `TRANS_WGSL`, tests |
| `src/render/trans.wgsl` | create | masked morph styles (wipe, iris, melt, cells) |
| `src/render/post.wgsl` | modify | `fs_hist` (HDR → 0..1 for the ring) |
| `scenes/_prelude.wgsl` | modify | binding 5 `hist_tex`, `hist(uv, k)` |
| `src/render/mod.rs` | modify | `needs_history`, pre-roll history fill, per-frame push, `TRANS_WGSL` in `job_hash` |
| `src/preview.rs` | modify | history flag + per-output-frame push |
| `src/score.rs` | modify | `Segment` literal (Task 1), `Engine::new(.., false)` (P2) |
| `src/director.rs` | modify | Reset near drops, hard shots first, styled morphs, `phrase_variants`, reroll mapping, chapters skip variants, `frame_state` wipe/reset |
| `src/main.rs` | modify | engine-2 drop `Reset` shots, `--generated` default 1 under engine 2 |
| `src/evolve.rs` | modify | `best_v2` gates by `regime::gate(regime, template)` |
| `src/wizard.rs` | modify | "Visual style?" defaults to all generated, explicit values |
| `scenes/blocks/comp_*` | create | mirror, kaleido, slitscan, echo |

---

## Task 0: Engine-2 goldens (orchestrator, before any change)

Characterization tests. They capture how a **saved Phase-1 engine-2 plan** renders now and must stay green through every later task. The engine-1 goldens (`engine1_genomes_are_frozen`, `engine1_plan_and_frames_are_frozen`) already exist and are never edited.

**Files:** `src/genome.rs` (tests), `src/director.rs` (tests)

- [ ] **Step 1: genome golden.** Append to `mod tests` in `src/genome.rs`:

```rust
    /// Saved engine-2 plans must keep rendering the same: a phase-1 engine-2 look's sources and params
    /// (captured before phase 2).
    #[test]
    fn engine2_saved_looks_are_frozen() {
        let b = blocks().pinned(2);
        let sig = |s: String| format!("{:016x}", fnv1a(s.as_bytes()));
        let g = Genome { space: "space_kaleido".into(), blend: "screen".into(), regime: "glitch".into(), ..genome() };
        let layered = Genome { backdrop: "back_sky".into(), overlay: "over_hud".into(), plate: Some(1), feedback: Some(0.6), text: "HI".into(), ..g.clone() };
        assert_eq!(sig(wgsl(&g, &b).unwrap()), "CAPTURE");
        assert_eq!(sig(layers_wgsl(&layered, &b).unwrap().unwrap()), "CAPTURE");
        assert_eq!(sig(format!("{:?}", to_scene(&layered, &b, Path::new(".")).unwrap().params)), "CAPTURE");
    }
```

- [ ] **Step 2: director golden.** Append to `mod tests` in `src/director.rs`:

```rust
    /// Saved engine-2 plans must keep rendering the same: per-frame state of a fixed engine-2 plan (captured
    /// before phase 2). Single mode with no blocks, so no phase-2 planning change can reach the plan itself.
    #[test]
    fn engine2_saved_frames_are_frozen() {
        let mut a = test_analysis();
        a.snares = vec![Kick { time: 31.0, strength: 1.0 }];
        let scenes = test_scenes();
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Single, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 0.0, vibe: None, genome_for: None, blocks: None, gen_dir: Path::new("."), shots: &[], engine: 2, pool: Pool::Builtin };
        let mut p = plan(&req).unwrap();
        // A second segment entered by a plain morph, carrying a glitch look.
        let mut s2 = p.segments[0].clone();
        s2.start = 96.0;
        s2.scene = "s2".into();
        s2.transition_in = Transition { kind: TransitionKind::Morph, duration: 4.0 };
        s2.genome = Some(Genome { post: Post { strobe: 0.5, tear: 0.5, ..Default::default() }, ..Default::default() });
        p.segments[0].end = 96.0;
        p.segments.push(s2);
        let mut s = serde_json::to_string(&p).unwrap();
        for t in [0.3, 31.02, 63.75, 64.05, 95.0, 96.0, 97.9, 300.0] {
            let f = frame_state(&p, &a, &scenes, t, 64, 64, Quality::Draft);
            let u = &f.a;
            s += &format!(
                "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?} {:?} {} {} {} {} {:?}\n",
                u.res_time, u.beat, u.bands, u.shape, u.p0, u.p1, u.p2, u.p3, u.pal, u.misc, u.hits, f.b.as_ref().map(|b| &b.0), f.mix, f.flash, f.exposure, f.saturation, f.post
            );
        }
        assert_eq!(format!("{:016x}", fnv1a(s.as_bytes())), "CAPTURE");
    }
```

- [ ] **Step 3: capture.** Run `cargo test --release engine2_saved -- --nocapture`. Each `assert_eq!` fails and prints `left: "…"`. Paste each left value over its `"CAPTURE"`, in order: the genome test fails at the first assert, so fix and re-run three times. Then re-run until both pass.
- [ ] **Step 4:** Commit `test(core): engine-2 saved-plan goldens` (`git add src/genome.rs src/director.rs`).

---

## Task 1: Contract (orchestrator)

Every Wave-1 task codes against this. It changes no output; the goldens prove it.

### 1.1 `src/types.rs`

- [ ] Replace `TransitionKind` with:

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum TransitionKind {
    Cut,
    Flash,
    Morph,
    /// Engine 2: a hard cut that clears the feedback (black, then the new look rebuilds).
    Reset,
    /// Engine-2 styled morphs (render/trans.wgsl): the incoming look arrives through a mask that grows over
    /// the window: its brightest parts first, an iris zooming through the centre, a melt from the top, tiles
    /// in random order.
    Wipe,
    Iris,
    Melt,
    Cells,
}

impl TransitionKind {
    /// Blends two looks over `duration` (Morph and the styled morphs).
    pub fn is_morph(self) -> bool {
        matches!(self, Self::Morph | Self::Wipe | Self::Iris | Self::Melt | Self::Cells)
    }

    /// trans.wgsl style index (0 = Morph's constant crossfade, also every non-morph kind).
    pub fn style(self) -> u32 {
        match self {
            Self::Wipe => 1,
            Self::Iris => 2,
            Self::Melt => 3,
            Self::Cells => 4,
            _ => 0,
        }
    }
}

/// Engine-2 morph styles the seed picks from (Morph = plain crossfade).
pub const MORPHS: [TransitionKind; 5] = [TransitionKind::Morph, TransitionKind::Wipe, TransitionKind::Iris, TransitionKind::Melt, TransitionKind::Cells];
```

- [ ] `Segment`: after `genome`:

```rust
    /// Engine 2: a phrase variant of the segment before it (not a chapter; --reroll maps it to its parent).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub variant: bool,
```

- [ ] `FrameState`: after `post`:

```rust
    /// Morph style (TransitionKind::style; 0 = constant crossfade).
    pub wipe: u32,
    /// Clear the feedback before this step (a Reset cut starts here).
    pub reset: bool,
```

### 1.2 `src/regime.rs`

- [ ] Add after `get`:

```rust
/// The judge's ranges for a regime and recipe (genome::TEMPLATES): stateless looks redraw every frame, so
/// they move more and carry more fine detail than trails; dual looks stack two emitters and clip a bit more.
// ponytail: hand-set factors; retune from the gate's smoke renders if a recipe never (or always) passes.
pub fn gate(r: &Regime, template: &str) -> Thresholds {
    let th = r.th;
    match template {
        "stateless" => Thresholds { motion: (th.motion.0, th.motion.1 * 1.5), max_grain: th.max_grain * 1.5, ..th },
        "dual" => Thresholds { max_clipped: th.max_clipped * 1.3, ..th },
        _ => th,
    }
}
```

  and the test:

```rust
    #[test]
    fn recipes_widen_the_regime_gate() {
        let r = get("calm").unwrap();
        assert_eq!(gate(r, "").motion, r.th.motion);
        let s = gate(r, "stateless");
        assert!(s.motion.1 > r.th.motion.1 && s.max_grain > r.th.max_grain && s.mean == r.th.mean);
        assert!(gate(r, "dual").max_clipped > r.th.max_clipped);
    }
```

### 1.3 `src/genome.rs`: genes, comp kind, accessors, `vary`

- [ ] `Genome`: append after `regime`:

```rust
    /// Engine-2 recipe (genome::TEMPLATES; "" = the feedback recipe), the second emitter of a "dual" look,
    /// and the display-only comp block (kind comp; "" = none). Empty = left out of the JSON, so saved ids hold.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub template: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub emitter_b: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comp: String,
```

- [ ] `is_v2`: `!self.space.is_empty() || !self.blend.is_empty() || !self.regime.is_empty() || !self.template.is_empty()`.
- [ ] Kinds:
  - `pub const KINDS: [&str; 7] = ["motion", "emitter", "colour", "backdrop", "overlay", "space", "comp"];`
  - `const SLOTS: [usize; 7] = [3, 3, 2, 4, 2, 2, 0];`
  - Doc comments: "The last four (layers, space, comp) are optional", and "comp (engine 2, display only) has no params".
- [ ] `Blocks`: add `/// Engine-2 display-only framings (optional kind). pub comp: Vec<Block>,` after `space`. Every match/array over kinds gets a 7th arm:
  - `Genome::block_mut`: `5 => &mut self.space, _ => &mut self.comp`.
  - `Blocks::list`: append `&self.comp` to the array.
  - `list_mut`: `5 => &mut self.space, _ => &mut self.comp`.
  - `load`: add `comp: vec![]` to the literal.
  - `subset`: add `comp: pick(&self.comp, |g| &g.comp),`.
  - `param()`: append `&self.comp` to the array.
- [ ] Accessors after `space_part`:

```rust
    /// The genome's comp block ("" = none), or an error naming an unknown id.
    pub fn comp_part(&self, g: &Genome) -> Result<Option<&Block>> {
        Self::opt(&self.comp, &g.comp)
    }

    /// The second emitter of a "dual" look ("" = none), or an error naming an unknown id.
    pub fn emitter_b_part(&self, g: &Genome) -> Result<Option<&Block>> {
        Self::opt(&self.emitter, &g.emitter_b)
    }
```

- [ ] `prune`: replace the first line and the chain:

```rust
        let (Ok(parts), Ok(layers), Ok(space), Ok(eb), Ok(comp)) =
            (blocks.parts(self), blocks.layer_parts(self), blocks.space_part(self), blocks.emitter_b_part(self), blocks.comp_part(self))
        else {
            return self.params.clear();
        };
```

  and `.chain(space).chain(eb).chain(comp)` after `.chain(layers.into_iter().flatten())` (replacing `.chain(space)`).
- [ ] `V2_BLOCKS`: append a line `"comp_echo", "comp_kaleido", "comp_mirror", "comp_slitscan",`.
- [ ] Below `BLENDS`:

```rust
/// Engine-2 recipes besides feedback ("" in Genome::template): "stateless" draws the emitter fresh every frame
/// (no trails, crisp 3D); "dual" adds a second emitter (Genome::emitter_b) over the feedback.
pub const TEMPLATES: [&str; 2] = ["stateless", "dual"];
```

- [ ] `vary` (after `mutate_v2`):

```rust
/// Engine-2 phrase variant: the same look with one structural change: another space fold (or none),
/// another comp framing (or none), or a fresh cosine palette. Never the motion, emitter, blend, recipe or
/// regime (those can go black, and variants aren't judged). `r` = 64-bit hash.
// ponytail: unjudged; run variants through the judge if one ever renders black.
pub fn vary(g: &Genome, blocks: &Blocks, r: u64) -> Genome {
    let h = |k: u64| crate::types::hash(r, k);
    // Another id of `list` or "" (none), never the current one.
    let other = |list: &[Block], cur: &str, k: u64| {
        let ids: Vec<&str> = std::iter::once("").chain(list.iter().map(|b| b.id.as_str())).filter(|id| *id != cur).collect();
        ids[(h(k) % ids.len() as u64) as usize].to_string()
    };
    let mut out = g.clone();
    match h(1) % 3 {
        0 if !blocks.space.is_empty() => out.space = other(&blocks.space, &g.space, 2),
        1 if !blocks.comp.is_empty() => out.comp = other(&blocks.comp, &g.comp, 3),
        _ => out.palette = cosine_palette(h(4)),
    }
    if out.palette != g.palette && out.post.tint_amount > 0.0 {
        out.post.tint = crate::scenes::hex(&out.palette[2]).unwrap_or([1.0; 3]);
    }
    out.prune(blocks);
    out
}
```

- [ ] Tests (in `mod tests`):
  - In `library_pins`, append `assert!(v1.comp.is_empty());`.
  - In `engine2_block_helpers_are_prefixed`, change `contract` to `["motion", "motion_decay", "emit", "colour", "space", "comp"]` and the iterated lists to `[&b.motion, &b.emitter, &b.space, &b.comp]`.
  - Append:

```rust
    #[test]
    fn vary_changes_exactly_one_structural_gene() {
        let mut b = blocks().pinned(1);
        for k in 0..2 {
            b.add("space", space_block(&format!("space_t{k}"), "")).unwrap();
            b.add("comp", Block { id: format!("comp_t{k}"), params: vec![], wgsl: String::new(), toml: String::new() }).unwrap();
        }
        let mono = crate::regime::get("mono").unwrap();
        let g = Genome { space: "space_t0".into(), regime: mono.name.into(), template: "stateless".into(), post: mono.post, ..genome() };
        let mut kinds = std::collections::HashSet::new();
        for r in 0..48u64 {
            let v = vary(&g, &b, crate::types::hash(5, r));
            assert_eq!(v, vary(&g, &b, crate::types::hash(5, r)), "deterministic");
            let changed = [v.space != g.space, v.comp != g.comp, v.palette != g.palette];
            assert_eq!(changed.iter().filter(|&&c| c).count(), 1, "{v:?}");
            assert_eq!((&v.motion, &v.emitter, &v.colour, &v.blend, &v.regime, &v.template), (&g.motion, &g.emitter, &g.colour, &g.blend, &g.regime, &g.template));
            if v.palette != g.palette {
                assert_eq!(v.post.tint, crate::scenes::hex(&v.palette[2]).unwrap(), "mono tint follows the palette");
            }
            kinds.insert(changed.iter().position(|&c| c).unwrap());
        }
        assert_eq!(kinds.len(), 3, "fold, comp and palette changes all occur");
    }
```

### 1.4 Compile fixes (no behaviour change)

- [ ] `director.rs`:
  - In the `segments.push(Segment { … })` literal in `plan()`, add `variant: false,`.
  - In the `FrameState { … }` literal at the end of `frame_state`, add `wipe: 0, reset: false,` (P4 fills them in).
  - In the test `generated_segments_carry_their_genome`, add `comp: vec![],` to the `Blocks { … }` literal.
- [ ] `score.rs`: add `variant: false,` to the `Segment { … }` literal in `Judge::score`.
- [ ] `engine.rs` tests: add `wipe: 0, reset: false` to the three explicit `FrameState { … }` literals (lines ~579, ~659, ~701). Leave the `..fs.clone()` one as it is.

### 1.5 `scenes/blocks/CONTRACT.md`

- [ ] Append:

````markdown
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
````

- [ ] **Verify:** `cargo build --release`, then `cargo test --release engine1 engine2_saved library_pins vary gate`. All green, with golden values unchanged.
- [ ] Commit `feat(core): phase-2 contract (comp kind, recipes, transitions, variants)`. Stage `src/types.rs src/regime.rs src/genome.rs src/director.rs src/score.rs src/render/engine.rs scenes/blocks/CONTRACT.md`.

---

## Phase P2: Engine, history ring, transitions · [opus] · owns `engine.rs`, `post.wgsl`, `trans.wgsl`, `_prelude.wgsl`, `render/mod.rs`, `preview.rs`, the `Engine::new` line in `score.rs`

### Task P2.1: `src/render/trans.wgsl` (new)

- [ ] Create:

```wgsl
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
```

### Task P2.2: `src/render/post.wgsl`

- [ ] Append:

```wgsl
// History ring write: the feedback frame, squeezed from HDR into 0..1 for the Rgba8 ring (prelude hist() undoes it).
@fragment
fn fs_hist(in: VsOut) -> @location(0) vec4<f32> {
    let c = s(in.uv);
    return vec4<f32>(c / (1.0 + c), 1.0);
}
```

### Task P2.3: `scenes/_prelude.wgsl`

- [ ] After the `plate_samp` binding line:

```wgsl
@group(0) @binding(5) var hist_tex: texture_2d_array<f32>; // display history ring (comp blocks: hist())
```

- [ ] After `fn plate(…)`:

```wgsl
// Display history (comp blocks only): the feedback `k` output frames ago (0..63, ~2 s), at uv, HDR.
// Black until the ring has filled, and in looks whose comp block doesn't call it (1-layer dummy ring).
// 30 = OUT_FPS: the engine stores output frame floor(time * 30) in layer (frame mod layers).
fn hist(uv: vec2<f32>, k: f32) -> vec3<f32> {
    let n = i32(textureNumLayers(hist_tex));
    let i = i32(floor(time() * 30.0)) - i32(clamp(k, 0.0, f32(n - 1)));
    let x = textureSampleLevel(hist_tex, plate_samp, uv, ((i % n) + n) % n, 0.0).rgb;
    return x / max(vec3<f32>(1.0) - x, vec3<f32>(0.004));
}
```

### Task P2.4: `src/render/engine.rs`

- [ ] Constants, next to `FINISH_WGSL`:

```rust
pub const TRANS_WGSL: &str = concat!(include_str!("common.wgsl"), include_str!("trans.wgsl"));
/// Output frames the display history keeps (comp blocks' hist(); ~2.1 s at OUT_FPS).
pub const HIST_LEN: u32 = 64;
/// Ring format: half output size, 8-bit (64 frames ≈ 133 MB at 1080p, 236 MB at 1440p).
const HIST: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
```

- [ ] `Engine` fields (after `layers`):

```rust
    /// Display history ring, one view per layer (output frame mod HIST_LEN); None = not kept (no look reads hist()).
    hist: Option<Vec<wgpu::TextureView>>,
    hist_pl: wgpu::RenderPipeline,
    /// Styled morph (trans.wgsl): B from scratch, masked over fb[next].
    trans_pl: wgpu::RenderPipeline,
    tbuf: wgpu::Buffer,
    trans_bg: wgpu::BindGroup,
```

- [ ] `Engine::new`: add the last parameter `history: bool` and document it ("keep the display history ring: some look's comp block calls hist()"). Inside:
  - After `let plates = plate_array(d, queue, plates);`:

```rust
        // History ring (a 1x1x1 dummy when not kept: the scene layout always binds one).
        let (hw, hh, hn) = if history { ((width / 2).max(1), (height / 2).max(1), HIST_LEN) } else { (1, 1, 1) };
        let hist_tex = d.create_texture(&wgpu::TextureDescriptor {
            label: Some("history"),
            size: wgpu::Extent3d { width: hw, height: hh, depth_or_array_layers: hn },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HIST,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let hist_arr = hist_tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let hist = history.then(|| {
            (0..hn)
                .map(|i| {
                    hist_tex.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: i,
                        array_layer_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect()
        });
```

  - `scene_bgl` becomes `bgl(&[uniform_e(0), tex_e(1), samp_e(2), arr_e(3), samp_e(4), arr_e(5)])`. `scene_bind` appends `R::TextureView(&hist_arr)` to its resource list.
  - After `finish_layout`:

```rust
        let trans_bgl = bgl(&[uniform_e(0), tex_e(1), samp_e(2)]);
        let trans_layout = layout(&trans_bgl);
```

  - After `let fbuf = …;`: `let tbuf = buf(16);`.
  - After `finish_bg`: `let trans_bg = bg(d, &trans_bgl, &[tbuf.as_entire_binding(), R::TextureView(&scratch), R::Sampler(&clamp)]);`
  - After the `finish` module:

```rust
        let trans = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("trans"),
            source: wgpu::ShaderSource::Wgsl(TRANS_WGSL.into()),
        });
```

  - After `finish_pl`:

```rust
        let over = blend(wgpu::BlendFactor::SrcAlpha, wgpu::BlendFactor::OneMinusSrcAlpha);
        let trans_pl = pipeline(d, &trans, &trans_layout, "fs_trans", HDR, Some(wgpu::BlendState { color: over, alpha: over }));
        let hist_pl = pipeline(d, &post, &tex_layout, "fs_hist", HIST, None);
```

  - Add `hist, hist_pl, trans_pl, tbuf, trans_bg,` to the `Self { … }` literal.
- [ ] `step`: replace the body from `let mut enc = …` through the morph block with:

```rust
        let mut enc = self.device.create_command_encoder(&Default::default());
        if fs.reset {
            // A Reset cut: the new look starts from black.
            drop(begin(&mut enc, &self.fb[cur], true));
        }
        draw(&mut enc, &self.fb[next], true, pa, &self.scene_bg[0][cur], 1.0);
        if let Some((id_b, ub)) = &fs.b {
            if let Some(pb) = self.pipelines.get(id_b) {
                self.queue.write_buffer(&self.ubuf[1], 0, bytemuck::bytes_of(ub));
                draw(&mut enc, &self.scratch, true, pb, &self.scene_bg[1][cur], 1.0);
                if fs.wipe == 0 {
                    draw(&mut enc, &self.fb[next], false, &self.copy_pl, &self.scratch_bg, fs.mix as f64);
                } else {
                    let t = [fs.mix, fs.wipe as f32, self.sim.0 as f32 / self.sim.1 as f32, 0.0];
                    self.queue.write_buffer(&self.tbuf, 0, bytemuck::bytes_of(&t));
                    draw(&mut enc, &self.fb[next], false, &self.trans_pl, &self.trans_bg, 1.0);
                }
            }
        }
```

  (The accumulate block, submit and `self.cur = next` stay.)
- [ ] After `step`:

```rust
    /// Stores the newest feedback frame as output frame floor(t · OUT_FPS) of the history ring (comp blocks
    /// read it with hist()). Call once per output frame, after its last sub-step; a no-op without a ring.
    pub fn push_history(&self, t: f64) {
        let Some(views) = &self.hist else { return };
        let mut enc = self.device.create_command_encoder(&Default::default());
        draw(&mut enc, &views[hist_layer(t, views.len())], true, &self.hist_pl, &self.fb_bg[self.cur], 1.0);
        self.queue.submit([enc.finish()]);
    }
```

  and at module level:

```rust
/// Ring layer of the output frame at `t` (prelude hist() derives the same index from time()).
fn hist_layer(t: f64, n: usize) -> usize {
    ((t * OUT_FPS as f64).floor() as i64).rem_euclid(n as i64) as usize
}
```

- [ ] Tests:
  - Add `, false` as the last argument of the five existing `Engine::new` calls in `engine_runs`.
  - Append this test:

```rust
    /// A render pushes after each output frame's last sub-step (t0 + (2f + 1) / 60): consecutive frames take
    /// consecutive layers and wrap at the ring size, also for a pre-roll's negative frames.
    #[test]
    fn history_layers_follow_output_frames() {
        let at = |f: i64| hist_layer(10.0 + (2 * f + 1) as f64 / SIM_HZ as f64, HIST_LEN as usize);
        for f in -70..70 {
            assert_eq!(at(f + 1), (at(f) + 1) % HIST_LEN as usize, "frame {f}");
        }
    }
```

  - At the end of `engine_runs` (after the engine-2 section; the 64x36 `out` texture from the layer section is still in scope):

```rust
        // Engine 2, phase 2: comp layer passes (hist() on a real ring), every recipe, every morph style and a
        // reset step run without validation errors (wgpu panics on one).
        let (nc, nt) = (b2.comp.len() + 1, crate::genome::TEMPLATES.len() + 1);
        let mut v3 = Vec::new();
        for i in 0..nc.max(nt).max(5) {
            let template = if i % nt == 0 { String::new() } else { crate::genome::TEMPLATES[i % nt - 1].to_string() };
            let g = crate::genome::Genome {
                motion: b2.motion[i % b2.motion.len()].id.clone(),
                emitter: b2.emitter[i % b2.emitter.len()].id.clone(),
                emitter_b: if template == "dual" { b2.emitter[(i + 1) % b2.emitter.len()].id.clone() } else { String::new() },
                colour: b2.colour[i % b2.colour.len()].id.clone(),
                palette: b2.palettes[0].clone(),
                space: b2.space[i % b2.space.len()].id.clone(),
                regime: "maximal".into(),
                template,
                comp: if i % nc == 0 { String::new() } else { b2.comp[i % nc - 1].id.clone() },
                ..Default::default()
            };
            v3.push(crate::genome::write(&g, &b2, &gen_dir).unwrap());
        }
        assert_eq!(b2.comp.len(), 4, "all comp blocks present");
        let mut e = Engine::new(&device, &queue, &v3, dir, Quality::Draft, 64, 36, wgpu::TextureFormat::Rgba8Unorm, &[], true);
        for (i, s) in v3.iter().enumerate() {
            e.compile(&s.id).unwrap_or_else(|err| panic!("{}: {err:#}", s.id));
            let mut u = SceneUniforms::default();
            u.res_time = [e.sim.0 as f32, e.sim.1 as f32, 1.0 + i as f32 / 30.0, 1.0 / SIM_HZ as f32];
            u.p0 = [1.0; 4];
            let b = Some((v3[(i + 1) % v3.len()].id.clone(), u));
            let fs = FrameState { scene_a: s.id.clone(), a: u, b, mix: 0.5, flash: 0.0, exposure: 1.0, saturation: 1.0, post: Post::default(), wipe: (i % 5) as u32, reset: i % 2 == 0 };
            e.step(&fs, Some(1.0)).unwrap_or_else(|err| panic!("{}: {err:#}", s.id));
            e.push_history(u.res_time[2] as f64);
            let mut enc = device.create_command_encoder(&Default::default());
            e.finish(&mut enc, &out.create_view(&Default::default()), &fs, false, false);
            queue.submit([enc.finish()]);
        }
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
```

### Task P2.5: `src/render/mod.rs`

- [ ] Add:

```rust
/// A look in the plan reads the display history (its comp block calls hist()), so the engine keeps the ring.
pub fn needs_history(plan: &Plan, scenes: &[Scene]) -> bool {
    scenes
        .iter()
        .filter(|s| plan.segments.iter().any(|g| g.scene == s.id))
        .any(|s| std::fs::read_to_string(scenes::layers_path(s)).is_ok_and(|src| src.contains("hist(")))
}
```

- [ ] In `render_video`, the `Engine::new(…)` call gets the last argument `needs_history(job.plan, job.scenes)`.
- [ ] In `render_chunk`, replace the pre-roll loop with:

```rust
    if t0 > 0.0 {
        for n in -PREROLL_STEPS.min((t0 * SIM_HZ as f64) as i64)..0 {
            let t = t0 + n as f64 / SIM_HZ as f64;
            eng.step(&fs_at(t), None)?;
            // The display history fills exactly as in a continuous render: once per output frame, after its
            // last sub-step (t0 is frame-aligned). 20 s of pre-roll covers the 64-frame ring.
            if n.rem_euclid(SUB as i64) == SUB as i64 - 1 {
                eng.push_history(t);
            }
        }
    }
```

- [ ] In the frame loop, after the `for j in 0..SUB { … }` loop: `eng.push_history(t0 + (SUB * f + SUB - 1) as f64 / SIM_HZ as f64);`
- [ ] `job_hash`: after `FINISH_WGSL`, add `data.extend(engine::TRANS_WGSL.bytes());`.

### Task P2.6: `src/preview.rs`, `src/score.rs`

- [ ] `preview.rs`: both `Engine::new(…)` calls (init and resize) get the last argument `crate::render::needs_history(self.plan, self.scenes)`. In `frame()`'s stepping loop, after `l.fs = Some(fs);`:

```rust
                // One history frame per output frame, as in a render.
                let sub = (SIM_HZ / OUT_FPS) as i64;
                if ((l.sim_t * SIM_HZ as f64).round() as i64).rem_euclid(sub) == sub - 1 {
                    l.engine.push_history(l.sim_t);
                }
```

- [ ] `score.rs`: in `Judge::new`, `Engine::new(…, &[], false)`. The judge scores the feedback with layers off, so it never needs the ring.
- [ ] `cargo build --release`, then `cargo test --release history_layers`. (`engine_runs` needs P3 and P5: it runs at the gate.)

---

## Phase P3: Recipes, comp layer pass · [opus] · owns `src/genome.rs`

### Task P3.1: templates and dispatch

- [ ] Below `NO_SPACE`:

```rust
/// Stateless recipe: the emitter alone, drawn fresh every frame (no trails; crisp, best with raymarched or
/// dense emitters). Shares TEMPLATE_V2's head; motion is defined but unused.
const STATELESS_SCENE: &str = r#"fn scene(uv: vec2<f32>) -> vec4<f32> {
    let p = space(to_centered(uv));
    let drive = {DRIVE} * (0.9 + 0.6 * punch() + 0.3 * snare() + 0.6 * drop_env()) * (1.0 - 0.6 * anticipation());
    var col = colour(emit(p)) * drive;
    col = col / (1.0 + 0.12 * dot(col, vec3<f32>(0.2126, 0.7152, 0.0722)));
    return vec4<f32>(col, 1.0);
}
"#;

/// Dual recipe: the feedback recipe with two emitters; the brighter wins each pixel, B is shown half a hue turn
/// away and swells on snares, so two worlds interleave.
const DUAL_SCENE: &str = r#"fn scene(uv: vec2<f32>) -> vec4<f32> {
    let p = space(to_centered(uv));
    var col = prev(motion(to_uv(p))) * min(motion_decay() * {DECAY}, 0.995);
    let drive = {DRIVE} * (0.5 + 0.9 * punch() + 0.3 * snare() + 0.6 * drop_env()) * (1.0 - 0.6 * anticipation());
    let a = emit(p);
    let b = emit_b(p) * vec2<f32>(0.7 + 0.6 * snare(), 1.0);
    let e = select(a, vec2<f32>(b.x, fract(b.y + 0.5)), b.x > a.x);
    col = blend(col, colour(e) * drive);
    col = col / (1.0 + 0.12 * dot(col, vec3<f32>(0.2126, 0.7152, 0.0722)));
    return vec4<f32>(col, 1.0);
}
"#;
```

- [ ] `wgsl()`: replace the engine-2 part (after the `!g.is_v2()` early return) with:

```rust
    let r = crate::regime::get(&g.regime);
    // A recipe swaps the scene function; everything above it is TEMPLATE_V2's (the feedback recipe is unchanged).
    let head = &TEMPLATE_V2[..TEMPLATE_V2.find("fn scene(").expect("TEMPLATE_V2 has a scene fn")];
    let tpl = match g.template.as_str() {
        "stateless" => format!("{head}{STATELESS_SCENE}"),
        "dual" => {
            let eb = blocks.emitter_b_part(g)?.context("a dual look needs emitter_b")?;
            // Renamed so both emitters link: emit -> emit_b, its params EP -> EP2 (slots 8..10).
            let b = eb.wgsl.replace("fn emit(", "fn emit_b(").replace("EP(", "EP2(");
            format!("{head}fn EP2(i: i32) -> f32 {{ return P(8 + i); }}\n{b}\n{DUAL_SCENE}")
        }
        _ => TEMPLATE_V2.to_string(),
    };
    Ok(tpl
        .replace("{IMG}", &img_wgsl(g))
        .replace("{SPACE}", blocks.space_part(g)?.map_or(NO_SPACE, |b| &b.wgsl))
        .replace("{MOTION}", &m.wgsl)
        .replace("{EMITTER}", &e.wgsl)
        .replace("{COLOUR}", &c.wgsl)
        .replace("{BLEND}", &blend_wgsl(&g.blend))
        .replace("{DECAY}", &format!("{:?}", r.map_or(1.0, |r| r.decay)))
        .replace("{DRIVE}", &format!("{:?}", r.map_or(1.0, |r| r.drive))))
```

### Task P3.2: comp layer pass, dual slots, dress

- [ ] `layers_wgsl`: change the guard to `if !g.has_layers() && g.comp.is_empty() { return Ok(None); }`. Build the source into `let src = LAYERS.replace(…)…;` (same chain as today), then:

```rust
    // Engine 2: a comp block reframes the feedback for display only (this pass is never fed back). Without one
    // the source is byte-for-byte the old one.
    Ok(Some(match blocks.comp_part(g)? {
        Some(c) => src.replace("backdrop(p) + prev(uv) * ", "backdrop(p) + comp(uv) * ") + "\n" + &c.wgsl,
        None => src,
    }))
```

- [ ] `to_scene`: replace the slot building (from `let mut slots` through the space block) with:

```rust
    let eb = blocks.emitter_b_part(g)?;
    if eb.is_some() && g.has_layers() {
        bail!("{}: a dual-emitter look can't have a backdrop or overlay (they share param slots)", g.id());
    }
    let mut slots = vec![(Some(m), SLOTS[0]), (Some(e), SLOTS[1]), (Some(c), SLOTS[2])];
    if g.has_layers() {
        slots.extend([(bd, SLOTS[3]), (ov, SLOTS[4])]);
    }
    // A dual look's second emitter takes slots 8..10 (EP2), padded to 14.
    if let Some(eb) = eb {
        slots.extend([(Some(eb), SLOTS[1]), (None, SLOTS[3] + SLOTS[4] - SLOTS[1])]);
    }
    // Space params live in slots 14..15 (padded when nothing fills 8..13).
    if let Some(sp) = blocks.space_part(g)? {
        if !g.has_layers() && eb.is_none() {
            slots.extend([(None, SLOTS[3]), (None, SLOTS[4])]);
        }
        slots.push((Some(sp), SLOTS[5]));
    }
```

  Update its doc comment: "8 param slots, 14 with layers or a dual emitter, 16 with a space block".
- [ ] `dress`: first line of the body:

```rust
    // A dual look's second emitter uses the layer slots.
    if g.template == "dual" {
        return g.clone();
    }
```

### Task P3.3: `random_v2` picks recipes and comps

- [ ] In `random_v2`, after the cosine-palette line (new hash keys 6..9, so every earlier gene of a given `r` is unchanged):

```rust
    // Recipe: stateless a sixth of the time, dual (a second, different emitter) a sixth, else feedback.
    match h(6) % 6 {
        0 => g.template = "stateless".into(),
        1 if blocks.emitter.len() > 1 => {
            let others: Vec<&Block> = blocks.emitter.iter().filter(|b| b.id != g.emitter).collect();
            g.template = "dual".into();
            g.emitter_b = others[(h(7) % others.len() as u64) as usize].id.clone();
        }
        _ => {}
    }
    // A display-only comp framing a third of the time.
    if !blocks.comp.is_empty() && h(8) % 3 == 0 {
        g.comp = nth_id(&blocks.comp, h(9) as usize);
    }
```

  Update the doc comment to mention recipes and comps.

### Task P3.4: tests

- [ ] Append to `mod tests`:

```rust
    fn comp_block(id: &str, wgsl: &str) -> Block {
        Block { id: id.into(), params: vec![], wgsl: wgsl.into(), toml: String::new() }
    }

    #[test]
    fn recipes_comp_and_dual_slots() {
        let mut b = blocks().pinned(1);
        b.add("space", space_block("space_test", "fn space(p: vec2<f32>) -> vec2<f32> { return p; }")).unwrap();
        b.add("comp", comp_block("comp_test", "fn comp(uv: vec2<f32>) -> vec3<f32> { return prev(uv.yx); }")).unwrap();
        let base = Genome { regime: "calm".into(), space: "space_test".into(), ..genome() };
        let st = wgsl(&Genome { template: "stateless".into(), ..base.clone() }, &b).unwrap();
        assert!(st.contains("fn space") && !st.contains("prev(motion("), "stateless draws no feedback");
        let eb = b.emitter.iter().find(|x| x.id != base.emitter && !x.params.is_empty()).unwrap();
        let dual = Genome { template: "dual".into(), emitter_b: eb.id.clone(), ..base.clone() };
        let src = wgsl(&dual, &b).unwrap();
        assert!(src.contains("fn emit_b(") && src.contains("fn EP2(") && src.contains("emit_b(p)"));
        let sc = to_scene(&dual, &b, Path::new(".")).unwrap();
        assert_eq!(sc.params.len(), 16);
        assert_eq!((sc.params[8].min, sc.params[8].max), (eb.params[0].min, eb.params[0].max), "emitter B in slots 8..10");
        assert!(to_scene(&Genome { backdrop: "back_sky".into(), ..dual.clone() }, &b, Path::new(".")).is_err(), "dual has no layer slots");
        assert_eq!(dress(&dual, &b, 3, 1, "HI"), dual, "dual looks are never dressed");
        // Comp: a layer pass of its own (none without comp or layers), reframing the feedback only.
        assert!(layers_wgsl(&base, &b).unwrap().is_none());
        let l = layers_wgsl(&Genome { comp: "comp_test".into(), ..base.clone() }, &b).unwrap().unwrap();
        assert!(l.contains("comp(uv) * 1.0") && l.contains("prev(uv.yx)"));
        assert_eq!(wgsl(&base, &b).unwrap(), wgsl(&Genome { comp: "comp_test".into(), ..base.clone() }, &b).unwrap(), "comp never touches the feedback");
    }

    #[test]
    fn random_v2_picks_recipes_and_comps() {
        let mut b = blocks().pinned(1);
        for k in 0..2 {
            b.add("comp", comp_block(&format!("comp_t{k}"), "")).unwrap();
        }
        let reg = &crate::regime::REGIMES[0];
        let gs: Vec<Genome> = (0..96).map(|r| random_v2(&b, crate::types::hash(11, r), reg)).collect();
        let distinct = |f: &dyn Fn(&Genome) -> String| gs.iter().map(f).collect::<std::collections::HashSet<_>>().len();
        assert_eq!(distinct(&|g| g.template.clone()), 3, "feedback + stateless + dual");
        assert_eq!(distinct(&|g| g.comp.clone()), 3, "none + 2 comps");
        assert!(gs.iter().all(|g| (g.template == "dual") == !g.emitter_b.is_empty() && g.emitter_b != g.emitter));
    }
```

  If a distinct-count assert flakes on this exact hash set, raise 96 → 192. Don't weaken the assert.
- [ ] `cargo test --release recipes_comp random_v2 engine1 engine2_saved vary` → green.

---

## Phase P4: Director · [opus] · owns `src/director.rs`

### Task P4.1: drops reset, hard shots first, styled morphs

- [ ] In `plan()`, replace `flash_near_drop`:

```rust
    // A change landing near a drop flashes (engine 2: clears the feedback) instead of its usual transition.
    let hit = Transition { kind: if req.engine >= 2 { TransitionKind::Reset } else { TransitionKind::Flash }, duration: 0.0 };
    let flash_near_drop = |t: f64, other: Transition| if near_drop(t) { hit } else { other };
```

- [ ] Shot ordering: replace the comment and `is_flash` with:

```rust
        // Hard cuts (flash, reset) are placed first, so a morph too close to one is the shot that gets dropped.
        let is_flash = |s: &&Shot| !s.transition.kind.is_morph();
```

- [ ] Morph-window clamp loop: `if t.kind == TransitionKind::Morph` → `if t.kind.is_morph()`.
- [ ] Right before that clamp loop:

```rust
    // Engine 2: generated segments develop at their phrases, and morphs get a seed-chosen style.
    if req.engine >= 2 {
        if let Some(blocks) = req.blocks {
            segments = phrase_variants(a, segments, blocks, req.gen_dir, req.mode == Mode::Single, req.engine)?;
        }
        for s in segments.iter_mut().filter(|s| !s.locked && s.transition_in.kind == TransitionKind::Morph) {
            s.transition_in.kind = MORPHS[(hash(s.seed, 9) % MORPHS.len() as u64) as usize];
        }
    }
```

  (`let mut segments` already exists.)

### Task P4.2: phrase variants, reroll mapping, chapters

- [ ] Below `pick`:

```rust
/// How a phrase variant is entered: a plain cut or a styled morph one bar long.
const PHRASE_CUTS: [TransitionKind; 5] = [TransitionKind::Cut, TransitionKind::Wipe, TransitionKind::Iris, TransitionKind::Melt, TransitionKind::Cells];

/// Engine 2: a generated segment develops at its 16/32-bar phrases. From 8 bars in until 8 bars before its
/// end, each such phrase (none within a bar of a drop: drops get their own cut) starts a variant of the look
/// before it (genome::vary: another fold, comp framing or palette). Locked segments are left alone.
fn phrase_variants(a: &Analysis, segs: Vec<Segment>, blocks: &Blocks, gen_dir: &std::path::Path, single: bool, engine: u32) -> Result<Vec<Segment>> {
    let bar = bar_len(a);
    let mut out = Vec::with_capacity(segs.len());
    for mut s in segs {
        let Some(mut g) = s.genome.clone().filter(|_| !s.locked && !s.variant) else {
            out.push(s);
            continue;
        };
        let cuts: Vec<f64> = a
            .phrases
            .iter()
            .filter(|p| p.bars >= 16)
            .map(|p| p.time)
            .filter(|&t| t >= s.start + 8.0 * bar && t <= s.end - 8.0 * bar && !a.drops.iter().any(|d| (d.time - t).abs() <= bar))
            .collect();
        let mut pieces = Vec::with_capacity(cuts.len());
        for (k, &t) in cuts.iter().enumerate() {
            let seed = hash(s.seed, 0x7A00 + k as u64);
            g = genome::vary(&g, blocks, seed);
            let sc = genome::to_scene(&g, blocks, gen_dir)?;
            let end = cuts.get(k + 1).copied().unwrap_or(s.end);
            let kind = PHRASE_CUTS[(hash(seed, 9) % PHRASE_CUTS.len() as u64) as usize];
            pieces.push(Segment {
                start: t,
                end,
                scene: g.id(),
                seed,
                locked: false,
                palette: 0,
                title: None,
                transition_in: Transition { kind, duration: if kind.is_morph() { bar } else { 0.0 } },
                variations: variations(a, &sc, seed, t, end, single, engine),
                mods: mods(&sc, seed, engine),
                genome: Some(g.clone()),
                variant: true,
            });
        }
        if let Some(&first) = cuts.first() {
            s.end = first;
        }
        out.push(s);
        out.extend(pieces);
    }
    Ok(out)
}
```

- [ ] Reroll mapping: at the top of `plan()` after the `bail!`:

```rust
    // --reroll counts the saved plan's segments; a phrase variant belongs to the segment it varies.
    let reroll: Vec<usize> = match req.previous {
        Some(p) if p.segments.iter().any(|s| s.variant) => {
            req.reroll.iter().map(|&i| p.segments.iter().take(i + 1).filter(|s| !s.variant).count().saturating_sub(1)).collect()
        }
        _ => req.reroll.to_vec(),
    };
```

  and in the segments loop: `let rerolls = reroll.iter().filter(|&&r| r == i).count() as u64;`.
- [ ] `chapters`: iterate `plan.segments.iter().filter(|s| !s.variant)` in the folding loop (variants are not chapters).

### Task P4.3: `frame_state`

- [ ] `in_morph`: `s.transition_in.kind == TransitionKind::Morph` → `s.transition_in.kind.is_morph()`.
- [ ] Replace Task 1's `wipe: 0, reset: false,` in the `FrameState` literal with:

```rust
        wipe: morph.map_or(0, |(j, _)| segs[j].transition_in.kind.style()),
        // A Reset cut clears the feedback on the one step at its start.
        reset: segs.get(i).is_some_and(|s| s.transition_in.kind == TransitionKind::Reset && t >= s.start && t - s.start < 1.0 / SIM_HZ as f64),
```

### Task P4.4: tests

- [ ] Append to `mod tests`:

```rust
    #[test]
    fn engine2_cuts_reset_on_drops_and_morphs_get_styles() {
        let mut a = test_analysis();
        a.track_changes[0].kind = ChangeKind::Cut; // 96 s
        a.drops.push(Drop { time: 96.0, build_start: 80.0, strength: 1.0 });
        let scenes = test_scenes();
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Auto, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 0.0, vibe: None, genome_for: None, blocks: None, gen_dir: Path::new("."), shots: &[], engine: 2, pool: Pool::Builtin };
        let p = plan(&req).unwrap();
        // A cut near a drop clears the feedback, on one step only; engine 1 flashes instead.
        assert_eq!(p.segments[1].transition_in.kind, TransitionKind::Reset);
        assert!(frame_state(&p, &a, &scenes, 96.0, 64, 64, Quality::Draft).reset);
        assert!(!frame_state(&p, &a, &scenes, 96.0 + 1.0 / 60.0, 64, 64, Quality::Draft).reset);
        assert_eq!(plan(&PlanRequest { engine: 1, ..req }).unwrap().segments[1].transition_in.kind, TransitionKind::Flash);
        // Morphs (track blends from 192 s) get seed-chosen styles, and the frame carries the style.
        let kinds: std::collections::HashSet<String> = (0..16)
            .flat_map(|seed| plan(&PlanRequest { seed, ..req }).unwrap().segments.into_iter().skip(2).map(|s| format!("{:?}", s.transition_in.kind)))
            .collect();
        assert!(kinds.len() >= 3 && kinds.iter().all(|k| k != "Cut"), "{kinds:?}");
        let s = &p.segments[2];
        let f = frame_state(&p, &a, &scenes, s.start, 64, 64, Quality::Draft);
        assert!(f.b.is_some() && f.wipe == s.transition_in.kind.style());
        // Hard shots are placed before morph shots: the reset at 32 s wins over the morph that snaps onto it.
        let shots = [
            Shot { time: 30.0, transition: Transition { kind: TransitionKind::Morph, duration: MORPH_LEN } },
            Shot { time: 32.0, transition: Transition { kind: TransitionKind::Reset, duration: 0.0 } },
        ];
        let single = plan(&PlanRequest { mode: Mode::Single, shots: &shots, ..req }).unwrap();
        assert_eq!(single.segments.iter().map(|s| s.transition_in.kind).collect::<Vec<_>>(), [TransitionKind::Cut, TransitionKind::Reset]);
    }

    #[test]
    fn phrase_variants_develop_generated_segments() {
        let a = test_analysis();
        let scenes = test_scenes();
        let block = |id: &str| crate::genome::Block {
            id: id.into(),
            params: vec![crate::scenes::ParamRange { name: String::new(), min: 0.0, max: 1.0, fixed: false }],
            wgsl: String::new(),
            toml: String::new(),
        };
        let blocks = Blocks {
            motion: vec![block("m0")],
            emitter: vec![block("e0")],
            colour: vec![block("c0")],
            backdrop: vec![],
            overlay: vec![],
            space: vec![block("space_a"), block("space_b")],
            comp: vec![],
            palettes: vec![["#000000".into(), "#ff0000".into(), "#00ff00".into(), "#0000ff".into()]],
        };
        let gen = |_, _, r| Some(genome::random(&blocks, r));
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Single, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 1.0, vibe: None, genome_for: Some(&gen), blocks: Some(&blocks), gen_dir: Path::new("."), shots: &[], engine: 2, pool: Pool::Builtin };
        let p = plan(&req).unwrap();
        assert!(p.segments.len() > 10, "{}", p.segments.len());
        assert!(!p.segments[0].variant && p.segments[1..].iter().all(|s| s.variant));
        for w in p.segments.windows(2) {
            assert_eq!(w[0].end, w[1].start);
            let (x, y) = (w[0].genome.as_ref().unwrap(), w[1].genome.as_ref().unwrap());
            assert!(x.space != y.space || x.palette != y.palette, "each variant changes something");
            assert_eq!(y.id(), w[1].scene);
        }
        assert!(p.segments.iter().all(|s| (s.start - 64.0).abs() > 2.0), "no variant on the drop");
        assert!(chapters(&p).is_none(), "variants are not chapters");
        // --reroll of a variant re-rolls the segment it belongs to.
        let p2 = plan(&PlanRequest { previous: Some(&p), reroll: &[3], ..req }).unwrap();
        assert_ne!(p2.segments[0].seed, p.segments[0].seed);
        // Engine 1 never splits.
        assert_eq!(plan(&PlanRequest { engine: 1, ..req }).unwrap().segments.len(), 1);
    }
```

- [ ] `cargo test --release engine2 phrase_variants director engine1 shots` → green.

---

## Phase P5: Comp blocks · [opus] · owns only new files `scenes/blocks/comp_*`

Rules (CONTRACT.md):
- No params; vary with `seed()`.
- Prefix every helper with the block id.
- Read the feedback with `prev()` or `hist()` only.
- Write `1.0 - smoothstep(a, b, x)`, never reversed edges.

Ids must match `V2_BLOCKS`. Nothing to run: the gate's `engine_runs` compiles and runs each block.

- [ ] `comp_mirror.toml`

```toml
kind = "comp"
description = "Mirror (display only): the finished frame shown as a left-right or four-way mirror image; the simulation underneath runs unfolded"
```

  `comp_mirror.wgsl`

```wgsl
// Mirror (display only): the finished frame folds into a left-right mirror image, or a four-way one (by
// seed). The simulation underneath runs unfolded, so this is only a framing.
fn comp(uv: vec2<f32>) -> vec3<f32> {
    var q = uv;
    q.x = 0.5 - abs(q.x - 0.5);
    if (fract(seed() * 7.0) > 0.5) { q.y = 0.5 - abs(q.y - 0.5); }
    return prev(q);
}
```

- [ ] `comp_kaleido.toml`

```toml
kind = "comp"
description = "Kaleidoscope (display only): the finished frame shown as 4-9 slowly turning mirrored wedges; the feedback keeps its own shape"
```

  `comp_kaleido.wgsl`

```wgsl
// Kaleidoscope (display only): the frame is shown as 4-9 mirrored wedges (by seed) that turn slowly; the
// feedback underneath keeps its own shape, so the same simulation gets a second framing.
fn comp(uv: vec2<f32>) -> vec3<f32> {
    let seg = TAU / (4.0 + floor(seed() * 6.0));
    let p = to_centered(uv);
    var a = atan2(p.y, p.x) + 0.05 * time();
    a = a - seg * floor(a / seg);
    a = abs(a - 0.5 * seg);
    return prev(to_uv(vec2<f32>(cos(a), sin(a)) * length(p)));
}
```

- [ ] `comp_slitscan.toml`

```toml
kind = "comp"
description = "Slit-scan (display only): each row (or column) shows the frame from a different moment up to 2 s back, so motion smears into rubbery time-stretches"
```

  `comp_slitscan.wgsl`

```wgsl
// Slit-scan (display only): each row (or column, by seed) shows the frame from a different moment, up to
// ~2 s back, so moving shapes stretch into rubbery time-smears. The depth breathes with the phrase.
fn comp(uv: vec2<f32>) -> vec3<f32> {
    let along = select(uv.y, uv.x, fract(seed() * 13.0) > 0.5);
    return hist(uv, along * 63.0 * (0.6 + 0.4 * sin(TAU * phrase_phase())));
}
```

- [ ] `comp_echo.toml`

```toml
kind = "comp"
description = "Time echo (display only): the frame plus three fading, slightly larger, hue-shifted copies from 6, 12 and 18 frames ago; kicks brighten the echoes"
```

  `comp_echo.wgsl`

```wgsl
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
```

---

## Phase P6: Wiring · [opus] · owns `src/main.rs`, `src/evolve.rs`, `src/wizard.rs`

### Task P6.1: `src/evolve.rs`

- [ ] In `best_v2`'s `measure` closure, replace the last line with:

```rust
        // Each recipe widens the regime's ranges its own way (regime::gate).
        score::passes(&m, &crate::regime::gate(regime, &g.template)).then(|| (g, score::descriptor(&m)))
```

### Task P6.2: `src/main.rs`

- [ ] `Cli::generated` doc: "Share of scenes that are freshly generated (0 = only the hand-made ones, 1 = all generated). Default: the saved plan's value, else 1 with engine 2 (0.5 with engine 1)."
- [ ] The default:

```rust
    // Unset = what the saved plan used (so a plain re-run reuses it), else all generated with engine 2 (its takes
    // share no screen time with the hand-made scenes) and half with engine 1.
    let generated = cli.generated.or(previous.as_ref().map(|p| p.generated)).unwrap_or(if engine >= 2 { 1.0 } else { 0.5 });
```

- [ ] Replace the `let shots: Vec<Shot> = if cli.layers { … } else { Vec::new() };` block with:

```rust
            // Drops are hard cuts into a new shot: --layers flashes; engine 2 resets (black, then the new world
            // rebuilds), with or without --layers. --layers also morphs into each build-up.
            let drop_cut = Transition { kind: if looks >= 2 { TransitionKind::Reset } else { TransitionKind::Flash }, duration: 0.0 };
            let shots: Vec<Shot> = analysis
                .drops
                .iter()
                .flat_map(|d| {
                    let build = cli.layers.then_some(Shot { time: d.build_start, transition: Transition { kind: TransitionKind::Morph, duration: MORPH_LEN } });
                    let drop = (cli.layers || looks >= 2).then_some(Shot { time: d.time, transition: drop_cut });
                    build.into_iter().chain(drop)
                })
                .collect();
```

  Engine 1 is unchanged: with `--layers` the shots are still [morph, flash] per drop in that order; without it there are none.

### Task P6.3: `src/wizard.rs`

- [ ] Replace the "Visual style?" block with (the values are explicit, so a saved plan's share never overrides the choice):

```rust
    let styles = vec![
        "Freshly generated (recommended)",
        "Mix of hand-made and freshly generated",
        "Hand-made only",
    ];
    let cur = match d.generated {
        Some(g) if g == 0.0 => 2,
        Some(g) if g < 1.0 => 1,
        _ => 0,
    };
    a.generated = Some(match Select::new("Visual style?", styles).with_starting_cursor(cur).raw_prompt()?.index {
        1 => 0.5,
        2 => 0.0,
        _ => 1.0,
    });
```

- [ ] `cargo build --release`, then `cargo test --release regime wizard evolve` → green.

---

## Gate

- [ ] `cargo test --release`: everything green, including:
  - `engine1_genomes_are_frozen`, `engine1_plan_and_frames_are_frozen`, `engine2_saved_looks_are_frozen`, `engine2_saved_frames_are_frozen`, `library_pins` (golden values never edited)
  - `engine_runs` (all 4 comp blocks on a real history ring, both recipes, every morph style, reset), `engine2_block_helpers_are_prefixed`, `history_layers_follow_output_frames`
  - `vary_changes_exactly_one_structural_gene`, `recipes_widen_the_regime_gate`, `recipes_comp_and_dual_slots`, `random_v2_picks_recipes_and_comps`
  - `engine2_cuts_reset_on_drops_and_morphs_get_styles`, `phrase_variants_develop_generated_segments`

  Fix a failing block in its own `.wgsl`, then re-run only `cargo test --release engine_runs`. If a dual pairing fails to link (two emitters defining the same helper name), fix the emitter's helper prefix. Don't special-case the pair.
- [ ] **Engine 1 is byte-identical to `main`** (`docs/MANUAL-TESTS.md` §9 method):
  - Build `main` in a separate worktree (`git worktree add ..\rv-main main`, `cargo build --release`). Give each tree its own copy of `testdata\dnb8.wav` in its own folder; copy it from `C:\Prathamesh\Ableton\Music\dnb5` if it's missing.
  - **Caveat:** libx264's multithreaded encode isn't repeatable. In BOTH trees, temporarily add `-threads 1` to the x264 args in `src\render\mod.rs` (the `-c:v libx264` line), rebuild, and revert afterwards.
  - Run `visualiser <wav> --replan --preview 1:00 --quality draft` in each tree, adding `--engine 1` on the branch only. Then `ffmpeg -i <preview>.mp4 -f framemd5 -an <main|branch>.md5`. The two files must be equal.
  - Repeat with `--layers`. For the `main` run, first move the 10 untracked `scenes\blocks\*runner*`, `*trippy*`, `colour_cube_mono.*` and `emit_beat_cube.*` files into the scratchpad, then put them back. Main loads them; the branch pins them out.
  - Any difference: find the change on the default path and fix it (usual suspects: the shot ordering, `flash_near_drop`, the `step` morph branch). Don't loosen the check.
- [ ] **A saved engine-2 plan renders byte-identically.**
  - Make a worktree at the base (`git worktree add ..\rv-base 231afa2`) with the same `-threads 1` patch.
  - Run `visualiser <wav> --engine 2 --generated 1 --replan --preview 1:00 --quality draft --seed 3` there.
  - Copy that project's `plan.json` (from the printed `project data:` folder) into the branch tree's data folder for its own wav copy.
  - Run the branch with `--engine 2 --generated 1 --preview 1:00 --quality draft --seed 3` and **no** `--replan`. It must print "using saved plan".
  - The framemd5 files must be equal. Revert `-threads 1`; remove both worktrees.
- [ ] **Planning time** (row 14: every drop adds a judged segment):
  - Time `visualiser testdata\dnb8.wav --engine 2 --generated 1 --replan --plan-only --quality draft` on the base and on the branch (`Measure-Command { … }`), and record both.
  - If the branch takes more than 2× as long, halve the budget for short segments. In `main.rs` `evolved`, before `evolve::best_v2`, add:

```rust
                    // ponytail: drop cuts add a judged segment each; short ones get half the budget.
                    let budget = if end - start < 32.0 * director::bar_len(&analysis) { (budget / 2).max(4) } else { budget };
```

    Then re-time.
- [ ] **Smoke renders** for the user to watch:
  - Open the dnb8 project's `analysis.json` and note the first drop time `D`.
  - For N = 1, 2, 3, 4, run `visualiser testdata\dnb8.wav --engine 2 --generated 1 --replan --preview <D-8s> --quality draft --seed N`.
  - In each, check:
    - The drop goes black and a new world rebuilds.
    - Transitions are styled (wipe, iris, melt or tiles, not only crossfades).
    - At least one phrase variant shows inside the 30 s: a fold, framing or palette change.
    - No flashes come faster than once per bar.
  - Grep the four `plan.json` files for `"template"` and `"comp"`. If none uses `comp_slitscan`/`comp_echo`, `stateless` or `dual`, try seeds 5–12 until one of each is found, and watch it: slit-scan rows should lag in time, and the echo should leave ghost trails.
- [ ] `/simplify` → `/review` over `git diff 231afa2...HEAD`. Fix the findings, then run `cargo test --release` once.
- [ ] Docs [sonnet]:
  - `docs/USAGE.md`, the Engine 2 section gains "Shows":
    - Drop resets.
    - Phrase variants every 16/32 bars, which are not chapters; `--reroll N` of a variant re-rolls its segment.
    - Transition styles.
    - Comp framings and slit-scan / time echo (display only).
    - The `stateless` and `dual` recipes.
    - `--generated` default 1 for new engine-2 projects; saved projects keep theirs.
    - The history ring's memory (~133 MB at 1080p, only when a look uses it).
    - Planning takes longer: one judged look per drop.

    Also update the `--generated` option row and the wizard's "Visual style?" bullet.
  - `README.md`: one line on shows (drop resets, phrase development, styled transitions, slit-scan).
  - `docs/MANUAL-TESTS.md`: new §10 with the smoke checklist, the saved-plan framemd5 check and the planning-time measurement.
- [ ] Commit `docs: engine 2 shows (core variety phase 2)`.

Wave-1 commits (orchestrator, after the wave joins and the per-task tests pass, by explicit paths):
- `feat(core): history ring, styled morphs, reset cuts` (P2 files)
- `feat(core): stateless and dual recipes, comp layer pass` (`src/genome.rs`)
- `feat(core): drop resets, styled morphs, phrase variants` (`src/director.rs`)
- `feat(blocks): comp framings, slit-scan and time echo` (`scenes/blocks/comp_*`)
- `feat(core): engine-2 drop cuts, all-generated default, recipe gates` (P6 files)

---

## Self-review notes

- **Spec coverage:**

  | Row | What the user sees | Where |
  |---|---|---|
  | 13 | One simulation in several framings | Task 1 comp kind and `comp_part`; P3 comp layer pass and `random_v2` comps; P4 variants swap comps; P5 blocks |
  | 14 | A new world on the drop, black-and-rebuild | Task 1 `Reset`/`FrameState.reset`; P6 drop shots; P4 `flash_near_drop`, hard shots first, `frame_state.reset`; P2 `step` clear |
  | 15 | The track develops every 16/32 bars | Task 1 `vary`, `Segment.variant`; P4 `phrase_variants`, reroll mapping, chapters |
  | 16 | Cuts look designed | Task 1 kinds, `style`, `MORPHS`; P2 `trans.wgsl` and the `step` branch; P4 styled morphs and variant entries |
  | 17 | Rubbery, time-smeared motion | P2 ring, `push_history`, pre-roll fill, `needs_history`, prelude `hist`; P5 `comp_slitscan`, `comp_echo` |
  | 19 | Takes don't share screen time with the 7 hand-made scenes | P6 `--generated` default 1 under engine 2; wizard explicit values, generated recommended |
  | 21 | Other render grammars per seed | Task 1 `TEMPLATES` and `regime::gate`; P3 stateless/dual and dual slots; P6 gate in `best_v2` |

- **Engine verdicts:** all seven rows are engine 2 (table in Architecture). Phase 3 applies the same rule: extension → engine 2; a different render grammar or pipeline → engine 3.
- **Byte-identical engine 1:**
  - New kinds and fields never appear in engine-1 plans.
  - `Morph` keeps `wipe = 0`, so `step` takes the old `copy_pl` branch.
  - `reset` is false unless the kind is `Reset`.
  - Shot ordering is unchanged for Flash/Morph shots.
  - The drop shots for engine 1 are the old ones.
  - `LAYERS` output is unchanged without a comp; `dress` changes only for dual looks.
  - The prelude's new binding and `hist()` are unused by engine-1 scenes. `job_hash` changes, so old resume chunks are discarded once, which is expected.
  - The goldens and the framemd5 gate check all of this.
- **Saved engine-2 plans reproducible:**
  - Genes and `variant` are skipped when empty or false.
  - The feedback template text is untouched: `head` plus `TEMPLATE_V2`.
  - `random_v2` only appends draws on new hash keys.
  - Planning changes only reach new plans.
  - `engine2_saved_*` and the base-commit framemd5 check prove it.
- **Flash limit:**
  - Styled morphs switch each pixel once.
  - `Reset` lands on drops (or on a cut within a bar of one), together with the existing drop flash: one flash per drop, far apart.
  - Phrase variants come ≥ 16 bars apart.
  - Strobe and invert gating is unchanged (≤ 1 per bar).
- **Cost:**
  - **Memory:** the history ring is allocated only when a plan's comp calls `hist(`: 133 MB at 1080p, 236 MB at 1440p.
  - **Planning:** one judged look per drop. Draft budget 10, so a mix with 40 drops runs about 400 more judge renders, measured in the gate, with a fallback. Variants are free.
  - **Rendering:** dual looks with two raymarched emitters are heavy, which is fine offline.
- **Names used across tasks:**
  - Types: `TransitionKind::{Reset, Wipe, Iris, Melt, Cells}`, `is_morph`, `style`, `MORPHS`, `FrameState.{wipe, reset}`, `Segment.variant`.
  - Genome and regime: `Genome.{template, emitter_b, comp}`, `Blocks.comp`, `comp_part`, `emitter_b_part`, `TEMPLATES`, `vary`, `STATELESS_SCENE`, `DUAL_SCENE`, `regime::gate`.
  - Render: `Engine::new(.., history)`, `push_history`, `hist_layer`, `HIST_LEN`, `TRANS_WGSL`, `fs_trans`, `fs_hist`, `render::needs_history`, prelude `hist`/`hist_tex`.
  - Director: `PHRASE_CUTS`, `phrase_variants`.
  - Blocks: `comp_echo`, `comp_kaleido`, `comp_mirror`, `comp_slitscan`.

  These are consistent across tasks.
- **Dependencies inside Wave 1:** P4 uses `genome::vary`, `to_scene` and `MORPHS`, and P6 uses `regime::gate`, `TransitionKind::Reset` and `TEMPLATES`. All of them come from Task 1, so the five tasks compile independently. `engine_runs` needs P3 (templates, comp pass) and P5 (blocks), so it runs only at the gate.
