mod analysis;
mod author;
mod director;
mod evolve;
mod font;
mod genome;
mod llm;
mod markers;
mod plates;
mod preview;
mod regime;
mod render;
mod scenes;
mod score;
mod types;
mod vibe;
mod wizard;

use anyhow::{anyhow, bail, Context, Result};
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
    /// Motion graphics on generated scenes: a backdrop (sky, raymarched sea world, your --image), sometimes a HUD/grid/text overlay, and a new shot at each drop.
    #[arg(long)]
    layers: bool,
    /// A picture the looks can show (with --layers or --vibe); repeatable, up to 8.
    #[arg(long)]
    image: Vec<PathBuf>,
    /// Finishing look for the whole video: clean, vhs, crt or mono.
    #[arg(long, value_enum)]
    look: Option<Look>,
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
    let asked = || -> Result<Option<wizard::Answers>> {
        let mut a = wizard::ask(None)?;
        // Live preview, then a menu, until something renders or the user quits.
        while a.start.is_some() {
            println!("\x1b[2mPreview window keys: Space pause, R restart, D debug overlay, Esc close.\x1b[0m");
            // A child process: winit allows only one event loop per process, and the user may preview again.
            match std::process::Command::new(std::env::current_exe()?).args(wizard::to_args(&a)).status() {
                Ok(s) if s.success() => {}
                Ok(s) => eprintln!("preview failed ({s})"),
                Err(e) => eprintln!("preview failed: {e}"),
            }
            match wizard::after_preview(&a)? {
                Some(next) => a = next,
                None => return Ok(None),
            }
        }
        Ok(Some(a))
    };
    let answers = match asked() {
        // Esc / Ctrl-C: the user quit, not an error.
        Err(e) if matches!(e.downcast_ref(), Some(OperationCanceled | OperationInterrupted)) => return Ok(()),
        r => r?,
    };
    let Some(answers) = answers else { return Ok(()) };
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
    // A vibe or --layers only steers generated scenes, so they imply some.
    let generated = if (cli.vibe.is_some() || cli.layers) && generated == 0.0 { 1.0 } else { generated };
    // Absolute, so the saved plan still finds them from another working directory.
    let user_plates = cli
        .image
        .iter()
        .take(plates::MAX)
        .map(|p| std::fs::canonicalize(p).with_context(|| format!("--image {}", p.display())))
        .collect::<Result<Vec<PathBuf>>>()?;
    if cli.image.len() > plates::MAX {
        eprintln!("warning: only the first {} --image pictures are used", plates::MAX);
    }
    if !user_plates.is_empty() && !cli.layers && cli.vibe.is_none() {
        eprintln!("warning: --image is only shown with --layers or --vibe");
    }
    // The vibe's pictures (<data>/plates/) follow the --image ones in a saved plan.
    let ai_plates = data.join("plates");
    let reuse = !cli.replan
        && !cli.reanalyse
        && cli.reroll.is_empty()
        && cli.markers.is_none()
        && cli.scene.is_none()
        && previous.as_ref().is_some_and(|p| p.same_inputs(cli.seed, mode, generated, cli.vibe.as_deref(), 1))
        && previous.as_ref().is_some_and(|p| {
            p.layers == cli.layers && p.plates.starts_with(&user_plates) && p.plates[user_plates.len()..].iter().all(|q| q.starts_with(&ai_plates))
        });
    let gen_dir = data.join("gen");
    // The vibe runs before blocks load, so the blocks it writes are included. Its judge is reused by evolution.
    let (judge, design) = match &cli.vibe {
        Some(v) if !reuse && generated > 0.0 && cli.keep.is_none() => vibe_seeds(&cli, v, &user_plates, &scenes, &scenes_dir, &gen_dir, &analysis, &data),
        _ => (None, vibe::Design::default()),
    };
    let vibe::Design { genomes: seeds, plates: vibe_plates, shots: story } = design;
    // The vibe's plate list (--image files, then its pictures) replaces the user's when it made one.
    let plates = if vibe_plates.is_empty() { user_plates } else { vibe_plates };
    // A storyboard directs every segment, so none rolls a curated scene. Saved in the plan, so a plain re-run
    // (generated taken from the plan) still reuses it; the vibe only runs when not reusing, so reuse never sees this.
    let generated = if story.is_empty() { generated } else { 1.0 };
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
    let mut plan = match previous {
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
            let k = Cell::new(0usize);
            let budget = if cli.quality == Quality::Draft { 10 } else { 20 };
            let evolved = |start: f64, end: f64, r| {
                let blocks = blocks.as_ref().filter(|_| generated > 0.0)?;
                eprintln!("generating a scene for {}..{}", hms(start as u64, false), hms(end as u64, false));
                let judge = gen.get_or_init(|| score::Judge::new(&scenes, &scenes_dir, cli.cpu).map(RefCell::new)).as_ref().ok()?;
                // The take's seed shifts the rotation, so new takes start from other looks.
                let i = k.replace(k.get() + 1).wrapping_add(hash(cli.seed, 0x5EED) as usize) % seeds.len().max(1);
                let rotated: Vec<_> = seeds[i..].iter().chain(&seeds[..i]).cloned().collect();
                evolve::best(&mut judge.borrow_mut(), sub.as_ref().unwrap_or(blocks), &gen_dir, &analysis, start, end, r, &rotated, budget)
            };
            let text = audio.file_stem().map(|s| s.to_string_lossy().replace(['_', '-'], " ")).unwrap_or_default();
            // --layers dresses the evolved look (unless it already has layers, e.g. from the vibe).
            let dressed = |start: f64, end: f64, r: u64| {
                let g = evolved(start, end, r)?;
                match blocks.as_ref() {
                    Some(b) if cli.layers && !g.has_layers() => Some(genome::dress(&g, b, hash(r, 7), plates.len(), &text)),
                    _ => Some(g),
                }
            };
            // --layers: a morph into each build-up and a flash on each drop.
            let shots: Vec<Shot> = if cli.layers {
                analysis
                    .drops
                    .iter()
                    .flat_map(|d| {
                        [
                            Shot { time: d.build_start, transition: Transition { kind: TransitionKind::Morph, duration: MORPH_LEN } },
                            Shot { time: d.time, transition: Transition { kind: TransitionKind::Flash, duration: 0.0 } },
                        ]
                    })
                    .collect()
            } else {
                Vec::new()
            };
            // A storyboard picks each shot's look as designed (no evolution): the AI is directing.
            // The shot nearest the segment start (the director snaps shots up to 2 bars and may drop some);
            // else the last one before it.
            let bar = director::bar_len(&analysis);
            let storyboard = |start: f64, _end: f64, _r: u64| {
                blocks.as_ref()?;
                let near = story.iter().filter(|(s, _)| (s.time - start).abs() <= 2.0 * bar).min_by(|x, y| (x.0.time - start).abs().total_cmp(&(y.0.time - start).abs()));
                let (_, look) = near.or_else(|| story.iter().rev().find(|(s, _)| s.time <= start)).or(story.first())?;
                seeds.get(*look).cloned()
            };
            let gf: &dyn Fn(f64, f64, u64) -> Option<genome::Genome> = if story.is_empty() { &dressed } else { &storyboard };
            let shots = if story.is_empty() { shots } else { story.iter().map(|(s, _)| *s).collect() };
            let mut plan = director::plan(&director::PlanRequest {
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
                genome_for: Some(gf),
                blocks: blocks.as_ref(),
                gen_dir: &gen_dir,
                shots: &shots,
                engine: 1,
            })?;
            plan.plates = plates.clone();
            plan.layers = cli.layers;
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
    // A render setting like --no-grain: set on every run, never reused.
    plan.post = cli.look.map(Look::post);

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
        let used: Vec<String> = plan
            .segments
            .iter()
            .filter_map(|s| s.genome.as_ref())
            .map(|g| g.id().to_string())
            .collect();
        for e in std::fs::read_dir(&gen_dir).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            // `<id>.wgsl` and `<id>.layers.wgsl` both belong to genome `<id>`.
            if name.ends_with(".wgsl") && !used.iter().any(|id| name.split('.').next() == Some(id.as_str())) {
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

/// The vibe's design (starting genomes, plates, storyboard), plus the judge (a small headless renderer) it made, for evolution.
/// AI trouble never fails a render: warn and evolve without seeds.
#[allow(clippy::too_many_arguments)]
fn vibe_seeds(cli: &Cli, v: &str, images: &[PathBuf], scenes: &[scenes::Scene], scenes_dir: &Path, gen_dir: &Path, analysis: &Analysis, data: &Path) -> (Option<score::Judge>, vibe::Design) {
    let mut judge = match score::Judge::new(scenes, scenes_dir, cli.cpu) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("warning: vibe needs the scene judge ({e:#}); continuing without it");
            return (None, vibe::Design::default());
        }
    };
    let Some(c) = llm::load() else {
        eprintln!("warning: no AI service set up (run visualiser --setup-ai); continuing without the vibe");
        return (Some(judge), vibe::Design::default());
    };
    let dir = scenes_dir.join("blocks");
    let design = genome::Blocks::load(&dir)
        .and_then(|b| vibe::run(&c, &mut judge, &dir, gen_dir, &b, analysis, v, images, &data.join("vibe.json")))
        .unwrap_or_else(|e| {
            eprintln!("warning: vibe failed ({e:#}); continuing without it");
            vibe::Design::default()
        });
    (Some(judge), design)
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
