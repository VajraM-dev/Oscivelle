mod analysis;
mod author;
mod director;
mod evolve;
mod font;
mod genome;
mod llm;
mod markers;
mod preview;
mod render;
mod scenes;
mod score;
mod types;
mod vibe;
mod wizard;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use std::cell::{Cell, OnceCell, RefCell};
use std::path::{Path, PathBuf};
use types::*;

/// Turns a long audio file into an MP4 with music-aware visuals.
/// Drag an audio file onto the program, or run with options.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// WAV, FLAC, MP3 or AIFF file. Leave out to be asked a few questions instead.
    audio: Option<PathBuf>,
    /// What changes the scene. Defaults to markers when --markers is given, else auto.
    #[arg(long, value_enum)]
    mode: Option<Mode>,
    /// CUE file, Audacity label file or plain text list of times.
    #[arg(long)]
    markers: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "final")]
    quality: Quality,
    /// Output height: 720, 1080 or 1440. Default 1080 (720 for draft).
    #[arg(long)]
    resolution: Option<u32>,
    /// Same file + settings + seed = same video.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Re-roll one segment (index from the plan file); repeatable.
    #[arg(long)]
    reroll: Vec<usize>,
    /// Render only a 30-second window starting here (seconds or mm:ss).
    #[arg(long)]
    preview: Option<String>,
    /// Turn off the track title captions.
    #[arg(long)]
    no_titles: bool,
    /// Turn off film grain (A/B test it after a YouTube upload).
    #[arg(long)]
    no_grain: bool,
    /// Also write a click track and an analysis overlay video.
    #[arg(long)]
    debug: bool,
    /// Force the software renderer (WARP), to benchmark no-GPU machines.
    #[arg(long)]
    cpu: bool,
    /// Ignore the saved analysis and analyse again.
    #[arg(long)]
    reanalyse: bool,
    /// Ignore the saved plan's unlocked segments and plan again.
    #[arg(long)]
    replan: bool,
    /// Stop after writing analysis + plan.
    #[arg(long)]
    plan_only: bool,
    /// Open the developer preview window (live, shaders reload on save) instead of rendering.
    #[arg(long)]
    dev: bool,
    /// Force one scene id (file stem in scenes/) for the whole file; with --dev or --preview to audition it. Not saved to the plan.
    #[arg(long)]
    scene: Option<String>,
    /// With --dev: start playback here (seconds or mm:ss).
    #[arg(long)]
    start: Option<String>,
    /// Scenes folder. Default: scenes/ next to the program, else ./scenes, else the source tree's scenes/.
    #[arg(long)]
    scenes: Option<PathBuf>,
    /// Share of scenes that are freshly generated (0 = only the hand-made ones, 1 = all generated). Default: the saved plan's value, else 0.5.
    #[arg(long)]
    generated: Option<f32>,
    /// Copy segment N's generated scene into scenes/ as a hand-made one (keep a look you love).
    #[arg(long)]
    keep: Option<usize>,
    /// Describe the visuals in your own words; the AI tunes, combines and, when needed, writes new shader blocks (needs an AI service: see --setup-ai).
    #[arg(long)]
    vibe: Option<String>,
    /// Set up an OpenAI-compatible AI service (URL, key, model) for the AI features.
    #[arg(long)]
    setup_ai: bool,
    /// Remove the stored AI settings.
    #[arg(long)]
    forget_ai: bool,
    /// Dev: have the AI write a new hand-editable scene from a description (needs --setup-ai), then audition it with --dev --scene <id>.
    #[arg(long)]
    author: Option<String>,
}

