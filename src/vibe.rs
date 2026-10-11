//! "Describe the vibe" → starting genomes for the evolution. The block library is a starting
//! vocabulary, not a limit: the AI tunes params, and writes new blocks (compiled and judged
//! before they are saved to scenes/blocks/) when nothing in the library fits.
//! Looks may add layers (backdrop/overlay), a finishing look and AI pictures (plates); an optional
//! storyboard picks shot changes. Every answer is validated, so the AI can't break a render.
//! Cached in <name>.vibe.json: same vibe text + images = no new call, identical re-renders.
use crate::genome::{self, Block, Blocks, Genome};
use crate::llm::{self, AiConfig};
use crate::score::Judge;
use crate::types::{fnv1a, hms, Analysis, Post, Shot, Transition, TransitionKind, MORPH_LEN};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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

const STANCE: &str = "The blocks are a starting vocabulary, not a limit. If no block expresses something the vibe asks for \
(a shape, texture, kind of motion, colour treatment), add it to new_blocks rather than settling for a poor match. Tune params to the vibe.";

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
fn images_text(user: usize, room: usize) -> String {
    let mut s = String::from("## Pictures\nLooks show a picture (plate) through img(), e.g. the back_plate and back_sprite backdrops. plate = index, -1 = none.\n");
    if user > 0 {
        s += &format!("The user gave {user} picture(s): plates 0..{} (content unknown; make them the heroes).\n", user - 1);
    }
    if room > 0 {
        s += &format!(
            "You may request up to {room} new pictures (plates: prompt + sprite); they become plates {user} onwards, in order. \
sprite = true for one object on a transparent background (floating cut-outs, back_sprite); false for a full-frame picture \
(sky, landscape, texture; back_plate). Describe one strong image, no text in it.\n"
        );
    } else {
        s += "Picture generation is not available: request no plates.\n";
    }
    s
}

/// Candidate shot moments, sorted, at most 60: the start and each drop's build-up and drop, then phrase
/// starts spread evenly over the song to fill the rest (time, label).
fn moments(a: &Analysis) -> Vec<(f64, String)> {
    const MAX: usize = 60;
    let c = &a.curves;
    let at = |t: f64, v: &[f32]| crate::director::sample(v, c.rate, t);
    let mut m: Vec<(f64, &str)> = vec![(0.0, "start")];
    for d in &a.drops {
        m.push((d.build_start, "build-up starts"));
        m.push((d.time, "DROP"));
    }
    // Stable sort, then drop near-duplicates keeping the first.
    m.sort_by(|x, y| x.0.total_cmp(&y.0));
    m.dedup_by(|x, y| (x.0 - y.0).abs() < 1.0);
    // A drop wins over a phrase at the same time.
    let mut p: Vec<f64> = a.phrases.iter().map(|p| p.time).filter(|&t| t > 1.0 && m.iter().all(|x| (x.0 - t).abs() >= 1.0)).collect();
    p.sort_by(f64::total_cmp);
    p.dedup_by(|x, y| (*x - *y).abs() < 1.0);
    let room = MAX.saturating_sub(m.len()).min(p.len());
    m.extend((0..room).map(|i| (p[i * p.len() / room], "phrase")));
    m.sort_by(|x, y| x.0.total_cmp(&y.0));
    m.truncate(MAX);
    m.into_iter()
        .map(|(t, k)| (t, format!("{} {k}, energy {:.2}, tension {:.2}", hms(t as u64, false), at(t, &c.energy), at(t, &c.tension))))
        .collect()
}

