//! Scene library: scenes/<id>.wgsl + scenes/<id>.toml, loaded at start-up.
//!
//! scene.toml:
//!   palettes = [["#0b0221", "#6a00f4", "#f72585", "#4cc9f0"], ...]   # sRGB hex, 4 each
//!   [[param]]
//!   name = "zoom"   # label only (ignored, like the scene's own `name`)
//!   min = 1.002
//!   max = 1.02

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

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
    pub min: f32,
    pub max: f32,
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

/// Full shader source for a scene: render/common.wgsl + prelude + scene file (re-read from disk each call, for hot reload).
pub fn shader_source(dir: &Path, scene: &Scene) -> Result<String> {
    let prelude = std::fs::read_to_string(prelude_path(dir)).context("reading _prelude.wgsl")?;
    let body = std::fs::read_to_string(&scene.wgsl_path)
        .with_context(|| format!("reading {}", scene.wgsl_path.display()))?;
    Ok(format!("{}{prelude}\n{body}", include_str!("render/common.wgsl")))
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
        if t.params.len() > 8 {
            bail!("{id}: at most 8 params");
        }
        if t.palettes.is_empty() {
            bail!("{id}: needs at least one palette");
        }
        let palettes = t
            .palettes
            .iter()
            .map(|p| -> Result<[[f32; 3]; 4]> {
                Ok([hex(&p[0])?, hex(&p[1])?, hex(&p[2])?, hex(&p[3])?])
            })
            .collect::<Result<_>>()
            .with_context(|| format!("{id}: palette"))?;
        scenes.push(Scene { id, params: t.params, palettes, wgsl_path });
    }
    if scenes.is_empty() {
        bail!("no scenes found in {}", dir.display());
    }
    scenes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(scenes)
}

fn hex(s: &str) -> Result<[f32; 3]> {
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
