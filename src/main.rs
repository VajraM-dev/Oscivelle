mod analysis;
mod director;
mod evolve;
mod genome;
#[allow(dead_code)] mod llm;
mod markers;
mod preview;
mod render;
mod scenes;
mod score;
mod types;
mod wizard;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use std::cell::RefCell;
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
    /// Share of scenes that are freshly generated (0 = only the hand-made ones, 1 = all generated).
    #[arg(long, default_value_t = 0.5)]
    generated: f32,
    /// Copy segment N's generated scene into scenes/ as a hand-made one (keep a look you love).
    #[arg(long)]
    keep: Option<usize>,
}

fn main() {
    let argc = std::env::args().len();
    // No arguments = double-clicked / Start Menu (guided mode); one = a file dragged onto the exe
    // or the right-click menu. Keep the console open in both cases so the result can be read.
    // Scripts / redirected stdin get neither the questions nor the pause.
    let tty = std::io::IsTerminal::is_terminal(&std::io::stdin());
    let pause = tty && argc <= 2;
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
    let audio = std::path::absolute(cli.audio.as_deref().ok_or_else(|| anyhow!("no audio file given (run with no arguments for guided mode)"))?)?;
    if !audio.is_file() {
        bail!("not found: {}", audio.display());
    }
    // Beside the exe (packaged zip), else ./scenes, else the source tree's (running a dev build from anywhere).
    let scenes_dir = cli.scenes.clone().unwrap_or_else(|| {
        [beside_exe("scenes"), Some("scenes".into()), Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("scenes"))]
            .into_iter()
            .flatten()
            .find(|p| p.is_dir())
            .unwrap_or_else(|| "scenes".into())
    });
    let mut scenes = scenes::load_all(&scenes_dir)?;
    if !(0.0..=1.0).contains(&cli.generated) {
        bail!("--generated must be between 0 and 1");
    }

    let analysis = load_or_analyse(&audio, cli.reanalyse)?;

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

    let plan_path = sibling(&audio, "plan.json");
    let previous: Option<Plan> = std::fs::read_to_string(&plan_path).ok().and_then(|s| serde_json::from_str(&s).ok());
    let reuse = !cli.replan
        && !cli.reanalyse
        && cli.reroll.is_empty()
        && cli.markers.is_none()
        && cli.scene.is_none()
        && previous.as_ref().is_some_and(|p| p.seed == cli.seed && p.mode == mode && p.generated == cli.generated);
    // Blocks only when something is (or was) generated.
    let gen_dir = sibling(&audio, "gen");
    let blocks = if cli.generated > 0.0 || previous.as_ref().is_some_and(|p| p.segments.iter().any(|s| s.genome.is_some())) {
        Some(genome::Blocks::load(&scenes_dir.join("blocks"))?)
    } else {
        None
    };
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
            // The judge (a small headless renderer) only exists while planning with generation on.
            let judge = match &blocks {
                Some(_) if cli.generated > 0.0 => Some(RefCell::new(score::Judge::new(&scenes, &scenes_dir, cli.cpu)?)),
                _ => None,
            };
            let budget = if cli.quality == Quality::Draft { 10 } else { 20 };
            let evolved = |start: f64, end: f64, r| {
                let (judge, blocks) = (judge.as_ref()?, blocks.as_ref()?);
                eprintln!("generating a scene for {}..{}", hms(start as u64, false), hms(end as u64, false));
                evolve::best(&mut judge.borrow_mut(), blocks, &gen_dir, &analysis, start, end, r, &[], budget)
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
                generated: cli.generated,
                genome_for: Some(&evolved),
                blocks: blocks.as_ref(),
                gen_dir: &gen_dir,
            })?;
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

    if let Some(i) = cli.keep {
        let seg = plan.segments.get(i).ok_or_else(|| anyhow!("--keep {i}: the plan has {} segments", plan.segments.len()))?;
        let g = seg.genome.as_ref().ok_or_else(|| anyhow!("--keep {i}: segment {i} ({}) is not generated", seg.scene))?;
        let id = genome::export(g, blocks.as_ref().unwrap(), &scenes_dir)?;
        eprintln!("kept as scenes/{id}");
        return Ok(());
    }

    if let Some(text) = director::chapters(&plan) {
        let p = sibling(&audio, "chapters.txt");
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
        let click = sibling(&audio, "click.wav");
        analysis::write_click_track(&audio, &analysis, &click)?;
        eprintln!("wrote {}", click.display());
        render::render_video(&job(sibling(&audio, "debug.mp4"), Quality::Draft, 1280, 720, true))?;
    }
    render::render_video(&job(out.clone(), cli.quality, width, height, false))?;
    eprintln!("done: {}", out.display());
    Ok(())
}

fn load_or_analyse(audio: &Path, reanalyse: bool) -> Result<Analysis> {
    let path = sibling(audio, "analysis.json");
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

/// "<dir>/<stem>.<suffix>" next to the source audio.
fn sibling(audio: &Path, suffix: &str) -> PathBuf {
    let stem = audio.file_stem().unwrap_or_default().to_string_lossy();
    audio.with_file_name(format!("{stem}.{suffix}"))
}
