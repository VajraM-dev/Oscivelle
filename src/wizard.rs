//! Guided mode: run with no arguments (double-click, Start Menu) → a few questions →
//! prints the equivalent command → runs it. The answers become ordinary CLI args, so
//! clap validates them exactly as if typed.
use crate::{llm, markers};
use crate::types::{Look, Mode, Quality};
use anyhow::{bail, Result};
use clap::ValueEnum;
use inquire::validator::Validation;
use inquire::{Confirm, CustomType, Select, Text};
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Answers {
    pub audio: PathBuf,
    pub markers: Option<PathBuf>,
    pub mode: Option<Mode>,
    pub generated: Option<f32>,
    pub vibe: Option<String>,
    pub quality: Option<Quality>,
    pub resolution: Option<u32>,
    pub preview: Option<String>,
    /// Live preview window from here (`--dev --start`).
    pub start: Option<String>,
    pub seed: Option<u64>,
    pub no_titles: bool,
    pub no_grain: bool,
    pub layers: bool,
    pub look: Option<Look>,
    pub images: Vec<PathBuf>,
    pub engine: Option<u32>,
}

fn val<T: ValueEnum>(t: &T) -> String {
    t.to_possible_value().expect("no skipped variants").get_name().to_string()
}

/// Program args (without argv[0]); only options that differ from the defaults.
pub fn to_args(a: &Answers) -> Vec<String> {
    let mut v = vec![a.audio.display().to_string()];
    if let Some(m) = &a.markers {
        v.extend(["--markers".into(), m.display().to_string()]);
    }
    if let Some(m) = &a.mode {
        v.extend(["--mode".into(), val(m)]);
    }
    if let Some(g) = a.generated {
        v.extend(["--generated".into(), g.to_string()]);
    }
    if let Some(t) = &a.vibe {
        v.extend(["--vibe".into(), t.clone()]);
    }
    if let Some(q) = &a.quality {
        v.extend(["--quality".into(), val(q)]);
    }
    if let Some(r) = a.resolution {
        v.extend(["--resolution".into(), r.to_string()]);
    }
    if let Some(p) = &a.preview {
        v.extend(["--preview".into(), p.clone()]);
    }
    if let Some(t) = &a.start {
        v.extend(["--dev".into(), "--start".into(), t.clone()]);
    }
    if let Some(s) = a.seed {
        v.extend(["--seed".into(), s.to_string()]);
    }
    if a.no_titles {
        v.push("--no-titles".into());
    }
    if a.no_grain {
        v.push("--no-grain".into());
    }
    if a.layers {
        v.push("--layers".into());
    }
    if let Some(l) = &a.look {
        v.extend(["--look".into(), val(l)]);
    }
    for i in &a.images {
        v.extend(["--image".into(), i.display().to_string()]);
    }
    if let Some(e) = a.engine {
        v.extend(["--engine".into(), e.to_string()]);
    }
    v
}

