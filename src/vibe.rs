//! "Describe the vibe" → starting genomes for the evolution. The block library is a starting
//! vocabulary, not a limit: the AI tunes params, and writes new blocks (compiled and judged
//! before they are saved to scenes/blocks/) when nothing in the library fits.
//! Every answer is validated, so the AI can't break a render.
//! Cached in <name>.vibe.json: same vibe text = no new call, identical re-renders.
use crate::genome::{self, Block, Blocks, Genome};
use crate::llm::{self, AiConfig};
use crate::score::Judge;
use crate::types::Analysis;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Cache {
    vibe: String,
    note: String,
    genomes: Vec<Genome>,
}

const STANCE: &str = "The blocks are a starting vocabulary, not a limit. If no block expresses something the vibe asks for \
(a shape, texture, kind of motion, colour treatment), add it to new_blocks rather than settling for a poor match. Tune params to the vibe.";

/// Cached in `cache` (<song>.vibe.json): same vibe text and all genomes still valid => no AI call.
/// May write new blocks into `blocks_dir` (scenes/blocks). Prints progress + the note to stderr.
#[allow(clippy::too_many_arguments)]
pub fn run(c: &AiConfig, judge: &mut Judge, blocks_dir: &Path, gen_dir: &Path, blocks: &Blocks, a: &Analysis, vibe: &str, cache: &Path) -> Result<Vec<Genome>> {
    if let Some(hit) = std::fs::read_to_string(cache).ok().and_then(|s| serde_json::from_str::<Cache>(&s).ok()).filter(|h| h.vibe == vibe) {
        if !hit.genomes.is_empty() && keep(hit.genomes.clone(), blocks) == hit.genomes {
            eprintln!("vibe: {}", hit.note);
            return Ok(hit.genomes);
        }
    }
    let contract = std::fs::read_to_string(blocks_dir.join("CONTRACT.md")).context("blocks/CONTRACT.md")?;
    let prelude = std::fs::read_to_string(crate::scenes::prelude_path(blocks_dir.parent().unwrap_or(blocks_dir))).context("_prelude.wgsl")?;
    let mut menu = String::new();
    for kind in genome::KINDS {
        for b in blocks.list(kind)? {
            menu += &format!("### {} ({kind})\n```toml\n{}\n```\n", b.id, b.toml.trim());
        }
    }
    let pals = blocks.palettes.iter().map(|p| p.join(" ")).collect::<Vec<_>>().join("\n");
    let system = format!(
        "You design music visuals by combining building blocks. Each look = one motion + one emitter + one colour block + a 4-colour palette \
         (sRGB hex; first colour dark, the background) + param values.\n\n{contract}\n\n## Blocks\n{menu}\n## Palette bank\n{pals}\n\n{STANCE}"
    );
    let user = format!(
        "Vibe: {vibe}\nMusic: {:.0} BPM, {:.0} minutes, {} drops, {} track changes.\n\
         Give 1 to 6 looks, best first, only looks that fit the vibe (fewer is fine). New blocks: 0 to 3.",
        a.bpm,
        a.duration / 60.0,
        a.drops.len(),
        a.track_changes.len()
    );
    let schema = json!({
        "type": "object",
        "properties": {
            "note": {"type": "string", "description": "one plain sentence: what the visuals will look like"},
            "new_blocks": {"type": "array", "items": {"type": "object", "properties": {
                "id": {"type": "string", "description": "^(motion|emit|colour)_[a-z0-9_]{2,30}$, not an existing block id"},
                "kind": {"type": "string", "enum": ["motion", "emitter", "colour"]},
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
                }, "required": ["key", "value"]}}
            }, "required": ["motion", "emitter", "colour", "palette", "params"]}}
        },
        "required": ["note", "new_blocks", "looks"]
    });
    eprintln!("vibe: designing looks ...");
    let d = llm::call_tool(c, &system, &user, "submit_design", "Submit the design", schema, 8000)?;
    let looks: Vec<Genome> = d["looks"].as_array().into_iter().flatten().filter_map(look).take(6).collect();
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
    let genomes = keep(looks, &all);
    let note = d["note"].as_str().unwrap_or_default().trim().to_string();
    eprintln!("vibe: {note}");
    if genomes.is_empty() {
        bail!("the AI gave no usable looks");
    }
    // A failed cache write must not lose the genomes.
    let _ = std::fs::write(cache, serde_json::to_string_pretty(&Cache { vibe: vibe.into(), note: note.clone(), genomes: genomes.clone() })?);
    Ok(genomes)
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
    let ex = all.list(kind)?.first().context("no example block")?;
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
    let mut g = looks.iter().find(|g| [&g.motion, &g.emitter, &g.colour].iter().any(|s| s.as_str() == id)).cloned().unwrap_or_else(|| base.clone());
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
    judge.audition(genome::write(&g, &trial, gen_dir)?, a)?;
    Ok(b)
}

/// One look from the design call; params come as [{key, value}] (strict schemas have no free-form objects).
fn look(v: &Value) -> Option<Genome> {
    let s = |k: &str| v[k].as_str().map(String::from);
    let pal: Vec<String> = v["palette"].as_array()?.iter().filter_map(|c| c.as_str().map(String::from)).collect();
    let params = v["params"].as_array().into_iter().flatten().filter_map(|p| Some((p["key"].as_str()?.to_string(), p["value"].as_f64()? as f32))).collect();
    Some(Genome { motion: s("motion")?, emitter: s("emitter")?, colour: s("colour")?, palette: pal.try_into().ok()?, params, ..Default::default() })
}

/// Valid looks only (known blocks, parseable palette), with unknown or non-finite param keys dropped.
fn keep(looks: Vec<Genome>, blocks: &Blocks) -> Vec<Genome> {
    looks
        .into_iter()
        .filter_map(|mut g| {
            blocks.parts(&g).ok()?;
            crate::scenes::palette(&g.palette).ok()?;
            g.prune(blocks);
            Some(g)
        })
        .collect()
}

/// ^(motion|emit|colour)_[a-z0-9_]{2,30}$
fn valid_id(id: &str) -> bool {
    ["motion_", "emit_", "colour_"]
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
        let kept = keep(looks, &blocks);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].params.keys().collect::<Vec<_>>(), [&known]);
        assert!(valid_id("emit_cumulus") && !valid_id("emit_x") && !valid_id("fog_cumulus") && !valid_id("emit_Fog"));
    }
}
