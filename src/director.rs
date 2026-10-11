//! OWNER: director agent. Contract signatures below must stay.
use crate::genome::{self, Blocks, Genome};
use crate::markers::Marker;
use crate::scenes::Scene;
use crate::types::*;
use anyhow::{bail, Result};

pub struct PlanRequest<'a> {
    pub analysis: &'a Analysis,
    pub scenes: &'a [Scene],
    pub mode: Mode,
    pub markers: &'a [Marker],
    pub seed: u64,
    /// Existing plan: its locked segments are kept verbatim.
    pub previous: Option<&'a Plan>,
    /// Segment indices to re-roll (new per-segment seed), everything else unchanged.
    pub reroll: &'a [usize],
    /// Dev tool: force this scene id on every segment, in any mode.
    pub force_scene: Option<&'a str>,
    /// Fraction (0..1) of auto-picked segments that get a generated scene.
    pub generated: f32,
    /// --vibe text (only steers generated scenes; a change replans unlocked segments).
    pub vibe: Option<&'a str>,
    /// Supplies a genome for a generated segment: (start, end, seed) -> genome, None = fall back to curated.
    pub genome_for: Option<&'a dyn Fn(f64, f64, u64) -> Option<Genome>>,
    pub blocks: Option<&'a Blocks>,
    /// Where generated scenes' .wgsl files live.
    pub gen_dir: &'a std::path::Path,
    /// Shot changes inside the music (storyboard / --layers); ignored in markers mode.
    pub shots: &'a [Shot],
    /// Visual engine (Plan.engine).
    pub engine: u32,
    /// Auto-pick pool (Plan.pool).
    pub pool: Pool,
}

// ---------------------------------------------------------------- rng

/// splitmix64 stream.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix64(self.0)
    }
    /// Uniform in -1..1.
    fn signed(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
    }
}

// ---------------------------------------------------------------- plan

/// A scene boundary before scene/palette/variations are chosen.
struct Boundary {
    start: f64,
    title: Option<String>,
    scene: Option<String>,
    transition: Transition,
}

const CUT: Transition = Transition { kind: TransitionKind::Cut, duration: 0.0 };