/// Cached in `cache` (<song>.vibe.json): same vibe text and images, all genomes still valid => no AI call.
/// Pictures are cached per prompt in plates/ beside `cache`.
/// May write new blocks into `blocks_dir` (scenes/blocks). Prints progress + the note to stderr.
#[allow(clippy::too_many_arguments)]
pub fn run(c: &AiConfig, judge: &mut Judge, blocks_dir: &Path, gen_dir: &Path, blocks: &Blocks, a: &Analysis, vibe: &str, images: &[PathBuf], cache: &Path) -> Result<Design> {
    if let Some(hit) = std::fs::read_to_string(cache).ok().and_then(|s| serde_json::from_str::<Cache>(&s).ok()).filter(|h| h.vibe == vibe && h.images == images) {
        let g = &hit.design.genomes;
        // A deleted picture means a miss: the looks would silently lose it.
        if !g.is_empty() && keep(g.clone(), blocks, hit.design.plates.len()) == *g && hit.design.plates.iter().all(|p| p.exists()) {
            eprintln!("vibe: {}", hit.note);
            return Ok(hit.design);
        }
    }
    // Pictures the AI may request: up to 4, within the plan's plate limit; none without an image model.
    let room = if c.image_model.trim().is_empty() { 0 } else { 4.min(crate::plates::MAX.saturating_sub(images.len())) };
    let contract = std::fs::read_to_string(blocks_dir.join("CONTRACT.md")).context("blocks/CONTRACT.md")?;
    let prelude = std::fs::read_to_string(crate::scenes::prelude_path(blocks_dir.parent().unwrap_or(blocks_dir))).context("_prelude.wgsl")?;
    let mut menu = String::new();
    // space is an engine-2 core gene; not offered to the AI
    for kind in genome::KINDS.iter().filter(|k| **k != "space") {
        for b in blocks.list(kind)? {
            menu += &format!("### {} ({kind})\n```toml\n{}\n```\n", b.id, b.toml.trim());
        }
    }
    let pals = blocks.palettes.iter().map(|p| p.join(" ")).collect::<Vec<_>>().join("\n");
    let system = format!(
        "You design music visuals by combining building blocks. Each look = one motion + one emitter + one colour block + a 4-colour palette \
         (sRGB hex; first colour dark, the background) + param values.\n\n{contract}\n\n## Blocks\n{menu}\n## Palette bank\n{pals}\n\n{STANCE}\n\n{LAYERS}\n\n{}\n\n{STORYBOARD}",
        images_text(images.len(), room)
    );
    let ms = moments(a);
    let user = format!(
        "Vibe: {vibe}\nMusic: {:.0} BPM, {:.0} minutes, {} drops, {} track changes.\n\
         Give 1 to 6 looks, best first, only looks that fit the vibe (fewer is fine). New blocks: 0 to 3.\nMoments:\n{}",
        a.bpm,
        a.duration / 60.0,
        a.drops.len(),
        a.track_changes.len(),
        ms.iter().enumerate().map(|(i, m)| format!("{i}: {}", m.1)).collect::<Vec<_>>().join("\n")
    );
    let schema = json!({
        "type": "object",
        "properties": {
            "note": {"type": "string", "description": "one plain sentence: what the visuals will look like"},
            "new_blocks": {"type": "array", "items": {"type": "object", "properties": {
                "id": {"type": "string", "description": "^(motion|emit|colour|back|over)_[a-z0-9_]{2,30}$, not an existing block id"},
                "kind": {"type": "string", "enum": ["motion", "emitter", "colour", "backdrop", "overlay"]},
                "brief": {"type": "string", "description": "what the block draws or does, for the shader writer"}
            }, "required": ["id", "kind", "brief"]}},
            "looks": {"type": "array", "items": {"type": "object", "properties": {
                "motion": {"type": "string"},
                "emitter": {"type": "string"},
                "colour": {"type": "string"},
                "palette": {"type": "array", "items": {"type": "string", "description": "#rrggbb"}},
                "params": {"type": "array", "items": {"type": "object", "properties": {
                    "key": {"type": "string", "description": "<block_id>.<param_name>"},
                    "value": {"type": "number"}
                }, "required": ["key", "value"]}},
                "backdrop": {"type": "string", "description": "backdrop block id or \"\""},
                "overlay": {"type": "string", "description": "overlay block id or \"\""},
                "plate": {"type": "integer", "description": "plate index, -1 = none"},
                "feedback": {"type": "number"},
                "text": {"type": "string"},
                "post": {"type": "object", "properties": {
                    "vhs": {"type": "number"}, "scanlines": {"type": "number"}, "posterize": {"type": "number"},
                    "tint_amount": {"type": "number"}, "tint": {"type": "string", "description": "#rrggbb"}
                }, "required": ["vhs", "scanlines", "posterize", "tint_amount", "tint"]}
            }, "required": ["motion", "emitter", "colour", "palette", "params", "backdrop", "overlay", "plate", "feedback", "text", "post"]}},
            "plates": {"type": "array", "items": {"type": "object", "properties": {
                "prompt": {"type": "string"}, "sprite": {"type": "boolean"}
            }, "required": ["prompt", "sprite"]}},
            "shots": {"type": "array", "items": {"type": "object", "properties": {
                "moment": {"type": "integer"}, "look": {"type": "integer"},
                "transition": {"type": "string", "enum": ["cut", "flash", "morph"]}
            }, "required": ["moment", "look", "transition"]}}
        },
        "required": ["note", "new_blocks", "looks", "plates", "shots"]
    });
    eprintln!("vibe: designing looks ...");
    let d = llm::call_tool(c, &system, &user, "submit_design", "Submit the design", schema, 8000)?;
    // Indexed as the AI numbered them (shots refer to these); None = unparseable.
    let raw: Vec<Option<Genome>> = d["looks"].as_array().into_iter().flatten().take(6).map(look).collect();
    let req = d["plates"].as_array().map_or(&[][..], |v| &v[..v.len().min(room)]);
    // Only pictures a parsed look shows are painted (they cost money).
    let used: Vec<bool> = (0..req.len()).map(|i| raw.iter().flatten().any(|g| g.plate == Some((images.len() + i) as u32))).collect();
    let painted = paint(c, req, &used, &cache.with_file_name("plates"));
    // The AI's plate numbering -> the plan's (failed pictures drop out).
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
        .map(|g| {
            g.map(|mut g| {
                g.plate = g.plate.and_then(|i| remap.get(i as usize).copied().flatten());
                g
            })
        })
        .collect();
    let looks: Vec<Genome> = raw.iter().flatten().cloned().collect();
    let mut all = blocks.clone();
    for nb in d["new_blocks"].as_array().into_iter().flatten().take(3) {
        let s = |k: &str| nb[k].as_str().unwrap_or_default().to_string();
        let (asked, kind, brief) = (s("id"), s("kind"), s("brief"));
        if !valid_id(&asked) {
            eprintln!("warning: vibe: skipping new block with bad id '{asked}'");
            continue;
        }
        let id = asked;
        // An id already in the library (or written earlier in this reply) means that block: never overwrite it.
        if blocks_dir.join(format!("{id}.toml")).exists() || blocks_dir.join(format!("{id}.wgsl")).exists() {
            continue;
        }
        // Looks using a block that isn't saved are dropped by `keep`.
        let saved = write_block(c, judge, gen_dir, &all, a, vibe, &looks, &id, &kind, &brief, &prelude, &contract).and_then(|b| {
            std::fs::write(blocks_dir.join(format!("{id}.wgsl")), &b.wgsl)?;
            std::fs::write(blocks_dir.join(format!("{id}.toml")), &b.toml)?;
            all.add(&kind, b)
        });
        match saved {
            Ok(()) => eprintln!("  new block {id}: {brief}"),
            Err(e) => eprintln!("warning: vibe: new block {id} failed ({e:#}); dropping the looks that use it"),
        }
    }
    let kept: Vec<Option<Genome>> = raw.into_iter().map(|g| g.and_then(|g| keep_one(g, &all, plates.len()))).collect();
    let genomes: Vec<Genome> = kept.iter().flatten().cloned().collect();
    // The AI's look index -> its position in `genomes`.
    let pos: Vec<Option<usize>> = kept
        .iter()
        .scan(0, |n, g| {
            Some(g.as_ref().map(|_| {
                *n += 1;
                *n - 1
            }))
        })
        .collect();
    let note = d["note"].as_str().unwrap_or_default().trim().to_string();
    eprintln!("vibe: {note}");
    if genomes.is_empty() {
        bail!("the AI gave no usable looks");
    }
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
            let duration = if kind == TransitionKind::Morph { MORPH_LEN } else { 0.0 };
            Some((Shot { time, transition: Transition { kind, duration } }, look))
        })
        .collect();
    shots.sort_by(|x, y| x.0.time.total_cmp(&y.0.time));
    let design = Design { genomes, plates, shots };
    // A failed cache write must not lose the design.
    let _ = std::fs::write(cache, serde_json::to_string_pretty(&Cache { vibe: vibe.into(), note, images: images.to_vec(), design: design.clone() })?);
    Ok(design)
}

