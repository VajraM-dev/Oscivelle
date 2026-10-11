//! Generated scenes: a genome picks one motion, emitter and colour block (scenes/blocks/) plus a
//! palette, and compiles to an ordinary scene (.wgsl), so engine/preview/render are unchanged.
use crate::scenes::{ParamRange, Scene};
use crate::types::{fnv1a, Post};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Genome {
    pub motion: String,
    pub emitter: String,
    pub colour: String,
    /// sRGB hex, 4 colours.
    pub palette: [String; 4],
    /// Tuned param values, key "<block_id>.<param_name>", in the param's own units.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f32>,
    /// Layer blocks ("" = none), drawn behind / over the feedback in a separate pass (`layers_wgsl`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub backdrop: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub overlay: String,
    /// Plan plate index `img()` shows; None = no picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plate: Option<u32>,
    /// Feedback strength over the backdrop (layer looks), 0..1.5; None = 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback: Option<f32>,
    /// Text for overlay blocks that draw it (font.rs charset; anything else becomes a space).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// Finishing look.
    #[serde(default, skip_serializing_if = "Post::is_clean")]
    pub post: Post,
    /// Engine-2 genes ("" = none, the engine-1 recipe). Space block id, blend mode (genome::BLENDS) and
    /// regime name (regime::REGIMES). Empty genes are left out of the JSON, so engine-1 ids are unchanged.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub space: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub blend: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub regime: String,
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
            2 => &mut self.colour,
            3 => &mut self.backdrop,
            4 => &mut self.overlay,
            _ => &mut self.space,
        })
    }

    /// Uses the engine-2 template (has any engine-2 gene).
    pub fn is_v2(&self) -> bool {
        !self.space.is_empty() || !self.blend.is_empty() || !self.regime.is_empty()
    }

    /// Has a backdrop or overlay (so it also has a layer pass).
    pub fn has_layers(&self) -> bool {
        !self.backdrop.is_empty() || !self.overlay.is_empty()
    }

    /// Keeps only finite tuned values whose key names a param of this genome's blocks.
    pub fn prune(&mut self, blocks: &Blocks) {
        let (Ok(parts), Ok(layers), Ok(space)) = (blocks.parts(self), blocks.layer_parts(self), blocks.space_part(self)) else { return self.params.clear() };
        let keys: Vec<String> = parts
            .into_iter()
            .chain(layers.into_iter().flatten())
            .chain(space)
            .flat_map(|b| b.params.iter().filter(|p| !p.name.is_empty()).map(|p| param_key(&b.id, &p.name)))
            .collect();
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
    /// Layer blocks (optional kinds).
    pub backdrop: Vec<Block>,
    pub overlay: Vec<Block>,
    /// Engine-2 space folds (optional kind).
    pub space: Vec<Block>,
    /// Palette bank, sRGB hex.
    pub palettes: Vec<[String; 4]>,
}

#[derive(Deserialize)]
struct BlockToml {
    kind: String,
    #[serde(default, rename = "param")]
    params: Vec<ParamRange>,
}

/// Block kinds; `SLOTS` and the `Blocks` lists follow this order. The last three (layers, space) are optional.
pub const KINDS: [&str; 6] = ["motion", "emitter", "colour", "backdrop", "overlay", "space"];

/// Param slots per kind: 8 for the feedback (scene params 0..7), then 4 + 2 for the layers (8..13);
/// space (engine 2) uses slots 14..15.
const SLOTS: [usize; 6] = [3, 3, 2, 4, 2, 2];

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

