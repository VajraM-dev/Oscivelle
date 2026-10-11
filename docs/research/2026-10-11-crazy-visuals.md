# Research: making the visuals mind-blowing (core first, AI on top)

Date: 2026-10-11 · Branch: `feature/motion-layers` · Status: research only, nothing built yet.

Question from the product owner: how do we make visuals that leave users mind-blown, and make every `--seed`
retry look drastically different, **from the core engine alone** (many users will never set up AI)? And once
the core is strong, how does the existing AI setup take it to the next level? Every idea is grounded in how much
code it would change.

Method: one lead research agent with three sub-researchers in parallel: (A) VJ and live-visual practice,
(B) shader and generative-art techniques feasible on wgpu, (C) a read of this codebase to size each change.

---

## 1. Honest assessment

- **The engine is solid; what it has to work with is the limit.** Every generated look is one fixed recipe
  (`src/genome.rs` `TEMPLATE`: the previous frame warped and faded, plus one emitter coloured on top), built from
  19 blocks (6 motion, 8 emit, 5 colour). Most of those blocks are the 7 hand-made scenes taken apart:
  `motion_ink` is literally commented "Ink in water" and reuses that scene's curl-noise function. So every seed is
  a remix of 7 scenes. That is the "triangle swapped for square" feeling, and it is structural.
- **The ceiling of the current recipe is much higher than the output suggests.** MilkDrop built about 50,000
  presets on essentially this pipeline. What it had that we lack: a choice of how space is folded per preset, a
  separate display-only shader, blend modes, and a large curated library.
- **The real hard limits** of the current engine: one 4-channel half-float state per pixel
  (`src/render/engine.rs`), one pass per simulation step, 14 of 16 param slots used and every uniform vec4 taken.
  These block reaction-diffusion, fluids, particles and slit-scan. They do **not** block the cheapest big wins.
- **Biggest single jump:** a **space stage** in the recipe (kaleidoscope, mirror, crystal folds, Droste tunnel,
  hyperbolic tiling) plus **new block families not taken from the hand-made scenes**. Size M, no engine change.
  The biggest jump in absolute ceiling is a generic **simulation layer** (extra state textures, several passes per
  step), size L. It extends `engine.rs`; it is not a rewrite.
- **Nothing here needs an engine rewrite.** Compute particles (XL) come closest.
- **Hidden prerequisite:** default output depends on how many blocks and scenes exist (`nth_id(i % len)` in
  `src/genome.rs`, `pick()` in the director). Adding any block changes every seed's picks. AI-written blocks are
  saved into `scenes/blocks/` (`src/vibe.rs`) and `--author` scenes into `scenes/`, so AI runs already change
  core-only output. The default library must be pinned before anything else.

Corrections found while checking the brief: there are **7** hand-made scenes, not 8. And without `--vibe` the
evolution seed list is empty, so populations start from `genome::random`, not from the hand-made scenes.

### Is the "overfitting to the hand-made scenes" claim right?

Mostly yes, but the mechanism differs: evolution is not drifting toward the scenes; **the space it searches is
made of them.**

| # | Suspected mechanism | Verdict | Evidence |
|---|---|---|---|
| 1 | Blocks distilled from the scenes | **Strong, the main cause** | The 7 scenes map almost one-to-one onto blocks: `ink_in_water`→`motion_ink` (same curl-noise function and comment), `liquid_tunnel`→`motion_tunnel`, `fractal_zoom`→`motion_deepzoom`, `kaleidoscope_bloom`→`motion_kaleido`, `starfield_warp`→`motion_push`/`emit_stars`, `neon_geometry`→`emit_polygon`, `plasma_field`→`emit_clouds`. All added in one commit (267317c). |
| 2 | The single recipe mirrors their structure | **Strong** | Every hand-made scene is `prev(warp) * decay + emissive`, and `TEMPLATE` is exactly that. No fold, no blend mode, no display-only stage, no other template. |
| 3 | The judge rewards their style | **Moderate** | Thresholds are wide sanity bounds (`src/score.rs`: mean 0.04–0.75, contrast ≥ 0.04, motion 0.002–0.25), so a calm look isn't rejected outright. The bias is in ranking: score = `contrast + 2·kick_sync + 4·min(motion, 0.05)`, mutation parents are the top 2, and only candidates ≥ 0.7× the batch best can be picked (`src/evolve.rs` `GATE`). Dark, sparse, slow or monochrome looks score low and are never picked or bred. The judge also renders with layers off. |
| 4 | Mix mode (`--generated 0.5`) | **Weak for generation, real for perception** | It doesn't affect what is generated, but by default half of every video *is* hand-made scenes, so every seed shares about half its screen time. |
| 5 | Vibe / AI examples | **AI only** | `author.rs` uses `liquid_tunnel` as the worked example; the vibe uses an existing block as its example. No effect on core output. |