/// Paints (or reuses, cached by prompt) the `used` requested pictures, in parallel; None where unused,
/// blank or painting failed.
fn paint(c: &AiConfig, req: &[Value], used: &[bool], dir: &Path) -> Vec<Option<PathBuf>> {
    std::thread::scope(|s| {
        let jobs: Vec<_> = req
            .iter()
            .zip(used)
            .map(|(p, &used)| {
                let prompt = p["prompt"].as_str().unwrap_or_default();
                let sprite = p["sprite"].as_bool().unwrap_or(false);
                (used && !prompt.trim().is_empty()).then(|| {
                    s.spawn(move || {
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
                })
            })
            .collect();
        jobs.into_iter().map(|j| j.and_then(|j| j.join().ok().flatten())).collect()
    })
}

/// Has the AI write one block: up to 3 attempts, compile/judge errors fed back.
#[allow(clippy::too_many_arguments)]
fn write_block(
    c: &AiConfig,
    judge: &mut Judge,
    gen_dir: &Path,
    all: &Blocks,
    a: &Analysis,
    vibe: &str,
    looks: &[Genome],
    id: &str,
    kind: &str,
    brief: &str,
    prelude: &str,
    contract: &str,
) -> Result<Block> {
    // Layers: the fullest block teaches the most (back_sea for backdrops). Feedback kinds: the first.
    let list = all.list(kind)?;
    let ex = if matches!(kind, "backdrop" | "overlay") { list.iter().max_by_key(|b| b.wgsl.len()) } else { list.first() }.context("no example block")?;
    let system = format!(
        "You write {kind} blocks for a feedback-shader music visualiser (WGSL, wgpu).\n\n\
         The prelude below is prepended to every scene:\n```wgsl\n{prelude}\n```\n\n{contract}\n\n\
         Worked example, {}.wgsl:\n```wgsl\n{}\n```\n{}.toml:\n```toml\n{}\n```",
        ex.id, ex.wgsl, ex.id, ex.toml
    );
    let user = format!("Write the {kind} block '{id}' (prefix its helpers with '{id}'). Brief: {brief}\nThe vibe: {vibe}");
    let schema = json!({
        "type": "object",
        "properties": {"wgsl": {"type": "string", "description": "the block's .wgsl"}, "toml": {"type": "string", "description": "the block's .toml"}},
        "required": ["wgsl", "toml"]
    });
    llm::attempts(c, &system, &user, "submit_block", "Submit the finished block.", &schema, 8000, &format!("vibe: writing block {id}"), |v| {
        try_block(judge, gen_dir, all, a, looks, id, kind, v["wgsl"].as_str().unwrap_or_default(), v["toml"].as_str().unwrap_or_default())
    })
}

/// Parses the block, composes it with the first look that uses it and judges the result.
#[allow(clippy::too_many_arguments)]
fn try_block(judge: &mut Judge, gen_dir: &Path, all: &Blocks, a: &Analysis, looks: &[Genome], id: &str, kind: &str, wgsl: &str, toml: &str) -> Result<Block> {
    let (k, b) = Block::parse(id, toml, wgsl)?;
    if k != kind {
        bail!("kind must be \"{kind}\", not \"{k}\"");
    }
    let mut trial = all.clone();
    trial.add(kind, b.clone())?;
    // The first blocks and palette.
    let mut base = genome::random(&trial, 0);
    let mut g = looks.iter().find(|g| [&g.motion, &g.emitter, &g.colour, &g.backdrop, &g.overlay].iter().any(|s| s.as_str() == id)).cloned().unwrap_or_else(|| base.clone());
    *g.block_mut(kind)? = id.into();
    // Other kinds may name a block that isn't written (yet): use the base's.
    for k in genome::KINDS {
        let s = g.block_mut(k)?;
        if !trial.list(k)?.iter().any(|b| b.id == *s) {
            *s = base.block_mut(k)?.clone();
        }
    }
    if crate::scenes::palette(&g.palette).is_err() {
        g.palette = base.palette;
    }
    let scene = genome::write(&g, &trial, gen_dir)?;
    // Normal judging skips the layer pass, so a new layer block is judged with it drawn.
    if matches!(kind, "backdrop" | "overlay") {
        judge.audition_layered(scene, a)?
    } else {
        judge.audition(scene, a)?
    }
    Ok(b)
}

/// One look from the design call; params come as [{key, value}] (strict schemas have no free-form objects).
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
            ..Default::default()
        },
        ..Default::default()
    })
}

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

