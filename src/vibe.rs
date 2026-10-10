//! "Describe the vibe" → starting genomes for the evolution. The AI only picks from the block
//! menu (ids are an enum in the schema; every answer is validated), so it can't break a render.
//! Cached in <name>.vibe.json: same vibe text = no new call, identical re-renders.
use crate::genome::{Block, Blocks, Genome};
use crate::llm::{self, AiConfig};
use crate::types::Analysis;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Cache {
    vibe: String,
    genomes: Vec<Genome>,
}

pub fn genomes(c: &AiConfig, blocks: &Blocks, a: &Analysis, vibe: &str, cache: &Path) -> Result<Vec<Genome>> {
    if let Some(hit) = std::fs::read_to_string(cache).ok().and_then(|s| serde_json::from_str::<Cache>(&s).ok()).filter(|h| h.vibe == vibe) {
        let ok: Vec<_> = hit.genomes.into_iter().filter(|g| valid(g, blocks)).collect();
        if !ok.is_empty() {
            return Ok(ok);
        }
    }
    let menu = |list: &[Block]| list.iter().map(|b| format!("- {}: {}", b.id, b.description)).collect::<Vec<_>>().join("\n");
    let ids = |list: &[Block]| list.iter().map(|b| b.id.clone()).collect::<Vec<_>>();
    let system = format!(
        "You design music visuals by combining building blocks. Each look = one motion + one emitter + one colour block + a 4-colour palette \
         (sRGB hex; first colour dark, the background). Dark backgrounds with vivid accents read best on video.\n\nMotion blocks:\n{}\n\nEmitter blocks:\n{}\n\nColour blocks:\n{}",
        menu(&blocks.motion),
        menu(&blocks.emitter),
        menu(&blocks.colour)
    );
    let user = format!(
        "Vibe: {vibe}\nMusic: {:.0} BPM, {:.0} minutes, {} drops, {} track changes.\nGive 6 distinct looks that fit this vibe.",
        a.bpm,
        a.duration / 60.0,
        a.drops.len(),
        a.track_changes.len()
    );
    let schema = json!({
        "type": "object",
        "properties": {"genomes": {"type": "array", "minItems": 1, "maxItems": 8, "items": {
            "type": "object",
            "properties": {
                "motion": {"type": "string", "enum": ids(&blocks.motion)},
                "emitter": {"type": "string", "enum": ids(&blocks.emitter)},
                "colour": {"type": "string", "enum": ids(&blocks.colour)},
                "palette": {"type": "array", "minItems": 4, "maxItems": 4, "items": {"type": "string", "pattern": "^#[0-9a-fA-F]{6}$"}}
            },
            "required": ["motion", "emitter", "colour", "palette"]
        }}},
        "required": ["genomes"]
    });
    let input = llm::call_tool(c, &system, &user, "submit_looks", "Submit the looks", schema, 2000)?;
    let out = parse(&input, blocks);
    // A failed cache write must not lose the genomes.
    let _ = std::fs::write(cache, serde_json::to_string_pretty(&Cache { vibe: vibe.into(), genomes: out.clone() })?);
    Ok(out)
}

/// Valid genomes only: known block ids and parseable palettes; the rest are dropped.
fn parse(input: &serde_json::Value, blocks: &Blocks) -> Vec<Genome> {
    input["genomes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| serde_json::from_value::<Genome>(g.clone()).ok())
        .filter(|g| valid(g, blocks))
        .collect()
}

fn valid(g: &Genome, blocks: &Blocks) -> bool {
    blocks.parts(g).is_ok() && crate::scenes::palette(&g.palette).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keeps_only_valid_genomes() {
        let blocks = Blocks::load(Path::new("scenes/blocks")).unwrap();
        let (m, e, c) = (&blocks.motion[0].id, &blocks.emitter[0].id, &blocks.colour[0].id);
        let pal = json!(["#000000", "#ff0000", "#00ff00", "#0000ff"]);
        let input = json!({"genomes": [
            {"motion": m, "emitter": e, "colour": c, "palette": pal},
            {"motion": "nope", "emitter": e, "colour": c, "palette": pal},
            {"motion": m, "emitter": e, "colour": c, "palette": ["red", "#ff0000", "#00ff00", "#0000ff"]},
        ]});
        assert_eq!(parse(&input, &blocks).len(), 1);
    }
}
