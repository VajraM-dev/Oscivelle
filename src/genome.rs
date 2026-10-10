//! Generated scenes: a genome picks one motion, emitter and colour block (scenes/blocks/) plus a
//! palette, and compiles to an ordinary scene (.wgsl), so engine/preview/render are unchanged.
use crate::scenes::{ParamRange, Scene};
use crate::types::fnv1a;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Genome {
    pub motion: String,
    pub emitter: String,
    pub colour: String,
    /// sRGB hex, 4 colours.
    pub palette: [String; 4],
}

impl Genome {
    /// Stable scene id, e.g. "gen_3fa9c01b22de".
    pub fn id(&self) -> String {
        format!("gen_{:012x}", fnv1a(serde_json::to_string(self).unwrap().as_bytes()) & 0xffff_ffff_ffff)
    }
}

pub struct Block {
    pub id: String,
    #[allow(dead_code)] // ponytail: read by the AI genome picker (Phase 5).
    pub description: String,
    pub params: Vec<ParamRange>,
    pub wgsl: String,
}

pub struct Blocks {
    pub motion: Vec<Block>,
    pub emitter: Vec<Block>,
    pub colour: Vec<Block>,
    /// Palette bank, sRGB hex.
    pub palettes: Vec<[String; 4]>,
}

#[derive(Deserialize)]
struct BlockToml {
    kind: String,
    description: String,
    #[serde(default, rename = "param")]
    params: Vec<ParamRange>,
}

/// Param slots per kind: motion, emitter, colour (8 total, the scene uniform limit).
const SLOTS: [usize; 3] = [3, 3, 2];

impl Blocks {
    /// Loads scenes/blocks/*.toml (+ matching .wgsl) and palettes.toml.
    pub fn load(dir: &Path) -> Result<Blocks> {
        #[derive(Deserialize)]
        struct Pal {
            palettes: Vec<[String; 4]>,
        }
        let pal: Pal = toml::from_str(&std::fs::read_to_string(dir.join("palettes.toml")).context("blocks/palettes.toml")?)?;
        let mut b = Blocks { motion: vec![], emitter: vec![], colour: vec![], palettes: pal.palettes };
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?.filter_map(|e| Some(e.ok()?.path())).collect();
        paths.sort();
        for path in paths {
            let id = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") || id == "palettes" {
                continue;
            }
            let t: BlockToml = toml::from_str(&std::fs::read_to_string(&path)?).with_context(|| format!("parsing {}", path.display()))?;
            let wgsl = std::fs::read_to_string(path.with_extension("wgsl")).with_context(|| format!("{id}.wgsl"))?;
            let (list, max) = match t.kind.as_str() {
                "motion" => (&mut b.motion, SLOTS[0]),
                "emitter" => (&mut b.emitter, SLOTS[1]),
                "colour" => (&mut b.colour, SLOTS[2]),
                k => bail!("{id}: unknown kind {k}"),
            };
            if t.params.len() > max {
                bail!("{id}: at most {max} params");
            }
            list.push(Block { id, description: t.description, params: t.params, wgsl });
        }
        if b.motion.is_empty() || b.emitter.is_empty() || b.colour.is_empty() || b.palettes.is_empty() {
            bail!("scenes/blocks needs at least one motion, emitter, colour block and one palette");
        }
        Ok(b)
    }

    fn get<'a>(list: &'a [Block], id: &str) -> Result<&'a Block> {
        list.iter().find(|b| b.id == id).with_context(|| format!("unknown block '{id}'"))
    }

    /// The three blocks, or an error naming the unknown id (AI output goes through this).
    pub fn parts(&self, g: &Genome) -> Result<[&Block; 3]> {
        Ok([Self::get(&self.motion, &g.motion)?, Self::get(&self.emitter, &g.emitter)?, Self::get(&self.colour, &g.colour)?])
    }
}

const TEMPLATE: &str = r#"
fn MP(i: i32) -> f32 { return P(i); }
fn EP(i: i32) -> f32 { return P(3 + i); }
fn CP(i: i32) -> f32 { return P(6 + i); }
{MOTION}
{EMITTER}
{COLOUR}
fn scene(uv: vec2<f32>) -> vec4<f32> {
    var col = prev(motion(uv)) * motion_decay();
    let e = emit(to_centered(uv));
    col += colour(e) * (0.6 + 0.8 * kick() + 0.6 * drop_env());
    // Soft luminance cap: the feedback can never flood.
    col = col / (1.0 + 0.12 * dot(col, vec3<f32>(0.2126, 0.7152, 0.0722)));
    return vec4<f32>(col, 1.0);
}
"#;

/// Scene body (prelude is prepended by scenes::shader_source as for any scene).
pub fn wgsl(g: &Genome, blocks: &Blocks) -> Result<String> {
    let [m, e, c] = blocks.parts(g)?;
    Ok(TEMPLATE.replace("{MOTION}", &m.wgsl).replace("{EMITTER}", &e.wgsl).replace("{COLOUR}", &c.wgsl))
}

/// In-memory Scene for a genome: 8 param slots (each block's params padded to its slot count)
/// and the genome's palette. `wgsl_path` = <gen_dir>/<id>.wgsl (written by `write`).
pub fn to_scene(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let pad = ParamRange { min: 0.0, max: 0.0, fixed: true };
    let mut params = Vec::with_capacity(8);
    for (b, n) in blocks.parts(g)?.iter().zip(SLOTS) {
        params.extend(b.params.iter().cloned());
        params.extend(std::iter::repeat_n(pad.clone(), n - b.params.len()));
    }
    let palette = crate::scenes::palette(&g.palette)?;
    Ok(Scene { id: g.id(), params, palettes: vec![palette], wgsl_path: gen_dir.join(format!("{}.wgsl", g.id())) })
}

/// Writes the genome's .wgsl into gen_dir and returns its Scene.
pub fn write(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let sc = to_scene(g, blocks, gen_dir)?;
    std::fs::create_dir_all(gen_dir)?;
    std::fs::write(&sc.wgsl_path, wgsl(g, blocks)?)?;
    Ok(sc)
}

/// Uniform random genome from `r` (a 64-bit hash; consumed in 16-bit pieces).
pub fn random(blocks: &Blocks, r: u64) -> Genome {
    let pick = |n: usize, k: u32| ((r >> (16 * k)) & 0xffff) as usize % n;
    Genome {
        motion: blocks.motion[pick(blocks.motion.len(), 0)].id.clone(),
        emitter: blocks.emitter[pick(blocks.emitter.len(), 1)].id.clone(),
        colour: blocks.colour[pick(blocks.colour.len(), 2)].id.clone(),
        palette: blocks.palettes[pick(blocks.palettes.len(), 3)].clone(),
    }
}
