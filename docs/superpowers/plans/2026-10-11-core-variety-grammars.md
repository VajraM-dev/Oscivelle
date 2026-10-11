# Core Variety: Visual Grammars Implementation Plan ("not another tunnel")

> **For agentic workers:** REQUIRED SUB-SKILL: follow `/code-implementation-build-rules` (orchestrator dispatches, subagents build; waves below). Use superpowers:subagent-driven-development for per-task dispatch. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Core-only engine-2 takes are different from each other but feel the same: a tunnel, a spin, a flow. Make looks differ in *grammar* (how the picture is organised and moves), not just in blocks.

**Measured** with an exact Python replica of `random_v2` over 20k seeds (`sim.py`, in the session scratchpad):

| Feature (by block) | now | target (`sim_grammar.py`, this plan) |
|---|---|---|
| C: centred radial/mirror symmetry | 91.1% | 13.2% |
| R: time-driven rotation | 75.6% | 12.5% |
| Z: steady radial zoom | 68.0% | 16.5% |
| T: tunnel / vanishing point | 39.6% | 6.6% |
| D: raymarched 3D camera | 20.1% | 4.0% |
| none of R Z T C D | 3.2% | 79.0% |

Causes:
1. A space fold is drawn 5/6 of the time, all 6 folds are centred, and `vary` rarely drops one.
2. 9 of 10 motion blocks zoom about the centre.
3. The spin is time-driven nearly everywhere.
4. The non-radial blocks are mostly fbm flow. There are no scroll, still, step, shear or grid blocks.
5. The judge rejects stillness (minimum motion) and is blind to geometry (cells bin brightness, motion and colourfulness only).