/// Copy-pasteable command line (args with spaces or shell metacharacters quoted).
pub fn command_line(args: &[String]) -> String {
    let plain = |a: &str| a.chars().all(|c| c.is_alphanumeric() || "-_.:/\\".contains(c));
    std::iter::once("visualiser".to_string())
        .chain(args.iter().map(|a| if plain(a) { a.clone() } else { format!("\"{a}\"") }))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The questions. `prev` (from "Change settings") pre-selects the earlier answers and keeps the audio file.
pub fn ask(prev: Option<&Answers>) -> Result<Answers> {
    let none = Answers::default();
    let d = prev.unwrap_or(&none);
    let audio = match prev {
        Some(p) if !Confirm::new("Use a different audio file?").with_default(false).prompt()? => p.audio.clone(),
        Some(_) => pick("Choose your audio file", AUDIO)?,
        None => {
            println!("Visualiser: answer a few questions and it makes the video. (Esc quits.)\n");
            pick("Choose your audio file", AUDIO)?
        }
    };
    // Seed, titles and grain carry over unless changed in the advanced options.
    let mut a = Answers { audio, seed: d.seed, no_titles: d.no_titles, no_grain: d.no_grain, ..Default::default() };

    if Confirm::new("Do you have a tracklist file (CUE, Audacity labels or a text list of times)?").with_default(d.markers.is_some()).prompt()? {
        a.markers = Some(match &d.markers {
            Some(m) if Confirm::new(&format!("Keep the tracklist {}?", m.display())).with_default(true).prompt()? => m.clone(),
            _ => pick("Choose the tracklist file", &["cue", "txt"])?,
        });
    } else {
        let modes = vec![
            "Auto: new scene at each track change (best for DJ mixes)",
            "Single: one scene that evolves with the music (best for one track)",
        ];
        let cur = usize::from(d.mode == Some(Mode::Single));
        if Select::new("How should the visuals change?", modes).with_starting_cursor(cur).raw_prompt()?.index == 1 {
            a.mode = Some(Mode::Single);
        }
    }

    // The engine is asked later, so the recommendation follows the earlier answer's: Classic (engine 1)
    // defaults to the mix, like `--generated` unset does.
    let classic = d.engine == Some(1);
    let rec = |yes: bool| if yes { " (recommended)" } else { "" };
    let styles = vec![
        format!("Freshly generated{}", rec(!classic)),
        format!("Mix of hand-made and freshly generated{}", rec(classic)),
        "Hand-made only".to_string(),
    ];
    let cur = match d.generated {
        Some(g) if g == 0.0 => 2,
        Some(g) if g < 1.0 => 1,
        Some(_) => 0,
        None => usize::from(classic),
    };
    a.generated = Some(match Select::new("Visual style?", styles).with_starting_cursor(cur).raw_prompt()?.index {
        1 => 0.5,
        2 => 0.0,
        _ => 1.0,
    });
    // Hand-made scenes are fixed: motion graphics, AI and pictures only apply to generated ones.
    if a.generated != Some(0.0) {
        a.layers = Confirm::new("Add motion graphics (backdrops, HUD, text, a new shot at each drop)?")
            .with_help_message("Yes = scenery behind the visuals (sky, 3D sea world, your pictures) + HUD/text on top. No = abstract visuals only.")
            .with_default(d.layers)
            .prompt()?;

        if let Some(ai) = llm::load() {
            let paints = if ai.image_model.is_empty() { "" } else { ", more if it paints pictures" };
            let who = vec![
                "Core engine only (free, works offline)".to_string(),
                format!("Let AI design it from a description (uses your AI credit: a few cents{paints})"),
            ];
            if Select::new("Who designs the visuals?", who).with_starting_cursor(usize::from(d.vibe.is_some())).raw_prompt()?.index == 1 {
                let t = Text::new("Describe the vibe in your own words:")
                    .with_initial_value(d.vibe.as_deref().unwrap_or(""))
                    .with_help_message("e.g. clouds and haze, molten gold, rainy neon city. New visuals are written when nothing fits.")
                    .with_validator(|s: &str| Ok(if s.trim().is_empty() { Validation::Invalid("type a few words, or Esc to quit".into()) } else { Validation::Valid }))
                    .prompt()?;
                a.vibe = Some(t.trim().to_string());
            }
        } else {
            println!("\x1b[2mTip: run \"visualiser --setup-ai\" to let AI design visuals from a description.\x1b[0m");
        }

        // The vibe evolves engine-1 looks, so the engine is only asked on the core path.
        if a.vibe.is_none() {
            let engines = vec!["Variety — new folds, moods, fractals, glitch (recommended)", "Classic — the original looks"];
            let i = Select::new("Visuals?", engines).with_starting_cursor(usize::from(d.engine == Some(1))).raw_prompt()?.index;
            a.engine = Some(if i == 1 { 1 } else { 2 });
        }

        if (a.layers || a.vibe.is_some())
            && Confirm::new("Show your own pictures in the video?")
                .with_help_message("Shown as backdrops (full-frame photo) or floating cut-outs (transparent PNG). Up to 8.")
                .with_default(!d.images.is_empty())
                .prompt()?
        {
            let keep = !d.images.is_empty() && Confirm::new(&format!("Keep the {} picture(s) chosen before?", d.images.len())).with_default(true).prompt()?;
            a.images = if keep { d.images.clone() } else { pick_many("Choose pictures", &["png", "jpg", "jpeg"])? };
        }
    }

    let looks = vec!["Clean (recommended)", "VHS tape", "CRT screen", "Mono (green hologram)"];
    let cur = match d.look {
        Some(Look::Vhs) => 1,
        Some(Look::Crt) => 2,
        Some(Look::Mono) => 3,
        _ => 0,
    };
    a.look = match Select::new("Finishing look?", looks).with_starting_cursor(cur).raw_prompt()?.index {
        1 => Some(Look::Vhs),
        2 => Some(Look::Crt),
        3 => Some(Look::Mono),
        _ => None,
    };

    let kinds = vec!["Live preview in a window (instant, nothing saved)", "Full video", "Quick 30-second test file (draft quality, fast)"];
    let cur = if d.preview.is_some() { 2 } else if prev.is_some() && d.start.is_none() { 1 } else { 0 };
    match Select::new("What do you want to make?", kinds).with_starting_cursor(cur).raw_prompt()?.index {
        1 => a.resolution = resolution(d.resolution)?,
        k => {
            let start = Text::new("Start at (mm:ss or seconds):")
                .with_default(d.start.as_deref().or(d.preview.as_deref()).unwrap_or("1:00"))
                .with_validator(|s: &str| {
                    Ok(if markers::parse_time(s).is_some() { Validation::Valid } else { Validation::Invalid("use mm:ss or seconds".into()) })
                })
                .prompt()?;
            if k == 0 {
                a.start = Some(start);
            } else {
                a.preview = Some(start);
            }
            a.quality = Some(Quality::Draft);
        }
    }

    // Titles come from the tracklist, so only offer them when there is one.
    let advanced = if a.markers.is_some() { "Change advanced options (seed, titles, film grain)?" } else { "Change advanced options (seed, film grain)?" };
    if Confirm::new(advanced).with_default(false).prompt()? {
        let seed = CustomType::<u64>::new("Seed (same seed = same video; another number = a new take):").with_default(d.seed.unwrap_or(1)).prompt()?;
        a.seed = (seed != 1).then_some(seed);
        if a.markers.is_some() {
            a.no_titles = !Confirm::new("Show track titles?").with_default(!d.no_titles).prompt()?;
        }
        a.no_grain = !Confirm::new("Add film grain?").with_default(!d.no_grain).prompt()?;
    }
    Ok(a)
}

const AUDIO: &[&str] = &["wav", "flac", "mp3", "aiff", "aif"];

fn resolution(d: Option<u32>) -> Result<Option<u32>> {
    let res = vec!["1080p (recommended)", "1440p (sharper, slower)", "720p (fastest)"];
    let cur = match d {
        Some(1440) => 1,
        Some(720) => 2,
        _ => 0,
    };
    Ok(match Select::new("Resolution:", res).with_starting_cursor(cur).raw_prompt()?.index {
        1 => Some(1440),
        2 => Some(720),
        _ => None,
    })
}

/// After a live preview: the next answers to run (another preview while `start` is set), or None to quit.
pub fn after_preview(a: &Answers) -> Result<Option<Answers>> {
    let next = vec![
        "Render the full video with these settings",
        "Render a 30-second test file",
        "Try another take (new seed)",
        "Change settings",
        "Quit",
    ];
    Ok(match Select::new("Preview closed. What next?", next).raw_prompt()?.index {
        // Same seed and inputs as the preview, so the render reuses its saved plan.
        0 => Some(Answers { start: None, quality: None, resolution: resolution(a.resolution)?, ..a.clone() }),
        1 => Some(Answers { start: None, preview: a.start.clone(), ..a.clone() }),
        2 => {
            let seed = a.seed.unwrap_or(1).wrapping_add(1);
            println!("New take: seed {seed}");
            Some(Answers { seed: Some(seed), ..a.clone() })
        }
        3 => Some(ask(Some(a))?),
        _ => None,
    })
}

/// Native file dialog; if it's cancelled or unavailable, type (or drag in) a path instead.
fn pick(title: &str, exts: &[&str]) -> Result<PathBuf> {
    if let Some(p) = rfd::FileDialog::new().set_title(title).add_filter("Supported files", exts).pick_file() {
        println!("> {title}: {}", p.display());
        return Ok(p);
    }
    let s = Text::new(&format!("{title}: type the path, or drag the file into this window:")).prompt()?;
    let p = PathBuf::from(s.trim().trim_matches('"'));
    if !p.is_file() {
        bail!("not found: {}", p.display());
    }
    Ok(p)
}

fn pick_many(title: &str, exts: &[&str]) -> Result<Vec<PathBuf>> {
    if let Some(ps) = rfd::FileDialog::new().set_title(title).add_filter("Pictures", exts).pick_files() {
        for p in &ps {
            println!("> {title}: {}", p.display());
        }
        return Ok(ps);
    }
    Ok(vec![pick(title, exts)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn answers_round_trip_through_clap() {
        let a = Answers {
            audio: "C:/My Music/mix.flac".into(),
            mode: Some(Mode::Single),
            generated: Some(1.0),
            vibe: Some("dark and liquid".into()),
            quality: Some(Quality::Draft),
            preview: Some("1:30".into()),
            seed: Some(7),
            no_grain: true,
            layers: true,
            look: Some(Look::Vhs),
            images: vec!["a.png".into()],
            engine: Some(1),
            ..Default::default()
        };
        let args = to_args(&a);
        assert_eq!(command_line(&args), "visualiser \"C:/My Music/mix.flac\" --mode single --generated 1 --vibe \"dark and liquid\" --quality draft --preview 1:30 --seed 7 --no-grain --layers --look vhs --image a.png --engine 1");
        let cli = crate::Cli::try_parse_from(std::iter::once("visualiser".to_string()).chain(args)).unwrap();
        assert_eq!(cli.audio.as_deref(), Some(std::path::Path::new("C:/My Music/mix.flac")));
        assert_eq!(cli.mode, Some(Mode::Single));
        assert_eq!(cli.quality, Quality::Draft);
        assert_eq!(cli.seed, 7);
        assert_eq!(cli.generated, Some(1.0));
        assert_eq!(cli.vibe.as_deref(), Some("dark and liquid"));
        assert!(cli.no_grain && !cli.no_titles);
        assert!(cli.layers && cli.look == Some(Look::Vhs));
        assert_eq!(cli.image, vec![PathBuf::from("a.png")]);
        assert_eq!(cli.engine, Some(1));

        let live = Answers { audio: "mix.flac".into(), start: Some("1:00".into()), quality: Some(Quality::Draft), ..Default::default() };
        let cli = crate::Cli::try_parse_from(std::iter::once("visualiser".to_string()).chain(to_args(&live))).unwrap();
        assert!(cli.dev && cli.start.as_deref() == Some("1:00") && cli.preview.is_none());
    }
}