fn main() {
    let argc = std::env::args().len();
    // No arguments = double-clicked / Start Menu (guided mode); one = a file dragged onto the exe
    // or the right-click menu. Keep the console open in both cases so the result can be read.
    // Scripts / redirected stdin get neither the questions nor the pause.
    let tty = std::io::IsTerminal::is_terminal(&std::io::stdin());
    // --forget-ai runs hidden from the uninstaller: nobody to press Enter.
    let pause = tty && argc <= 2 && std::env::args().nth(1).as_deref() != Some("--forget-ai");
    let result = if argc == 1 && tty { guided() } else { run(Cli::parse()) };
    if let Err(e) = &result {
        eprintln!("error: {e:#}");
    }
    if pause {
        eprintln!("\nPress Enter to close.");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
    if result.is_err() {
        std::process::exit(1);
    }
}

fn guided() -> Result<()> {
    use inquire::InquireError::{OperationCanceled, OperationInterrupted};
    let answers = match wizard::ask() {
        // Esc / Ctrl-C: the user quit, not an error.
        Err(e) if matches!(e.downcast_ref(), Some(OperationCanceled | OperationInterrupted)) => return Ok(()),
        r => r?,
    };
    let args = wizard::to_args(&answers);
    println!("\nSame thing as a command, for next time:\n  {}\n", wizard::command_line(&args));
    run(Cli::try_parse_from(std::iter::once("visualiser".to_string()).chain(args))?)
}

fn run(cli: Cli) -> Result<()> {
    if cli.setup_ai {
        return llm::setup();
    }
    if cli.forget_ai {
        llm::forget()?;
        println!("AI settings removed.");
        return Ok(());
    }
    let audio = std::path::absolute(cli.audio.as_deref().ok_or_else(|| anyhow!("no audio file given (run with no arguments for guided mode)"))?)?;
    if !audio.is_file() {
        bail!("not found: {}", audio.display());
    }
    // Everything but the videos goes to the track's data folder, so the audio's folder stays clean.
    let data = data_dir(&audio)?;
    eprintln!("project data: {}", data.display());
    // Beside the exe (packaged zip), else ./scenes, else the source tree's (running a dev build from anywhere).
    let scenes_dir = cli.scenes.clone().unwrap_or_else(|| {
        [beside_exe("scenes"), Some("scenes".into()), Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("scenes"))]
            .into_iter()
            .flatten()
            .find(|p| p.is_dir())
            .unwrap_or_else(|| "scenes".into())
    });
    let mut scenes = scenes::load_all(&scenes_dir)?;
    if cli.generated.is_some_and(|g| !(0.0..=1.0).contains(&g)) {
        bail!("--generated must be between 0 and 1");
    }

    let analysis = load_or_analyse(&audio, &data, cli.reanalyse)?;

    if let Some(prompt) = &cli.author {
        let c = llm::load().ok_or_else(|| anyhow!("--author needs an AI service: run visualiser --setup-ai"))?;
        let mut judge = score::Judge::new(&scenes, &scenes_dir, cli.cpu)?;
        let id = author::run(&c, &mut judge, &scenes_dir, &analysis, prompt)?;
        println!("new scene: scenes/{id}.wgsl  →  try: visualiser {} --dev --scene {id}", audio.display());
        return Ok(());
    }

    let markers = match &cli.markers {
        Some(p) => markers::load(p)?,
        None => Vec::new(),
    };
    let mode = cli.mode.unwrap_or(if cli.markers.is_some() { Mode::Markers } else { Mode::Auto });
    if mode == Mode::Markers && markers.is_empty() {
        bail!("markers mode needs --markers <file>");
    }
    if let Some(id) = &cli.scene {
        if !scenes.iter().any(|s| &s.id == id) {
            bail!("unknown scene {id}");
        }
    }

    let plan_path = data.join("plan.json");
    let previous: Option<Plan> = std::fs::read_to_string(&plan_path).ok().and_then(|s| serde_json::from_str(&s).ok());
    // Unset = what the saved plan used (so a plain re-run reuses it), else 0.5.
    let generated = cli.generated.or(previous.as_ref().map(|p| p.generated)).unwrap_or(0.5);
    // A vibe only steers generated scenes, so it implies some.
    let generated = if cli.vibe.is_some() && generated == 0.0 { 1.0 } else { generated };
    let reuse = !cli.replan
        && !cli.reanalyse
        && cli.reroll.is_empty()
        && cli.markers.is_none()
        && cli.scene.is_none()
        && previous.as_ref().is_some_and(|p| p.same_inputs(cli.seed, mode, generated, cli.vibe.as_deref()));
    let gen_dir = data.join("gen");
    // The vibe runs before blocks load, so the blocks it writes are included. Its judge is reused by evolution.
    let (judge, seeds) = match &cli.vibe {
        Some(v) if !reuse && generated > 0.0 && cli.keep.is_none() => vibe_seeds(&cli, v, &scenes, &scenes_dir, &gen_dir, &analysis, &data),
        _ => (None, Vec::new()),
    };
    // Blocks only when something is (or was) generated.
    let blocks = if generated > 0.0 || previous.as_ref().is_some_and(|p| p.segments.iter().any(|s| s.genome.is_some())) {
        Some(genome::Blocks::load(&scenes_dir.join("blocks"))?)
    } else {
        None
    };
    // --keep acts on the saved plan as it is (the one the video was rendered from), never a new one.
    if let Some(i) = cli.keep {
        let plan = previous.as_ref().ok_or_else(|| anyhow!("--keep needs a saved plan: render once first"))?;
        let seg = plan.segments.get(i).ok_or_else(|| anyhow!("--keep {i}: the plan has {} segments", plan.segments.len()))?;
        let g = seg.genome.as_ref().ok_or_else(|| anyhow!("--keep {i}: segment {i} ({}) is not generated", seg.scene))?;
        let id = genome::export(g, blocks.as_ref().unwrap(), &scenes_dir)?;
        eprintln!("kept as scenes/{id}");
        return Ok(());
    }
    let plan = match previous {
        Some(p) if reuse => {
            eprintln!("using saved plan {}", plan_path.display());
            p
        }
        mut previous => {
            // --replan keeps only locked segments from the saved plan.
            if cli.replan {
                if let Some(p) = &mut previous {
                    p.segments.retain(|s| s.locked);
                }
            }
            // The judge, made on the first segment that is actually generated (unless the vibe made it).
            let gen = OnceCell::new();
            if let Some(j) = judge {
                let _ = gen.set(Ok(RefCell::new(j)));
            }
            // Evolution stays on-vibe: random picks and mutations only use the seeds' blocks.
            let sub = blocks.as_ref().filter(|_| !seeds.is_empty()).map(|b| b.subset(&seeds));
            // Each generated segment starts from a different seed.
            let k = Cell::new(0);
            let budget = if cli.quality == Quality::Draft { 10 } else { 20 };
            let evolved = |start: f64, end: f64, r| {
                let blocks = blocks.as_ref().filter(|_| generated > 0.0)?;
                eprintln!("generating a scene for {}..{}", hms(start as u64, false), hms(end as u64, false));
                let judge = gen.get_or_init(|| score::Judge::new(&scenes, &scenes_dir, cli.cpu).map(RefCell::new)).as_ref().ok()?;
                let i = k.replace(k.get() + 1) % seeds.len().max(1);
                let rotated: Vec<_> = seeds[i..].iter().chain(&seeds[..i]).cloned().collect();
                evolve::best(&mut judge.borrow_mut(), sub.as_ref().unwrap_or(blocks), &gen_dir, &analysis, start, end, r, &rotated, budget)
            };
            let plan = director::plan(&director::PlanRequest {
                analysis: &analysis,
                scenes: &scenes,
                mode,
                markers: &markers,
                seed: cli.seed,
                previous: previous.as_ref(),
                reroll: &cli.reroll,
                force_scene: cli.scene.as_deref(),
                generated,
                vibe: cli.vibe.as_deref(),
                genome_for: Some(&evolved),
                blocks: blocks.as_ref(),
                gen_dir: &gen_dir,
                shots: &[],
            })?;
            // No judge (e.g. no GPU): the plan above already fell back to hand-made scenes.
            if let Some(Err(e)) = gen.into_inner() {
                eprintln!("warning: can't generate scenes ({e:#}); using hand-made ones");
            }
            // The dev --scene plan is throwaway; don't overwrite the real one.
            if cli.scene.is_none() {
                std::fs::write(&plan_path, serde_json::to_string_pretty(&plan)?)?;
                eprintln!("wrote {}", plan_path.display());
            }
            plan
        }
    };

    // Generated scenes become ordinary scenes: write each genome's .wgsl once.
    for g in plan.segments.iter().filter_map(|s| s.genome.as_ref()) {
        if !scenes.iter().any(|s| s.id == g.id()) {
            let blocks = blocks.as_ref().ok_or_else(|| anyhow!("plan has generated scenes but no blocks were loaded"))?;
            scenes.push(genome::write(g, blocks, &gen_dir)?);
        }
    }
    // A new plan leaves losing candidates (and the old plan's scenes) in gen_dir: drop them.
    // Not for the throwaway --scene plan, which would delete the saved plan's scenes.
    if !reuse && cli.scene.is_none() {
        let used: Vec<String> = plan.segments.iter().filter_map(|s| Some(format!("{}.wgsl", s.genome.as_ref()?.id()))).collect();
        for e in std::fs::read_dir(&gen_dir).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".wgsl") && !used.contains(&name) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    if let Some(text) = director::chapters(&plan) {
        let p = data.join("chapters.txt");
        std::fs::write(&p, text)?;
        eprintln!("wrote {}", p.display());
    }

    if cli.dev {
        let start = match cli.start.as_deref() {
            Some(s) => markers::parse_time(s).ok_or_else(|| anyhow!("bad --start {s:?}"))?,
            None => 0.0,
        };
        return preview::run(&audio, &analysis, &plan, &scenes, &scenes_dir, cli.quality, start);
    }
    if cli.plan_only {
        return Ok(());
    }

    let height = cli.resolution.unwrap_or(if cli.quality == Quality::Draft { 720 } else { 1080 });
    if ![720, 1080, 1440].contains(&height) {
        bail!("resolution must be 720, 1080 or 1440");
    }
    let width = height * 16 / 9;

    let (start, end, out) = match &cli.preview {
        Some(s) => {
            let start = markers::parse_time(s).ok_or_else(|| anyhow!("bad --preview {s:?}"))?;
            if start >= analysis.duration - 1.0 {
                bail!("--preview start {start:.1}s is at or past the end of the audio ({:.1}s)", analysis.duration);
            }
            let end = (start + 30.0).min(analysis.duration);
            let tag = cli.scene.as_deref().map(|s| format!("{s}.")).unwrap_or_default();
            (start, end, sibling(&audio, &format!("preview-{tag}{}s.mp4", start as u64)))
        }
        None => (0.0, analysis.duration, audio.with_extension("mp4")),
    };

    let job = |out: PathBuf, quality, width, height, debug_overlay| render::RenderJob {
        audio: audio.clone(),
        parts: data.join(format!("{}.parts", out.file_stem().unwrap_or_default().to_string_lossy())),
        out,
        plan: &plan,
        analysis: &analysis,
        scenes: &scenes,
        scenes_dir: scenes_dir.clone(),
        quality,
        width,
        height,
        start,
        end,
        titles: !cli.no_titles,
        grain: !cli.no_grain,
        debug_overlay,
        cpu: cli.cpu,
    };

    if cli.debug {
        let click = data.join("click.wav");
        analysis::write_click_track(&audio, &analysis, &click)?;
        eprintln!("wrote {}", click.display());
        render::render_video(&job(sibling(&audio, "debug.mp4"), Quality::Draft, 1280, 720, true))?;
    }
    render::render_video(&job(out.clone(), cli.quality, width, height, false))?;
    eprintln!("done: {}", out.display());
    Ok(())
}