impl Blocks {
    /// Loads scenes/blocks/*.toml (+ matching .wgsl) and palettes.toml.
    pub fn load(dir: &Path) -> Result<Blocks> {
        #[derive(Deserialize)]
        struct Pal {
            palettes: Vec<[String; 4]>,
        }
        let pal: Pal = toml::from_str(&std::fs::read_to_string(dir.join("palettes.toml")).context("blocks/palettes.toml")?)?;
        let mut b = Blocks { motion: vec![], emitter: vec![], colour: vec![], backdrop: vec![], overlay: vec![], space: vec![], palettes: pal.palettes };
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

    fn opt<'a>(list: &'a [Block], id: &str) -> Result<Option<&'a Block>> {
        if id.is_empty() {
            Ok(None)
        } else {
            Self::get(list, id).map(Some)
        }
    }

    /// The layer blocks ("" = none), or an error naming an unknown id (AI output goes through this).
    pub fn layer_parts(&self, g: &Genome) -> Result<[Option<&Block>; 2]> {
        Ok([Self::opt(&self.backdrop, &g.backdrop)?, Self::opt(&self.overlay, &g.overlay)?])
    }

    /// The genome's space block ("" = none), or an error naming an unknown id.
    pub fn space_part(&self, g: &Genome) -> Result<Option<&Block>> {
        Self::opt(&self.space, &g.space)
    }

    /// The blocks of `kind`.
    pub fn list(&self, kind: &str) -> Result<&Vec<Block>> {
        Ok([&self.motion, &self.emitter, &self.colour, &self.backdrop, &self.overlay, &self.space][kind_index(kind)?])
    }

    fn list_mut(&mut self, kind: &str) -> Result<&mut Vec<Block>> {
        Ok(match kind_index(kind)? {
            0 => &mut self.motion,
            1 => &mut self.emitter,
            2 => &mut self.colour,
            3 => &mut self.backdrop,
            4 => &mut self.overlay,
            _ => &mut self.space,
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
        Blocks {
            motion: pick(&self.motion, |g| &g.motion),
            emitter: pick(&self.emitter, |g| &g.emitter),
            colour: pick(&self.colour, |g| &g.colour),
            backdrop: pick(&self.backdrop, |g| &g.backdrop),
            overlay: pick(&self.overlay, |g| &g.overlay),
            space: pick(&self.space, |g| &g.space),
            palettes,
        }
    }

    /// The frozen library of `engine` (1 or 2): only its block ids, in load (sorted) order, and its palettes.
    pub fn pinned(&self, engine: u32) -> Blocks {
        let ok = |id: &str| V1_BLOCKS.contains(&id) || (engine >= 2 && V2_BLOCKS.contains(&id));
        let mut b = self.clone();
        for k in KINDS {
            b.list_mut(k).expect("known kind").retain(|x| ok(&x.id));
        }
        b.palettes.truncate(V1_PALETTES);
        b
    }

    /// The range of param `key` (see `param_key`).
    fn param(&self, key: &str) -> Option<&ParamRange> {
        let all = [&self.motion, &self.emitter, &self.colour, &self.backdrop, &self.overlay, &self.space].into_iter().flatten();
        all.flat_map(|b| b.params.iter().filter(|p| !p.name.is_empty() && param_key(&b.id, &p.name) == key)).next()
    }
}

const TEMPLATE: &str = r#"
fn MP(i: i32) -> f32 { return P(i); }
fn EP(i: i32) -> f32 { return P(3 + i); }
fn CP(i: i32) -> f32 { return P(6 + i); }
{IMG}
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

/// A look's layer pass: an ordinary scene whose prev() is the finished feedback (engine compose pass).
const LAYERS: &str = r#"
fn BP(i: i32) -> f32 { return P(8 + i); }
fn OP(i: i32) -> f32 { return P(12 + i); }
{IMG}
{FONT}
{BACKDROP}
{OVERLAY}
fn scene(uv: vec2<f32>) -> vec4<f32> {
    let p = to_centered(uv);
    var c = backdrop(p) + prev(uv) * {FEEDBACK};
    let o = overlay(p);
    c = mix(c, o.rgb, clamp(o.a, 0.0, 1.0));
    return vec4<f32>(c, 1.0);
}
"#;
const NO_BACKDROP: &str = "fn backdrop(p: vec2<f32>) -> vec3<f32> { return vec3<f32>(0.0); }";
const NO_OVERLAY: &str = "fn overlay(p: vec2<f32>) -> vec4<f32> { return vec4<f32>(0.0); }";

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

fn blend_wgsl(name: &str) -> String {
    let e = match name {
        "screen" => "a + b * max(vec3<f32>(1.0) - a, vec3<f32>(0.0))",
        "difference" => "abs(a - b)",
        // A pure multiply goes black in a feedback loop: the emitter reveals the feedback instead.
        "multiply" => "a * (vec3<f32>(0.4) + 1.6 * b) + 0.05 * b",
        "lighten" => "max(a, b)",
        _ => "a + b",
    };
    format!("fn blend(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {{ return {e}; }}")
}

fn img_wgsl(g: &Genome) -> String {
    let i = g.plate.map_or(-1, |i| i as i64);
    format!("// The look's picture (prelude plate()); transparent when it has none.\nfn img(p: vec2<f32>) -> vec4<f32> {{ return plate({i}, p); }}")
}

/// Scene body (prelude is prepended by scenes::shader_source as for any scene).
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
        .replace("{BLEND}", &blend_wgsl(&g.blend))
        .replace("{DECAY}", &format!("{:?}", r.map_or(1.0, |r| r.decay)))
        .replace("{DRIVE}", &format!("{:?}", r.map_or(1.0, |r| r.drive))))
}

