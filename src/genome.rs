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
    /// Engine-2 recipe (genome::TEMPLATES; "" = the feedback recipe), the second emitter of a "dual" look,
    /// and the display-only comp block (kind comp; "" = none). Empty = left out of the JSON, so saved ids hold.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub template: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub emitter_b: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comp: String,
    /// Engine-2 visual grammar (genome::GRAMMARS; "" = unconstrained): which blocks the look and its phrase
    /// variants pick from (BLOCK_TAGS). Empty = left out of the JSON, so saved ids hold.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub grammar: String,
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
            5 => &mut self.space,
            _ => &mut self.comp,
        })
    }

    /// Uses the engine-2 template (has any engine-2 gene).
    pub fn is_v2(&self) -> bool {
        !self.space.is_empty() || !self.blend.is_empty() || !self.regime.is_empty() || !self.template.is_empty()
    }

    /// Has a backdrop or overlay (so it also has a layer pass).
    pub fn has_layers(&self) -> bool {
        !self.backdrop.is_empty() || !self.overlay.is_empty()
    }

    /// Keeps only finite tuned values whose key names a param of this genome's blocks.
    pub fn prune(&mut self, blocks: &Blocks) {
        let (Ok(parts), Ok(layers), Ok(space), Ok(eb), Ok(comp)) =
            (blocks.parts(self), blocks.layer_parts(self), blocks.space_part(self), blocks.emitter_b_part(self), blocks.comp_part(self))
        else {
            return self.params.clear();
        };
        let keys: Vec<String> = parts
            .into_iter()
            .chain(layers.into_iter().flatten())
            .chain(space)
            .chain(eb)
            .chain(comp)
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
    /// Engine-2 display-only framings (optional kind).
    pub comp: Vec<Block>,
    /// Palette bank, sRGB hex.
    pub palettes: Vec<[String; 4]>,
}

#[derive(Deserialize)]
struct BlockToml {
    kind: String,
    #[serde(default, rename = "param")]
    params: Vec<ParamRange>,
}

/// Block kinds; `SLOTS` and the `Blocks` lists follow this order. The last four (layers, space, comp) are optional.
pub const KINDS: [&str; 7] = ["motion", "emitter", "colour", "backdrop", "overlay", "space", "comp"];

/// Param slots per kind: 8 for the feedback (scene params 0..7), then 4 + 2 for the layers (8..13);
/// space (engine 2) uses slots 14..15; comp (engine 2, display only) has no params.
const SLOTS: [usize; 7] = [3, 3, 2, 4, 2, 2, 0];

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
    "comp_echo", "comp_kaleido", "comp_mirror", "comp_slitscan",
    "motion_drift", "motion_shear", "motion_sink", "motion_still", "motion_stutter", "space_bands", "space_pixel", "space_tile",
    "emit_halftone", "emit_scan", "emit_scatter", "emit_steps", "emit_wave",
];

