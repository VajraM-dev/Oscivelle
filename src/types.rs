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

/// splitmix64 finaliser.
pub fn mix64(z: u64) -> u64 {
    let z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Stable 64-bit hash of two values (per-segment / per-candidate seeds).
pub fn hash(a: u64, b: u64) -> u64 {
    mix64(a ^ mix64(b))
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
    /// Plates (pictures) the plan's looks can show: --image files, then AI-made ones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plates: Vec<PathBuf>,
    /// --look override for the whole video (set on every run; None = each look's own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<Post>,
    /// Made with --layers.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub layers: bool,
}

impl Plan {
    /// Made with these settings (its unlocked segments still apply).
    pub fn same_inputs(&self, seed: u64, mode: Mode, generated: f32, vibe: Option<&str>) -> bool {
        self.seed == seed && self.mode == mode && self.generated == generated && self.vibe.as_deref() == vibe
    }
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
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

/// Plate (picture) size: every image is centre-cropped and fitted to this (3:2), RGBA8 sRGB.
pub const PLATE_W: u32 = 1536;
pub const PLATE_H: u32 = 1024;
/// Scene param slots (SceneUniforms p0..p3). Hand-made scenes use ≤ 8 in practice; layered genomes 14.
pub const MAX_PARAMS: usize = 16;

/// Finishing look applied by the finish pass. All zero = the clean default (output unchanged).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct Post {
    /// Tape wobble, chroma bleed and a rolling tracking band, 0..1.
    #[serde(default)]
    pub vhs: f32,
    /// Scanline darkening, 0..1.
    #[serde(default)]
    pub scanlines: f32,
    /// Colour banding, 0 off .. 1 = 4 levels per channel.
    #[serde(default)]
    pub posterize: f32,
    /// Monochrome tint strength towards `tint`, 0..1.
    #[serde(default)]
    pub tint_amount: f32,
    /// Linear RGB.
    #[serde(default)]
    pub tint: [f32; 3],
}

impl Post {
    pub fn is_clean(&self) -> bool {
        *self == Post::default()
    }

    pub fn lerp(&self, o: &Post, f: f32) -> Post {
        let l = |a: f32, b: f32| a + (b - a) * f;
        Post {
            vhs: l(self.vhs, o.vhs),
            scanlines: l(self.scanlines, o.scanlines),
            posterize: l(self.posterize, o.posterize),
            tint_amount: l(self.tint_amount, o.tint_amount),
            tint: [0, 1, 2].map(|k| l(self.tint[k], o.tint[k])),
        }
    }
}

/// `--look` presets.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Look {
    Clean,
    Vhs,
    Crt,
    Mono,
}

impl Look {
    pub fn post(self) -> Post {
        match self {
            Look::Clean => Post::default(),
            Look::Vhs => Post { vhs: 0.7, scanlines: 0.25, ..Default::default() },
            Look::Crt => Post { scanlines: 0.7, vhs: 0.15, ..Default::default() },
            Look::Mono => Post { tint_amount: 0.85, tint: [0.15, 1.0, 0.25], posterize: 0.5, scanlines: 0.3, ..Default::default() },
        }
    }
}

/// A requested scene change inside the music (storyboard or --layers); the director snaps it to a phrase.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub time: f64,
    pub transition: Transition,
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
    /// Scene params 8..11 (backdrop block), 12..15 (overlay block); unused = 0.
    pub p2: [f32; 4],
    pub p3: [f32; 4],
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
    /// Finishing look (finish pass).
    pub post: Post,
}
