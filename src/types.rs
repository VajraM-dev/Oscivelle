//! Shared contract between analysis, director and renderer.
//! analysis.json = `Analysis`, plan.json = `Plan`. Renderer consumes `FrameState`.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Simulation steps per second (feedback dynamics are tuned to this).
pub const SIM_HZ: u32 = 60;
/// Output video frame rate; SIM_HZ / OUT_FPS sim steps per output frame.
pub const OUT_FPS: u32 = 30;

/// FNV-1a, stable across builds (used for plan seeds and render-resume hashes).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

/// "M:SS", or "H:MM:SS" when `long` (or the value is an hour or more).
pub fn hms(secs: u64, long: bool) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if long || h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m:02}:{s:02}") }
}

/// Value in sorted `xs` closest to `t`.
pub fn nearest(xs: &[f64], t: f64) -> Option<f64> {
    let i = xs.partition_point(|&x| x < t);
    [i.checked_sub(1), Some(i)].into_iter().flatten().filter_map(|j| xs.get(j).copied()).min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
}

/// `name` in the program's folder, if it exists.
pub fn beside_exe(name: &str) -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join(name)).filter(|p| p.exists())
}

// ---------------------------------------------------------------- analysis

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Analysis {
    pub version: u32,
    /// Source audio file name (not full path).
    pub source: String,
    /// Seconds.
    pub duration: f64,
    /// Global median tempo.
    pub bpm: f64,
    /// Beat times, seconds, ascending.
    pub beats: Vec<f64>,
    /// Downbeat times (bar starts), seconds, subset of `beats`.
    pub downbeats: Vec<f64>,
    /// Phrase boundaries, ascending. Every 8-bar boundary appears once, tagged with
    /// the largest of 32/16/8 it qualifies as.
    pub phrases: Vec<Phrase>,
    /// Detected kick drums only (not every onset).
    pub kicks: Vec<Kick>,
    /// Continuous curves sampled at `curves.rate` Hz.
    pub curves: Curves,
    pub drops: Vec<Drop>,
    /// Track boundaries in a mix (excludes t=0).
    pub track_changes: Vec<TrackChange>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Phrase {
    pub time: f64,
    /// 8, 16 or 32.
    pub bars: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Kick {
    pub time: f64,
    /// 0..1
    pub strength: f32,
}

/// All vectors have the same length; index i is time i / rate.
/// Every curve is smoothed and levelled per track to roughly 0..1.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Curves {
    pub rate: f64,
    pub sub: Vec<f32>,
    pub bass: Vec<f32>,
    pub mid: Vec<f32>,
    pub high: Vec<f32>,
    /// Overall intensity, levelled per track.
    pub energy: Vec<f32>,
    /// 0..1: rises across a build, falls to 0 exactly at the drop.
    pub tension: Vec<f32>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Drop {
    /// Snapped to a downbeat.
    pub time: f64,
    /// Start of the build leading into it.
    pub build_start: f64,
    /// 0..1
    pub strength: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Cut,
    Blend,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct TrackChange {
    /// Centre of the change, snapped to a downbeat.
    pub time: f64,
    /// 0..1
    pub confidence: f32,
    pub kind: ChangeKind,
    /// Seconds the blend spans (0 for a cut).
    pub blend_len: f64,
}

// ---------------------------------------------------------------- settings

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    Auto,
    Single,
    Markers,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Quality {
    Draft,
    Final,
}

// ---------------------------------------------------------------- plan

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Plan {
    pub version: u32,
    pub source: String,
    pub seed: u64,
    pub mode: Mode,
    /// Share of generated scenes the plan was made with (--generated).
    #[serde(default)]
    pub generated: f32,
    /// --vibe text the plan was made with.
    #[serde(default)]
    pub vibe: Option<String>,
    /// Scene changes (one per track/marker); contiguous, first starts at 0, last ends at duration.
    pub segments: Vec<Segment>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    /// Scene id = file stem in scenes/ (e.g. "liquid_tunnel").
    pub scene: String,
    /// Per-segment seed: derived from (plan seed, segment index) unless re-rolled.
    pub seed: u64,
    /// Locked segments are kept verbatim when re-planning.
    pub locked: bool,
    /// Index into the scene's palettes.
    pub palette: usize,
    /// "Artist - Title" when known from cue/marker file.
    pub title: Option<String>,
    pub transition_in: Transition,
    /// Phrase-level variations, ascending by time; the first one is at `start`.
    pub variations: Vec<Variation>,
    /// Audio routings for this segment: param += depth * (param range) * source, on top of the variations.
    #[serde(default)]
    pub mods: Vec<Mod>,
    /// Set when the scene is generated: `scene` is then `genome.id()` and its .wgsl is rebuilt from this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genome: Option<crate::genome::Genome>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Mod {
    /// Scene param index (scene.toml order).
    pub param: usize,
    pub source: ModSource,
    /// Fraction of the param's range at source = 1; negative inverts.
    pub depth: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModSource {
    Kick,
    Bass,
    Mid,
    High,
    Energy,
    Tension,
    Drop,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum TransitionKind {
    Cut,
    Flash,
    Morph,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Transition {
    pub kind: TransitionKind,
    /// Seconds (Morph only; centred on segment start).
    pub duration: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Variation {
    pub time: f64,
    /// Values for the scene's params (same order as scene.toml), inside their ranges.
    pub params: Vec<f32>,
    /// Palette hue rotation, turns (0..1).
    pub hue_shift: f32,
}

// ---------------------------------------------------------------- per frame

/// Uniform block bound at group(0) binding(0) of every scene shader.
/// Must match `struct U` in scenes/_prelude.wgsl exactly.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct SceneUniforms {
    /// x,y = render resolution px, z = time s, w = sim dt s
    pub res_time: [f32; 4],
    /// kick envelope 0..1, beat phase 0..1, bar phase 0..1, phrase phase 0..1
    pub beat: [f32; 4],
    /// sub, bass, mid, high (0..1)
    pub bands: [f32; 4],
    /// energy, tension, drop envelope (1 at drop, decays), segment seed fraction 0..1
    pub shape: [f32; 4],
    /// scene params 0..3, 4..7 (unused = 0)
    pub p0: [f32; 4],
    pub p1: [f32; 4],
    /// 4 palette colours, linear RGB, w = 1
    pub pal: [[f32; 4]; 4],
    /// x = hue shift (turns), y = quality (0 draft, 1 final),
    /// z = seconds since last kick, w = seconds since last drop (large if none yet)
    pub misc: [f32; 4],
}

/// Everything the renderer needs for one simulation step.
#[derive(Clone, Debug)]
pub struct FrameState {
    pub scene_a: String,
    pub a: SceneUniforms,
    /// During a Morph: the incoming scene; output = mix(a, b, mix).
    pub b: Option<(String, SceneUniforms)>,
    pub mix: f32,
    /// Finish-pass globals.
    /// Brightness flash 0..1 (kick flash + transition flash).
    pub flash: f32,
    /// Exposure multiplier (~1.0).
    pub exposure: f32,
    /// Saturation multiplier (drains with tension, ~0.6..1.2).
    pub saturation: f32,
}