pub fn plan(req: &PlanRequest) -> Result<Plan> {
    let a = req.analysis;
    if req.scenes.is_empty() {
        bail!("no scenes");
    }
    // --reroll counts the saved plan's segments; a phrase variant belongs to the segment it varies.
    let reroll: Vec<usize> = match req.previous {
        Some(p) if p.segments.iter().any(|s| s.variant) => {
            req.reroll.iter().map(|&i| p.segments.iter().take(i + 1).filter(|s| !s.variant).count().saturating_sub(1)).collect()
        }
        _ => req.reroll.to_vec(),
    };
    // Auto picks come from the shipped scenes only (unless --pool all); a library without them (tests) uses all.
    let builtin: Vec<Scene> = req.scenes.iter().filter(|s| crate::scenes::is_builtin(&s.id)).cloned().collect();
    let pool: &[Scene] = if builtin.is_empty() || req.pool == Pool::All { req.scenes } else { &builtin };
    let bar = bar_len(a);
    let near_drop = |t: f64| a.drops.iter().any(|d| (d.time - t).abs() <= bar);
    let known = |id: &Option<String>| match id {
        Some(s) if !req.scenes.iter().any(|sc| &sc.id == s) => {
            eprintln!("warning: unknown scene '{s}' in markers, ignored");
            None
        }
        other => other.clone(),
    };

    // A change landing near a drop flashes (engine 2: clears the feedback) instead of its usual transition.
    let hit = Transition::drop_cut(req.engine);
    let flash_near_drop = |t: f64, other: Transition| if near_drop(t) { hit } else { other };
    let change_transition = |c: &TrackChange| match c.kind {
        ChangeKind::Cut => flash_near_drop(c.time, CUT),
        ChangeKind::Blend => Transition { kind: TransitionKind::Morph, duration: c.blend_len.clamp(2.0, 16.0) },
    };

    let mut bounds = vec![Boundary { start: 0.0, title: None, scene: None, transition: CUT }];
    match req.mode {
        Mode::Auto => {
            let mut changes: Vec<_> = a.track_changes.iter().filter(|c| c.confidence >= 0.35).collect();
            changes.sort_by(|x, y| x.time.total_cmp(&y.time));
            for c in changes {
                let last = bounds.last().unwrap().start;
                if c.time - last < 30.0 || a.duration - c.time < 30.0 {
                    continue;
                }
                bounds.push(Boundary { start: c.time, title: None, scene: None, transition: change_transition(c) });
            }
        }
        Mode::Single => {}
        Mode::Markers => {
            for m in req.markers {
                let t = snap(a, m.time, bar);
                // A marker within the first bar names the opening segment (snapping may move it off 0).
                if t < 0.5 || m.time < bar {
                    bounds[0].title = m.name.clone();
                    bounds[0].scene = known(&m.scene);
                    continue;
                }
                // ponytail: markers that collapse onto the previous one or the very end are dropped.
                if t - bounds.last().unwrap().start < 1.0 || a.duration - t < 1.0 {
                    continue;
                }
                // A detected track change within 2 bars tells us how the music actually changes here.
                let transition = match a.track_changes.iter().find(|c| (c.time - t).abs() <= 2.0 * bar) {
                    Some(c) => change_transition(c),
                    None => flash_near_drop(t, Transition { kind: TransitionKind::Morph, duration: MORPH_LEN }),
                };
                bounds.push(Boundary { start: t, title: m.name.clone(), scene: known(&m.scene), transition });
            }
        }
    }

    // Shot changes inside the music (storyboard / --layers). Markers mode keeps the user's own cuts.
    // Hard cuts (flash, reset) are placed first, so a morph too close to one is the shot that gets dropped.
    // Engine 1 keeps its frozen order (only Flash first; its Cut shots stay in the second group).
    if req.mode != Mode::Markers {
        let is_hard = |s: &&Shot| if req.engine >= 2 { !s.transition.kind.is_morph() } else { s.transition.kind == TransitionKind::Flash };
        for s in req.shots.iter().filter(is_hard).chain(req.shots.iter().filter(|s| !is_hard(s))) {
            let t = snap(a, s.time, bar);
            if t < 8.0 * bar || a.duration - t < 8.0 * bar || bounds.iter().any(|b| (b.start - t).abs() < 8.0 * bar) {
                continue;
            }
            bounds.push(Boundary { start: t, title: None, scene: None, transition: s.transition });
        }
        bounds.sort_by(|x, y| x.start.total_cmp(&y.start));
    }

    // Locked segments must survive small boundary shifts (re-analysis, marker edits): pull the nearest
    // boundary within 2 bars onto each locked start, or insert one there.
    if let Some(p) = req.previous {
        let locks: Vec<f64> = p
            .segments
            .iter()
            .filter(|s| s.locked && s.start > 0.05 && s.start < a.duration - 1.0)
            .map(|s| s.start)
            .collect();
        let is_lock = |t: f64| locks.contains(&t);
        for &l in &locks {
            let near = (1..bounds.len())
                .filter(|&j| !is_lock(bounds[j].start) && (bounds[j].start - l).abs() <= 2.0 * bar)
                .min_by(|&x, &y| (bounds[x].start - l).abs().total_cmp(&(bounds[y].start - l).abs()));
            match near {
                Some(j) => bounds[j].start = l,
                None => bounds.push(Boundary { start: l, title: None, scene: None, transition: CUT }),
            }
        }
        bounds.sort_by(|x, y| x.start.total_cmp(&y.start));
        // Boundaries at least 1 s apart; a lock wins over an unlocked neighbour (never over the one at 0).
        let mut out: Vec<Boundary> = Vec::with_capacity(bounds.len());
        for b in bounds {
            match out.last_mut() {
                Some(last) if b.start - last.start < 1.0 => {
                    if is_lock(b.start) && !is_lock(last.start) && out.len() > 1 {
                        *out.last_mut().unwrap() = b;
                    }
                }
                _ => out.push(b),
            }
        }
        bounds = out;
    }

    if let Some(f) = req.force_scene {
        for b in &mut bounds {
            b.scene = Some(f.to_string());
        }
    }

    let n = bounds.len();
    let ends: Vec<f64> = (0..n).map(|i| bounds.get(i + 1).map_or(a.duration, |b| b.start)).collect();
    // Previous segment at the same start: kept verbatim if locked, else its seed is reused
    // (only when the plan seed and mode are unchanged) so earlier re-rolls stick.
    let prev: Vec<Option<&Segment>> = bounds
        .iter()
        .map(|b| req.previous.and_then(|p| p.segments.iter().find(|s| (s.start - b.start).abs() <= 0.05)))
        .collect();
    let same_plan = req.previous.is_some_and(|p| p.same_inputs(req.seed, req.mode, req.generated, req.vibe, req.engine, req.pool));
    let locked = |i: usize| prev[i].filter(|s| s.locked);
    let kept = |i: usize| prev[i].filter(|_| same_plan);
    // Source name mixed in so different files get different takes at the same seed.
    let base_seed = |i: usize| kept(i).map_or_else(|| hash(req.seed ^ fnv1a(a.source.as_bytes()), i as u64), |s| s.seed);

    // A generated scene from genome_for, if this segment rolls one (needs blocks to build it).
    let generate = |i: usize| {
        let f = req.genome_for.filter(|_| req.blocks.is_some())?;
        let roll = (hash(base_seed(i), 5) >> 11) as f64 / (1u64 << 53) as f64;
        if roll < req.generated as f64 { f(bounds[i].start, ends[i], hash(base_seed(i), 6)) } else { None }
    };

    // Pass 1: every segment as if nothing were re-rolled. `genomes` runs parallel to `scenes`.
    let mut scenes: Vec<String> = Vec::with_capacity(n);
    let mut genomes: Vec<Option<Genome>> = Vec::with_capacity(n);
    for i in 0..n {
        let (id, g) = match (locked(i).or(kept(i)), &bounds[i].scene) {
            (Some(s), _) if s.locked => (s.scene.clone(), s.genome.clone()),
            (_, Some(forced)) => (forced.clone(), None),
            (Some(s), None) => (s.scene.clone(), s.genome.clone()),
            (None, None) => match generate(i) {
                Some(g) => (g.id(), Some(g)),
                None => {
                    let recent: Vec<&str> = scenes.iter().rev().take(3).map(|s| s.as_str()).collect();
                    let hard: Vec<&str> = scenes.last().map(|s| s.as_str()).into_iter().collect();
                    (pick(pool, &recent, &hard, hash(base_seed(i), 1)), None)
                }
            },
        };
        scenes.push(id);
        genomes.push(g);
    }

    let mut segments = Vec::with_capacity(n);
    for (i, b) in bounds.iter().enumerate() {
        if let Some(s) = locked(i) {
            let scene = req.force_scene.map_or_else(|| s.scene.clone(), str::to_string);
            // Saved mods index the saved scene's params; a forced scene gets its own.
            // A forced scene also drops the genome (force always wins).
            let (mods, genome) = match req.force_scene.filter(|&f| f != s.scene) {
                Some(f) => (req.scenes.iter().find(|sc| sc.id == f).map_or_else(Vec::new, |sc| mods(sc, s.seed, req.engine)), None),
                None => (s.mods.clone(), s.genome.clone()),
            };
            segments.push(Segment { end: ends[i], scene, mods, genome, ..s.clone() });
            continue;
        }
        let rerolls = reroll.iter().filter(|&&r| r == i).count() as u64;
        // Unlocked segments regenerate from their kept seed (base_seed), so earlier re-rolls stick
        // while titles/transitions follow the current markers and analysis.
        let seed = if rerolls > 0 { hash(base_seed(i), 0xDEAD + rerolls) } else { base_seed(i) };
        // Pass 2: a re-rolled generated segment gets a new genome (curated if genome_for gives none);
        // a re-rolled curated one picks a new scene that fits its fixed neighbours.
        let scene = if rerolls > 0 && b.scene.is_none() {
            let regen = genomes[i].as_ref().and(req.genome_for).and_then(|f| f(b.start, ends[i], hash(seed, 6)));
            let picked = match &regen {
                Some(g) => g.id(),
                None => {
                    let neighbours: Vec<&str> = [i.checked_sub(1), Some(i + 1)]
                        .into_iter()
                        .flatten()
                        .filter_map(|j| scenes.get(j).map(|s| s.as_str()))
                        .collect();
                    let mut avoid: Vec<&str> = scenes[i.saturating_sub(3)..i].iter().map(|s| s.as_str()).collect();
                    avoid.extend(&neighbours);
                    avoid.push(&scenes[i]);
                    pick(pool, &avoid, &neighbours, hash(seed, 1))
                }
            };
            // Later re-rolled neighbours must see this choice, not the old one.
            scenes[i] = picked.clone();
            genomes[i] = regen;
            picked
        } else {
            scenes[i].clone()
        };
        let genome = genomes[i].clone();
        let owned;
        let sc = match (&genome, req.blocks) {
            (Some(g), Some(blocks)) => {
                owned = genome::to_scene(g, blocks, req.gen_dir)?;
                &owned
            }
            _ => req.scenes.iter().find(|s| s.id == scene).unwrap_or(&req.scenes[0]),
        };
        let single = req.mode == Mode::Single;
        segments.push(Segment {
            start: b.start,
            end: ends[i],
            palette: (hash(seed, 2) % sc.palettes.len().max(1) as u64) as usize,
            variations: variations(a, sc, seed, b.start, ends[i], single, req.engine),
            scene,
            seed,
            locked: false,
            title: b.title.clone().or_else(|| kept(i).and_then(|s| s.title.clone())),
            transition_in: if i == 0 { CUT } else { b.transition },
            mods: mods(sc, seed, req.engine),
            genome,
            variant: false,
        });
    }
    // Engine 2: generated segments develop at their phrases, and morphs get a seed-chosen style.
    if req.engine >= 2 {
        if let Some(blocks) = req.blocks {
            segments = phrase_variants(a, segments, blocks, req.gen_dir, req.mode == Mode::Single, req.engine)?;
        }
        for s in segments.iter_mut().filter(|s| !s.locked && s.transition_in.kind == TransitionKind::Morph) {
            s.transition_in.kind = MORPHS[(hash(s.seed, 9) % MORPHS.len() as u64) as usize];
        }
    }
    // A morph window (centred on the start) may not exceed either neighbouring segment, so windows never overlap.
    for i in 1..segments.len() {
        let span = (segments[i - 1].end - segments[i - 1].start).min(segments[i].end - segments[i].start);
        let t = &mut segments[i].transition_in;
        if t.kind.is_morph() {
            t.duration = t.duration.min(span);
        }
    }
    Ok(Plan { version: 1, source: a.source.clone(), seed: req.seed, mode: req.mode, generated: req.generated, vibe: req.vibe.map(str::to_string), segments, plates: vec![], post: None, layers: false, engine: req.engine, pool: req.pool })
}