/// ^(motion|emit|colour|back|over)_[a-z0-9_]{2,30}$
fn valid_id(id: &str) -> bool {
    ["motion_", "emit_", "colour_", "back_", "over_"]
        .iter()
        .find_map(|p| id.strip_prefix(p))
        .is_some_and(|r| (2..=30).contains(&r.len()) && r.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_valid_looks_and_known_params() {
        let blocks = Blocks::load(Path::new("scenes/blocks")).unwrap();
        let (m, e, c) = (&blocks.motion[0], &blocks.emitter[0].id, &blocks.colour[0].id);
        let known = genome::param_key(&m.id, &m.params[0].name);
        let pal = json!(["#000000", "#ff0000", "#00ff00", "#0000ff"]);
        let p = json!([{"key": known, "value": 0.5}, {"key": "nope.x", "value": 1.0}]);
        let looks: Vec<Genome> = [
            json!({"motion": m.id, "emitter": e, "colour": c, "palette": pal, "params": p}),
            json!({"motion": "nope", "emitter": e, "colour": c, "palette": pal, "params": []}),
            json!({"motion": m.id, "emitter": e, "colour": c, "palette": ["red", "#ff0000", "#00ff00", "#0000ff"], "params": []}),
            json!({"motion": m.id, "emitter": e, "colour": c, "palette": ["#000000"], "params": []}),
        ]
        .iter()
        .filter_map(look)
        .collect();
        assert_eq!(looks.len(), 3);
        let kept = keep(looks, &blocks, 0);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].params.keys().collect::<Vec<_>>(), [&known]);
        assert!(valid_id("emit_cumulus") && !valid_id("emit_x") && !valid_id("fog_cumulus") && !valid_id("emit_Fog"));
    }

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
        assert!(valid_id("back_ocean") && valid_id("over_ticks"));
        let mut a: Analysis = serde_json::from_str(r#"{"version":1,"source":"x","duration":200,"bpm":174,"beats":[],"downbeats":[],"phrases":[],"kicks":[],"curves":{"rate":1,"sub":[],"bass":[],"mid":[],"high":[],"energy":[0.5],"tension":[0.1]},"drops":[],"track_changes":[]}"#).unwrap();
        a.phrases = vec![crate::types::Phrase { time: 64.0, bars: 16 }];
        a.drops = vec![crate::types::Drop { time: 64.0, build_start: 40.0, strength: 1.0 }];
        let ms = moments(&a);
        assert_eq!(ms.iter().map(|m| m.0).collect::<Vec<_>>(), [0.0, 40.0, 64.0]);
        assert!(ms[2].1.contains("DROP"), "a drop wins over a phrase at the same time");
        // A long mix: a late drop survives the cap; phrases fill the rest, spread out, sorted.
        a.duration = 4000.0;
        a.phrases = (1..200).map(|i| crate::types::Phrase { time: i as f64 * 20.0, bars: 16 }).collect();
        a.drops = vec![crate::types::Drop { time: 3990.0, build_start: 3970.0, strength: 1.0 }];
        let ms = moments(&a);
        assert_eq!(ms.len(), 60);
        assert!(ms.iter().any(|m| m.0 == 3990.0 && m.1.contains("DROP")));
        assert!(ms.iter().filter(|m| m.0 > 2000.0).count() > 20 && ms.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