/// Layer pass source (None when the genome has no layer blocks).
pub fn layers_wgsl(g: &Genome, blocks: &Blocks) -> Result<Option<String>> {
    if !g.has_layers() {
        return Ok(None);
    }
    let [bd, ov] = blocks.layer_parts(g)?;
    let fb = g.feedback.unwrap_or(1.0).clamp(0.0, 1.5);
    Ok(Some(
        LAYERS
            .replace("{IMG}", &img_wgsl(g))
            .replace("{FONT}", &crate::font::wgsl(&g.text))
            .replace("{BACKDROP}", bd.map_or(NO_BACKDROP, |b| &b.wgsl))
            .replace("{OVERLAY}", ov.map_or(NO_OVERLAY, |b| &b.wgsl))
            .replace("{FEEDBACK}", &format!("{fb:?}")),
    ))
}

/// In-memory Scene for a genome: 8 param slots, 14 with layers, 16 with a space block (each block's params padded to its slot count)
/// and the genome's palette. `wgsl_path` = <gen_dir>/<id>.wgsl (written by `write`).
/// A tuned value (`g.params`) narrows its range to value ± 15% of the width (fixed: exactly value).
pub fn to_scene(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let pad = ParamRange { name: String::new(), min: 0.0, max: 0.0, fixed: true };
    let [m, e, c] = blocks.parts(g)?;
    let [bd, ov] = blocks.layer_parts(g)?;
    let mut slots = vec![(Some(m), SLOTS[0]), (Some(e), SLOTS[1]), (Some(c), SLOTS[2])];
    if g.has_layers() {
        slots.extend([(bd, SLOTS[3]), (ov, SLOTS[4])]);
    }
    // Space params live in slots 14..15 (after the layers' 8..13, padded when there are none).
    if let Some(sp) = blocks.space_part(g)? {
        if !g.has_layers() {
            slots.extend([(None, SLOTS[3]), (None, SLOTS[4])]);
        }
        slots.push((Some(sp), SLOTS[5]));
    }
    let mut params = Vec::with_capacity(crate::types::MAX_PARAMS);
    for (b, n) in slots {
        let own = b.map_or(0, |b| b.params.len());
        for p in b.iter().flat_map(|b| &b.params) {
            let key = b.filter(|_| !p.name.is_empty()).map(|b| param_key(&b.id, &p.name));
            params.push(match key.and_then(|k| g.params.get(&k)) {
                Some(&v) => narrow(p, v),
                None => p.clone(),
            });
        }
        params.extend(std::iter::repeat_n(pad.clone(), n - own));
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
    let lp = crate::scenes::layers_path(&sc);
    match layers_wgsl(g, blocks)? {
        Some(src) => std::fs::write(lp, src)?,
        None => {
            let _ = std::fs::remove_file(lp);
        }
    }
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
/// genome has tuned params) nudge one tuned value by ±15% of its range. A third of the time
/// (chosen by `r`) it jumps instead: two changes at once, a param re-drawn across its whole range.
pub fn mutate(g: &Genome, blocks: &Blocks, r: u64) -> Genome {
    let big = crate::types::mix64(r).is_multiple_of(3);
    let out = mutate_once(g, blocks, r, big);
    if big { mutate_once(&out, blocks, crate::types::mix64(r ^ 1), true) } else { out }
}

fn mutate_once(g: &Genome, blocks: &Blocks, r: u64, big: bool) -> Genome {
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
                *v = if big {
                    p.nudge(p.min.min(p.max), ((i >> 8) & 0xffff) as f32 / 65535.0)
                } else {
                    p.nudge(*v, if i & 0x8000_0000 == 0 { 0.15 } else { -0.15 })
                };
            }
        }
    }
    // A swapped-out block's tuned params would only change the id, not the render.
    out.prune(blocks);
    out
}

/// 16-bit slice `k` of the hash `r`.
fn piece(r: u64, k: u32) -> usize {
    ((r >> (16 * k)) & 0xffff) as usize
}

/// Uniform random genome from `r` (a 64-bit hash; consumed in 16-bit pieces).
pub fn random(blocks: &Blocks, r: u64) -> Genome {
    Genome {
        motion: nth_id(&blocks.motion, piece(r, 0)),
        emitter: nth_id(&blocks.emitter, piece(r, 1)),
        colour: nth_id(&blocks.colour, piece(r, 2)),
        palette: blocks.palettes[piece(r, 3) % blocks.palettes.len()].clone(),
        params: BTreeMap::new(),
        ..Default::default()
    }
}

