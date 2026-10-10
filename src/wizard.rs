//! Guided mode: run with no arguments (double-click, Start Menu) → a few questions →
//! prints the equivalent command → runs it. The answers become ordinary CLI args, so
//! clap validates them exactly as if typed.
use crate::{llm, markers};
use crate::types::{Mode, Quality};
use anyhow::{bail, Result};
use clap::ValueEnum;
use inquire::validator::Validation;
use inquire::{Confirm, CustomType, Select, Text};
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq)]
pub struct Answers {
    pub audio: PathBuf,
    pub markers: Option<PathBuf>,
    pub mode: Option<Mode>,
    pub generated: Option<f32>,
    pub vibe: Option<String>,
    pub quality: Option<Quality>,
    pub resolution: Option<u32>,
    pub preview: Option<String>,
    pub seed: Option<u64>,
    pub no_titles: bool,
    pub no_grain: bool,
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
    if let Some(s) = a.seed {
        v.extend(["--seed".into(), s.to_string()]);
    }
    if a.no_titles {
        v.push("--no-titles".into());
    }
    if a.no_grain {
        v.push("--no-grain".into());
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

pub fn ask() -> Result<Answers> {
    println!("Visualiser: answer a few questions and it makes the video. (Esc quits.)\n");
    let mut a = Answers { audio: pick("Choose your audio file", &["wav", "flac", "mp3", "aiff", "aif"])?, ..Default::default() };

    if Confirm::new("Do you have a tracklist file (CUE, Audacity labels or a text list of times)?").with_default(false).prompt()? {
        a.markers = Some(pick("Choose the tracklist file", &["cue", "txt"])?);
    } else {
        let modes = vec![
            "Auto: new scene at each track change (best for DJ mixes)",
            "Single: one scene that evolves with the music (best for one track)",
        ];
        if Select::new("How should the visuals change?", modes).raw_prompt()?.index == 1 {
            a.mode = Some(Mode::Single);
        }
    }

    let styles = vec![
        "Mix of hand-made and freshly generated (recommended)",
        "Hand-made only",
        "All freshly generated",
    ];
    a.generated = match Select::new("Visual style?", styles).raw_prompt()?.index {
        1 => Some(0.0),
        2 => Some(1.0),
        _ => None,
    };
    // Hand-made only has nothing for a vibe to steer.
    if a.generated != Some(0.0) {
        if llm::load().is_some() {
            let t = Text::new("Describe the vibe (optional, Enter to skip):").prompt()?;
            a.vibe = Some(t.trim().to_string()).filter(|t| !t.is_empty());
        } else {
            println!("\x1b[2mTip: run \"visualiser --setup-ai\" to describe visuals in words.\x1b[0m");
        }
    }

    let kinds = vec!["Full video", "Quick 30-second test (draft quality, fast)"];
    if Select::new("What do you want to make?", kinds).raw_prompt()?.index == 1 {
        let start = Text::new("Start the test at (mm:ss or seconds):")
            .with_default("1:00")
            .with_validator(|s: &str| {
                Ok(if markers::parse_time(s).is_some() { Validation::Valid } else { Validation::Invalid("use mm:ss or seconds".into()) })
            })
            .prompt()?;
        a.preview = Some(start);
        a.quality = Some(Quality::Draft);
    } else {
        let res = vec!["1080p (recommended)", "1440p (sharper, slower)", "720p (fastest)"];
        a.resolution = match Select::new("Resolution:", res).raw_prompt()?.index {
            1 => Some(1440),
            2 => Some(720),
            _ => None,
        };
    }

    if Confirm::new("Change advanced options (seed, titles, film grain)?").with_default(false).prompt()? {
        let seed = CustomType::<u64>::new("Seed (same seed = same video; another number = a new take):").with_default(1).prompt()?;
        a.seed = (seed != 1).then_some(seed);
        a.no_titles = !Confirm::new("Show track titles?").with_default(true).prompt()?;
        a.no_grain = !Confirm::new("Add film grain?").with_default(true).prompt()?;
    }
    Ok(a)
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
            ..Default::default()
        };
        let args = to_args(&a);
        assert_eq!(command_line(&args), "visualiser \"C:/My Music/mix.flac\" --mode single --generated 1 --vibe \"dark and liquid\" --quality draft --preview 1:30 --seed 7 --no-grain");
        let cli = crate::Cli::try_parse_from(std::iter::once("visualiser".to_string()).chain(args)).unwrap();
        assert_eq!(cli.audio.as_deref(), Some(std::path::Path::new("C:/My Music/mix.flac")));
        assert_eq!(cli.mode, Some(Mode::Single));
        assert_eq!(cli.quality, Quality::Draft);
        assert_eq!(cli.seed, 7);
        assert_eq!(cli.generated, Some(1.0));
        assert_eq!(cli.vibe.as_deref(), Some("dark and liquid"));
        assert!(cli.no_grain && !cli.no_titles);
    }
}