**Architecture:** Everything stays in **engine 2** (it extends the pipeline with a gene, blocks and judge changes; no new pipeline).
- **Grammar gene.** `Genome.grammar` is one of `GRAMMARS = [radial, flow, grid, scroll, still]`. `best_v2` draws it per segment from the segment seed, like the regime, **before any block**.
- **Grammar pools.** A frozen table `BLOCK_TAGS` in `genome.rs` (pinning style, like `V2_BLOCKS`) says which grammars each motion, emitter, space and comp block belongs to. `Blocks::grammar_pool(grammar)` keeps only those blocks; colours, layers and palettes are grammar-neutral. `random_v2`, `mutate_v2` (through `best_v2`'s pool) and `vary` pick only from the grammar's pool. `radial` is the whole pre-existing library, so a radial look is today's look. Grammar `""` (old genomes, tests) means no constraint.
- **Folds less often.** A fold is drawn 1/3 of the time (was 5/6). `vary` drops a present fold or framing half the time.
- **New non-radial blocks (13, new files only):**
  - 5 motions: `drift` (seeded direction, wraps, steps on kicks), `shear` (row wave), `still` (decay only), `stutter` (holds, then jumps a tile on each kick), `sink` (inward and off-centre, no spin).
  - 3 spaces: `tile` (mirrored wallpaper), `pixel` (beat-re-rolled mosaic), `bands` (sliding strips).
  - 5 emitters: `steps` (step sequencer), `scan` (beat-locked scan bars), `wave` (band-driven oscilloscope), `scatter` (kick-scattered sparks), `halftone`.
- **Judge (engine-2 paths only):**
  - `regime::gate` widens the minimum motion for `still` looks.
  - `Metrics.symmetry` becomes a centredness measure: the Pearson correlation of the frame with its left-right mirror or with itself turned 1/2 to 1/5 of a turn.
  - `score::cell` gains a centred/not-centred axis (54 cells), so `pick_v2`'s seed-chosen cell stops favouring tunnels. Only `best_v2`/`pick_v2` use the descriptor and cell; engine 1's `verdict` reads neither, so it is unchanged.

**Saved work stays reproducible:**
- `grammar` is `skip_serializing_if` empty, so saved genome ids, plan JSON and WGSL are unchanged.
- No existing block file is edited. The `engine1_*` and `engine2_saved_*` goldens are never edited.
- What a given seed produces under engine 2 changes. The user approved this.

**Grammar pools** (counts on the pinned engine-2 library after this plan):

| Grammar | Motions | Emitters | Spaces | Comps |
|---|---|---|---|---|
| radial | the 10 existing | the 17 existing | the 6 existing | all 4 |
| flow | drift, etch, ink, sink, swirl (5) | blobs, cells, clouds, contours, filaments, lyapunov, scatter, warpfbm (8) | none | slitscan |
| grid | mosh, shear, sort, stutter (4) | cells, halftone, scan, steps, truchet (5) | bands, pixel, tile | slitscan |
| scroll | drift, shear, sort (3) | clouds, scan, scatter, stars, truchet, wave (6) | bands | slitscan |
| still | still, stutter (2) | blobs, cells, contours, halftone, lyapunov, scatter, stars, steps, wave (9) | pixel | slitscan |

**Tech Stack:** Rust 2021, wgpu, WGSL. No new crates.

**Not in this plan:**
- Procedural emitters (next plan).
- A separate `depth` grammar. The raymarched 3D emitters stay in `radial`; split them out if radial still feels samey.
- Grammar weights other than uniform. Each grammar is 1/5, so a radial (tunnel-capable) look is 1 in 5. Retune `GRAMMARS` order and weights once the user has watched takes.
- A CLI flag or wizard question to force a grammar.
- Judging phrase variants (unchanged from phase 2; `vary` still only makes safe changes).
- Retagging AI-written blocks: they are pinned out, so they never reach a grammar pool.

---

## Build setup (from /code-implementation-build-rules)

`T2 Standard · no web frontend (CLI + GPU shaders; design gate n/a) · review: /simplify → /review`

- **Branch:** stays `feature/core-variety-2`. Record `BASE=$(git rev-parse HEAD)` before Task 0. It is the base for the saved-engine-2 framemd5 check. Never commit to `main`; the user merges.
- **Untracked AI blocks:** `scenes/blocks/*runner*`, `*trippy*`, `colour_cube_mono.*` and `emit_beat_cube.*` (14 files) are AI-written and untracked. **Never commit or delete them.** Stage by explicit path only; never `git add -A` or `git add .`. Pinning (`V1_BLOCKS`/`V2_BLOCKS`) and `BLOCK_TAGS` exclude them.
- **Orchestrator** plans, dispatches, joins and commits. Subagents do the work and **don't commit**. Independent Wave-1 tasks go out as multiple `Agent` calls **in a single message**.
- **Model routing:** `opus` for genome, judge and every shader block; `sonnet` for docs.
- **Testing discipline:** during the wave, Rust tasks run `cargo build --release` plus their own named tests. Block tasks run nothing: WGSL compiles in the gate's `engine_runs`. Full `cargo test --release` runs once at the gate; after a fix, re-run only the failing test, then one full run.
- **ponytail:** minimum code, one runnable `#[test]` per non-trivial logic, `// ponytail:` on deliberate shortcuts naming the ceiling.
- **Commits** (orchestrator, by explicit paths): `feat(core): …`, `feat(blocks): …`, `docs: …`.

### Waves

```
Wave 0 (orchestrator): Task 0 contract → cargo test --release engine1 engine2_saved library_pins vary gate random_v2 regime
Wave 1 (parallel, 6 agents):
  G  genome picking: random_v2 body, vary, grammar + variety tests [opus]
  J  judge: centredness metric, 54 cells, pick_v2 test [opus]
  M  motion blocks: drift, shear, sink, still, stutter [opus]
  S  space blocks: bands, pixel, tile [opus]
  E1 emitter blocks: steps, scan, wave [opus]
  E2 emitter blocks: scatter, halftone [opus]
Gate → /simplify → /review → docs [sonnet] → commit
```

**File ownership (no collisions):**
- **Task 0** (orchestrator) owns:
  - `src/regime.rs` and `scenes/blocks/CONTRACT.md`.
  - The contract edits in `src/genome.rs`: the `grammar` field, `GRAMMARS`, `BLOCK_TAGS`, `V2_BLOCKS`, `grammar_pool`, the `random_v2` signature and gene line, and test-literal fixes.
  - `best_v2`'s body in `src/evolve.rs`.
  - The `b2.space.len()` assert in `src/render/engine.rs`.
- **Wave 1:**
  - **G** owns `src/genome.rs` outside Task 0's edits: `random_v2` body, `vary`, new tests.
  - **J** owns `src/score.rs` and the `mod tests` of `src/evolve.rs`.
  - **M** owns only the new files `scenes/blocks/motion_{drift,shear,sink,still,stutter}.{toml,wgsl}`.
  - **S** owns only the new files `scenes/blocks/space_{bands,pixel,tile}.{toml,wgsl}`.
  - **E1** owns only the new files `scenes/blocks/emit_{steps,scan,wave}.{toml,wgsl}`.
  - **E2** owns only the new files `scenes/blocks/emit_{scatter,halftone}.{toml,wgsl}`.
- **Docs** ([sonnet], after the gate) own `docs/USAGE.md` and `docs/MANUAL-TESTS.md`.

---

## File map

| File | Status | Responsibility |
|---|---|---|
| `src/genome.rs` | modify | `Genome.grammar`, `GRAMMARS`, `BLOCK_TAGS`, `V2_BLOCKS` += 13, `Blocks::grammar_pool`, `random_v2(.., grammar)` (pool, fold 1/3), `vary` (grammar pool, favours none), tests |
| `src/regime.rs` | modify | `gate(r, template, grammar)`: `still` widens the minimum motion |
| `src/evolve.rs` | modify | `best_v2` draws the grammar and picks from its pool; `pick_v2` test literal |
| `src/score.rs` | modify | `symmetry` becomes centredness (mirror + 1/2..1/5 turns, Pearson); `cell` × centred (54); tests; `calibrate` prints descriptors |
| `src/render/engine.rs` | modify | `engine_runs`: 9 space blocks |
| `scenes/blocks/CONTRACT.md` | modify | grammars, `BLOCK_TAGS`, wrap-around motions |
| `scenes/blocks/motion_{drift,shear,sink,still,stutter}.*` | create | non-radial motions |
| `scenes/blocks/space_{bands,pixel,tile}.*` | create | non-radial folds |
| `scenes/blocks/emit_{steps,scan,wave,scatter,halftone}.*` | create | non-radial emitters |
| `docs/USAGE.md`, `docs/MANUAL-TESTS.md` | modify | grammars; §11 checks |

---

## Task 0: Contract (orchestrator)

Every Wave-1 task codes against this. With grammar `""`, `random_v2` behaves exactly as before. The goldens prove nothing saved moves.

### 0.1 `src/genome.rs`

- [ ] `Genome`: append after `comp`:

```rust
    /// Engine-2 visual grammar (genome::GRAMMARS; "" = unconstrained): which blocks the look and its phrase
    /// variants pick from (BLOCK_TAGS). Empty = left out of the JSON, so saved ids hold.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub grammar: String,
```

- [ ] `V2_BLOCKS`: append the line

```rust
    "motion_drift", "motion_shear", "motion_sink", "motion_still", "motion_stutter", "space_bands", "space_pixel", "space_tile",
    "emit_halftone", "emit_scan", "emit_scatter", "emit_steps", "emit_wave",
```

- [ ] Below `TEMPLATES`:

```rust
/// Engine-2 visual grammars, one per generated segment (drawn before any block, evolve::best_v2): radial
/// (the classic library: tunnels, folds, spins), flow (advected fields, no centre), grid (tiles, mosaics,
/// sequencers), scroll (everything slides one way) and still (the camera holds; only the emitter moves).
pub const GRAMMARS: [&str; 5] = ["radial", "flow", "grid", "scroll", "still"];

/// The grammars each pinned engine-2 motion, emitter, space and comp block belongs to, and what it looks like
/// (R time-driven rotation, Z steady radial zoom, T tunnel/vanishing point, C centred symmetry, D raymarched
/// 3D camera, F flowing field; read by the variety test and the scratchpad sim). Frozen like V2_BLOCKS:
/// retagging changes what new seeds pick, never a saved plan. Colours, layers and palettes suit every grammar.
const BLOCK_TAGS: &[(&str, &str, &str)] = &[
    ("comp_echo", "radial", "Z"),
    ("comp_kaleido", "radial", "RC"),
    ("comp_mirror", "radial", "C"),
    ("comp_slitscan", "radial flow grid scroll still", ""),
    ("emit_blobs", "radial flow still", ""),
    ("emit_cells", "radial flow grid still", ""),
    ("emit_clouds", "radial flow scroll", ""),
    ("emit_contours", "radial flow still", "F"),
    ("emit_filaments", "radial flow", "F"),
    ("emit_halftone", "grid still", ""),
    ("emit_hypertile", "radial", "C"),
    ("emit_kifs3d", "radial", "RD"),
    ("emit_lissajous", "radial", "RC"),
    ("emit_lyapunov", "radial flow still", ""),
    ("emit_mandelbox", "radial", "RD"),
    ("emit_orbit", "radial", "C"),
    ("emit_polygon", "radial", "RZC"),
    ("emit_rings", "radial", "ZC"),
    ("emit_scan", "grid scroll", ""),
    ("emit_scatter", "flow scroll still", ""),
    ("emit_sdftunnel", "radial", "RZTD"),
    ("emit_stars", "radial scroll still", ""),
    ("emit_steps", "grid still", ""),
    ("emit_truchet", "radial grid scroll", ""),
    ("emit_warpfbm", "radial flow", "F"),
    ("emit_wave", "scroll still", ""),
    ("motion_deepzoom", "radial", "RZT"),
    ("motion_drift", "flow scroll", ""),
    ("motion_erode", "radial", "Z"),
    ("motion_etch", "radial flow", "F"),
    ("motion_ink", "radial flow", "FZ"),
    ("motion_kaleido", "radial", "RZC"),
    ("motion_mosh", "radial grid", ""),
    ("motion_push", "radial", "RZT"),
    ("motion_shear", "grid scroll", ""),
    ("motion_sink", "flow", "F"),
    ("motion_sort", "radial grid scroll", ""),
    ("motion_still", "still", ""),
    ("motion_stutter", "grid still", ""),
    ("motion_swirl", "radial flow", "F"),
    ("motion_tunnel", "radial", "RZT"),
    ("space_bands", "grid scroll", ""),
    ("space_droste", "radial", "RZTC"),
    ("space_hyper", "radial", "C"),
    ("space_kaleido", "radial", "RC"),
    ("space_kifs", "radial", "C"),
    ("space_mirror", "radial", "C"),
    ("space_pixel", "grid still", ""),
    ("space_polar", "radial", "C"),
    ("space_tile", "grid", ""),
];
```

- [ ] `impl Blocks`, after `pinned`:

```rust
    /// The blocks a look of `grammar` may use ("" = all): motion, emitter, space and comp blocks tagged with it
    /// in BLOCK_TAGS (untagged ones only under ""); colours, layers and palettes all.
    pub fn grammar_pool(&self, grammar: &str) -> Blocks {
        let mut b = self.clone();
        if grammar.is_empty() {
            return b;
        }
        let ok = |id: &str| BLOCK_TAGS.iter().any(|(t, gs, _)| *t == id && gs.split(' ').any(|g| g == grammar));
        for k in ["motion", "emitter", "space", "comp"] {
            b.list_mut(k).expect("known kind").retain(|x| ok(&x.id));
        }
        // ponytail: as in pinned(), a custom --scenes folder may tag nothing; a required kind left empty keeps
        // its whole list (that grammar then isn't kept, but it runs).
        for k in ["motion", "emitter"] {
            if b.list(k).expect("known kind").is_empty() {
                *b.list_mut(k).expect("known kind") = self.list(k).expect("known kind").clone();
            }
        }
        b
    }
```

- [ ] `random_v2`: the signature becomes `pub fn random_v2(blocks: &Blocks, r: u64, regime: &crate::regime::Regime, grammar: &str) -> Genome`. Add `g.grammar = grammar.into();` right after `g.regime = regime.name.into();`. The body is otherwise unchanged (G changes it).
- [ ] Test-literal fixes: add the argument `""` to every `random_v2(` call in `mod tests` (in `random_v2_varies_every_gene_deterministically` and `random_v2_picks_recipes_and_comps`).

### 0.2 `src/regime.rs`

- [ ] Replace `gate` with:

```rust
/// The judge's ranges for a regime, recipe (genome::TEMPLATES) and grammar (genome::GRAMMARS): stateless looks
/// redraw every frame, so they move more and carry more fine detail than trails; dual looks stack two emitters
/// and clip a bit more; still looks hold the camera, so only their emitter moves.
// ponytail: hand-set factors; retune from the gate's smoke renders if a recipe or grammar never (or always) passes.
pub fn gate(r: &Regime, template: &str, grammar: &str) -> Thresholds {
    let th = r.th;
    let th = match template {
        "stateless" => Thresholds { motion: (th.motion.0, th.motion.1 * 1.5), max_grain: th.max_grain * 1.5, ..th },
        "dual" => Thresholds { max_clipped: th.max_clipped * 1.3, ..th },
        _ => th,
    };
    match grammar {
        "still" => Thresholds { motion: (th.motion.0 * 0.25, th.motion.1), ..th },
        _ => th,
    }
}
```

- [ ] Replace the test `recipes_widen_the_regime_gate` with:

```rust
    #[test]
    fn recipes_and_still_widen_the_regime_gate() {
        let r = get("calm").unwrap();
        assert_eq!(gate(r, "", "").motion, r.th.motion);
        let s = gate(r, "stateless", "radial");
        assert!(s.motion.1 > r.th.motion.1 && s.max_grain > r.th.max_grain && s.mean == r.th.mean);
        assert!(gate(r, "dual", "").max_clipped > r.th.max_clipped);
        let still = gate(r, "", "still");
        assert!(still.motion.0 < r.th.motion.0 && still.motion.1 == r.th.motion.1);
    }
```

### 0.3 `src/evolve.rs`: `best_v2` picks from the grammar's pool

- [ ] In `best_v2`, after the `let regime = …;` line:

```rust
    // The look's visual grammar, drawn before any block: every candidate and mutation comes from its pool.
    let grammar = genome::GRAMMARS[(hash(seed, 0x6A4D) % genome::GRAMMARS.len() as u64) as usize];
    let blocks = &blocks.grammar_pool(grammar);
```

- [ ] In `measure`: `score::passes(&m, &crate::regime::gate(regime, &g.template, &g.grammar))`.
- [ ] `fresh`: `genome::random_v2(blocks, hash(seed, k as u64), regime, grammar)`.
- [ ] Doc comment of `best_v2`: add "The segment's grammar (genome::GRAMMARS) is drawn by seed, like its regime; candidates and mutations use only that grammar's blocks."

### 0.4 `src/render/engine.rs`

- [ ] In `engine_runs`: `assert_eq!(b2.space.len(), 9, "all space blocks present");`. It fails until S lands, but it only runs at the gate.

### 0.5 `scenes/blocks/CONTRACT.md`

- [ ] Append:

````markdown
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
````

- [ ] **Verify:** `cargo build --release`, then `cargo test --release engine1 engine2_saved library_pins vary gate random_v2 regime`. All green, with golden values unchanged.
- [ ] Commit `feat(core): grammar contract (gene, block tags, pools, still gate)`. Stage `src/genome.rs src/regime.rs src/evolve.rs src/render/engine.rs scenes/blocks/CONTRACT.md`.

---

## Phase G: Genome picking · [opus] · owns `src/genome.rs` (outside Task 0's edits)

### Task G.1: `random_v2` picks from the grammar's pool, fold 1/3

- [ ] Replace the start of `random_v2`'s body, through the space draw:

```rust
    let h = |k: u64| crate::types::hash(r, k);
    // Only the grammar's blocks ("" = all).
    let blocks = &blocks.grammar_pool(grammar);
    let mut g = random(blocks, r);
    // A fold a third of the time (when the grammar has any).
    if !blocks.space.is_empty() && h(1) % 3 == 0 {
        g.space = nth_id(&blocks.space, h(2) as usize);
    }
```

  The rest (blend, palette, recipe, comp, regime, grammar, retint) is unchanged; `dual`'s second emitter and the comp now come from the pool too.
- [ ] Doc comment: "Engine-2 random genome from `grammar`'s blocks (Blocks::grammar_pool): `random`'s blocks plus a space fold a third of the time, a blend mode (add half the time), a cosine palette half the time, a recipe (stateless or dual a sixth of the time each), a comp framing a third of the time, and the segment's `regime` with its finishing look (mono tints with palette colour 2). `r` = 64-bit hash."

### Task G.2: `vary` keeps the grammar and favours none

- [ ] Replace `vary` with:

```rust
/// Engine-2 phrase variant: the same look with one structural change, inside its grammar: a fold or comp
/// framing dropped (half the time one is there) or swapped for another of the grammar's, or a fresh cosine
/// palette. Never the motion, emitter, blend, recipe, grammar or regime (those can go black, and variants
/// aren't judged). `r` = 64-bit hash.
// ponytail: unjudged; run variants through the judge if one ever renders black.
pub fn vary(g: &Genome, blocks: &Blocks, r: u64) -> Genome {
    let h = |k: u64| crate::types::hash(r, k);
    let pool = blocks.grammar_pool(&g.grammar);
    // The new id for `cur` of `list`: "" half the time one is set, else another id; None = no change possible.
    let other = |list: &[Block], cur: &str, k: u64| -> Option<String> {
        let ids: Vec<&str> = list.iter().map(|b| b.id.as_str()).filter(|id| *id != cur).collect();
        if ids.is_empty() || (!cur.is_empty() && h(k) % 2 == 0) {
            return (!cur.is_empty()).then(String::new);
        }
        Some(ids[(h(k + 10) % ids.len() as u64) as usize].to_string())
    };
    let mut out = g.clone();
    let swapped = match h(1) % 3 {
        0 => other(&pool.space, &g.space, 2).map(|s| out.space = s),
        1 => other(&pool.comp, &g.comp, 3).map(|c| out.comp = c),
        _ => None,
    };
    if swapped.is_none() {
        out.palette = cosine_palette(h(4));
    }
    settle(g, out, blocks)
}
```

### Task G.3: tests (append to `mod tests`)

- [ ] In `random_v2_varies_every_gene_deterministically`, raise `(0..48)` to `(0..96)` (folds are now drawn a third of the time). Keep every assert.
- [ ] Append:

```rust
    /// The library as BLOCK_TAGS describes it (empty sources): the picking logic without the files.
    fn tagged_library() -> Blocks {
        let blk = |id: &str| Block { id: id.into(), params: vec![], wgsl: String::new(), toml: String::new() };
        let mut b = Blocks { motion: vec![], emitter: vec![], colour: vec![blk("colour_x")], backdrop: vec![], overlay: vec![], space: vec![], comp: vec![], palettes: vec![genome().palette] };
        for (id, _, _) in BLOCK_TAGS {
            let kind = match id.split('_').next().unwrap() {
                "emit" => "emitter",
                k => k,
            };
            b.add(kind, blk(id)).unwrap();
        }
        b
    }

    /// The point of grammars: over many takes few looks are centred, spinning or zooming, every grammar
    /// appears, and only radial looks ever use a centred, spinning, tunnel or 3D block.
    #[test]
    fn takes_vary_their_visual_grammar() {
        let b = tagged_library();
        let looks = |id: &str| BLOCK_TAGS.iter().find(|t| t.0 == id).map_or("", |t| t.2);
        let reg = &crate::regime::REGIMES[0];
        let n = 4000u64;
        let (mut centred, mut spin, mut zoom, mut plain) = (0u64, 0u64, 0u64, 0u64);
        let mut seen = std::collections::HashSet::new();
        for s in 0..n {
            let r = crate::types::hash(21, s);
            // As evolve::best_v2 draws it.
            let grammar = GRAMMARS[(crate::types::hash(r, 0x6A4D) % GRAMMARS.len() as u64) as usize];
            let g = random_v2(&b, r, reg, grammar);
            assert_eq!(g.grammar, grammar);
            seen.insert(grammar);
            let mut ids = vec![&g.emitter, &g.emitter_b, &g.space, &g.comp];
            if g.template != "stateless" {
                ids.push(&g.motion);
            }
            let f: String = ids.iter().map(|id| looks(id)).collect();
            if grammar != "radial" {
                assert!(!f.contains(['R', 'T', 'C', 'D']), "{grammar}: {g:?}");
            }
            centred += f.contains('C') as u64;
            spin += f.contains('R') as u64;
            zoom += f.contains('Z') as u64;
            plain += !f.contains(['R', 'Z', 'T', 'C', 'D']) as u64;
        }
        let pct = |k: u64| 100 * k / n;
        // Before grammars (sim.py): centred 91%, spin 76%, zoom 68%, none of them 3%.
        assert!(pct(centred) <= 20 && pct(spin) <= 20 && pct(zoom) <= 25 && pct(plain) >= 70, "centred {}% spin {}% zoom {}% plain {}%", pct(centred), pct(spin), pct(zoom), pct(plain));
        assert_eq!(seen.len(), GRAMMARS.len());
        // Each non-radial grammar has enough to vary: at least two motions and three emitters.
        for gr in &GRAMMARS[1..] {
            let p = b.grammar_pool(gr);
            assert!(p.motion.len() >= 2 && p.emitter.len() >= 3, "{gr}");
        }
    }

    /// Every block a grammar can use is tagged and loads; tags name real grammars.
    #[test]
    fn block_tags_cover_the_v2_library() {
        let b = blocks().pinned(2);
        let lib: Vec<&Block> = [&b.motion, &b.emitter, &b.space, &b.comp].into_iter().flatten().collect();
        for x in &lib {
            assert!(BLOCK_TAGS.iter().any(|t| t.0 == x.id), "{} has no BLOCK_TAGS entry", x.id);
        }
        for (id, gs, _) in BLOCK_TAGS {
            assert!(gs.split(' ').all(|g| GRAMMARS.contains(&g)), "{id}: {gs}");
            assert!(lib.iter().any(|x| x.id == *id), "{id} is not in the engine-2 library");
        }
    }

    #[test]
    fn vary_stays_in_the_grammar_and_drops_folds() {
        let b = tagged_library();
        let g = Genome { space: "space_tile".into(), grammar: "grid".into(), regime: "calm".into(), ..random(&b.grammar_pool("grid"), 3) };
        let mut dropped = 0;
        for r in 0..64u64 {
            let v = vary(&g, &b, crate::types::hash(8, r));
            let p = b.grammar_pool("grid");
            assert!(v.space.is_empty() || p.space.iter().any(|x| x.id == v.space), "{}", v.space);
            assert!(v.comp.is_empty() || p.comp.iter().any(|x| x.id == v.comp), "{}", v.comp);
            dropped += v.space.is_empty() as u32;
        }
        assert!(dropped >= 4, "a fold is dropped about a sixth of the time: {dropped}");
        // A flow look has no folds: a fold change becomes a palette change, never a no-op.
        let f = Genome { grammar: "flow".into(), space: String::new(), ..g.clone() };
        assert!((0..32u64).all(|r| vary(&f, &b, crate::types::hash(9, r)) != f));
    }
```

  `vary_changes_exactly_one_structural_gene` stays as is (grammar `""`, so `grammar_pool` returns everything) and must stay green.
- [ ] `cargo build --release`, then `cargo test --release random_v2 vary takes_vary engine1 engine2_saved`. `block_tags_cover_the_v2_library` needs the new block files, so it runs at the gate.

---

## Phase J: Judge · [opus] · owns `src/score.rs`, `mod tests` of `src/evolve.rs`

Engine 1's `verdict` never reads `symmetry`, the descriptor or the cell, so engine 1 is unchanged.

### Task J.1: centredness

- [ ] `Metrics.symmetry` doc: `// 1 = mirror- or rotation-symmetric about the centre (mirrors, wedges, rings, tunnels), 0 = none (engine 2)`.
- [ ] Add below `metrics`:

```rust
/// How centred one frame is, 0..1: the best Pearson correlation of the frame with its left-right mirror or
/// with itself turned 1/2, 1/3, 1/4 or 1/5 of a turn about the centre (inside the inscribed circle).
// ponytail: four turns (nearest pixel); 7+ wedge counts read lower. Add turns if the picker lets them through.
fn centred(f: &[f32], w: usize) -> f32 {
    let h = f.len() / w.max(1);
    let (a, b): (Vec<f32>, Vec<f32>) = f.chunks(w).flat_map(|r| (0..w).map(move |x| (r[x], r[w - 1 - x]))).unzip();
    let mirror = pearson(&a, &b);
    let (cx, cy, rr) = (w as f32 / 2.0, h as f32 / 2.0, h as f32 / 2.0);
    let turn = |n: u32| {
        let (s, c) = (std::f32::consts::TAU / n as f32).sin_cos();
        let (a, b): (Vec<f32>, Vec<f32>) = (0..f.len())
            .filter_map(|i| {
                let (dx, dy) = ((i % w) as f32 + 0.5 - cx, (i / w) as f32 + 0.5 - cy);
                let (x, y) = (cx + c * dx - s * dy, cy + s * dx + c * dy);
                (dx * dx + dy * dy < rr * rr).then(|| (f[i], f[(y as usize).min(h - 1) * w + (x as usize).min(w - 1)]))
            })
            .unzip();
        pearson(&a, &b)
    };
    (2..=5).map(turn).fold(mirror, f32::max).clamp(0.0, 1.0)
}
```

- [ ] In `metrics`, replace the `symmetry` computation with `let symmetry = frames.iter().map(|f| centred(f, w)).sum::<f32>() / n;`.
- [ ] `descriptor` doc: "brightness, motion, centredness, colourfulness, density, fine detail".
- [ ] Replace `cell`:

```rust
/// Quality-diversity cell (0..54): brightness × motion × colourfulness (3 bins each) × centred or not, so a
/// seed-chosen cell is as likely to be off-centre as a tunnel.
// ponytail: 0.6 is hand-set; retune from the gate's calibrate output (tunnels above, flat looks below).
pub fn cell(d: &[f32; 6]) -> usize {
    let b = |x: f32| ((x * 3.0) as usize).min(2);
    b(d[0]) + 3 * b(d[1]) + 9 * b(d[3]) + 27 * usize::from(d[2] > 0.6)
}
```

### Task J.2: tests

- [ ] In `descriptors_see_symmetry_density_colour_and_cells`, change `cell(&d) < 27` to `cell(&d) < 54`. Then append:

```rust
        // Centredness: rings about the centre score high; scattered noise and an off-centre blob score low.
        let (w, h) = (64usize, 36usize);
        let at = |f: &dyn Fn(f32, f32) -> f32| (0..w * h).map(|i| f((i % w) as f32 + 0.5 - 32.0, (i / w) as f32 + 0.5 - 18.0)).collect::<Vec<f32>>();
        let rings = at(&|x, y| 0.5 + 0.5 * (0.6 * (x * x + y * y).sqrt()).sin());
        let noise: Vec<f32> = (0..w * h).map(|i| (crate::types::mix64(i as u64) % 1000) as f32 / 1000.0).collect();
        let blob = at(&|x, y| (-((x - 14.0).powi(2) + (y - 6.0).powi(2)) / 30.0).exp());
        assert!(centred(&rings, w) > 0.9, "{}", centred(&rings, w));
        assert!(centred(&noise, w) < 0.3 && centred(&blob, w) < 0.3, "{} {}", centred(&noise, w), centred(&blob, w));
        let mut flat = d;
        flat[2] = 0.1;
        let mut round = d;
        round[2] = 0.9;
        assert_ne!(cell(&flat), cell(&round), "centredness is a cell axis");
```

- [ ] `calibrate`: print the descriptor too. Replace its loop body with:

```rust
            let m = j.measure(s.clone(), &a, 60.0, 4.0).unwrap();
            println!("{:24} {:?} {:.2?}", s.id, verdict(&m), descriptor(&m));
```

- [ ] `src/evolve.rs`, test `pick_v2_spreads_over_cells_and_avoids_seen_looks`: `bright2` must stay in `bright`'s cell. Change its third value from `0.9` to `0.55`: `let bright2 = [0.95, 0.95, 0.55, 0.95, 0.9, 0.9];`. Everything else stays.
- [ ] `cargo build --release`, then `cargo test --release score evolve`.

---

## Phases M, S, E1, E2: Blocks · [opus] · new files only

Rules (CONTRACT.md):
- Helpers are prefixed with the block id. `fn motion(` / `fn motion_decay(`, `fn space(`, `fn emit(` are each defined once. Emitters never call `emit` themselves and read params only through `EP(`, and no identifier may end in `EP(`; `emitters_survive_the_dual_rename` enforces this.
- `MP/EP/SP` never multiply `time()`. No derivatives in motion blocks.
- No time-driven rotation, no zoom about the frame centre, nothing centred.

Ids must match `V2_BLOCKS` and `BLOCK_TAGS`. Nothing to run: the gate's `engine_runs` compiles and steps each block, and `engine2_block_helpers_are_prefixed` checks the prefixes.

### Phase M: motions (`motion_drift`, `motion_shear`, `motion_sink`, `motion_still`, `motion_stutter`)

- [ ] `motion_drift.toml`

```toml
kind = "motion"
description = "Drift: the whole frame slides one way (seeded direction) and wraps around, stepping ahead on each kick; no zoom, no spin"
[[param]]
name = "speed"      # px per step
min = 0.5
max = 2.5
[[param]]
name = "kick_step"  # extra px per step at a kick onset
min = 4.0
max = 14.0
[[param]]
name = "persistence"
min = 0.9
max = 0.975
```

  `motion_drift.wgsl`

```wgsl
// Drift: the whole frame slides one way (one of 8 directions, by seed) and wraps around the edges, so trails
// stream across like a scrolling landscape; kick onsets and drops shove it further. No zoom, no spin.
fn motion_drift_dir() -> vec2<f32> {
    let a = TAU * floor(seed() * 8.0) / 8.0;
    return vec2<f32>(cos(a), sin(a));
}

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let speed = MP(0) * (0.5 + 0.5 * energy()) + MP(1) * punch() + 3.0 * drop_env();
    return fract(uv - motion_drift_dir() * speed / resolution());
}

fn motion_decay() -> f32 {
    return MP(2);
}
```

- [ ] `motion_shear.toml`

```toml
kind = "motion"
description = "Shear: rows (or columns) slide sideways along a wave that travels down the frame, so trails ripple into ribbons; kicks swell the wave"
[[param]]
name = "waves"      # wave cycles over the frame
min = 1.0
max = 4.0
[[param]]
name = "amount"     # px per step
min = 0.8
max = 3.0
[[param]]
name = "persistence"
min = 0.9
max = 0.97
```

  `motion_shear.wgsl`

```wgsl
// Shear: each row (or column, by seed) slides sideways by a wave travelling along the frame, on top of a slow
// one-way drift, so trails ripple into flags and ribbons; kicks swell the wave, drops whip it.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let cols = fract(seed() * 17.0) > 0.5;
    let along = select(uv.y, uv.x, cols);
    let wave = sin(TAU * (along * MP(0) - 0.25 * time()));
    let amt = MP(1) * (0.5 + wave) * (1.0 + 2.0 * kick()) + 4.0 * drop_env() * wave;
    let d = select(vec2<f32>(amt, 0.0), vec2<f32>(0.0, amt), cols) / resolution();
    return fract(uv - d);
}

fn motion_decay() -> f32 {
    return MP(2);
}
```

- [ ] `motion_sink.toml`

```toml
kind = "motion"
description = "Sink: the frame drains slowly inward toward an off-centre point that jumps every quarter phrase; no spin"
[[param]]
name = "pull"       # inward zoom per step
min = 0.001
max = 0.004
[[param]]
name = "persistence"
min = 0.9
max = 0.97
```

  `motion_sink.wgsl`

```wgsl
// Sink: the frame drains slowly inward (zoom < 1) toward a point off the centre that jumps to a new place
// every quarter phrase, so trails gather into a drifting focus instead of a tunnel. No spin.
fn motion_sink_at() -> vec2<f32> {
    let k = floor(phrase_phase() * 4.0) + 4.0 * floor(seed() * 64.0);
    let h = vec2<f32>(hash21(vec2<f32>(k, 1.3)), hash21(vec2<f32>(k, 7.9)));
    return (h - 0.5) * vec2<f32>(1.4 * aspect(), 1.4);
}

fn motion(uv: vec2<f32>) -> vec2<f32> {
    let zoom = 1.0 - MP(0) * (0.6 + 0.4 * energy()) - 0.004 * kick();
    return warp(uv, zoom, 0.0, 0.0, motion_sink_at());
}

fn motion_decay() -> f32 {
    return MP(1);
}
```

- [ ] `motion_still.toml`

```toml
kind = "motion"
description = "Still: the camera holds; nothing moves but what the emitter draws, trails fade in place and kicks cut them short"
[[param]]
name = "persistence"
min = 0.85
max = 0.96
[[param]]
name = "kick_cut"   # share of the trail a kick onset removes
min = 0.0
max = 0.25
```

  `motion_still.wgsl`

```wgsl
// Still: the camera holds. Nothing moves except what the emitter draws; trails fade in place, a little slower
// in tense build-ups, and kick onsets cut them short so each beat reads as a fresh frame.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    return uv;
}

fn motion_decay() -> f32 {
    return MP(0) * (1.0 - MP(1) * punch()) * (1.0 + 0.03 * tension());
}
```

- [ ] `motion_stutter.toml`

```toml
kind = "motion"
description = "Stutter: the frame holds still between kicks, then jumps a whole tile in a re-rolled direction on each kick, like a glitching freeze-frame"
[[param]]
name = "tiles"      # tiles per frame height (the jump is one tile)
min = 2.0
max = 6.0
fixed = true
[[param]]
name = "persistence"
min = 0.9
max = 0.97
```

  `motion_stutter.wgsl`

```wgsl
// Stutter: the frame holds still between kicks; on the one step a kick lands it jumps by a whole tile (1/n of
// the height) in a direction re-rolled per kick, wrapping round, like a glitching freeze-frame. Drops jump two.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let kid = floor((time() - since_kick()) * 4.0);   // constant between kicks
    let a = floor(hash21(vec2<f32>(kid, floor(seed() * 97.0))) * 4.0) * (TAU / 4.0);
    let on = select(0.0, 1.0, since_kick() < dt());
    let tile = vec2<f32>(1.0 / (MP(0) * aspect()), 1.0 / MP(0));
    return fract(uv - vec2<f32>(cos(a), sin(a)) * tile * on * (1.0 + floor(2.0 * drop_env())));
}

fn motion_decay() -> f32 {
    return MP(1);
}
```

### Phase S: spaces (`space_bands`, `space_pixel`, `space_tile`)

- [ ] `space_tile.toml`

```toml
kind = "space"
description = "Tile: the plane repeats as a wallpaper of mirrored tiles, offset so none sits on the centre; trails repeat across the grid"
[[param]]
name = "tiles"      # tiles per frame height
min = 2.0
max = 4.0
fixed = true
```

  `space_tile.wgsl`

```wgsl
// Tile: the plane repeats as a wallpaper of mirrored tiles (n per frame height); the grid is offset by seed so
// no tile is centred on the frame. Each tile shows the central square of the plane, so shapes and trails
// repeat across the grid, and tile edges meet without seams.
fn space(p: vec2<f32>) -> vec2<f32> {
    let q = p * 0.5 * SP(0) + vec2<f32>(fract(seed() * 7.0), fract(seed() * 3.0));
    // Mirrored repeat: a triangle wave 0..1..0, one ramp per tile.
    let f = abs(fract(q * 0.5) * 2.0 - 1.0);
    return f * 2.0 - 1.0;
}
```

- [ ] `space_pixel.toml`

```toml
kind = "space"
description = "Pixel: the plane snaps to a coarse grid of square cells (a blocky mosaic); the cell size re-rolls eight times a phrase and shrinks on drops"
[[param]]
name = "cells"      # cells per unit height
min = 10.0
max = 36.0
```

  `space_pixel.wgsl`

```wgsl
// Pixel: the plane snaps to a grid of square cells, so trails and shapes become a blocky mosaic. The cell size
// re-rolls eight times a phrase (half to one and a half times the param) and the cells shrink on drops.
fn space(p: vec2<f32>) -> vec2<f32> {
    let k = floor(phrase_phase() * 8.0);
    let n = SP(0) * mix(0.5, 1.5, hash21(vec2<f32>(k, floor(seed() * 977.0)))) * (1.0 + drop_env());
    return (floor(p * n) + 0.5) / n;
}
```

- [ ] `space_bands.toml`

```toml
kind = "space"
description = "Bands: the plane is cut into strips (horizontal or vertical) that each slide by their own amount, re-rolled every kick; every other strip is mirrored"
[[param]]
name = "bands"      # strips per unit
min = 2.0
max = 8.0
fixed = true
[[param]]
name = "shift"      # largest slide, centred units
min = 0.1
max = 0.6
```

  `space_bands.wgsl`

```wgsl
// Bands: the plane is cut into horizontal strips (vertical, by seed); each slides sideways by its own amount,
// re-rolled on every kick, and every other strip is mirrored, so the picture shears into offset bands.
fn space(p: vec2<f32>) -> vec2<f32> {
    let vert = fract(seed() * 23.0) > 0.5;
    var q = select(p, p.yx, vert);
    let band = floor(q.y * SP(0));
    let kid = floor((time() - since_kick()) * 4.0);   // constant between kicks
    q.x += (hash21(vec2<f32>(band, kid)) - 0.5) * SP(1);
    if (i32(band) % 2 != 0) { q.x = -q.x; }
    return select(q, q.yx, vert);
}
```

### Phase E1: emitters (`emit_steps`, `emit_scan`, `emit_wave`)

- [ ] `emit_steps.toml`

```toml
kind = "emitter"
description = "Step sequencer: a 16-step grid of rounded pads; a seeded pattern switches pads on and the playhead column walks one step per 16th, flashing on kicks"
[[param]]
name = "rows"
min = 4.0
max = 8.0
fixed = true
[[param]]
name = "density"    # share of pads switched on
min = 0.2
max = 0.6
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_steps.wgsl`

```wgsl
// Step sequencer: 16 columns of rounded pads by 4-8 rows. A seeded pattern switches some pads on; the playhead
// column walks one step per 16th of the bar, lighting its pads (the switched-on ones flash on kicks).
fn emit(p: vec2<f32>) -> vec2<f32> {
    let rows = EP(0);
    let q = vec2<f32>((p.x / aspect() + 1.0) * 8.0, (p.y + 1.0) * 0.5 * rows);
    let c = floor(q);
    let f = fract(q) - 0.5;
    let on = step(1.0 - EP(1), hash21(c + vec2<f32>(floor(seed() * 977.0), 3.0)));
    let lit = select(0.0, 1.0, c.x == floor(bar_phase() * 16.0));
    let d = length(max(abs(f) - vec2<f32>(0.28), vec2<f32>(0.0))) - 0.12;
    let pad = 1.0 - smoothstep(0.0, 0.03, d);
    let i = pad * (0.08 + 0.15 * lit + on * (0.35 + lit * (0.8 + 1.2 * kick()))) * (1.0 + 0.5 * drop_env());
    return vec2<f32>(i, fract(EP(2) + 0.5 * c.y / rows + 0.25 * lit));
}
```

- [ ] `emit_scan.toml`

```toml
kind = "emitter"
description = "Scan bars: three bright bars sweep down (or across) the frame locked to the beat; the lead bar thickens on kicks, drops flood it"
[[param]]
name = "width"      # bar half-width, 0..1 of the sweep
min = 0.01
max = 0.05
[[param]]
name = "gain"
min = 0.6
max = 1.4
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_scan.wgsl`

```wgsl
// Scan bars: three bars a third of a beat apart sweep down the frame (or across it, by seed), one full sweep
// per beat, so they lock to the tempo; the lead bar thickens on kicks, drops brighten all of them.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let across = fract(seed() * 41.0) > 0.5;
    let x = select(0.5 - 0.5 * p.y, 0.5 + 0.5 * p.x / aspect(), across);
    var i = 0.0;
    var hue = 0.0;
    for (var k = 0; k < 3; k++) {
        let fk = f32(k);
        let d0 = abs(x - fract(beat_phase() + fk / 3.0));
        let d = min(d0, 1.0 - d0);
        let w = EP(0) * select(1.0, 1.0 + 2.0 * kick(), k == 0);
        let b = exp(-d * d / (w * w));
        i += b;
        hue += b * fk / 3.0;
    }
    return vec2<f32>(i * EP(1) * (1.0 + drop_env()), fract(EP(2) + hue / max(i, 1e-4)));
}
```

- [ ] `emit_wave.toml`

```toml
kind = "emitter"
description = "Waveform: an oscilloscope trace across the frame at a seeded height; its wiggle is built from the four bands and kicks punch it"
[[param]]
name = "amplitude"
min = 0.4
max = 1.2
[[param]]
name = "width"      # glow width, centred units
min = 0.008
max = 0.03
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_wave.wgsl`

```wgsl
// Waveform: an oscilloscope trace across the frame at a seeded height. Its wiggle is built from the four
// bands (sub = slow swell, highs = fine jitter); kick onsets and drops punch the amplitude.
fn emit_wave_y(x: f32) -> f32 {
    let t = time();
    return 0.35 * sub() * sin(x * 1.5 + t * 0.7) + 0.25 * bass() * sin(x * 4.0 - t * 2.1)
        + 0.12 * mid() * sin(x * 11.0 + t * 5.3) + 0.06 * high() * sin(x * 29.0 - t * 9.0);
}

fn emit(p: vec2<f32>) -> vec2<f32> {
    let y0 = fract(seed() * 13.0) - 0.5;
    let amp = EP(0) * (1.0 + 1.5 * punch() + drop_env());
    let d = abs(p.y - y0 - amp * emit_wave_y(p.x));
    let w = EP(1) * (1.0 + kick());
    return vec2<f32>(w * w / (d * d + w * w), fract(EP(2) + 0.1 * p.x));
}
```

### Phase E2: emitters (`emit_scatter`, `emit_halftone`)

- [ ] `emit_scatter.toml`

```toml
kind = "emitter"
description = "Scatter: sparks on a jittered grid; each kick re-rolls which light up and where, so the frame sparkles in fresh random spots every beat"
[[param]]
name = "density"    # cells per unit
min = 3.0
max = 9.0
[[param]]
name = "size"       # spark radius, centred units
min = 0.01
max = 0.04
[[param]]
name = "share"      # share of cells lit
min = 0.2
max = 0.5
```

  `emit_scatter.wgsl`

```wgsl
// Scatter: sparks on a jittered grid. Each kick re-rolls which cells light up and where in their cell the spark
// sits, so the frame sparkles in fresh random spots every beat; sparks fade between kicks, drops light more.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let q = p * EP(0);
    let c = floor(q);
    let kid = floor((time() - since_kick()) * 4.0);   // constant between kicks
    let h = hash21(c + vec2<f32>(kid * 7.0, floor(seed() * 131.0)));
    let o = vec2<f32>(fract(h * 37.1), fract(h * 91.7)) * 0.7 + 0.15;
    let d = length(fract(q) - o) / EP(0);
    let r = EP(1) * (0.5 + 0.8 * fract(h * 13.3));
    let on = step(1.0 - EP(2) - 0.4 * drop_env(), fract(h * 57.3));
    let spark = exp(-d * d / (r * r)) * on * (0.25 + 1.2 * exp(-since_kick() * 3.0));
    return vec2<f32>(spark, fract(h * 3.7));
}
```

- [ ] `emit_halftone.toml`

```toml
kind = "emitter"
description = "Halftone: a printed dot screen at a seeded angle; dot size follows a slowly drifting noise image, kicks swell every dot"
[[param]]
name = "dots"       # dots per unit
min = 8.0
max = 24.0
[[param]]
name = "size"       # largest dot radius, share of a cell
min = 0.7
max = 1.0
[[param]]
name = "hue_offset"
min = 0.0
max = 1.0
fixed = true
```

  `emit_halftone.wgsl`

```wgsl
// Halftone: a printed dot screen at a seeded angle. Each dot's size follows a slowly drifting noise image
// sampled at the dot's centre (brighter = bigger), so shapes read as newsprint; kicks swell every dot.
fn emit(p: vec2<f32>) -> vec2<f32> {
    let a = 0.26 + 0.5 * fract(seed() * 7.0);
    let q = rot2(a) * p * EP(0);
    let c = floor(q) + 0.5;
    let src = fbm(rot2(-a) * c / EP(0) * 1.3 + vec2<f32>(time() * 0.05, floor(seed() * 97.0)));
    let tone = smoothstep(0.3, 0.75, src);
    let r = 0.5 * sqrt(tone) * (EP(1) + 0.25 * kick());
    let i = (1.0 - smoothstep(r - 0.06, r, length(q - c))) * (0.6 + 0.4 * tone) * (1.0 + 0.5 * drop_env());
    return vec2<f32>(i, fract(EP(2) + 0.3 * tone));
}
```

---

## Gate

- [ ] `cargo test --release`: everything green, including:
  - `engine1_genomes_are_frozen`, `engine1_plan_and_frames_are_frozen`, `engine2_saved_looks_are_frozen`, `engine2_saved_frames_are_frozen` (golden values never edited).
  - `engine_runs`: every new block compiles and steps; 9 spaces.
  - `engine2_block_helpers_are_prefixed`, `emitters_survive_the_dual_rename`, `block_tags_cover_the_v2_library`.
  - `takes_vary_their_visual_grammar`, `vary_stays_in_the_grammar_and_drops_folds`, `vary_changes_exactly_one_structural_gene`, `random_v2_*`, `recipes_and_still_widen_the_regime_gate`.
  - `descriptors_see_symmetry_density_colour_and_cells`, `pick_v2_spreads_over_cells_and_avoids_seen_looks`.

  Fix a failing block in its own `.wgsl`, then re-run only `cargo test --release engine_runs`.
- [ ] **Engine 1 is byte-identical to `main`** (`docs/MANUAL-TESTS.md` §9 method):
  - Use a `main` worktree (`git worktree add ..\rv-main main`) with `-threads 1` temporarily added to the x264 args in `src\render\mod.rs` in BOTH trees. Rebuild both. Give each tree its own copy of `testdata\dnb8.wav`.
  - Run `visualiser <wav> --engine 1 --replan --preview 1:00 --quality draft --seed 1` in each tree, then `ffmpeg -i <preview>.mp4 -f framemd5 -an <main|branch>.md5`. The files must be equal.
  - Repeat with `--layers`. For the `main` run, first move the 14 untracked AI block files into the scratchpad, then put them back.
- [ ] **A saved engine-2 plan renders byte-identically:**
  - Make a worktree at `BASE` (`git worktree add ..\rv-base $BASE`) with the same `-threads 1` patch.
  - Run `visualiser <wav> --engine 2 --generated 1 --replan --preview 1:00 --quality draft --seed 3` there.
  - Copy that project's `plan.json` into the branch's data folder for its own wav copy.
  - Run the branch with the same args and **no** `--replan`. It must print "using saved plan".
  - The framemd5 files must be equal.
  - Revert `-threads 1` in all trees and remove the worktrees.
- [ ] **Variety re-measure** (scratchpad):
  - Check that `sim_grammar.py`'s `TAGS` equal `BLOCK_TAGS` in `genome.rs`, and copy them over if they differ.
  - Run `python sim.py` (before) and `python sim_grammar.py` (after).
  - Report the before/after table (R, Z, T, C, D, F, none-of-RZTCD, grammar shares). Expected after: C ≈ 13%, R ≈ 12.5%, Z ≈ 16.5%, none ≈ 79%, each grammar ≈ 20%.
- [ ] **Centredness calibration:** if `testdata\dnb5.analysis.json` exists, run `cargo test --release calibrate -- --ignored --nocapture`. Tunnel-like hand-made scenes should print `d[2]` above 0.6 and flat ones below. If not, move `cell`'s 0.6 to sit between them (one constant) and re-run `cargo test --release score evolve`.
- [ ] **Smoke renders, one per grammar:**
  - Run `target\release\visualiser testdata\dnb8.wav --engine 2 --generated 1 --replan --preview 1:00 --quality draft --seed N` for N = 1, 2, 3, ….
  - After each run, grep that run's `plan.json` for `"grammar"`, copy the preview to `<scratchpad>\grammar_<name>_seed<N>.mp4`, and extract one frame: `ffmpeg -ss 20 -i <mp4> -frames:v 1 <scratchpad>\grammar_<name>_seed<N>.png`.
  - Stop once all five grammars have a render (cap: 25 seeds).
  - Check (view the PNGs; the user watches the mp4s):
    - flow, grid, scroll and still looks show no tunnel, spin or centred fold.
    - still looks hold their camera.
    - every segment in `plan.json` has a `"genome"`, so no grammar fell back to a hand-made scene.
  - If a grammar never yields a genome, widen its gate in `regime::gate` (one line, `ponytail:` note) and re-run that seed.
- [ ] `/simplify` → `/review` over `git diff $BASE...HEAD`. Fix findings, then run `cargo test --release` once.
- [ ] Docs [sonnet]:
  - `docs/USAGE.md`, Engine 2 section: a "Grammars" subsection covering:
    - Each generated look is radial, flow, grid, scroll or still, drawn per segment by the seed.
    - What each looks like.
    - Phrase variants stay in their grammar.
    - Folds are rarer.
    - Saved plans are unchanged.
  - `docs/MANUAL-TESTS.md`: new §11 "Engine 2 grammars" with the per-grammar smoke checklist (with explicit `--seed`), the variety re-measure, and the two framemd5 checks.
- [ ] Commit `docs: engine 2 grammars`.

Wave-1 commits (orchestrator, after the wave joins and the per-task tests pass, by explicit paths):
- `feat(core): grammar picking and grammar-true phrase variants` (`src/genome.rs`)
- `feat(core): centredness in the engine-2 judge` (`src/score.rs src/evolve.rs`)
- `feat(blocks): drift, shear, sink, still and stutter motions` (`scenes/blocks/motion_{drift,shear,sink,still,stutter}.{toml,wgsl}`)
- `feat(blocks): tile, pixel and bands folds` (`scenes/blocks/space_{bands,pixel,tile}.{toml,wgsl}`)
- `feat(blocks): step, scan, wave, scatter and halftone emitters` (`scenes/blocks/emit_{steps,scan,wave,scatter,halftone}.{toml,wgsl}`)

---

## Self-review notes

- **Spec coverage:**

  | Approved item | Where |
  |---|---|
  | 1. Folds ≈1/3, `vary` favours none | G.1 (`h(1) % 3 == 0`); G.2 (`other` drops half the time); grammar pools without spaces (flow) |
  | 2. Grammar gene drawn first, constraining pools | Task 0 `grammar`, `GRAMMARS`, `BLOCK_TAGS`, `grammar_pool`, `best_v2` draw; G.1 `random_v2`; G.2 `vary`; `mutate_v2` through `best_v2`'s pool |
  | 3. New non-radial blocks (≥2 motions, ≥3 emitters per grammar) | M, S, E1, E2; pool table in Architecture; asserted in `takes_vary_their_visual_grammar` |
  | 4. Zoom/spin zero or inward and off-centre, by new blocks only | drift, shear, still, stutter (none); sink (inward, off-centre); no existing block edited |
  | 5. Judge: still passes, centredness, engine-2 only | Task 0 `regime::gate(.., grammar)`; J centredness and 54 cells; `verdict` untouched |
  | Variety test over many seeds | G.3 `takes_vary_their_visual_grammar` (4000 seeds); gate re-measure over 20k |

- **Byte-identical engine 1:** no V1 block or engine-1 path changes. `verdict` reads `mean`, `contrast`, `motion`, `clipped`, `grain` and `kick_sync`, never `symmetry`. `random`/`mutate`/`dress` are untouched. The goldens and the main framemd5 gate prove it.
- **Saved engine-2 plans:** no existing block file is edited. `grammar` is skipped when empty. `vary`/`random_v2` run only while planning. `engine2_saved_*` and the `BASE` framemd5 check prove it.
- **Seeds:** a given engine-2 seed now picks differently (user-approved). The intent of the old tests holds: `random_v2_*` keep their asserts (96 samples), `vary_changes_exactly_one_structural_gene` is unchanged, and `pick_v2`'s literal only stays in its cell.
- **Single mode:** untouched (no shots or variants); its one look gets a grammar like any segment.
- **Flash limit:** stutter jumps once per kick; pixel and bands re-roll at most per kick or eighth-phrase, and they are positional, not brightness flashes. scan sweeps once per beat. Strobe gating is unchanged.
- **Names used across tasks:** `Genome.grammar`, `GRAMMARS`, `BLOCK_TAGS` `(id, grammars, looks)`, `Blocks::grammar_pool`, `random_v2(blocks, r, regime, grammar)`, `regime::gate(r, template, grammar)`, the grammar hash key `0x6A4D` (in `best_v2` and the variety test), `score::centred`, `cell` (0..54). Blocks: `motion_{drift,shear,sink,still,stutter}`, `space_{bands,pixel,tile}`, `emit_{steps,scan,wave,scatter,halftone}`. These are consistent across tasks.
- **Dependencies inside Wave 1:** G and J compile against Task 0 only. Block tasks are files only. `engine_runs` and `block_tags_cover_the_v2_library` need all four block tasks, so they run at the gate.