Also: in core mode, mutation never tunes params, because random genomes have no tuned params. It only swaps
blocks and palettes (`src/genome.rs` `mutate`).

---

## 2. Core ideas (no AI), sorted by wow ÷ effort

Code size: **S** < 100 lines · **M** 100–400 · **L** 400–1000 · **XL** > 1000. Every row assumes opt-in behind a
flag or a library version, with default output byte-identical.

| # | Idea | What the user sees | VJ / art precedent | Wow | Variety / seed | Code | Files / mechanism | Risk & notes |
|---|---|---|---|---|---|---|---|---|
| 0 | **Pin the default library; add a v2 opt-in** (prerequisite) | Nothing yet; makes every row below safe to ship | – | – | – | S | `genome.rs` frozen id list for v1, new blocks sorted after it; `vibe.rs`/`author.rs` save to a separate dir; engine version in `job_hash` (`render/mod.rs`) | Fixes today's leak where AI blocks, `--keep` and untracked files change default output |
| 1 | **Judge as a sanity gate only; the seed picks uniformly among passing looks** | Calm, dark, sparse and slow looks finally appear | MilkDrop "Cream of the Crop" (keep anything not broken) | 2 | 4 | S | `evolve.rs` `pick()`: drop the 0.7× gate; parents chosen by seed, not top score | Lower-energy looks can win; pair with row 5 so they land in the right sections |
| 2 | **Onset "hits" + per-band separation** (kick flux → punch, hat flux → detail) | Visuals snap onto transients instead of swelling with loudness | Synesthesia Level/Hits/Presence; "onsets for punches, envelopes for drift" | 4 | 2 | S–M | `analysis.rs` already computes kick/hat flux; new `U` vec4 in `types.rs` + prelude; `director.rs` mods | New uniform vec4; serde default on `Analysis` |
| 3 | **Drop anticipation with offline lookahead** | Half a bar before each drop: squeeze, desaturate, near-black, then explode on the first kick | VJ show arcs; club blackout drops | 4 | 2 | S | `director.rs` `frame_state`, using drop / build_start / time-since-drop | Deterministic: uses analysis only |
| 4 | **Cosine palettes** (seed-chosen a, b, c, d; frequency c of 0.5/1/2/3 changes banding) | Unlimited coherent colour schemes, from tight banding to broad washes | iquilezles.org palettes; MilkDrop3 colour re-randomise | 2 | 3 | S | Palette generator next to `palettes.toml`; `scenes.rs` | New hash stream |
| 5 | **Seed-chosen aesthetic regime** (calm / dark / mono / sparse / glitch / maximal): sets judge ranges, decay, finish look and how hard the music drives the image | Seeds differ in mood, not only shape | VJ rule "contrast beats intensity" | 4 | 5 | S–M | `score.rs` thresholds per regime; `evolve.rs`; `Post`/`Look` presets | Directly addresses the bias toward hand-made-style looks |
| 6 | **Feedback-operator motion blocks**: Sobel/emboss edges, block-quantised datamosh, threshold-smear "pixel sort", subtract-and-darken | Looks that melt, glitch, etch and crystallise instead of only zooming and swirling | MilkDrop warp shaders; datamosh (Buck); Geiss's tips | 4 | 4 | S each (~4 blocks) | New `scenes/blocks/motion_*.wgsl` + toml | Must be inside the row 0 pin; neighbour taps cost more at 1.5× supersampling |
| 7 | **Emitter families not taken from any scene**: domain-warp fbm, voronoi/worley, multi-scale truchet, hyperbolic {p,q}, Lyapunov, julia orbit traps | Marbled liquid, cells, mazes and Escher tilings feeding the trails | IQ domain warping; Shadertoy staples | 3 | 4 | S each (~6 blocks) | New `emit_*` blocks | Use integer hashes, not `fract(sin())` |
| 8 | **Space stage in the recipe** (none, mirror, kaleido-N, KIFS fold ×k, log-polar/Droste, polar repeat, hyperbolic) | Any look becomes a mandala, an infinite Escher tunnel or a crystal. **Best variety gain per line of code** | MilkDrop per-vertex warp; Droste / Print Gallery; KIFS | 4 | 5 | M | `genome.rs` new kind in KINDS/SLOTS/struct/random/mutate; TEMPLATE becomes `prev(motion(space(uv)))`, `emit(space(p))`; CONTRACT.md; ~6 blocks; needs a p4 vec4 (`types.rs`, prelude, `director::param_mut`) | "none" keeps v1 identical; the engine test compiles every combination, so the test matrix grows (6×8×5×N) |
| 9 | **Raymarched KIFS / mandelbox / SDF-tunnel emitters** feeding the feedback loop | Real 3D depth and parallax: fractal cathedrals smeared into trails | Revision demoscene; Synesthesia `syn_BassTime` camera | 5 | 5 | M | New emit blocks, no engine change; fold set and mandelbox scale sign as structural genes | Expensive (~64 steps × 2 sim steps × 1.5² pixels). Fine offline; live preview will crawl, so add a half-res preview option |
| 10 | **Novelty / quality-diversity (MAP-Elites) selection** over descriptors: brightness, motion speed, symmetry, colourfulness, density, edge frequency | Each seed lands in a different region of "look space"; no two segments in a video look alike | Quality-diversity / MAP-Elites; MilkDrop no-repeat lists | 3 | 5 | M | `score.rs` computes descriptors from frames it already renders; `evolve.rs` keeps a per-run archive; seed picks a cell first; reward distance from chosen looks and from the 7 hand-made scenes | Deterministic given the judge (per GPU, as today); reuse the 20-render budget |
| 11 | **Blend-mode stage** (add / screen / difference / multiply / lighten) for emitter over feedback, plus an optional second emitter | Psychedelic inversions and masks instead of only additive glow | Resolume blend modes | 3 | 4 | S–M | TEMPLATE `col += …` becomes `blend_k(col, …)`; a genome gene | Difference/multiply can go black; the gate catches it |
| 12 | **Strobe / RGB split / scanline tear / invert, gated by onsets** | Glitch hits on snares, flash frames on downbeats | Club strobe practice; glitch VJing | 3 | 2 | S each | `finish.wgsl` + `FinishUniforms` + `Post` (an `if amt > 0` gate already exists) | **Flash limit:** large-area flashes ≤ 3 Hz (WCAG 2.3.1). At 174 BPM flash on downbeats or snares, not every kick |
| 13 | **Display-only comp stage** (MilkDrop "comp" shader: kaleido, echo, mirror, grade on the output only, not fed back) | The same simulation shown in several framings | MilkDrop warp/comp split; video echo | 3 | 4 | M | The LAYERS template / compose pass already exists; add a comp block kind | Keep it out of the feedback loop for stability |
| 14 | **Hard cuts between different genomes on drops and 16/32-bar phrases, with optional feedback reset** | Real edits: a new world on the drop, black-and-rebuild | MilkDrop3 hardcut; VJ phrase switching | 4 | 3 | S–M | `main.rs` shots (exist), `director.rs`; reset = clear the feedback buffer in `engine.rs` | Each generated segment costs ~20 judge renders, so planning time grows |
| 15 | **Structural phrase variations** (change fold count, palette regime or one block every 8/16 bars) | The track develops instead of looping one look | VJ "8 bars palette, 16 bars module" | 4 | 3 | M | `director.rs` variations today only walk params; add per-phrase genome deltas compiled up front | More pipelines to compile |
| 16 | **Transition shaders**: luma wipe, zoom-through, displacement melt, pattern blend | Cuts look designed | Butterchurn blends; MilkDrop3 blend patterns | 3 | 2 | M | `engine.rs` has only a constant-blend copy today; add a transition pipeline | – |
| 17 | **Slit-scan / time-echo history ring** | Rubbery, time-smeared motion where each row shows a different past moment | TouchDesigner Time Machine; slit-scan film | 5 | 3 | M | `engine.rs` texture array + copy per frame; prelude `hist(uv, k)`; `render/mod.rs` pre-roll must fill it | Memory: ~37 MB per frame at final res, so half-res Rgba8 (64 frames ≈ 130 MB) |
| 18 | **Extra finish looks**: variable chromatic aberration, lens distortion, god rays, duotone / mono grade | Lens character, light shafts | Film and VJ grading | 2 | 3 | S–M | `finish.wgsl` / bloom chain | God rays add one pass |
| 19 | **v2 mix default: no hand-made scenes in generated takes** | Videos stop sharing half their screen time with the same 7 scenes | – | 2 | 3 | S | `main.rs` default share | Changes default behaviour, so only under v2 |
| 20 | **Param tuning in mutation** | Mutants explore within a block, not only swap blocks | – | 1 | 2 | S | `genome.rs` `mutate()` | New hash stream |
| 21 | **Recipe (template) variants**: stateless (no feedback, crisp 3D), raymarch-first with feedback as post, dual-emitter | Fundamentally different render grammars per seed | Shadertoy (stateless) vs MilkDrop (feedback) | 4 | 5 | M (L with new kinds) | Genome `template` field, `wgsl()` dispatch, a contract per template, `Blocks` keyed by template | Judge thresholds were tuned on feedback looks; needs row 1 or 5 |
| 22 | **Generic simulation layer**: N persistent state textures + K passes per step → **Gray-Scott reaction-diffusion** with curated regimes (coral, mitosis, labyrinth, worms, spots) | Living Turing patterns that grow and split; kicks inject chemical | Karl Sims RD; Pearson's regime map | 5 | 4 | L | `engine.rs` (textures, bind layout, ping-pong, step loop), multi-pass scene format, `scenes.rs`, prelude | **Chunk resume:** RD doesn't decay, so the 20 s pre-roll causes seams every chunk; needs a state snapshot or a much longer pre-roll. Run at half res |
| 23 | **Lenia / SmoothLife** on the simulation layer | Gliding, rotating artificial creatures | Lenia (Chan) | 5 | 4 | +S–M after #22 | Pass WGSL only | Large kernel cost; half res |
| 24 | **Stable fluids** on the simulation layer (velocity, pressure Jacobi), forced by kick splats | Real ink and smoke turbulence punched by the bass | GPU Gems ch. 38 (Stam) | 5 | 3 | +M after #22 | 25–85 passes per step | Heavy at 1.5× supersampling; quarter res |
| 25 | **Flow-field datamosh** on the simulation layer | Real P-frame smear that amplifies motion | Buck "moshing"; datamosh shaders | 4 | 3 | +S after #22 | Flow state texture | – |
| 26 | **Independent crossfade / two-world spatial mix** (scene B gets its own feedback pair) | Two live worlds blended through a plasma or checker mask | MilkDrop3 double preset | 4 | 3 | M–L | `engine.rs` morph (B currently samples A's buffer) | Double memory and cost during morphs |
| 27 | **Compute particles and strange-attractor density** (u32 fixed-point atomics) | Millions of glowing points; Clifford / de Jong attractor clouds | Paul Bourke attractors; GPU particle VJ | 5 | 4 | L–XL | `engine.rs` compute pipelines, storage buffers, splat → texture; new contract; reject dud attractors on the CPU first (Lyapunov estimate) | **No float atomics in WGSL:** use u32 fixed point (order-independent, so deterministic). Particles must respawn within the pre-roll |
| 28 | **Palette from your own picture** (k-means on an `--image`) | "Make it look like this cover" partly works with no AI | Moodboard-driven design | 2 | 2 | S | `plates.rs` + palette path | Core feature; the AI version (A4) only interprets style on top |

### Recommended core phasing

**Phase 1: "Every seed is a different piece"** (all S/M, no engine change, ~1.5–2.5k lines)
- Rows 0, 1, 5, 10: pin the library and decouple the judge.
- Rows 4, 6, 7, 8, 9, 11: palettes, ~15 new blocks not taken from the scenes, the space stage, raymarch emitters, blend modes.
- Rows 2, 3, 12: lock to the music and land the drops.

Unlocks structural variety per seed (space × motion × emitter × blend × regime × palette), breaks the coupling to
the hand-made scenes, and gives drops that hit.

**Phase 2: "Shows, not loops"** (M)
- Rows 13–17, 19, 21: comp stage, phrase-level structural variation, hard cuts and transition shaders, slit-scan
  history, template variants, v2 mix default.

Unlocks build-up and drop dramaturgy, edits within a track, and time effects.

**Phase 3: "Living systems"** (L/XL)
- Rows 22–25: the simulation layer, with state snapshotting for chunk resume.
- Row 26: two-world crossfades.
- Row 27: compute particles and attractors.

Unlocks reaction-diffusion, Lenia, fluids and particles, the looks a feedback trick can't fake. Needs the resume
and pre-roll redesign and a per-GPU determinism note.

---

## 3. AI on top of a strong core

### What exists today (checked in code)

- **`src/llm.rs`:** one OpenAI-compatible `/chat/completions` endpoint with the user's own key
  (`OPENAI_BASE_URL`, `OPENAI_API_KEY`, `VISUALISER_AI_MODEL` or saved settings). A forced tool call returns JSON,
  with a plain-text fallback for local models. `image()` calls `/images/generations` (gpt-image).
  **Messages are text only: no model ever sees an image.** The user's `--image` files are described to the model
  as "content unknown" (`vibe.rs`).
- **`--vibe` (`src/vibe.rs`):** one `submit_design` call returns looks (blocks, params, palette, layers, post,
  text), `new_blocks` (each written, compiled, auditioned by the engine judge, then **saved into
  `scenes/blocks/`**), up to 4 plate prompts that gpt-image paints, and up to 8 storyboard shots on analysed
  moments. Cached in `<song>.vibe.json`; plates cached per prompt. Evolution then stays within the seeds' blocks.
- **`--author` (`src/author.rs`):** writes a whole hand-editable scene using `liquid_tunnel` as the worked example;
  3 tries with compile errors fed back; must pass the judge; saved to `scenes/`.

**The core-vs-AI rule the code currently breaks:** AI output lands in the same folders the core reads, so every AI
run changes core-only output for every later track. Every idea below assumes A0 is done first.

### How a richer core changes the AI's job

Today the AI picks from about 6×8×5 blocks in one fixed recipe. Once the core has space transforms, regimes, blend
modes, template variants, quality-diversity descriptors and simulation layers, the AI's job moves from "write WGSL
that might work" to "direct a large, already-safe space". That is cheaper, more reliable and far more expressive.

### AI ideas, sorted by wow ÷ effort

Costs are rough estimates at current OpenAI prices, not measured: a design call ~$0.02–0.30 depending on the model;
a gpt-image plate ~$0.02–0.20. "Core rows" refers to section 2.

| # | Idea | What the user sees | Wow | Variety | Code | Files / mechanism | Cost per run & risk | Core rows |
|---|---|---|---|---|---|---|---|---|
| A0 | **AI writes to its own library folder, never into the core's** (prerequisite) | Nothing visible; core stays pure, AI growth is opt-in | – | – | S | `vibe.rs` / `author.rs` write to `<data>/ai_blocks/`; `Blocks::load` merges it only with `--vibe` or an explicit flag | Free. Fixes the current leak | 0 |
| A1 | **Art director per section:** picks regime and quality-diversity niche per section (calm or mono in build-ups, maximal on drops, organic in breakdowns); core evolution fills in the rest | An intentional arc that follows the music and the vibe text | 4 | 4 | S–M | `vibe.rs` schema: per shot `{regime, niche, space, transition}`; `evolve.rs` searches inside the niche; `director.rs` applies it | One cached call (~$0.01–0.05). Low risk: on failure the seed chooses | 5, 8, 10, 14 |
| A2 | **Vision critique loop:** render 4–6 frames, a vision model scores them against the vibe and suggests edits, 2–3 rounds | Looks that actually match "rainy neon Tokyo" instead of matching block names | 4 | 3 | M | `llm.rs` adds `image_url` (base64 PNG) message parts (don't exist today); judge frames read back to PNG (`score.rs`); loop in `vibe.rs` | ~$0.05–0.50 per run; the biggest cost driver. Not every compatible server supports vision: detect and skip. Cache by (vibe, genome id) | 10 |
| A3 | **Library growth loop:** AI writes new content for the richer kinds (space transforms, feedback operators, emitters, comp styles). Kept only if it passes the judge **and** a novelty check; saved to a user library; a human can promote reviewed blocks into a versioned core pack | The library broadens over time; promoted packs make non-AI users better too | 5 | 5 | M | `vibe.rs` `write_block`/`try_block` + novelty test (row 10 descriptors) + dedupe hash; save to the A0 folder; optional `--promote` | ~$0.05–0.3 per block attempt. Without row 0 / A0 this breaks byte-identical core output. Promotion is a human decision | 0, 8, 10, A0 |
| A4 | **Reference image → style** (vision input): album cover or photo becomes palette, regime, symmetry and texture choices; the image itself can be a plate | "Make it look like this cover" actually works | 4 | 3 | S–M | Same vision plumbing as A2; replaces "content unknown" in the vibe prompt | ~$0.01–0.05. Palette extraction stays in core (row 28). Privacy: the image goes to the user's own endpoint; say so in the wizard | 4, 5, 8, 28 |
| A5 | **Plates as material, not just backdrops:** displacement maps, feedback seeds (a frame injected on drops), LUT / gradient-map sources, masks for blend modes | The painted image melts, mutates and dissolves into the feedback loop | 4 | 4 | M | Core first: `plate()` sampled inside motion / emit / blend blocks (`plate_tex` is already bound); new blocks `motion_displace_plate`, `emit_plate_inject`; the AI paints plates per role | ~$0.02–0.20 per image, cached per prompt. Works in core with `--image` and no AI | 6, 11, 13, 22 |
| A6 | **Per-drop set pieces:** for each drop a one-off design (special block chain, anticipation shape, release transition) and a contrasting look for the breakdown before it | Every drop is a distinct hero moment, not "same look, brighter" | 5 | 4 | S–M | Storyboard shots gain `set_piece` genomes + row 3 anticipation params; director cuts with feedback reset | One cached call (~$0.02–0.1); quality depends on row 9/21 content | 3, 8, 12, 14, 16 |
| A7 | **Track title, genre and optional lyrics → themes** | Imagery and overlay words match the track | 3 | 3 | S | `vibe.rs`: build the vibe from file stem, tags and tracklist when no `--vibe` is given; lyrics only if the user supplies them (no fetching, never upload audio) | ~$0.01. Weak on instrumental DnB | – |
| A8 | **Text → palette and grade** | "Toxic sunset" becomes an exact cosine palette and finish grade | 2 | 2 | S | Schema field with the (a, b, c, d) values and a tint | Practically free (rides inside the design call) | 4, 18 |
| A9 | **AI authors whole recipe templates** (stateless 3D, dual emitter) under core-defined contracts | New grammars beyond feedback, e.g. a raymarched cathedral per vibe | 5 | 5 | M–L | `author.rs` pattern pointed at row 21 contracts; examples from core templates, not `liquid_tunnel` | ~$0.1–1 per attempt; frequent compile failures (3-try loop exists); GPU-hang risk: iteration caps in the contract and an audition timeout | 21 |
| A10 | **Simulation recipes:** AI picks reaction-diffusion regimes or Lenia species and spatial f/k maps tied to the vibe | "Coral that grows through the breakdown, mitosis on the drop" | 4 | 3 | S | Design schema fields; the core simulation layer does the work | Small call | 22, 23 |
| A11 | **Caching and sharing:** designs keyed by (vibe, audio hash, library version); export / import `.look` packs (genomes, blocks, plates) | Re-renders and shared packs cost nothing | 2 | 3 | S–M | Cache key gains the library version; a pack is a zip/dir of toml, wgsl and png | Imported WGSL is code: compile it in the judge first and keep it in the user library | 0, A0 |
| A12 | **Reroll by intent:** the wizard's "new take" plus a nudge such as "darker, more glitch" | Fast, conversational iteration in the preview loop | 3 | 2 | S | Wizard preview menu calls the vibe with the previous design plus a delta | < $0.02, cached per nudge | – |
| A13 | **Learning the user's taste:** `--keep` picks and retake choices summarised into the next vibe prompt | Takes drift toward what this user likes | 3 | 2 | M | Log choices; next prompt includes liked / rejected summaries | Per user only; never touches core defaults | 10 |
| A14 | **AI-written phrase variations** (8/16-bar structural deltas per section) | The look develops in deliberate steps through the track | 3 | 3 | S | Schema for per-phrase genome deltas | Small call | 15 |

**Core-vs-AI split stays explicit.** Every AI row produces only data the core already understands: genomes,
regime and niche choices, palettes, plates, and blocks in their own folder. Nothing in the core comes to depend on
AI. The one genuinely new AI capability is **vision** (A2, A4): about 50–100 lines in `llm.rs`, and only worth it
once the core has a rich space to critique.

### Recommended AI order (after core Phase 1)

1. **A0** right away (small fix for an existing problem).
2. **A1 + A8** (cheap, safe, big perceived jump), then **A6** (per-drop set pieces).
3. **A4, then A2** (vision).
4. **A5 and A3** (plates as material, library growth into reviewed core packs: the highest-leverage AI idea,
   because promoted packs improve the core for users without AI).
5. **A9, A10, A11, A12, A13, A14** as they become useful.

---

## 4. Sources

**VJ practice**
- https://www.geisswerks.com/milkdrop/milkdrop_preset_authoring.html
- https://github.com/A-Archives-and-Forks/MilkDrop3
- https://github.com/jberg/butterchurn
- https://docs.webamp.org/docs/features/mikdrop/
- https://getwacup.com/community/index.php?topic=677.0
- https://github.com/xoxodin/butterchurn-visualizer
- https://github.com/projectM-visualizer/presets-cream-of-the-crop
- https://github.com/projectM-visualizer/projectm
- https://vjloopstudio.com/posts/audio-reactive-guide/
- https://yitingintech.substack.com/p/how-i-built-real-time-audio-analysis
- https://vortexia.live/visuals/techno-vj-loops-browser/
- https://heavym.net/how-to-sync-visuals-with-music-audio-reactive-projection-mapping-made-simple/
- https://app.synesthesia.live/docs/ssf/audio_uniforms.html
- https://app.synesthesia.live/docs/ssf/best_practices.html
- https://resolume.com/support/en/blend-modes
- https://vjacademy.info/resolume-layers
- https://josleys.com/article_show.php?id=82
- https://gamedev.net/forums/topic/590070-glsl-droste/
- https://static.laszlokorte.de/escher/
- https://derivative.ca/community-post/tutorial/glsl-slit-scan-effect-touchdesigner/74007
- https://lecture.nakayasu.com/en/docs/touchdesigner/slit-scan/
- https://tympanus.net/codrops/2026/09/02/breaking-the-frame-building-a-real-time-datamosh-effect-with-three-js/
- https://github.com/niccolofanton/data-mosh
- https://github.com/Gabe-flomo/shader-studio/pull/448
- https://interactiveimmersive.io/blog/touchdesigner-resources/datamoshing-in-touchdesigner-part-2/
- https://buck.co/research/moshing
- https://github.com/evoluteur/reaction-diffusion
- https://jc-lights.com/dj-lighting-effects-guide-for-clubs-and-stage-shows/
- https://w3c.github.io/wcag21/understanding/three-flashes-or-below-threshold.html
- https://pmc.ncbi.nlm.nih.gov/articles/PMC11872230/

**Shader and generative-art techniques**
- https://iquilezles.org/articles/palettes/
- https://iquilezles.org/articles/sdfrepetition/
- https://iquilezles.org/articles/warp/ (seen only in a search snippet; the page returned 404)
- https://shadertoy.com/view/lsl3RH
- https://karlsims.com/rd.html
- https://developer.nvidia.com/gpugems/gpugems/part-vi-beyond-triangles/chapter-38-fast-fluid-dynamics-simulation-gpu
- https://paulbourke.net/fractals/clifford/
- https://www.cs.ubc.ca/~rbridson/docs/bridson-siggraph2007-curlnoise.pdf
- http://blog.hvidtfeldts.net/index.php/2011/08/distance-estimated-3d-fractals-iii-folding-space/
- http://blog.hvidtfeldts.net/index.php/2011/11/distance-estimated-3d-fractals-vi-the-mandelbox/
- https://woo4.me/wooscripter/kaleidoscopic-ifs-fractals/
- https://en.wikipedia.org/wiki/Lenia
- https://arxiv.org/pdf/2005.03742
- https://www.w3.org/TR/WGSL/
- https://github.com/gpuweb/gpuweb/issues/1360
- https://toji.dev/webgpu-best-practices/compute-vertex-data.html
- https://www.alanzucconi.com/2017/12/13/gpu-sorting-2/
- https://ciphrd.com/2020/04/08/pixel-sorting-on-shader-using-well-crafted-sorting-filters-glsl/

**Caveats.** Some content came from general knowledge rather than fetched pages: GPU cost estimates, Gray-Scott
regimes beyond Karl Sims' two presets, the GPU pixel-sorting entry, hyperbolic / Droste / slit-scan specifics, the
determinism notes, and MAP-Elites (no URL). Beeple, Notch and VDMX are not covered by any source. AI costs are
estimates, not measurements.
