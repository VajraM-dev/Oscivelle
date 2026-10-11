//! Generated scenes: a genome picks one motion, emitter and colour block (scenes/blocks/) plus a
//! palette, and compiles to an ordinary scene (.wgsl), so engine/preview/render are unchanged.
use crate::scenes::{ParamRange, Scene};
use crate::types::fnv1a;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Genome {
    pub motion: String,
    pub emitter: String,
    pub colour: String,
    /// sRGB hex, 4 colours.
    pub palette: [String; 4],
    /// Tuned param values, key "<block_id>.<param_name>", in the param's own units.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f32>,
}

impl Genome {
    /// Stable scene id, e.g. "gen_3fa9c01b22de".
    pub fn id(&self) -> String {
        format!("gen_{:012x}", fnv1a(serde_json::to_string(self).unwrap().as_bytes()) & 0xffff_ffff_ffff)
    }

    /// The block id field of `kind`.
    pub fn block_mut(&mut self, kind: &str) -> Result<&mut String> {
        Ok(match kind_index(kind)? {
            0 => &mut self.motion,
            1 => &mut self.emitter,
            _ => &mut self.colour,
        })
    }

    /// Keeps only finite tuned values whose key names a param of this genome's three blocks.
    pub fn prune(&mut self, blocks: &Blocks) {
        let Ok(parts) = blocks.parts(self) else { return self.params.clear() };
        let keys: Vec<String> = parts.iter().flat_map(|b| b.params.iter().filter(|p| !p.name.is_empty()).map(|p| param_key(&b.id, &p.name))).collect();
        self.params.retain(|k, v| keys.contains(k) && v.is_finite());
    }
}

/// Key of a tuned value in `Genome::params`; unnamed params have none.
pub fn param_key(block_id: &str, name: &str) -> String {
    format!("{block_id}.{name}")
}

#[derive(Clone)]
pub struct Block {
    pub id: String,
    pub params: Vec<ParamRange>,
    pub wgsl: String,
    /// Raw .toml text (shown to the AI).
    pub toml: String,
}

#[derive(Clone)]
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
    #[serde(default, rename = "param")]
    params: Vec<ParamRange>,
}

/// Block kinds; `SLOTS` and the `Blocks` lists follow this order.
pub const KINDS: [&str; 3] = ["motion", "emitter", "colour"];

/// Param slots per kind (8 total, the scene uniform limit).
const SLOTS: [usize; 3] = [3, 3, 2];

fn kind_index(kind: &str) -> Result<usize> {
    KINDS.iter().position(|k| *k == kind).with_context(|| format!("unknown kind {kind}"))
}

