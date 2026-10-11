//! Scene library: scenes/<id>.wgsl + scenes/<id>.toml, loaded at start-up.
//!
//! scene.toml:
//!   palettes = [["#0b0221", "#6a00f4", "#f72585", "#4cc9f0"], ...]   # sRGB hex, 4 each
//!   [[param]]
//!   name = "zoom"   # label (blocks key tuned values by it; scenes ignore it)
//!   min = 1.002
//!   max = 1.02
//!   fixed = true   # optional: never audio-modulated

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The shipped scenes: the only ones the director picks on its own (pinned, like genome V1_BLOCKS).
/// --keep / --author scenes stay reachable through --scene and markers.
pub const BUILTIN: [&str; 7] = ["fractal_zoom", "ink_in_water", "kaleidoscope_bloom", "liquid_tunnel", "neon_geometry", "plasma_field", "starfield_warp"];

#[derive(Clone, Debug)]
pub struct Scene {
    /// File stem, e.g. "liquid_tunnel".
    pub id: String,
    pub params: Vec<ParamRange>,
    /// Linear RGB.
    pub palettes: Vec<[[f32; 3]; 4]>,
    pub wgsl_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ParamRange {
    /// "" when unnamed.
    #[serde(default)]
    pub name: String,
    pub min: f32,
    pub max: f32,
    /// Never audio-modulated: integer params (floor/round in the shader), params that multiply
    /// time() (the phase would jump) and hue offsets (hue flicker on kicks looks cheap).
    #[serde(default)]
    pub fixed: bool,
}

impl ParamRange {
    /// `v` moved by `amt` × the range width, kept inside the range (min may exceed max).
    pub fn nudge(&self, v: f32, amt: f32) -> f32 {
        let (lo, hi) = (self.min.min(self.max), self.min.max(self.max));
        (v + amt * (hi - lo)).clamp(lo, hi)
    }
}

#[derive(Deserialize)]
struct SceneToml {
    palettes: Vec<[String; 4]>,
    #[serde(default, rename = "param")]
    params: Vec<ParamRange>,
}

pub fn prelude_path(dir: &Path) -> PathBuf {
    dir.join("_prelude.wgsl")
}

/// `<id>.layers.wgsl` next to the scene: its layer pass (backdrop + feedback + overlay), run once per output frame.
pub fn layers_path(scene: &Scene) -> PathBuf {
    scene.wgsl_path.with_extension("layers.wgsl")
}

fn source(dir: &Path, path: &Path) -> Result<String> {
    let prelude = std::fs::read_to_string(prelude_path(dir)).context("reading _prelude.wgsl")?;
    let body = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(format!("{}{prelude}
{body}", include_str!("render/common.wgsl")))
}

/// Full shader source for a scene: render/common.wgsl + prelude + scene file (re-read from disk each call, for hot reload).
pub fn shader_source(dir: &Path, scene: &Scene) -> Result<String> {
    source(dir, &scene.wgsl_path)
}

/// Full source of the scene's layer pass; None when it has none.
pub fn layers_source(dir: &Path, scene: &Scene) -> Option<Result<String>> {
    let path = layers_path(scene);
    path.exists().then(|| source(dir, &path))
}

/// Loads every <id>.toml with a matching <id>.wgsl, sorted by id.
pub fn load_all(dir: &Path) -> Result<Vec<Scene>> {
    let mut scenes = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("scenes folder {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let wgsl_path = path.with_extension("wgsl");
        if !wgsl_path.exists() {
            continue;
        }
        let id = path.file_stem().unwrap().to_string_lossy().into_owned();
        let t: SceneToml = toml::from_str(&std::fs::read_to_string(&path)?)
            .with_context(|| format!("parsing {}", path.display()))?;
        if t.params.len() > crate::types::MAX_PARAMS {
            bail!("{id}: at most {} params", crate::types::MAX_PARAMS);
        }
        if t.palettes.is_empty() {
            bail!("{id}: needs at least one palette");
        }
        let palettes = t.palettes.iter().map(palette).collect::<Result<_>>().with_context(|| format!("{id}: palette"))?;
        scenes.push(Scene { id, params: t.params, palettes, wgsl_path });
    }
    if scenes.is_empty() {
        bail!("no scenes found in {}", dir.display());
    }
    scenes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(scenes)
}

/// sRGB hex palette to linear RGB.
pub fn palette(p: &[String; 4]) -> Result<[[f32; 3]; 4]> {
    Ok([hex(&p[0])?, hex(&p[1])?, hex(&p[2])?, hex(&p[3])?])
}

pub fn hex(s: &str) -> Result<[f32; 3]> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        bail!("bad colour {s}");
    }
    let v = u32::from_str_radix(s, 16)?;
    let lin = |c: u32| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    Ok([lin(v >> 16 & 255), lin(v >> 8 & 255), lin(v & 255)])
}