/// Avoid `soft` if anything is left, else avoid `hard`, else anything.
fn pick(scenes: &[Scene], soft: &[&str], hard: &[&str], r: u64) -> String {
    for avoid in [soft, hard, &[]] {
        let c: Vec<&Scene> = scenes.iter().filter(|s| !avoid.contains(&s.id.as_str())).collect();
        if !c.is_empty() {
            return c[(r % c.len() as u64) as usize].id.clone();
        }
    }
    unreachable!("scenes is non-empty")
}

/// How a phrase variant is entered: a plain cut or a styled morph one bar long.
const PHRASE_CUTS: [TransitionKind; 5] = [TransitionKind::Cut, TransitionKind::Wipe, TransitionKind::Iris, TransitionKind::Melt, TransitionKind::Cells];

/// Engine 2: a generated segment develops at its 16/32-bar phrases. From 8 bars in until 8 bars before its
/// end, each such phrase (none within a bar of a drop: drops get their own cut) starts a variant of the look
/// before it (genome::vary: another fold, comp framing or palette). Locked segments are left alone.
fn phrase_variants(a: &Analysis, segs: Vec<Segment>, blocks: &Blocks, gen_dir: &std::path::Path, single: bool, engine: u32) -> Result<Vec<Segment>> {
    let bar = bar_len(a);
    let mut out = Vec::with_capacity(segs.len());
    for mut s in segs {
        let Some(mut g) = s.genome.clone().filter(|_| !s.locked) else {
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

pub(crate) fn bar_len(a: &Analysis) -> f64 {
    240.0 / if a.bpm > 0.0 { a.bpm } else { 120.0 }
}

/// Snap to a phrase start within 2 bars, else a downbeat within 1 bar, else unchanged.
fn snap(a: &Analysis, t: f64, bar: f64) -> f64 {
    let phrases: Vec<f64> = a.phrases.iter().map(|p| p.time).collect();
    nearest(&phrases, t)
        .filter(|p| (p - t).abs() <= 2.0 * bar)
        .or_else(|| nearest(&a.downbeats, t).filter(|d| (d - t).abs() <= bar))
        .unwrap_or(t)
}

fn variations(a: &Analysis, sc: &Scene, seed: u64, start: f64, end: f64, single: bool, engine: u32) -> Vec<Variation> {
    let min_bars = if single { 8 } else { 16 };
    // (time, is_drop); a drop on a phrase boundary counts as a drop.
    let mut marks: Vec<(f64, bool)> = a
        .phrases
        .iter()
        .filter(|p| p.bars >= min_bars)
        .map(|p| (p.time, false))
        .chain(a.drops.iter().map(|d| (d.time, true)))
        .filter(|&(t, _)| t > start + 0.1 && t < end)
        .collect();
    marks.sort_by(|x, y| x.0.total_cmp(&y.0).then(y.1.cmp(&x.1)));
    marks.dedup_by(|later, earlier| later.0 - earlier.0 < 0.1);

    let mut rng = Rng(hash(seed, 3));
    let mut params: Vec<f32> = sc.params.iter().map(|p| lerp(p.min, p.max, rng.signed() * 0.5 + 0.5)).collect();
    let mut hue = 0.0f32;
    let mut out = vec![Variation { time: start, params: params.clone(), hue_shift: hue }];
    for (k, &(time, drop)) in marks.iter().enumerate() {
        let mut rng = Rng(hash(seed, 100 + k as u64));
        let (step, hue_step) = if drop { (0.6, 0.25) } else { (0.25, 0.08) };
        for (v, p) in params.iter_mut().zip(&sc.params) {
            // rng drawn either way so later picks don't shift; engine 2 holds fixed (structural) params,
            // engine 1 keeps its frozen behaviour.
            let amt = rng.signed() * step;
            if engine < 2 || !p.fixed {
                *v = p.nudge(*v, amt);
            }
        }
        hue = (hue + rng.signed() * hue_step).rem_euclid(1.0);
        out.push(Variation { time, params: params.clone(), hue_shift: hue });
    }
    out
}

/// Scene param j's uniform slot (MAX_PARAMS params split over four vec4s).
fn param_mut(u: &mut SceneUniforms, j: usize) -> &mut f32 {
    let row = match j / 4 {
        0 => &mut u.p0,
        1 => &mut u.p1,
        2 => &mut u.p2,
        _ => &mut u.p3,
    };
    &mut row[j % 4]
}

/// Engine 2 adds the onset sources (appended, so engine-1 picks are unchanged).
const SOURCES: [ModSource; 9] = [
    ModSource::Kick,
    ModSource::Bass,
    ModSource::Mid,
    ModSource::High,
    ModSource::Energy,
    ModSource::Tension,
    ModSource::Drop,
    ModSource::Snare,
    ModSource::Punch,
];

/// 1-2 audio routings onto random non-fixed params (own rng stream, so scene/palette/variation picks are unchanged).
fn mods(sc: &Scene, seed: u64, engine: u32) -> Vec<Mod> {
    let sources = &SOURCES[..if engine >= 2 { 9 } else { 7 }];
    let free: Vec<usize> = (0..sc.params.len()).filter(|&i| !sc.params[i].fixed).collect();
    if free.is_empty() {
        return Vec::new();
    }
    let mut rng = Rng(hash(seed, 4));
    let mut out: Vec<Mod> = Vec::new();
    for _ in 0..1 + rng.next() % 2 {
        let param = free[(rng.next() % free.len() as u64) as usize];
        let source = sources[(rng.next() % sources.len() as u64) as usize];
        let depth = rng.signed().signum() * (0.15 + 0.25 * (rng.signed() * 0.5 + 0.5));
        if !out.iter().any(|m| m.param == param) {
            out.push(Mod { param, source, depth });
        }
    }
    out
}

// ---------------------------------------------------------------- per frame

/// Per-frame state at time `t` seconds. Called 60x per second of audio; must be cheap (binary searches).
pub fn frame_state(plan: &Plan, analysis: &Analysis, scenes: &[Scene], t: f64, width: u32, height: u32, quality: Quality) -> FrameState {
    let a = analysis;
    let bar = bar_len(a);
    let beat_len = bar / 4.0;
    let segs = &plan.segments;
    let seg_at = |t: f64| segs.partition_point(|s| s.start <= t).saturating_sub(1);
    let i = seg_at(t);

    // Audio-driven part, shared by both scenes of a morph.
    let kick = onset_env(&a.kicks, t, 0.12);
    let since_kick = since(&a.kicks, t, |k| k.time);
    let d = a.drops.partition_point(|d| d.time <= t);
    let (drop_env, drop_dt) = match d.checked_sub(1).map(|j| a.drops[j]) {
        Some(dr) => (dr.strength as f64 * (-(t - dr.time) / (2.0 * bar)).exp(), t - dr.time),
        None => (0.0, f64::INFINITY),
    };
    // Half a bar before each drop: 0 → drop strength (engine 2 darkens and drains towards it; shaders squeeze).
    let antic = a.drops.get(d).map_or(0.0, |dr| {
        let pre = dr.time - t;
        if pre > 0.0 && pre < bar / 2.0 { smoothstep((1.0 - pre / (bar / 2.0)) as f32) * dr.strength } else { 0.0 }
    });
    let snare = onset_env(&a.snares, t, 0.09);
    let c = &a.curves;
    let curve = |v: &[f32]| sample(v, c.rate, t);
    let (energy, tension) = (curve(&c.energy), curve(&c.tension));
    let base = SceneUniforms {
        res_time: [width as f32, height as f32, t as f32, 1.0 / SIM_HZ as f32],
        beat: [
            kick as f32,
            phase(&a.beats, |&x| x, t, beat_len),
            phase(&a.downbeats, |&x| x, t, bar),
            phase(&a.phrases, |p| p.time, t, 8.0 * bar),
        ],
        bands: [curve(&c.sub), curve(&c.bass), curve(&c.mid), curve(&c.high)],
        shape: [energy, tension, drop_env as f32, 0.0],
        misc: [0.0, if quality == Quality::Final { 1.0 } else { 0.0 }, since_kick as f32, drop_dt.min(1e3) as f32],
        hits: [snare as f32, onset_env(&a.kicks, t, 0.045) as f32, since(&a.snares, t, |k| k.time).min(1e3) as f32, antic],
        ..Default::default()
    };

    let build = |s: &Segment| -> (String, SceneUniforms) {
        let sc = scenes.iter().find(|x| x.id == s.scene).unwrap_or(&scenes[0]);
        let mut u = base;
        u.shape[3] = (s.seed % 10000) as f32 / 10000.0;
        let pal = sc.palettes.get(s.palette).or(sc.palettes.first());
        if let Some(pal) = pal {
            for (dst, src) in u.pal.iter_mut().zip(pal) {
                *dst = [src[0], src[1], src[2], 1.0];
            }
        }
        let vs = &s.variations;
        let v = vs.partition_point(|v| v.time <= t).saturating_sub(1);
        if let Some(cur) = vs.get(v) {
            let prev = &vs[v.saturating_sub(1)];
            let f = smoothstep(((t - cur.time) / bar) as f32);
            for (j, (x, y)) in prev.params.iter().zip(&cur.params).take(sc.params.len().min(MAX_PARAMS)).enumerate() {
                let val = lerp(*x, *y, f);
                *param_mut(&mut u, j) = val;
            }
            let dh = (cur.hue_shift - prev.hue_shift + 0.5).rem_euclid(1.0) - 0.5;
            u.misc[0] = (prev.hue_shift + dh * f).rem_euclid(1.0);
        }
        // Audio modulation on top of the phrase variations, kept inside each param's range.
        // Levels (bands, energy) sit around 0.5, so they're centred to swing both ways instead of
        // pinning the param at one end; kick/tension/drop are envelopes that rest at 0.
        let src = |m: ModSource| match m {
            ModSource::Kick => base.beat[0],
            ModSource::Bass => base.bands[1] - 0.5,
            ModSource::Mid => base.bands[2] - 0.5,
            ModSource::High => base.bands[3] - 0.5,
            ModSource::Energy => base.shape[0] - 0.5,
            ModSource::Tension => base.shape[1],
            ModSource::Drop => base.shape[2],
            ModSource::Snare => base.hits[0],
            ModSource::Punch => base.hits[1],
        };
        for m in &s.mods {
            // Scenes have ≤ MAX_PARAMS params (checked at load); a hand-edited plan may name a missing or fixed one.
            let Some(p) = sc.params.get(m.param).filter(|p| !p.fixed) else { continue };
            let slot = param_mut(&mut u, m.param);
            *slot = p.nudge(*slot, m.depth * src(m.source));
        }
        (sc.id.clone(), u)
    };

    // Morph window centred on a segment start: either this one's or the next one's.
    let in_morph = |j: usize| {
        let s = segs.get(j)?;
        let dur = s.transition_in.duration;
        (j > 0 && s.transition_in.kind.is_morph() && (t - s.start).abs() < dur / 2.0)
            .then(|| (j, smoothstep(((t - (s.start - dur / 2.0)) / dur) as f32)))
    };
    let morph = in_morph(i).or_else(|| in_morph(i + 1));
    let (scene_a, ua, b, mix) = match morph {
        Some((j, m)) => {
            let (ida, ua) = build(&segs[j - 1]);
            (ida, ua, Some(build(&segs[j])), m)
        }
        None => match segs.get(i) {
            Some(s) => {
                let (ida, ua) = build(s);
                (ida, ua, None, 0.0)
            }
            None => (scenes[0].id.clone(), base, None, 0.0),
        },
    };

    let trans_flash = segs
        .get(i)
        .filter(|s| s.transition_in.kind == TransitionKind::Flash && t >= s.start)
        .map_or(0.0, |s| (1.0 - (t - s.start) / 0.4).max(0.0).powi(2));
    let v2 = plan.engine >= 2;
    let drop_flash = if drop_dt < beat_len { (if v2 { 0.8 } else { 0.4 }) * drop_env } else { 0.0 };
    // Finishing look: --look wins, else the segment's (blended through a morph).
    let post_of = |s: &Segment| plan.post.or_else(|| s.genome.as_ref().map(|g| g.post)).unwrap_or_default();
    let mut post = match morph {
        Some((j, m)) => post_of(&segs[j - 1]).lerp(&post_of(&segs[j]), m),
        None => segs.get(i).map(post_of).unwrap_or_default(),
    };
    if post.strobe + post.invert + post.rgb_split + post.tear > 0.0 {
        // Flashes (strobe, invert) only on downbeats: ≤ 1 per bar, under the 3 Hz flash limit (WCAG 2.3.1)
        // below 720 BPM. Displacement glitches (split, tear) ride the snares.
        let since_down = since(&a.downbeats, t, |&x| x);
        let down = (-since_down / 0.08).exp() as f32;
        post.strobe *= down;
        post.invert *= down;
        post.rgb_split *= snare as f32;
        post.tear *= snare as f32;
    }
    let dark = if v2 { antic } else { 0.0 };
    FrameState {
        scene_a,
        a: ua,
        b,
        mix,
        flash: (0.06 * kick + trans_flash + drop_flash).clamp(0.0, 1.0) as f32,
        exposure: (1.0 + 0.1 * energy) * (1.0 - 0.85 * dark),
        saturation: (1.0 - 0.45 * tension + 0.15 * drop_env as f32).clamp(0.4, 1.2) * (1.0 - 0.8 * dark),
        post,
        morph_style: morph.map_or(0, |(j, _)| segs[j].transition_in.kind.style()),
        // A Reset cut clears the feedback on the first step its segment is active (same lookup as `i`).
        reset: segs.get(i).is_some_and(|s| s.transition_in.kind == TransitionKind::Reset) && i != seg_at(t - 1.0 / SIM_HZ as f64),
    }
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

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

/// Seconds since the last of `xs` (by `time`) at or before `t` (1e3 if none).
fn since<T>(xs: &[T], t: f64, time: impl Fn(&T) -> f64) -> f64 {
    xs[..xs.partition_point(|x| time(x) <= t)].last().map_or(1e3, |x| t - time(x))
}

/// Linear interpolation into a curve sampled at `rate` Hz.
pub(crate) fn sample(v: &[f32], rate: f64, t: f64) -> f32 {
    if v.is_empty() || rate <= 0.0 {
        return 0.0;
    }
    let x = (t * rate).max(0.0);
    let i = (x as usize).min(v.len() - 1);
    let j = (i + 1).min(v.len() - 1);
    lerp(v[i], v[j], (x - i as f64).min(1.0) as f32)
}

/// 0..1 between consecutive marks; outside them, extrapolated with `period`.
fn phase<T>(xs: &[T], key: impl Fn(&T) -> f64, t: f64, period: f64) -> f32 {
    let i = xs.partition_point(|x| key(x) <= t);
    let p = match (i.checked_sub(1).map(|j| key(&xs[j])), xs.get(i).map(&key)) {
        (Some(lo), Some(hi)) if hi > lo => (t - lo) / (hi - lo),
        (Some(lo), _) => ((t - lo) / period).rem_euclid(1.0),
        (None, Some(hi)) => ((t - hi) / period).rem_euclid(1.0),
        (None, None) => (t / period).rem_euclid(1.0),
    };
    p as f32
}

// ---------------------------------------------------------------- chapters

/// YouTube chapters text ("00:00 Title" lines), or None if rules can't be met (<3 chapters).
pub fn chapters(plan: &Plan) -> Option<String> {
    let duration = plan.segments.last()?.end;
    // Segments under 10 s fold into the chapter before them.
    let mut ch: Vec<(f64, Option<&str>)> = Vec::new();
    for s in plan.segments.iter().filter(|s| !s.variant) {
        if ch.last().is_none_or(|&(start, _)| s.start - start >= 10.0) {
            ch.push((s.start, s.title.as_deref()));
        }
    }
    if ch.last().is_some_and(|&(start, _)| duration - start < 10.0) {
        ch.pop();
    }
    if ch.len() < 3 {
        return None;
    }
    let lines: Vec<String> = ch
        .iter()
        .enumerate()
        .map(|(n, &(start, title))| {
            let ts = hms(start.round() as u64, duration >= 3600.0);
            let title = title.map_or_else(|| format!("Track {}", n + 1), str::to_string);
            format!("{ts} {title}")
        })
        .collect();
    Some(lines.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_analysis() -> Analysis {
        let beats: Vec<f64> = (0..1200).map(|i| i as f64 * 0.5).collect();
        let downbeats: Vec<f64> = beats.iter().step_by(4).copied().collect();
        Analysis {
            version: 1,
            source: "x.wav".into(),
            duration: 600.0,
            bpm: 120.0,
            phrases: downbeats.iter().step_by(8).map(|&time| Phrase { time, bars: 16 }).collect(),
            kicks: beats.iter().map(|&time| Kick { time, strength: 1.0 }).collect(),
            beats,
            downbeats,
            curves: Curves {
                rate: 10.0,
                sub: vec![0.5; 6000],
                bass: vec![0.5; 6000],
                mid: vec![0.5; 6000],
                high: vec![0.5; 6000],
                energy: vec![0.5; 6000],
                tension: vec![0.2; 6000],
            },
            drops: vec![Drop { time: 64.0, build_start: 48.0, strength: 1.0 }],
            track_changes: (1..6)
                .map(|k| TrackChange { time: k as f64 * 96.0, confidence: 0.9, kind: ChangeKind::Blend, blend_len: 8.0 })
                .collect(),
            snares: vec![],
        }
    }

    fn test_scenes() -> Vec<Scene> {
        (0..5)
            .map(|k| Scene {
                id: format!("s{k}"),
                params: vec![crate::scenes::ParamRange { name: String::new(), min: 0.0, max: 1.0, fixed: false }],
                palettes: vec![[[0.0; 3]; 4]; 3],
                wgsl_path: Default::default(),
            })
            .collect()
    }

    #[test]
    fn director_deterministic_and_reroll_is_local() {
        let a = test_analysis();
        let scenes = test_scenes();
        let req = |reroll: &'static [usize]| PlanRequest {
            analysis: &a,
            scenes: &scenes,
            mode: Mode::Auto,
            markers: &[],
            seed: 7,
            previous: None,
            reroll,
            force_scene: None,
            generated: 0.0,
            vibe: None,
            genome_for: None,
            blocks: None,
            gen_dir: Path::new("."),
            shots: &[],
            engine: 1,
            pool: Default::default(),
        };
        let p1 = plan(&req(&[])).unwrap();
        let p2 = plan(&req(&[])).unwrap();
        assert_eq!(serde_json::to_string(&p1).unwrap(), serde_json::to_string(&p2).unwrap());
        assert_eq!(p1.segments.len(), 6);
        for w in p1.segments.windows(2) {
            assert_ne!(w[0].scene, w[1].scene);
        }

        let p3 = plan(&req(&[2])).unwrap();
        for (i, (x, y)) in p1.segments.iter().zip(&p3.segments).enumerate() {
            let same = serde_json::to_string(x).unwrap() == serde_json::to_string(y).unwrap();
            assert_eq!(same, i != 2, "segment {i}");
        }
        for w in p3.segments.windows(2) {
            assert_ne!(w[0].scene, w[1].scene);
        }
        // Morph mid-window yields two scenes.
        let f = frame_state(&p1, &a, &scenes, 96.0, 64, 64, Quality::Draft);
        assert!(f.b.is_some() && (f.mix - 0.5).abs() < 1e-3);

        // Adjacent re-rolls see each other's new scene.
        let p4 = plan(&req(&[2, 3])).unwrap();
        for w in p4.segments.windows(2) {
            assert_ne!(w[0].scene, w[1].scene);
        }

        // A locked segment survives its boundary shifting by less than 2 bars.
        let mut locked = p1.clone();
        locked.segments[2].locked = true;
        let mut a2 = a.clone();
        a2.track_changes[1].time += 3.0;
        let p5 = plan(&PlanRequest { analysis: &a2, previous: Some(&locked), ..req(&[]) }).unwrap();
        let s = p5.segments.iter().find(|s| s.locked).expect("lock kept");
        assert_eq!(serde_json::to_string(&s.scene).unwrap(), serde_json::to_string(&locked.segments[2].scene).unwrap());
        assert_eq!(s.start, locked.segments[2].start);
        for w in p5.segments.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn mods_skip_fixed_params_stay_in_range_and_move_params() {
        let a = test_analysis();
        let mut scenes = test_scenes();
        for s in &mut scenes {
            s.params = vec![
                crate::scenes::ParamRange { name: String::new(), min: 0.0, max: 1.0, fixed: true },
                crate::scenes::ParamRange { name: String::new(), min: 2.0, max: 4.0, fixed: false },
            ];
        }
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Auto, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 0.0, vibe: None, genome_for: None, blocks: None, gen_dir: Path::new("."), shots: &[], engine: 1, pool: Pool::Builtin };
        let p = plan(&req).unwrap();
        assert!(p.segments.iter().all(|s| !s.mods.is_empty() && s.mods.iter().all(|m| m.param == 1)));
        let mut flat = p.clone();
        for s in &mut flat.segments {
            s.mods.clear();
        }
        let mut moved = false;
        for t in [10.0, 64.0, 100.0, 300.0] {
            let f = frame_state(&p, &a, &scenes, t, 64, 64, Quality::Draft);
            assert!((2.0..=4.0).contains(&f.a.p0[1]), "t={t}: {}", f.a.p0[1]);
            moved |= f.a.p0[1] != frame_state(&flat, &a, &scenes, t, 64, 64, Quality::Draft).a.p0[1];
        }
        assert!(moved);
        // Engine 2 holds a fixed param at its first draw across phrase/drop variations.
        let p2 = plan(&PlanRequest { engine: 2, ..req }).unwrap();
        assert!(p2.segments.iter().all(|s| s.variations.iter().all(|v| v.params[0] == s.variations[0].params[0])));
        assert!(p2.segments.iter().any(|s| s.variations.len() > 1));
    }

    #[test]
    fn shots_split_a_single_segment() {
        let a = test_analysis();
        let scenes = test_scenes();
        let flash = Transition { kind: TransitionKind::Flash, duration: 0.0 };
        let morph = Transition { kind: TransitionKind::Morph, duration: MORPH_LEN };
        // 64 s is kept (a flash beats the earlier morph at 56 s); 70 s is within 8 bars of it and 2 s is
        // within 8 bars of the start: all three dropped.
        let shots = [
            Shot { time: 56.0, transition: morph },
            Shot { time: 64.0, transition: flash },
            Shot { time: 70.0, transition: flash },
            Shot { time: 2.0, transition: flash },
        ];
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
            engine: 1,
            pool: Default::default(),
        };
        let p = plan(&req).unwrap();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.segments[1].start, 64.0);
        assert_eq!(p.segments[1].transition_in.kind, TransitionKind::Flash);
        // Markers mode ignores shots.
        assert_eq!(plan(&PlanRequest { mode: Mode::Markers, ..req }).unwrap().segments.len(), 1);
    }

    #[test]
    fn generated_segments_carry_their_genome() {
        let a = test_analysis();
        let scenes = test_scenes();
        let block = |id: &str| crate::genome::Block {
            id: id.into(),
            params: vec![crate::scenes::ParamRange { name: String::new(), min: 0.0, max: 1.0, fixed: false }],
            wgsl: String::new(),
            toml: String::new(),
        };
        let blocks = Blocks {
            motion: vec![block("m0"), block("m1")],
            emitter: vec![block("e0"), block("e1")],
            colour: vec![block("c0")],
            backdrop: vec![],
            overlay: vec![],
            space: vec![],
            comp: vec![],
            palettes: vec![["#000000".into(), "#ff0000".into(), "#00ff00".into(), "#0000ff".into()]],
        };
        let gen = |_, _, r| Some(genome::random(&blocks, r));
        let req = |reroll: &'static [usize]| PlanRequest {
            analysis: &a,
            scenes: &scenes,
            mode: Mode::Auto,
            markers: &[],
            seed: 7,
            previous: None,
            reroll,
            force_scene: None,
            generated: 1.0,
            vibe: None,
            genome_for: Some(&gen),
            blocks: Some(&blocks),
            gen_dir: Path::new("."),
            shots: &[],
            engine: 1,
            pool: Default::default(),
        };
        let p = plan(&req(&[])).unwrap();
        for s in &p.segments {
            let g = s.genome.as_ref().expect("all generated");
            assert_eq!(s.scene, g.id());
            // Mods never land on padding slots (motion 1..3, emitter 4..6, colour 7).
            assert!(s.mods.iter().all(|m| [0, 3, 6].contains(&m.param)), "{:?}", s.mods);
        }
        let p2 = plan(&req(&[1])).unwrap();
        assert_eq!(p2.segments[1].scene, p2.segments[1].genome.as_ref().unwrap().id());
        assert_eq!(p2.segments[0].scene, p.segments[0].scene);
        // A forced scene wins over generation.
        let p3 = plan(&PlanRequest { force_scene: Some("s1"), ..req(&[]) }).unwrap();
        assert!(p3.segments.iter().all(|s| s.genome.is_none() && s.scene == "s1"));
    }

    /// Kept/authored scenes are auto-picked only with --pool all.
    #[test]
    fn pool_all_admits_non_builtin_scenes() {
        let a = test_analysis();
        let mut scenes = test_scenes();
        scenes[0].id = "fractal_zoom".into();
        scenes[1].id = "ink_in_water".into();
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Auto, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 0.0, vibe: None, genome_for: None, blocks: None, gen_dir: Path::new("."), shots: &[], engine: 1, pool: Pool::Builtin };
        assert!(plan(&req).unwrap().segments.iter().all(|s| crate::scenes::is_builtin(&s.scene)));
        let all = plan(&PlanRequest { pool: Pool::All, ..req }).unwrap();
        assert!(all.segments.iter().any(|s| !crate::scenes::is_builtin(&s.scene)));
        assert!(serde_json::to_string(&all).unwrap().contains("\"pool\":\"all\""));
    }

    /// Engine 1 must never change: plan JSON and per-frame uniforms (captured before engine 2 work).
    #[test]
    fn engine1_plan_and_frames_are_frozen() {
        let a = test_analysis();
        let scenes = test_scenes();
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Auto, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 0.0, vibe: None, genome_for: None, blocks: None, gen_dir: Path::new("."), shots: &[], engine: 1, pool: Pool::Builtin };
        let p = plan(&req).unwrap();
        let mut s = serde_json::to_string(&p).unwrap();
        for t in [0.3, 10.0, 63.9, 64.05, 96.0, 300.0] {
            let f = frame_state(&p, &a, &scenes, t, 64, 64, Quality::Draft);
            let u = &f.a;
            s += &format!(
                "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?} {} {} {} {}
",
                u.res_time, u.beat, u.bands, u.shape, u.p0, u.p1, u.p2, u.p3, u.pal, u.misc, f.mix, f.flash, f.exposure, f.saturation
            );
        }
        assert_eq!(format!("{:016x}", fnv1a(s.as_bytes())), "ef50e7299a0f9586");
    }

    #[test]
    fn engine2_hits_anticipation_and_gated_glitch() {
        let mut a = test_analysis();
        a.snares = vec![Kick { time: 31.0, strength: 1.0 }];
        let scenes = test_scenes();
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Auto, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None, generated: 0.0, vibe: None, genome_for: None, blocks: None, gen_dir: Path::new("."), shots: &[], engine: 2, pool: Pool::Builtin };
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
                "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?} {:?} {} {} {} {} {:?}
",
                u.res_time, u.beat, u.bands, u.shape, u.p0, u.p1, u.p2, u.p3, u.pal, u.misc, u.hits, f.b.as_ref().map(|b| &b.0), f.mix, f.flash, f.exposure, f.saturation, f.post
            );
        }
        assert_eq!(format!("{:016x}", fnv1a(s.as_bytes())), "7906e936205298a0");
    }

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
        assert!(f.b.is_some() && f.morph_style == s.transition_in.kind.style());
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
}