impl Block {
    /// Parses a block's .toml; returns its kind (motion|emitter|colour) and the block.
    pub fn parse(id: &str, toml: &str, wgsl: &str) -> Result<(String, Block)> {
        let t: BlockToml = toml::from_str(toml).with_context(|| format!("parsing {id}.toml"))?;
        let max = SLOTS[kind_index(&t.kind).with_context(|| id.to_string())?];
        if t.params.len() > max {
            bail!("{id}: at most {max} params");
        }
        let b = Block { id: id.into(), params: t.params, wgsl: wgsl.into(), toml: toml.into() };
        Ok((t.kind, b))
    }
}

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
            let toml = std::fs::read_to_string(&path)?;
            let wgsl = std::fs::read_to_string(path.with_extension("wgsl")).with_context(|| format!("{id}.wgsl"))?;
            let (kind, block) = Block::parse(&id, &toml, &wgsl)?;
            b.add(&kind, block)?;
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

    /// The blocks of `kind`.
    pub fn list(&self, kind: &str) -> Result<&Vec<Block>> {
        Ok([&self.motion, &self.emitter, &self.colour][kind_index(kind)?])
    }

    fn list_mut(&mut self, kind: &str) -> Result<&mut Vec<Block>> {
        Ok(match kind_index(kind)? {
            0 => &mut self.motion,
            1 => &mut self.emitter,
            _ => &mut self.colour,
        })
    }

    /// Adds a block to its kind's list, replacing one with the same id.
    pub fn add(&mut self, kind: &str, b: Block) -> Result<()> {
        let list = self.list_mut(kind).with_context(|| b.id.clone())?;
        match list.iter_mut().find(|x| x.id == b.id) {
            Some(x) => *x = b,
            None => list.push(b),
        }
        Ok(())
    }

    /// Only the blocks and palettes `gs` use (all of self when `gs` is empty).
    pub fn subset(&self, gs: &[Genome]) -> Blocks {
        if gs.is_empty() {
            return self.clone();
        }
        let pick = |list: &[Block], f: fn(&Genome) -> &String| list.iter().filter(|b| gs.iter().any(|g| *f(g) == b.id)).cloned().collect();
        let mut palettes: Vec<[String; 4]> = vec![];
        for g in gs {
            if !palettes.contains(&g.palette) {
                palettes.push(g.palette.clone());
            }
        }
        Blocks { motion: pick(&self.motion, |g| &g.motion), emitter: pick(&self.emitter, |g| &g.emitter), colour: pick(&self.colour, |g| &g.colour), palettes }
    }

    /// The range of param `key` (see `param_key`).
    fn param(&self, key: &str) -> Option<&ParamRange> {
        let all = [&self.motion, &self.emitter, &self.colour].into_iter().flatten();
        all.flat_map(|b| b.params.iter().filter(|p| !p.name.is_empty() && param_key(&b.id, &p.name) == key)).next()
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
/// A tuned value (`g.params`) narrows its range to value ± 15% of the width (fixed: exactly value).
pub fn to_scene(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let pad = ParamRange { name: String::new(), min: 0.0, max: 0.0, fixed: true };
    let mut params = Vec::with_capacity(8);
    for (b, n) in blocks.parts(g)?.iter().zip(SLOTS) {
        for p in &b.params {
            params.push(match (!p.name.is_empty()).then(|| param_key(&b.id, &p.name)).and_then(|k| g.params.get(&k)) {
                Some(&v) => narrow(p, v),
                None => p.clone(),
            });
        }
        params.extend(std::iter::repeat_n(pad.clone(), n - b.params.len()));
    }
    let palette = crate::scenes::palette(&g.palette)?;
    Ok(Scene { id: g.id(), params, palettes: vec![palette], wgsl_path: gen_dir.join(format!("{}.wgsl", g.id())) })
}

/// `p` narrowed to `v` ± 15% of its width (clamped to `p`); fixed params become exactly `v`.
fn narrow(p: &ParamRange, v: f32) -> ParamRange {
    let (lo, hi) = (p.min.min(p.max), p.min.max(p.max));
    let v = v.clamp(lo, hi);
    let w = if p.fixed { 0.0 } else { 0.15 * (hi - lo) };
    let (a, b) = ((v - w).max(lo), (v + w).min(hi));
    // Keep the original orientation (min may exceed max).
    let (min, max) = if p.min <= p.max { (a, b) } else { (b, a) };
    ParamRange { name: p.name.clone(), min, max, fixed: p.fixed }
}

/// Writes the genome's .wgsl into gen_dir and returns its Scene.
pub fn write(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let sc = to_scene(g, blocks, gen_dir)?;
    std::fs::create_dir_all(gen_dir)?;
    std::fs::write(&sc.wgsl_path, wgsl(g, blocks)?)?;
    Ok(sc)
}

/// Writes a genome as a hand-made scene (scenes_dir/<id>.wgsl + .toml); returns the id.
pub fn export(g: &Genome, blocks: &Blocks, scenes_dir: &Path) -> Result<String> {
    let sc = write(g, blocks, scenes_dir)?;
    let pal = g.palette.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", ");
    let mut toml = format!("name = \"{}\"\npalettes = [[{pal}]]\n", sc.id);
    for p in &sc.params {
        toml += &format!("\n[[param]]\nmin = {:?}\nmax = {:?}\nfixed = {}\n", p.min, p.max, p.fixed);
    }
    std::fs::write(sc.wgsl_path.with_extension("toml"), toml)?;
    Ok(sc.id)
}

