//! Dev tool: describe a scene, the AI writes a full scene (.wgsl + .toml); it is compiled,
//! errors are fed back (3 tries), scored on the given audio, then saved to scenes/.
use crate::scenes::{load_all, prelude_path};
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::path::Path;

const RULES: &str = "Rules:
- The scene file defines `fn scene(uv: vec2<f32>) -> vec4<f32>` plus any helpers; prefix helper names with a short scene tag. Never redeclare anything from the prelude.
- React to the kick and to drops (kick(), drop_env(), since_kick(), since_drop()), plus slow energy()/tension() shaping. Do not wire every band to something.
- P() never multiplies time(): the phase would jump when the param changes. Keep time-based phases in constants.
- Soft luminance cap: fade the feedback and compress bright values so the frame never floods to white; keep a mostly dark background with contrasty detail that moves.
- At most 8 params, each with a sensible min/max. Mark `fixed = true` on integer params (floor/round in the shader), params that multiply time() and hue offsets.
- Palettes: 4 sRGB hex colours each, the first one dark. Give 3-4 palettes.
- The .toml has the same shape as the example (name, palettes, [[param]] with name/min/max/optional fixed).";

/// Returns the new scene id (saved as `scenes_dir/<id>.wgsl` + `.toml`).
pub fn run(c: &crate::llm::AiConfig, judge: &mut crate::score::Judge, scenes_dir: &Path, analysis: &crate::types::Analysis, prompt: &str) -> Result<String> {
    let read = |f: &str| std::fs::read_to_string(scenes_dir.join(f)).with_context(|| format!("reading {f}"));
    let system = format!(
        "You write scenes for a feedback-shader music visualiser (WGSL, wgpu). Each scene is <id>.wgsl + <id>.toml.\n\n\
         The prelude below is prepended to every scene file:\n```wgsl\n{}\n```\n\n\
         Worked example, liquid_tunnel.wgsl:\n```wgsl\n{}\n```\nliquid_tunnel.toml:\n```toml\n{}\n```\n\n{RULES}",
        read("_prelude.wgsl")?,
        read("liquid_tunnel.wgsl")?,
        read("liquid_tunnel.toml")?,
    );
    let schema = json!({
        "type": "object",
        "properties": {
            "id": {"type": "string", "pattern": "^[a-z][a-z0-9_]{2,30}$", "description": "new scene id (file name), snake_case"},
            "wgsl": {"type": "string", "description": "the full scene .wgsl file"},
            "toml": {"type": "string", "description": "the full scene .toml file"},
        },
        "required": ["id", "wgsl", "toml"],
    });
    let tmp = std::env::temp_dir().join("visualiser_author");
    let user = format!("Write a new scene: {prompt}");
    let id = crate::llm::attempts(c, &system, &user, "submit_scene", "Submit the finished scene.", &schema, 8000, "writing a scene", |v| {
        let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
        let id = s("id");
        try_scene(judge, scenes_dir, analysis, &tmp, &id, &s("wgsl"), &s("toml"))?;
        Ok(id)
    })?;
    for ext in ["wgsl", "toml"] {
        let f = format!("{id}.{ext}");
        std::fs::copy(tmp.join(&f), scenes_dir.join(&f)).with_context(|| format!("saving {f}"))?;
    }
    Ok(id)
}

/// Writes the scene to `tmp`, parses, compiles and scores it. Retrying with the same id recompiles:
/// the judge forgets each scene after scoring it.
fn try_scene(judge: &mut crate::score::Judge, scenes_dir: &Path, analysis: &crate::types::Analysis, tmp: &Path, id: &str, wgsl: &str, toml: &str) -> Result<()> {
    if !valid_id(id, scenes_dir) {
        bail!("id '{id}' must match ^[a-z][a-z0-9_]{{2,30}}$ and not be an existing scene");
    }
    let _ = std::fs::remove_dir_all(tmp);
    std::fs::create_dir_all(tmp)?;
    std::fs::copy(prelude_path(scenes_dir), prelude_path(tmp))?;
    std::fs::write(tmp.join(format!("{id}.wgsl")), wgsl)?;
    std::fs::write(tmp.join(format!("{id}.toml")), toml)?;
    let scene = load_all(tmp)?.remove(0);
    judge.audition(scene, analysis)
}

/// Matches ^[a-z][a-z0-9_]{2,30}$ and isn't taken in `scenes_dir`.
fn valid_id(id: &str, scenes_dir: &Path) -> bool {
    let b = id.as_bytes();
    (3..=31).contains(&b.len())
        && b[0].is_ascii_lowercase()
        && b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
        && !scenes_dir.join(format!("{id}.wgsl")).exists()
        && !scenes_dir.join(format!("{id}.toml")).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        let dir = Path::new("scenes");
        assert!(valid_id("aurora_curtains2", dir));
        for bad in ["ab", "2fast", "Aurora", "has-dash", "_prelude", &"x".repeat(32)] {
            assert!(!valid_id(bad, dir), "{bad}");
        }
        assert!(!valid_id("liquid_tunnel", dir));
    }
}