/// Core --layers: gives a look a backdrop (one showing a picture only when there are plates), an overlay
/// half the time (text overlays only when there is text), a plate and the text. `r` = 64-bit hash.
pub fn dress(g: &Genome, blocks: &Blocks, r: u64, plates: usize, text: &str) -> Genome {
    // ponytail: needs are sniffed from the block source; a toml flag if a block ever calls them indirectly.
    let backs: Vec<&Block> = blocks.backdrop.iter().filter(|b| plates > 0 || !b.wgsl.contains("img(")).collect();
    let overs: Vec<&Block> = blocks.overlay.iter().filter(|b| !text.is_empty() || !b.wgsl.contains("text_px(")).collect();
    let mut out = g.clone();
    if let Some(b) = backs.get(piece(r, 0) % backs.len().max(1)) {
        out.backdrop = b.id.clone();
    }
    if piece(r, 1) % 2 == 0 {
        if let Some(b) = overs.get(piece(r, 2) % overs.len().max(1)) {
            out.overlay = b.id.clone();
        }
    }
    out.plate = (plates > 0).then(|| (piece(r, 3) % plates) as u32);
    out.text = text.into();
    out.feedback = Some(0.7);
    out
}

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

/// Engine-2 random genome: `random`'s blocks plus a space fold (none a sixth of the time), a blend mode
/// (add half the time), a cosine palette half the time, and the segment's `regime` with its finishing look
/// (mono tints with palette colour 2). `r` = 64-bit hash.
pub fn random_v2(blocks: &Blocks, r: u64, regime: &crate::regime::Regime) -> Genome {
    let h = |k: u64| crate::types::hash(r, k);
    let mut g = random(blocks, r);
    if !blocks.space.is_empty() && h(1) % 6 != 0 {
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
        Genome { motion: "motion_ink".into(), emitter: "emit_rings".into(), colour: "colour_hue".into(), palette: pal, params: BTreeMap::new(), ..Default::default() }
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
        // mutate's 5th case keeps tuned values inside their range (a big jump may swap the block away).
        for r in 0..64u64 {
            let m = mutate(&g, &b, (4 << 32) | (r * 0x0400_0001));
            assert!(m.params.get("motion_ink.current_speed").is_none_or(|v| (0.005..=0.009).contains(v)));
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

    #[test]
    fn layers_add_slots_and_a_layer_pass() {
        let b = blocks();
        let plain = genome();
        assert!(layers_wgsl(&plain, &b).unwrap().is_none());
        assert_eq!(to_scene(&plain, &b, Path::new(".")).unwrap().params.len(), 8);
        let g = Genome { backdrop: "back_sky".into(), overlay: "over_hud".into(), plate: Some(1), feedback: Some(0.6), ..genome() };
        assert_eq!(to_scene(&g, &b, Path::new(".")).unwrap().params.len(), 14);
        let src = layers_wgsl(&g, &b).unwrap().unwrap();
        assert!(src.contains("fn backdrop") && src.contains("fn overlay") && src.contains("plate(1, p)") && src.contains("prev(uv) * 0.6"));
        assert!(b.layer_parts(&Genome { backdrop: "nope".into(), ..genome() }).is_err());
        // Without plates, dress never picks a backdrop that needs a picture.
        for r in 0..16u64 {
            let d = dress(&plain, &b, r * 0x0001_0001_0001_0001, 0, "");
            assert!(d.plate.is_none() && !b.backdrop.iter().find(|x| x.id == d.backdrop).unwrap().wgsl.contains("img("));
        }
    }

    /// Engine 1 must never change: fingerprints of generated sources and picks (captured before engine 2 work).
    #[test]
    fn engine1_genomes_are_frozen() {
        let b = blocks().pinned(1);
        let sig = |s: String| format!("{:016x}", fnv1a(s.as_bytes()));
        let layered = Genome { backdrop: "back_sky".into(), overlay: "over_hud".into(), plate: Some(1), feedback: Some(0.6), text: "HI".into(), ..genome() };
        let picks: String = (0..16u64)
            .map(|r| {
                let h = crate::types::hash(1, r);
                let g = random(&b, h);
                format!("{} {} {}
", g.id(), mutate(&g, &b, h ^ 7).id(), dress(&g, &b, h ^ 9, 2, "HI").id())
            })
            .collect();
        assert_eq!(sig(wgsl(&genome(), &b).unwrap()), "a4fdfcc0cf60b4dd");
        assert_eq!(sig(layers_wgsl(&layered, &b).unwrap().unwrap()), "f1291d26fa6fffb5");
        assert_eq!(sig(picks), "bd4ab21b71a54d52");
    }

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
}