/// One random change: swap motion, emitter or colour, take another palette, or (when the
/// genome has tuned params) nudge one tuned value by ±15% of its range.
pub fn mutate(g: &Genome, blocks: &Blocks, r: u64) -> Genome {
    let mut out = g.clone();
    let i = (r & 0xffff_ffff) as usize;
    match (r >> 32) % if g.params.is_empty() { 4 } else { 5 } {
        0 => out.motion = nth_id(&blocks.motion, i),
        1 => out.emitter = nth_id(&blocks.emitter, i),
        2 => out.colour = nth_id(&blocks.colour, i),
        3 => out.palette = blocks.palettes[i % blocks.palettes.len()].clone(),
        _ => {
            let (key, v) = out.params.iter_mut().nth(i % g.params.len()).unwrap();
            if let Some(p) = blocks.param(key) {
                *v = p.nudge(*v, if i & 0x8000_0000 == 0 { 0.15 } else { -0.15 });
            }
        }
    }
    // A swapped-out block's tuned params would only change the id, not the render.
    out.prune(blocks);
    out
}

/// Uniform random genome from `r` (a 64-bit hash; consumed in 16-bit pieces).
pub fn random(blocks: &Blocks, r: u64) -> Genome {
    let piece = |k: u32| ((r >> (16 * k)) & 0xffff) as usize;
    Genome {
        motion: nth_id(&blocks.motion, piece(0)),
        emitter: nth_id(&blocks.emitter, piece(1)),
        colour: nth_id(&blocks.colour, piece(2)),
        palette: blocks.palettes[piece(3) % blocks.palettes.len()].clone(),
        params: BTreeMap::new(),
    }
}

fn nth_id(list: &[Block], i: usize) -> String {
    list[i % list.len()].id.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks() -> Blocks {
        Blocks::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("scenes/blocks")).unwrap()
    }

    fn genome() -> Genome {
        let pal = ["#000000".into(), "#ff0000".into(), "#00ff00".into(), "#0000ff".into()];
        Genome { motion: "motion_ink".into(), emitter: "emit_rings".into(), colour: "colour_hue".into(), palette: pal, params: BTreeMap::new() }
    }

    #[test]
    fn empty_params_keep_the_old_id() {
        let json = r##"{"motion":"motion_ink","emitter":"emit_rings","colour":"colour_hue","palette":["#000000","#ff0000","#00ff00","#0000ff"]}"##;
        assert_eq!(genome().id(), format!("gen_{:012x}", fnv1a(json.as_bytes()) & 0xffff_ffff_ffff));
        let mut g = genome();
        g.params.insert("motion_ink.current_speed".into(), 0.007);
        assert_ne!(g.id(), genome().id());
    }

    #[test]
    fn tuned_params_narrow_the_range() {
        let b = blocks();
        let mut g = genome();
        g.params.insert("motion_ink.current_speed".into(), 0.009); // range 0.005..0.009
        g.params.insert("colour_hue.hue_offset".into(), 0.3); // fixed
        g.params.insert("nope.x".into(), 1.0); // unknown: ignored
        let sc = to_scene(&g, &b, Path::new(".")).unwrap();
        let s = &sc.params[1];
        assert!((s.min - 0.0084).abs() < 1e-6 && (s.max - 0.009).abs() < 1e-6, "{s:?}");
        assert_eq!((sc.params[6].min, sc.params[6].max), (0.3, 0.3));
        assert_eq!((sc.params[0].min, sc.params[0].max), (1.2, 2.4)); // untouched
        // mutate's 5th case keeps tuned values inside their range.
        for r in 0..64u64 {
            let m = mutate(&g, &b, (4 << 32) | (r * 0x0400_0001));
            assert!((0.005..=0.009).contains(&m.params["motion_ink.current_speed"]));
        }
    }

    #[test]
    fn subset_keeps_only_used_blocks() {
        let b = blocks();
        let mut g2 = genome();
        g2.colour = "colour_duotone".into();
        let s = b.subset(&[genome(), g2]);
        assert_eq!(s.motion.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["motion_ink"]);
        assert_eq!(s.colour.len(), 2);
        assert_eq!(s.palettes.len(), 1);
        assert_eq!(b.subset(&[]).motion.len(), b.motion.len());
    }
}