/// The vibe's starting genomes, plus the judge (a small headless renderer) it made, for evolution.
/// AI trouble never fails a render: warn and evolve without seeds.
fn vibe_seeds(cli: &Cli, v: &str, scenes: &[scenes::Scene], scenes_dir: &Path, gen_dir: &Path, analysis: &Analysis, data: &Path) -> (Option<score::Judge>, Vec<genome::Genome>) {
    let mut judge = match score::Judge::new(scenes, scenes_dir, cli.cpu) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("warning: vibe needs the scene judge ({e:#}); continuing without it");
            return (None, Vec::new());
        }
    };
    let Some(c) = llm::load() else {
        eprintln!("warning: no AI service set up (run visualiser --setup-ai); continuing without the vibe");
        return (Some(judge), Vec::new());
    };
    let dir = scenes_dir.join("blocks");
    let seeds = genome::Blocks::load(&dir)
        .and_then(|b| vibe::run(&c, &mut judge, &dir, gen_dir, &b, analysis, v, &data.join("vibe.json")))
        .unwrap_or_else(|e| {
            eprintln!("warning: vibe failed ({e:#}); continuing without it");
            Vec::new()
        });
    (Some(judge), seeds)
}

fn load_or_analyse(audio: &Path, data: &Path, reanalyse: bool) -> Result<Analysis> {
    let path = data.join("analysis.json");
    if !reanalyse {
        if let Some(a) = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()) {
            eprintln!("using saved analysis {}", path.display());
            return Ok(a);
        }
    }
    let a = analysis::analyze(audio)?;
    std::fs::write(&path, serde_json::to_string(&a)?)?;
    eprintln!(
        "analysis: {:.0} BPM, {} kicks, {} drops, {} track changes -> {}",
        a.bpm,
        a.kicks.len(),
        a.drops.len(),
        a.track_changes.len(),
        path.display()
    );
    Ok(a)
}

/// The track's data folder: %LOCALAPPDATA%\Visualiser\projects\<stem>-<hash of the full path>
/// (same-named tracks in different folders don't collide). Created if missing.
fn data_dir(audio: &Path) -> Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let stem = audio.file_stem().unwrap_or_default().to_string_lossy();
    // Windows paths are case-insensitive.
    let hash = fnv1a(audio.to_string_lossy().to_lowercase().as_bytes()) & 0xffff_ffff;
    let dir = base.join("Visualiser").join("projects").join(format!("{stem}-{hash:08x}"));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// "<dir>/<stem>.<suffix>" next to the source audio (videos only).
fn sibling(audio: &Path, suffix: &str) -> PathBuf {
    let stem = audio.file_stem().unwrap_or_default().to_string_lossy();
    audio.with_file_name(format!("{stem}.{suffix}"))
}