impl Blocks {
    /// Loads scenes/blocks/*.toml (+ matching .wgsl) and palettes.toml.
    pub fn load(dir: &Path) -> Result<Blocks> {
        #[derive(Deserialize)]
        struct Pal {
            palettes: Vec<[String; 4]>,
        }
        let pal: Pal = toml::from_str(&std::fs::read_to_string(dir.join("palettes.toml")).context("blocks/palettes.toml")?)?;
        let mut b = Blocks { motion: vec![], emitter: vec![], colour: vec![], backdrop: vec![], overlay: vec![], space: vec![], comp: vec![], palettes: pal.palettes };
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

    /// The genome's comp block ("" = none), or an error naming an unknown id.
    pub fn comp_part(&self, g: &Genome) -> Result<Option<&Block>> {
        Self::opt(&self.comp, &g.comp)
    }

    /// The second emitter of a "dual" look ("" = none), or an error naming an unknown id.
    pub fn emitter_b_part(&self, g: &Genome) -> Result<Option<&Block>> {
        Self::opt(&self.emitter, &g.emitter_b)
    }

    /// The blocks of `kind`.
    pub fn list(&self, kind: &str) -> Result<&Vec<Block>> {
        Ok([&self.motion, &self.emitter, &self.colour, &self.backdrop, &self.overlay, &self.space, &self.comp][kind_index(kind)?])
    }

    fn list_mut(&mut self, kind: &str) -> Result<&mut Vec<Block>> {
        Ok(match kind_index(kind)? {
            0 => &mut self.motion,
            1 => &mut self.emitter,
            2 => &mut self.colour,
            3 => &mut self.backdrop,
            4 => &mut self.overlay,
            5 => &mut self.space,
            _ => &mut self.comp,
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
            comp: pick(&self.comp, |g| &g.comp),
            palettes,
        }
    }

    /// The frozen library of `engine` (1 or 2): only its block ids, in load (sorted) order, and its palettes.
    pub fn pinned(&self, engine: u32) -> Blocks {
        let mut b = self.keep(&KINDS, |id| V1_BLOCKS.contains(&id) || (engine >= 2 && V2_BLOCKS.contains(&id)));
        b.palettes.truncate(V1_PALETTES);
        b
    }

    /// The blocks a look of `grammar` may use ("" = all): motion, emitter, space and comp blocks tagged with it
    /// in BLOCK_TAGS (untagged ones only under ""); colours, layers and palettes all.
    pub fn grammar_pool(&self, grammar: &str) -> Blocks {
        if grammar.is_empty() {
            return self.clone();
        }
        self.keep(&["motion", "emitter", "space", "comp"], |id| BLOCK_TAGS.iter().any(|(t, gs, _)| *t == id && gs.split(' ').any(|g| g == grammar)))
    }

    /// A copy with only the `kinds` blocks whose id is `ok`.
    fn keep(&self, kinds: &[&str], ok: impl Fn(&str) -> bool) -> Blocks {
        let mut b = self.clone();
        for k in kinds {
            b.list_mut(k).expect("known kind").retain(|x| ok(&x.id));
        }
        // ponytail: a custom --scenes folder may hold none of the frozen (or tagged) ids; a required kind left
        // empty would panic in random()/nth_id (i % 0), so it keeps its whole list (unreproducible / off-grammar,
        // but runs).
        for k in ["motion", "emitter", "colour"] {
            if b.list(k).expect("known kind").is_empty() {
                *b.list_mut(k).expect("known kind") = self.list(k).expect("known kind").clone();
            }
        }
        b
    }

    /// The range of param `key` (see `param_key`).
    fn param(&self, key: &str) -> Option<&ParamRange> {
        let all = [&self.motion, &self.emitter, &self.colour, &self.backdrop, &self.overlay, &self.space, &self.comp].into_iter().flatten();
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
    var c = backdrop(p) + {FB} * {FEEDBACK};
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

/// Engine-2 blend modes of the emitter over the feedback; "" = add (the engine-1 recipe).
pub const BLENDS: [&str; 4] = ["screen", "difference", "multiply", "lighten"];

/// Engine-2 recipes besides feedback ("" in Genome::template): "stateless" draws the emitter fresh every frame
/// (no trails, crisp 3D); "dual" adds a second emitter (Genome::emitter_b) over the feedback.
pub const TEMPLATES: [&str; 2] = ["stateless", "dual"];

/// Engine-2 visual grammars, one per generated segment (drawn before any block, evolve::best_v2): radial
/// (the classic library: tunnels, folds, spins), flow (advected fields, no centre), grid (tiles, mosaics,
/// sequencers), scroll (everything slides one way) and still (the camera holds; only the emitter moves).
pub const GRAMMARS: [&str; 5] = ["radial", "flow", "grid", "scroll", "still"];

/// The visual grammar of the segment with `seed` (drawn before any block, evolve::best_v2).
pub fn grammar_for(seed: u64) -> &'static str {
    GRAMMARS[(crate::types::hash(seed, 0x6A4D) % GRAMMARS.len() as u64) as usize]
}

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
    ("motion_ink", "radial", "FZ"),
    ("motion_kaleido", "radial", "RZC"),
    ("motion_mosh", "radial grid", ""),
    ("motion_push", "radial", "RZT"),
    ("motion_shear", "grid scroll", ""),
    ("motion_sink", "flow", "FZ"),
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
}

/// Layer pass source (None when the genome has no layer blocks).
pub fn layers_wgsl(g: &Genome, blocks: &Blocks) -> Result<Option<String>> {
    if !g.has_layers() && g.comp.is_empty() {
        return Ok(None);
    }
    let [bd, ov] = blocks.layer_parts(g)?;
    let fb = g.feedback.unwrap_or(1.0).clamp(0.0, 1.5);
    // Engine 2: a comp block reframes the feedback for display only (this pass is never fed back). Without one
    // the source is byte-for-byte the old one.
    let comp = blocks.comp_part(g)?;
    let src = LAYERS
        .replace("{FB}", if comp.is_some() { "comp(uv)" } else { "prev(uv)" })
        .replace("{IMG}", &img_wgsl(g))
        .replace("{FONT}", &crate::font::wgsl(&g.text))
        .replace("{BACKDROP}", bd.map_or(NO_BACKDROP, |b| &b.wgsl))
        .replace("{OVERLAY}", ov.map_or(NO_OVERLAY, |b| &b.wgsl))
        .replace("{FEEDBACK}", &format!("{fb:?}"));
    Ok(Some(match comp {
        Some(c) => src + "\n" + &c.wgsl,
        None => src,
    }))
}

/// In-memory Scene for a genome: 8 param slots, 14 with layers or a dual emitter, 16 with a space block (each block's params padded to its slot count)
/// and the genome's palette. `wgsl_path` = <gen_dir>/<id>.wgsl (written by `write`).
/// A tuned value (`g.params`) narrows its range to value ± 15% of the width (fixed: exactly value).
pub fn to_scene(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let pad = ParamRange { name: String::new(), min: 0.0, max: 0.0, fixed: true };
    let [m, e, c] = blocks.parts(g)?;
    let [bd, ov] = blocks.layer_parts(g)?;
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
    // A dual look's second emitter uses the layer slots.
    if g.template == "dual" {
        return g.clone();
    }
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

/// Engine-2 random genome from `grammar`'s blocks (Blocks::grammar_pool): `random`'s blocks plus a space fold
/// a third of the time, a blend mode (add half the time), a cosine palette half the time, a recipe (stateless
/// or dual a sixth of the time each), a comp framing a third of the time, and the segment's `regime` with its
/// finishing look (mono tints with palette colour 2). `r` = 64-bit hash.
pub fn random_v2(blocks: &Blocks, r: u64, regime: &crate::regime::Regime, grammar: &str) -> Genome {
    let h = |k: u64| crate::types::hash(r, k);
    // Only the grammar's blocks ("" = all).
    let blocks = &blocks.grammar_pool(grammar);
    let mut g = random(blocks, r);
    // A fold a third of the time (when the grammar has any).
    if !blocks.space.is_empty() && h(1) % 3 == 0 {
        g.space = nth_id(&blocks.space, h(2) as usize);
    }
    g.blend = pick_blend(h(3));
    if h(4) % 2 == 0 {
        g.palette = cosine_palette(h(5));
    }
    // Recipe: stateless a sixth of the time, dual (a second, different emitter) a sixth, else feedback.
    match h(6) % 6 {
        0 => g.template = TEMPLATES[0].into(),
        1 if blocks.emitter.len() > 1 => {
            let others: Vec<&Block> = blocks.emitter.iter().filter(|b| b.id != g.emitter).collect();
            g.template = TEMPLATES[1].into();
            g.emitter_b = others[(h(7) % others.len() as u64) as usize].id.clone();
        }
        _ => {}
    }
    // A display-only comp framing a third of the time.
    if !blocks.comp.is_empty() && h(8) % 3 == 0 {
        g.comp = nth_id(&blocks.comp, h(9) as usize);
    }
    g.regime = regime.name.into();
    g.grammar = grammar.into();
    g.post = regime.post;
    retint(&mut g);
    g
}

/// A mono finishing look tints with palette colour 2.
fn retint(g: &mut Genome) {
    if g.post.tint_amount > 0.0 {
        g.post.tint = crate::scenes::hex(&g.palette[2]).unwrap_or([1.0; 3]);
    }
}

/// End of a change to `g`: retint if the palette changed, then prune.
fn settle(g: &Genome, mut out: Genome, blocks: &Blocks) -> Genome {
    if out.palette != g.palette {
        retint(&mut out);
    }
    // A dual look's two emitters must differ (same id = duplicate helper defs): keep the parent's.
    if out.template == "dual" && out.emitter == out.emitter_b {
        out.emitter = g.emitter.clone();
    }
    out.prune(blocks);
    out
}

/// Engine-2 mutation: a third of the time swap the space fold (or drop it), a third the blend mode, else
/// `mutate` (blocks / palette / params), from the genome's grammar's blocks. The regime and grammar never
/// change: they belong to the segment.
pub fn mutate_v2(g: &Genome, blocks: &Blocks, r: u64) -> Genome {
    let h = |k: u64| crate::types::hash(r, k);
    let blocks = &blocks.grammar_pool(&g.grammar);
    let out = match h(1) % 3 {
        0 if !blocks.space.is_empty() => Genome { space: if h(2) % 4 == 0 { String::new() } else { nth_id(&blocks.space, h(3) as usize) }, ..g.clone() },
        1 => Genome { blend: pick_blend(h(4)), ..g.clone() },
        _ => mutate(g, blocks, r),
    };
    settle(g, out, blocks)
}

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
    match h(1) % 3 {
        0 => if let Some(s) = other(&pool.space, &g.space, 2) { out.space = s } else { out.palette = cosine_palette(h(4)) },
        1 => if let Some(c) = other(&pool.comp, &g.comp, 3) { out.comp = c } else { out.palette = cosine_palette(h(4)) },
        _ => out.palette = cosine_palette(h(4)),
    }
    settle(g, out, blocks)
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
    fn emitters_survive_the_dual_rename() {
        // dual renames `fn emit(` -> `fn emit_b(` and `EP(` -> `EP2(` by plain text replace.
        for b in &blocks().emitter {
            assert_eq!(b.wgsl.matches("fn emit(").count(), 1, "{}: one fn emit(", b.id);
            for (k, _) in b.wgsl.match_indices("EP(") {
                let prev = b.wgsl[..k].chars().next_back();
                assert!(!prev.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_'), "{}: identifier ending in EP(", b.id);
            }
        }
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
        assert!(v1.comp.is_empty());
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
        let gs: Vec<Genome> = (0..96).map(|r| random_v2(&b, crate::types::hash(9, r), reg, "")).collect();
        assert_eq!(gs[3], random_v2(&b, crate::types::hash(9, 3), reg, ""));
        let distinct = |f: &dyn Fn(&Genome) -> String| gs.iter().map(f).collect::<std::collections::HashSet<_>>().len();
        assert_eq!(distinct(&|g| g.space.clone()), 4, "none + 3 folds");
        assert_eq!(distinct(&|g| g.blend.clone()), 5, "add + 4 modes");
        assert!(distinct(&|g| g.palette.join("")) > 30, "bank + cosine palettes");
        for (i, g) in gs.iter().enumerate() {
            assert!(crate::scenes::palette(&g.palette).is_ok());
            assert_eq!(mutate_v2(g, &b, crate::types::hash(4, i as u64)).regime, reg.name);
        }
        let mono = random_v2(&b, 5, crate::regime::get("mono").unwrap(), "");
        assert_eq!(mono.post.tint, crate::scenes::hex(&mono.palette[2]).unwrap());
        for i in 0..32 {
            let m = mutate_v2(&mono, &b, crate::types::hash(6, i));
            assert_eq!(m.post.tint, crate::scenes::hex(&m.palette[2]).unwrap(), "tint follows the palette");
        }
    }

    #[test]
    fn pinned_keeps_a_kind_with_no_frozen_ids() {
        let mut b = blocks();
        b.motion.retain(|x| x.id == "motion_ink");
        b.motion[0].id = "motion_custom".into();
        let p = b.pinned(1);
        assert_eq!(p.motion.len(), 1);
        random(&p, 7);
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
        let contract = ["motion", "motion_decay", "emit", "colour", "space", "comp"];
        for blk in [&b.motion, &b.emitter, &b.space, &b.comp].into_iter().flatten().filter(|x| V2_BLOCKS.contains(&x.id.as_str())) {
            for line in blk.wgsl.lines().filter(|l| l.starts_with("fn ") || l.starts_with("const ")) {
                let name = line.split([' ', '(', ':']).nth(1).unwrap_or_default();
                assert!(contract.contains(&name) || name.starts_with(&blk.id), "{}: helper {name} needs the prefix {}", blk.id, blk.id);
            }
        }
    }

    /// Saved engine-2 plans must keep rendering the same: a phase-1 engine-2 look's sources and params
    /// (captured before phase 2).
    #[test]
    fn engine2_saved_looks_are_frozen() {
        let b = blocks().pinned(2);
        let sig = |s: String| format!("{:016x}", fnv1a(s.as_bytes()));
        let g = Genome { space: "space_kaleido".into(), blend: "screen".into(), regime: "glitch".into(), ..genome() };
        let layered = Genome { backdrop: "back_sky".into(), overlay: "over_hud".into(), plate: Some(1), feedback: Some(0.6), text: "HI".into(), ..g.clone() };
        assert_eq!(sig(wgsl(&g, &b).unwrap()), "c741eb434d028a27");
        assert_eq!(sig(layers_wgsl(&layered, &b).unwrap().unwrap()), "f1291d26fa6fffb5");
        assert_eq!(sig(format!("{:?}", to_scene(&layered, &b, Path::new(".")).unwrap().params)), "c4cc527d35ec0a1a");
    }

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
        let gs: Vec<Genome> = (0..96).map(|r| random_v2(&b, crate::types::hash(11, r), reg, "")).collect();
        let distinct = |f: &dyn Fn(&Genome) -> String| gs.iter().map(f).collect::<std::collections::HashSet<_>>().len();
        assert_eq!(distinct(&|g| g.template.clone()), 3, "feedback + stateless + dual");
        assert_eq!(distinct(&|g| g.comp.clone()), 3, "none + 2 comps");
        assert!(gs.iter().all(|g| (g.template == "dual") == !g.emitter_b.is_empty() && g.emitter_b != g.emitter));
        // Mutation keeps a dual look's emitters distinct.
        for d in gs.iter().filter(|g| g.template == "dual") {
            assert!((0..64).all(|r| { let m = mutate_v2(d, &b, crate::types::hash(5, r)); m.emitter != m.emitter_b }));
        }
    }

    /// The library as BLOCK_TAGS describes it (empty sources): the picking logic without the files.
    fn tagged_library() -> Blocks {
        let mut b = Blocks { motion: vec![], emitter: vec![], colour: vec![comp_block("colour_x", "")], backdrop: vec![], overlay: vec![], space: vec![], comp: vec![], palettes: vec![genome().palette] };
        for (id, _, _) in BLOCK_TAGS {
            let kind = match id.split('_').next().unwrap() {
                "emit" => "emitter",
                k => k,
            };
            b.add(kind, comp_block(id, "")).unwrap();
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
            let grammar = grammar_for(r);
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
}
