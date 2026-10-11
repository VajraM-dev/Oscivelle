# Dynamic Visuals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: follow `/code-implementation-build-rules` (orchestrator dispatches, subagents build; waves below). Use superpowers:subagent-driven-development for per-task dispatch. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the visualiser easy for non-technical friends (guided wizard + Windows installer) and make its visuals generative: audio modulation, composable shader blocks, an evolve-and-score loop, and optional AI (bring your own Anthropic or Amazon Bedrock key) that turns a described vibe into visuals.

**Architecture:** Everything still flows analysis → plan → render. New visuals are expressed as a **genome** (motion + emitter + colour block ids + a palette) that compiles to an ordinary scene `.wgsl`, so the engine, preview and renderer stay unchanged. A headless scorer judges candidate genomes against the real music; the LLM only ever outputs genomes (JSON, via forced tool use), never shader code. The one exception is the dev-only `--author` tool.

**Tech Stack:** Rust 2021, wgpu 30, clap 4, serde/toml. New crates: `inquire` (prompts), `rfd` (native file dialog), `ureq` (blocking HTTPS), `keyring` (Windows Credential Manager). Installer: Inno Setup 6.

---

## Build setup (from /code-implementation-build-rules)

`T2 Standard · no web frontend (terminal wizard + Inno Setup pages; design gate n/a) · review: /simplify → /review`

- **Branch:** all work on `feature/dynamic-visuals` (already created from `main` @ `2c2fabb`). Never commit to `main`. The user merges later.
- **Orchestrator** plans, dispatches and joins. Subagents do the work. Independent tasks in one wave are dispatched as multiple `Agent` calls **in a single message**.
- **Model routing:** `sonnet` for mechanical tasks (spelled-out code, config, docs). `opus` for director logic, shader blocks, scorer, evolution, LLM client and anything ambiguous. No Haiku.
- **Testing discipline:** during a wave, subagents run only `cargo build` (or `cargo check`) for their own task, report, and stop. Tests run once at each **milestone gate**. After a fix, re-run only the failing test, then one full `cargo test` at the gate.
- **Milestones.** This deviates from the build rules on purpose: review happens twice, not once at the very end, because M1 is shippable to friends on its own.
  - **M1** = Phases 1–3 → gate → `/simplify` → `/review` → commit.
  - **M2** = Phases 4–7 → gate → `/simplify` → `/review` → commit.
- **ponytail:** minimum code. Each non-trivial piece of logic leaves ONE runnable check (a `#[test]`). No test suites for trivial code.
- **Commits:** one per phase on the feature branch, message style `feat(wizard): ...`.

### Waves

```
M1  Wave 1 (parallel): P1 wizard [sonnet] · P2 installer [sonnet] · P3 modulation [opus]
    Gate M1 → /simplify → /review → docs [sonnet] → commit
M2  Wave 2 (orchestrator): write scenes/blocks/CONTRACT.md (Task 4.0)
    Wave 3 (parallel): 4.1 genome core + wiring [opus] · 4.2 motion blocks [opus] · 4.3 emitter blocks [opus] · 4.4 colour blocks + palette bank [opus]
    Wave 4 (parallel): P5 scorer + evolution [opus] · 6.1 AI client + setup-ai [opus]
    Wave 5 (parallel): 6.2 vibe → genomes + wiring [opus] · P7 --author [opus]
    Gate M2 → /simplify → /review → docs [sonnet] → commit
```

File-collision check: P1 owns `main.rs`, `wizard.rs`, `Cargo.toml`. P2 owns `.cargo/`, `installer/`, `package.ps1`. P3 owns `types.rs`, `director.rs`, `scenes.rs`, `scenes/*.toml`. In Wave 3 only 4.1 touches Rust. In Wave 4, P5 owns `score.rs`, `evolve.rs`, `engine.rs` (one small method) and `main.rs`; 6.1 owns `llm.rs`, `Cargo.toml` and **must not edit `main.rs`**: it hands its two-line flag wiring to Wave 5. In Wave 5, 6.2 owns `vibe.rs`, `main.rs`, `wizard.rs` and `installer/`; P7 owns `author.rs` only, with its flag wiring done by 6.2's agent (spelled out in Task 7).

---

## File map

| File | Status | Responsibility |
|---|---|---|
| `src/main.rs` | modify | `audio` becomes optional; no args → wizard; new flags `--generated`, `--keep`, `--vibe`, `--setup-ai`, `--forget-ai`, `--author` |
| `src/wizard.rs` | create | Guided questions → CLI args (pure `to_args`) → run |
| `.cargo/config.toml` | create | Static C runtime (no `vcruntime140.dll` dependency) |
| `installer/visualiser.iss` | create | Per-user Inno Setup installer: PATH, shortcuts, right-click menu, AI setup task |
| `package.ps1` | modify | Also builds the installer when Inno Setup is installed |
| `src/types.rs` | modify | `Segment.mods`, `Mod`, `ModSource`, `Segment.genome` |
| `src/scenes.rs` | modify | `ParamRange.fixed` |
| `scenes/*.toml` | modify | Mark `fixed = true` params |
| `src/director.rs` | modify | Generate mods; apply mods per frame; generated segments via `genome_for` callback |
| `scenes/blocks/CONTRACT.md` | create | The block shader contract |
| `scenes/blocks/*.wgsl` + `*.toml` | create | Motion / emitter / colour blocks |
| `scenes/blocks/palettes.toml` | create | Palette bank (hex) |
| `src/genome.rs` | create | `Genome`, block loading, compose to WGSL, `to_scene`, `random`, `mutate`, `export` |
| `src/score.rs` | create | Pure metrics + headless `Judge` (render small, read back luma) |
| `src/evolve.rs` | create | Population search over genomes using `Judge` |
| `src/render/engine.rs` | modify | `Engine::add_scene` |
| `src/llm.rs` | create | AI config (env + Credential Manager), Anthropic/Bedrock endpoint, forced-tool call, `--setup-ai` |
| `src/vibe.rs` | create | Vibe text → genomes (cached in `<name>.vibe.json`) |
| `src/author.rs` | create | Dev-only: prompt → new scene `.wgsl`/`.toml`, compile-checked and scored |
| `docs/USAGE.md`, `README.md` | modify | Document everything above |

---

# Milestone 1 (Phases 1–3)

## Phase 1: Guided wizard  ·  [sonnet]

### Task 1.1: Dependencies

**Files:** Modify `Cargo.toml`

- [ ] **Step 1:** Run `cargo add inquire rfd`. Expected: both added under `[dependencies]` with their current versions.

### Task 1.2: `wizard.rs`

**Files:** Create `src/wizard.rs`

- [ ] **Step 1: Write the module**

```rust
//! Guided mode: run with no arguments (double-click, Start Menu) → a few questions →
//! prints the equivalent command → runs it. The answers become ordinary CLI args, so
//! clap validates them exactly as if typed.
use crate::markers;
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

/// Copy-pasteable command line (args with spaces quoted).
pub fn command_line(args: &[String]) -> String {
    std::iter::once("visualiser".to_string())
        .chain(args.iter().map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a.clone() }))
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
            quality: Some(Quality::Draft),
            preview: Some("1:30".into()),
            seed: Some(7),
            no_grain: true,
            ..Default::default()
        };
        let args = to_args(&a);
        assert_eq!(command_line(&args), "visualiser \"C:/My Music/mix.flac\" --mode single --quality draft --preview 1:30 --seed 7 --no-grain");
        let cli = crate::Cli::try_parse_from(std::iter::once("visualiser".to_string()).chain(args)).unwrap();
        assert_eq!(cli.audio.as_deref(), Some(std::path::Path::new("C:/My Music/mix.flac")));
        assert_eq!(cli.mode, Some(Mode::Single));
        assert_eq!(cli.quality, Quality::Draft);
        assert_eq!(cli.seed, 7);
        assert!(cli.no_grain && !cli.no_titles);
    }
}
```

### Task 1.3: Wire into `main.rs`

**Files:** Modify `src/main.rs`

- [ ] **Step 1:** Add `mod wizard;` after `mod types;`.
- [ ] **Step 2:** Make `audio` optional in `Cli`:

```rust
    /// WAV, FLAC, MP3 or AIFF file. Leave out to be asked a few questions instead.
    audio: Option<PathBuf>,
```

- [ ] **Step 3:** Replace `fn main()` with:

```rust
fn main() {
    let argc = std::env::args().len();
    // No arguments = double-clicked / Start Menu (guided mode); one = a file dragged onto the exe
    // or the right-click menu. Keep the console open in both cases so the result can be read.
    let pause = argc <= 2;
    let result = if argc == 1 { guided() } else { run(Cli::parse()) };
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
    let args = wizard::to_args(&wizard::ask()?);
    println!("\nSame thing as a command, for next time:\n  {}\n", wizard::command_line(&args));
    run(Cli::try_parse_from(std::iter::once("visualiser".to_string()).chain(args))?)
}
```

- [ ] **Step 4:** At the top of `run`, replace `let audio = std::path::absolute(&cli.audio)?;` with:

```rust
    let audio = std::path::absolute(cli.audio.as_deref().ok_or_else(|| anyhow!("no audio file given (run with no arguments for guided mode)"))?)?;
```

- [ ] **Step 5:** `cargo build`. Expected: compiles with no new warnings.
- [ ] **Step 6:** Commit: `git add -A && git commit -m "feat(wizard): guided mode when run with no arguments"`

---

## Phase 2: Installer + static CRT  ·  [sonnet]

### Task 2.1: Static C runtime

**Files:** Create `.cargo/config.toml`

- [ ] **Step 1:**

```toml
# Link the C runtime statically: friends' PCs may lack vcruntime140.dll.
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

- [ ] **Step 2:** `cargo build --release` (full rebuild, expected). Then `grep -aic "vcruntime140" target/release/visualiser.exe`. Expected: `0`.

### Task 2.2: Inno Setup script

**Files:** Create `installer/visualiser.iss`

- [ ] **Step 1:**

```iss
; Per-user installer (no admin prompt). Built by package.ps1; needs Inno Setup 6
; (winget install JRSoftware.InnoSetup). Packs whatever package.ps1 staged in dist\visualiser.
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{71CCD7C8-D92F-4643-A58F-EFFADAF0AF2D}
AppName=Visualiser
AppVersion={#AppVersion}
AppPublisher=Prathamesh Kulkarni
DefaultDirName={localappdata}\Programs\Visualiser
DisableProgramGroupPage=yes
DefaultGroupName=Visualiser
PrivilegesRequired=lowest
ChangesEnvironment=yes
OutputDir=..\dist
OutputBaseFilename=visualiser-setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\visualiser.exe

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut (you can also drop audio files on it)"
Name: contextmenu; Description: "Add ""Make visuals"" to the right-click menu of audio files"

[Files]
Source: "..\dist\visualiser\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{group}\Visualiser"; Filename: "{app}\visualiser.exe"; WorkingDir: "{app}"
Name: "{group}\Uninstall Visualiser"; Filename: "{uninstallexe}"
Name: "{userdesktop}\Visualiser"; Filename: "{app}\visualiser.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wav\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wav\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flac\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flac\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp3\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp3\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.aiff\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.aiff\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.aif\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.aif\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu

[Code]
const EnvKey = 'Environment';

// Position of Dir in the ';'-separated Paths (1-based index into Paths), 0 if absent.
function PathPos(Paths, Dir: string): Integer;
begin
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Paths) + ';');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var Paths: string;
begin
  if CurStep = ssPostInstall then begin
    if not RegQueryStringValue(HKCU, EnvKey, 'Path', Paths) then Paths := '';
    if PathPos(Paths, ExpandConstant('{app}')) = 0 then begin
      if (Paths <> '') and (Copy(Paths, Length(Paths), 1) <> ';') then Paths := Paths + ';';
      RegWriteExpandStringValue(HKCU, EnvKey, 'Path', Paths + ExpandConstant('{app}'));
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var Paths, Dir: string; P: Integer;
begin
  if CurUninstallStep = usPostUninstall then begin
    if RegQueryStringValue(HKCU, EnvKey, 'Path', Paths) then begin
      Dir := ExpandConstant('{app}');
      P := PathPos(Paths, Dir);
      if P > 0 then begin
        Delete(Paths, P, Length(Dir));
        if P > 1 then Delete(Paths, P - 1, 1)
        else if Copy(Paths, 1, 1) = ';' then Delete(Paths, 1, 1);
        RegWriteExpandStringValue(HKCU, EnvKey, 'Path', Paths);
      end;
    end;
  end;
end;
```

### Task 2.3: Build the installer from `package.ps1`

**Files:** Modify `package.ps1` (append at the end)

- [ ] **Step 1:**

```powershell
# Installer (optional): needs Inno Setup 6.
$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "${env:LOCALAPPDATA}\Programs\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($iscc) {
    & $iscc "/DAppVersion=$version" installer\visualiser.iss
    if ($LASTEXITCODE -ne 0) { throw "ISCC failed" }
    Write-Host "dist\visualiser-setup-$version.exe"
} else {
    Write-Host "Inno Setup 6 not found, skipping the installer (winget install JRSoftware.InnoSetup)"
}
```

- [ ] **Step 2:** Commit: `git add -A && git commit -m "feat(installer): per-user Inno Setup installer, static CRT"`

---

## Phase 3: Modulation matrix  ·  [opus]

### Task 3.1: Types

**Files:** Modify `src/types.rs`, `src/scenes.rs`

- [ ] **Step 1:** In `types.rs`, add a field at the end of `Segment` and the new types below it:

```rust
    /// Audio routings for this segment: param += depth * (param range) * source, on top of the variations.
    #[serde(default)]
    pub mods: Vec<Mod>,
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
```

- [ ] **Step 2:** In `scenes.rs`, add to `ParamRange`:

```rust
    /// Never audio-modulated: integer params (floor/round in the shader), params that multiply
    /// time() (the phase would jump) and hue offsets (hue flicker on kicks looks cheap).
    #[serde(default)]
    pub fixed: bool,
```

Update the header doc comment of `scenes.rs` with `#   fixed = true   # optional: never audio-modulated`.

### Task 3.2: Mark fixed params

**Files:** Modify `scenes/*.toml` (7 files)

- [ ] **Step 1:** For each scene, read its `.wgsl` and set `fixed = true` on every param whose `P(i)` is: inside `floor`/`round`/`i32`/`u32`, multiplied with `time()`/`t`, or used as a hue/palette offset. Example: `liquid_tunnel.toml` → `lobes` (P4, `floor`) and `hue_offset` (P5) get `fixed = true`.

### Task 3.3: Director generates and applies mods

**Files:** Modify `src/director.rs`

- [ ] **Step 1:** Add below `variations`:

```rust
const SOURCES: [ModSource; 7] =
    [ModSource::Kick, ModSource::Bass, ModSource::Mid, ModSource::High, ModSource::Energy, ModSource::Tension, ModSource::Drop];

/// 1-2 audio routings onto random non-fixed params (own rng stream, so scene/palette/variation picks are unchanged).
fn mods(sc: &Scene, seed: u64) -> Vec<Mod> {
    let free: Vec<usize> = (0..sc.params.len().min(8)).filter(|&i| !sc.params[i].fixed).collect();
    if free.is_empty() {
        return Vec::new();
    }
    let mut rng = Rng(hash(seed, 4));
    let mut out: Vec<Mod> = Vec::new();
    for _ in 0..1 + rng.next() % 2 {
        let param = free[(rng.next() % free.len() as u64) as usize];
        let source = SOURCES[(rng.next() % SOURCES.len() as u64) as usize];
        let depth = rng.signed().signum() * (0.15 + 0.25 * (rng.signed() * 0.5 + 0.5));
        if !out.iter().any(|m| m.param == param) {
            out.push(Mod { param, source, depth });
        }
    }
    out
}
```

- [ ] **Step 2:** In `plan()`, in the unlocked `segments.push(Segment { ... })`, add `mods: mods(sc, seed),`. (The locked path `..s.clone()` keeps saved mods.)
- [ ] **Step 3:** In `frame_state`'s `build` closure, right after the `if let Some(cur) = vs.get(v) { ... }` block and before `(sc.id.clone(), u)`, add:

```rust
        // Audio modulation on top of the phrase variations, kept inside each param's range.
        let src = |m: ModSource| match m {
            ModSource::Kick => base.beat[0],
            ModSource::Bass => base.bands[1],
            ModSource::Mid => base.bands[2],
            ModSource::High => base.bands[3],
            ModSource::Energy => base.shape[0],
            ModSource::Tension => base.shape[1],
            ModSource::Drop => base.shape[2],
        };
        for m in &s.mods {
            let Some(p) = sc.params.get(m.param).filter(|p| !p.fixed && m.param < 8) else { continue };
            let (lo, hi) = (p.min.min(p.max), p.min.max(p.max));
            let slot = if m.param < 4 { &mut u.p0[m.param] } else { &mut u.p1[m.param - 4] };
            *slot = (*slot + m.depth * (hi - lo) * src(m.source)).clamp(lo, hi);
        }
```

- [ ] **Step 4: Test.** In `director.rs` tests, move the inline `Analysis` and scenes construction from `director_deterministic_and_reroll_is_local` into helpers `fn test_analysis() -> Analysis` and `fn test_scenes() -> Vec<Scene>`. In `test_analysis`, set `sub`, `bass`, `mid` and `high` to `vec![0.5; 6000]`. In `test_scenes`, add `fixed: false` to the `ParamRange`. Make the existing test call the helpers. Then add:

```rust
    #[test]
    fn mods_skip_fixed_params_stay_in_range_and_move_params() {
        let a = test_analysis();
        let mut scenes = test_scenes();
        for s in &mut scenes {
            s.params = vec![
                crate::scenes::ParamRange { min: 0.0, max: 1.0, fixed: true },
                crate::scenes::ParamRange { min: 2.0, max: 4.0, fixed: false },
            ];
        }
        let req = PlanRequest { analysis: &a, scenes: &scenes, mode: Mode::Auto, markers: &[], seed: 7, previous: None, reroll: &[], force_scene: None };
        let p = plan(&req).unwrap();
        assert!(p.segments.iter().all(|s| !s.mods.is_empty() && s.mods.iter().all(|m| m.param == 1)));
        let mut flat = p.clone();
        for s in &mut flat.segments {
            s.mods.clear();
        }
        let mut moved = false;
        for t in [10.0, 64.0, 100.0, 300.0] {
            let f = frame_state(&p, &a, &scenes, t, 64, 64, Quality::Draft);
            assert!((2.0..=4.0).contains(&f.a.p0[1]), "t={t}: {}", f.a.p0[1]);
            moved |= f.a.p0[1] != frame_state(&flat, &a, &scenes, t, 64, 64, Quality::Draft).a.p0[1];
        }
        assert!(moved);
    }
```

- [ ] **Step 5:** `cargo build`. Commit: `git add -A && git commit -m "feat(director): per-segment audio modulation matrix"`

---

## Gate M1

- [ ] `cargo build --release` and `cargo test`. Expected: all pass, including `engine_runs`, which compiles every shipped scene.
- [ ] Smoke render: `target/release/visualiser testdata/dnb5.mp3 --replan --preview 1:00 --quality draft`. Expected: `testdata/preview-60s.mp4` written. Open `testdata/dnb5.plan.json` and confirm segments have `mods`.
- [ ] **Ask the user** to run `target/release/visualiser.exe` with no args (double-click works too) and finish the wizard with a 30 s test. An agent can't drive the interactive prompts.
- [ ] `powershell -File package.ps1`. If Inno Setup is installed, **ask the user** to run `dist/visualiser-setup-0.1.0.exe`, then in a **new** terminal run `visualiser --version`, right-click a `.wav` → "Make visuals", use the Start Menu "Visualiser" (opens the wizard), and finally uninstall. Expected: the PATH entry and right-click menu are gone after uninstall.
- [ ] `/simplify` → `/review`. Fix the findings, re-run only the affected tests.
- [ ] Docs [sonnet]: `docs/USAGE.md` gets a "Guided mode" section, an "Installer" section and `mods` under "Editing the plan file", including what `fixed = true` does in scene tomls. `README.md` gets one line each.
- [ ] Commit: `git commit -am "docs: wizard, installer, modulation"`

---

# Milestone 2 (Phases 4–7)

## Phase 4: Layered composition (genomes)

### Task 4.0: Block contract  ·  orchestrator, before Wave 3

**Files:** Create `scenes/blocks/CONTRACT.md` with exactly this content:

````markdown
# Scene blocks

A generated scene = one **motion** + one **emitter** + one **colour** block, glued by the
template in `src/genome.rs`. Each block is `<id>.wgsl` + `<id>.toml` in this folder. All prelude
helpers (`prev`, `warp`, `pal`, `kick()`, `fbm`, `line_glow`, ...) are available.

## <id>.toml
```toml
kind = "motion"            # motion | emitter | colour
description = "Flight down a tube: zoom + twist, trails stream outward"   # one line, shown to the AI
[[param]]                  # motion ≤ 3, emitter ≤ 3, colour ≤ 2 params
name = "zoom"
min = 1.002
max = 1.02
fixed = false              # optional, same meaning as in scene tomls
```

## Functions each block must define (exactly one block of each kind is included)
- motion:  `fn motion(uv: vec2<f32>) -> vec2<f32>`: uv to sample the previous frame at.
           `fn motion_decay() -> f32`: trail multiplier per step, 0.75..0.99.
- emitter: `fn emit(p: vec2<f32>) -> vec2<f32>`: p = centred coords (`to_centered(uv)`);
           returns (intensity ≥ 0, roughly 0..1.5; hue 0..1).
- colour:  `fn colour(e: vec2<f32>) -> vec3<f32>`: (intensity, hue) → linear HDR rgb, via `pal()`.

## Rules
- Params: motion uses `MP(0..2)`, emitter `EP(0..2)`, colour `CP(0..1)`. Never call `P()` directly.
- Helper functions and consts are prefixed with the block id (`tunnel_ridge`), so blocks never collide.
- React to the kick and drops (`kick()`, `drop_env()`, `since_kick()`), not to every band.
- `MP/EP/CP` never multiply `time()` (phase jumps when params change). Mark such params `fixed = true`.
- No `textureSample` other than through `prev()`; no derivatives (`dpdx`, `fwidth`) in motion blocks.
````

### Task 4.1: Genome core + wiring  ·  [opus]

**Files:** Create `src/genome.rs`. Modify `src/director.rs`, `src/types.rs`, `src/main.rs`, `src/render/engine.rs` (test only).

- [ ] **Step 1: `genome.rs` types, loading, composition**

```rust
//! Generated scenes: a genome picks one motion, emitter and colour block (scenes/blocks/) plus a
//! palette, and compiles to an ordinary scene (.wgsl), so engine/preview/render are unchanged.
use crate::scenes::{ParamRange, Scene};
use crate::types::fnv1a;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Genome {
    pub motion: String,
    pub emitter: String,
    pub colour: String,
    /// sRGB hex, 4 colours.
    pub palette: [String; 4],
}

impl Genome {
    /// Stable scene id, e.g. "gen_3fa9c01b22de".
    pub fn id(&self) -> String {
        format!("gen_{:012x}", fnv1a(serde_json::to_string(self).unwrap().as_bytes()) & 0xffff_ffff_ffff)
    }
}

pub struct Block {
    pub id: String,
    pub description: String,
    pub params: Vec<ParamRange>,
    pub wgsl: String,
}

pub struct Blocks {
    pub motion: Vec<Block>,
    pub emitter: Vec<Block>,
    pub colour: Vec<Block>,
    /// Palette bank, sRGB hex.
    pub palettes: Vec<[String; 4]>,
}

#[derive(Deserialize)]
struct BlockToml {
    kind: String,
    description: String,
    #[serde(default, rename = "param")]
    params: Vec<ParamRange>,
}

const SLOTS: [usize; 3] = [3, 3, 2];

impl Blocks {
    /// Loads scenes/blocks/*.toml (+ matching .wgsl) and palettes.toml.
    pub fn load(dir: &Path) -> Result<Blocks> {
        #[derive(Deserialize)]
        struct Pal {
            palettes: Vec<[String; 4]>,
        }
        let pal: Pal = toml::from_str(&std::fs::read_to_string(dir.join("palettes.toml")).context("blocks/palettes.toml")?)?;
        let mut b = Blocks { motion: vec![], emitter: vec![], colour: vec![], palettes: pal.palettes };
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?.filter_map(|e| Some(e.ok()?.path())).collect();
        paths.sort();
        for path in paths {
            let id = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") || id == "palettes" {
                continue;
            }
            let t: BlockToml = toml::from_str(&std::fs::read_to_string(&path)?).with_context(|| format!("parsing {}", path.display()))?;
            let wgsl = std::fs::read_to_string(path.with_extension("wgsl")).with_context(|| format!("{id}.wgsl"))?;
            let (list, max) = match t.kind.as_str() {
                "motion" => (&mut b.motion, SLOTS[0]),
                "emitter" => (&mut b.emitter, SLOTS[1]),
                "colour" => (&mut b.colour, SLOTS[2]),
                k => bail!("{id}: unknown kind {k}"),
            };
            if t.params.len() > max {
                bail!("{id}: at most {max} params");
            }
            list.push(Block { id, description: t.description, params: t.params, wgsl });
        }
        if b.motion.is_empty() || b.emitter.is_empty() || b.colour.is_empty() || b.palettes.is_empty() {
            bail!("scenes/blocks needs at least one motion, emitter, colour block and one palette");
        }
        Ok(b)
    }

    fn get<'a>(list: &'a [Block], id: &str) -> Result<&'a Block> {
        list.iter().find(|b| b.id == id).with_context(|| format!("unknown block '{id}'"))
    }

    /// The three blocks, or an error naming the unknown id (AI output goes through this).
    pub fn parts(&self, g: &Genome) -> Result<[&Block; 3]> {
        Ok([Self::get(&self.motion, &g.motion)?, Self::get(&self.emitter, &g.emitter)?, Self::get(&self.colour, &g.colour)?])
    }
}

const TEMPLATE: &str = r#"
fn MP(i: i32) -> f32 { return P(i); }
fn EP(i: i32) -> f32 { return P(3 + i); }
fn CP(i: i32) -> f32 { return P(6 + i); }
{MOTION}
{EMITTER}
{COLOUR}
fn scene(uv: vec2<f32>) -> vec4<f32> {
    var col = prev(motion(uv)) * motion_decay();
    let e = emit(to_centered(uv));
    col += colour(e) * (0.6 + 0.8 * kick() + 0.6 * drop_env());
    // Soft luminance cap: the feedback can never flood.
    col = col / (1.0 + 0.12 * dot(col, vec3<f32>(0.2126, 0.7152, 0.0722)));
    return vec4<f32>(col, 1.0);
}
"#;

/// Scene body (prelude is prepended by scenes::shader_source as for any scene).
pub fn wgsl(g: &Genome, blocks: &Blocks) -> Result<String> {
    let [m, e, c] = blocks.parts(g)?;
    Ok(TEMPLATE.replace("{MOTION}", &m.wgsl).replace("{EMITTER}", &e.wgsl).replace("{COLOUR}", &c.wgsl))
}

/// In-memory Scene for a genome: 8 param slots (each block's params padded to its slot count)
/// and the genome's palette. `wgsl_path` = <gen_dir>/<id>.wgsl (written by `write`).
pub fn to_scene(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let pad = ParamRange { min: 0.0, max: 0.0, fixed: true };
    let mut params = Vec::with_capacity(8);
    for (b, n) in blocks.parts(g)?.iter().zip(SLOTS) {
        params.extend(b.params.iter().cloned());
        params.extend(std::iter::repeat(pad.clone()).take(n - b.params.len()));
    }
    let palette = crate::scenes::palette(&g.palette)?;
    Ok(Scene { id: g.id(), params, palettes: vec![palette], wgsl_path: gen_dir.join(format!("{}.wgsl", g.id())) })
}

/// Writes the genome's .wgsl into gen_dir and returns its Scene.
pub fn write(g: &Genome, blocks: &Blocks, gen_dir: &Path) -> Result<Scene> {
    let sc = to_scene(g, blocks, gen_dir)?;
    std::fs::create_dir_all(gen_dir)?;
    std::fs::write(&sc.wgsl_path, wgsl(g, blocks)?)?;
    Ok(sc)
}

/// Uniform random genome from `r` (a 64-bit hash; consumed in 16-bit pieces).
pub fn random(blocks: &Blocks, r: u64) -> Genome {
    let pick = |n: usize, k: u32| ((r >> (16 * k)) & 0xffff) as usize % n;
    Genome {
        motion: blocks.motion[pick(blocks.motion.len(), 0)].id.clone(),
        emitter: blocks.emitter[pick(blocks.emitter.len(), 1)].id.clone(),
        colour: blocks.colour[pick(blocks.colour.len(), 2)].id.clone(),
        palette: blocks.palettes[pick(blocks.palettes.len(), 3)].clone(),
    }
}
```

- [ ] **Step 2:** In `scenes.rs`, extract the palette conversion inside `load_all` into `pub fn palette(p: &[String; 4]) -> Result<[[f32; 3]; 4]>` and use it in both places.
- [ ] **Step 3:** In `types.rs`, add to `Segment`:

```rust
    /// Set when the scene is generated: `scene` is then `genome.id()` and its .wgsl is rebuilt from this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genome: Option<crate::genome::Genome>,
```

- [ ] **Step 4: Director.** Extend `PlanRequest` with:

```rust
    /// Fraction (0..1) of auto-picked segments that get a generated scene.
    pub generated: f32,
    /// Supplies a genome for a generated segment: (start, end, seed) -> genome, None = fall back to curated.
    pub genome_for: Option<&'a dyn Fn(f64, f64, u64) -> Option<crate::genome::Genome>>,
    pub blocks: Option<&'a crate::genome::Blocks>,
    pub gen_dir: &'a std::path::Path,
```

Behaviour:
- **Pass 1, auto-pick branch** `(None, None)`: if `genome_for` and `blocks` are `Some` and `(hash(base_seed(i), 5) >> 11) as f64 / (1u64 << 53) as f64 < generated as f64`, call `genome_for(start, end, hash(base_seed(i), 6))`. On `Some(g)`, the scene id is `g.id()`; keep the genomes in a `Vec<Option<Genome>>` parallel to `scenes`.
- Kept or locked segments keep their saved `genome` (`..s.clone()` already does it; for kept unlocked segments, copy `s.genome`).
- **Re-roll** of a generated segment calls `genome_for` again with `hash(seed, 6)`.
- Where `sc` is looked up, build an owned `Scene` with `genome::to_scene(g, blocks, gen_dir)` when the segment has a genome.
- Set `genome` on the pushed `Segment`.
- `force_scene` and markers' `@scene` always win, so no genome is used there.

Update the test `req` builders with `generated: 0.0, genome_for: None, blocks: None, gen_dir: Path::new(".")`.

- [ ] **Step 5: main.rs.**
  - Add `mod genome;`.
  - New flag:

```rust
    /// Share of scenes that are freshly generated (0 = only the hand-made ones, 1 = all generated).
    #[arg(long, default_value_t = 0.0)]
    generated: f32,
```

  - Validate `0.0..=1.0`. Load blocks with `genome::Blocks::load(&scenes_dir.join("blocks"))` (only when `generated > 0`, or when the saved plan has any genome).
  - Set `gen_dir = sibling(&audio, "gen")`. Pass `genome_for: Some(&|_, _, r| Some(genome::random(&blocks, r)))`.
  - Add `cli.generated` to the reuse check: a changed value means replan. Store it in `Plan` as `#[serde(default)] pub generated: f32`, compared like `seed`/`mode`.
  - After the plan is final, for every segment with a genome: `scenes.push(genome::write(g, &blocks, &gen_dir)?)` (dedupe by id). `scenes` must become `let mut scenes`.
- [ ] **Step 6: Test** (in `engine.rs` tests, extending `engine_runs`): every motion × emitter × colour combination compiles.

```rust
        let blocks = crate::genome::Blocks::load(Path::new("scenes/blocks")).unwrap();
        let gen_dir = std::env::temp_dir().join("visualiser_gen_test");
        let mut gen = Vec::new();
        for m in &blocks.motion {
            for em in &blocks.emitter {
                for c in &blocks.colour {
                    let g = crate::genome::Genome { motion: m.id.clone(), emitter: em.id.clone(), colour: c.id.clone(), palette: blocks.palettes[0].clone() };
                    gen.push(crate::genome::write(&g, &blocks, &gen_dir).unwrap());
                }
            }
        }
        let mut e = Engine::new(&device, &queue, &gen, dir, Quality::Draft, 64, 36, wgpu::TextureFormat::Rgba8Unorm);
        for s in &gen {
            e.compile(&s.id).unwrap_or_else(|err| panic!("{}: {err:#}", s.id));
        }
```

- [ ] **Step 7:** `cargo build`. Commit after Wave 3 joins: `git commit -am "feat(genome): composable scene blocks and generated scenes"`

### Tasks 4.2–4.4: Block library  ·  [opus ×3, parallel]

Each agent reads `scenes/blocks/CONTRACT.md`, `scenes/_prelude.wgsl` and the existing scenes, then writes blocks that obey the contract. **Port the best ideas out of the existing 7 scenes** (they are tuned), simplified to the contract. Each agent runs `cargo test engine_runs` only after 4.1 has landed. If 4.1 isn't done yet, the agent compiles its blocks by writing a throwaway full scene that includes them and running `cargo run -- testdata/dnb5.mp3 --dev --scene <tmp>`, then deletes it.

- **4.2 Motion blocks** (`scenes/blocks/motion_*.wgsl/.toml`), 6 of them:
  - `motion_tunnel`: log-polar zoom + twist, from liquid_tunnel
  - `motion_swirl`: fbm swirl advection, from plasma_field
  - `motion_kaleido`: mirrored sector fold + slow rotate, from kaleidoscope_bloom
  - `motion_push`: radial outward push, from starfield_warp
  - `motion_ink`: curl-noise drift with slow decay, from ink_in_water
  - `motion_deepzoom`: steady zoom into a wandering point, from fractal_zoom
- **4.3 Emitter blocks** (`scenes/blocks/emit_*`), 7 of them:
  - `emit_filaments`: ridged fbm, from liquid_tunnel
  - `emit_rings`: kick-born rings
  - `emit_contours`: iso-lines of domain-warped noise, from plasma_field
  - `emit_polygon`: neon polygon outlines with `line_glow`, from neon_geometry
  - `emit_stars`: hashed points, from starfield_warp
  - `emit_blobs`: soft ink blobs, from ink_in_water
  - `emit_lissajous`: glowing Lissajous curve with `line_glow`
- **4.4 Colour blocks** (`scenes/blocks/colour_*`), 5 of them, plus `palettes.toml`:
  - `colour_hue`: `pal(hue)`
  - `colour_intensity`: `pal(intensity)`
  - `colour_iridescent`: hue shifted by intensity
  - `colour_whitehot`: palette with white cores at high intensity
  - `colour_duotone`: two palette entries only

  `palettes.toml` = every palette from all 7 scene tomls, as `palettes = [[...], ...]`. That's mechanical; the agent can do it in a few minutes.

The **orchestrator** commits after Wave 3 and Task 4.1's test step pass.

---

## Phase 5: Scorer + evolution  ·  [opus]

### Task 5.1: `score.rs`

**Files:** Create `src/score.rs`. Modify `src/render/engine.rs` (add `add_scene`).

- [ ] **Step 1: Pure metrics + verdict** (testable without a GPU):

```rust
//! Judge for generated scenes: render a few seconds small, reject the dull, the flooded,
//! the noisy and the deaf. Catches bad, not great; humans keep the favourites (--keep).
use crate::types::*;

pub struct Metrics {
    pub mean: f32,      // average luma
    pub contrast: f32,  // average per-frame luma std-dev
    pub motion: f32,    // average |frame - previous frame|
    pub clipped: f32,   // share of pixels near white
    pub grain: f32,     // average horizontal neighbour difference (TV-static detector)
    pub kick_sync: f32, // correlation of kick envelope with frame change
}

/// `frames`: luma 0..1, row-major, width `w`; `kicks[i]` = kick envelope at frame i.
pub fn metrics(frames: &[Vec<f32>], w: usize, kicks: &[f32]) -> Metrics {
    let n = frames.len().max(1) as f32;
    let px = frames.first().map_or(1, |f| f.len()).max(1) as f32;
    let mean_of = |f: &Vec<f32>| f.iter().sum::<f32>() / px;
    let mean = frames.iter().map(mean_of).sum::<f32>() / n;
    let contrast = frames.iter().map(|f| { let m = mean_of(f); (f.iter().map(|v| (v - m).powi(2)).sum::<f32>() / px).sqrt() }).sum::<f32>() / n;
    let clipped = frames.iter().map(|f| f.iter().filter(|&&v| v > 0.98).count() as f32 / px).sum::<f32>() / n;
    let grain = frames.iter().map(|f| f.chunks(w).map(|r| r.windows(2).map(|p| (p[1] - p[0]).abs()).sum::<f32>()).sum::<f32>() / px).sum::<f32>() / n;
    let diffs: Vec<f32> = frames.windows(2).map(|p| p[0].iter().zip(&p[1]).map(|(a, b)| (a - b).abs()).sum::<f32>() / px).collect();
    let motion = diffs.iter().sum::<f32>() / diffs.len().max(1) as f32;
    Metrics { mean, contrast, motion, clipped, grain, kick_sync: pearson(&kicks[1.min(kicks.len())..], &diffs) }
}

fn pearson(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    if n < 3 {
        return 0.0;
    }
    let (ma, mb) = (a[..n].iter().sum::<f32>() / n as f32, b[..n].iter().sum::<f32>() / n as f32);
    let (mut c, mut va, mut vb) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (x, y) = (a[i] - ma, b[i] - mb);
        c += x * y;
        va += x * x;
        vb += y * y;
    }
    if va * vb > 1e-12 { c / (va * vb).sqrt() } else { 0.0 }
}

// ponytail: hand-set thresholds. Calibrate with `cargo test calibrate -- --ignored --nocapture`
// (every hand-made scene must pass); a learned scorer only if these keep misjudging.
const MEAN: (f32, f32) = (0.04, 0.75);
const MIN_CONTRAST: f32 = 0.04;
const MOTION: (f32, f32) = (0.002, 0.2);
const MAX_CLIPPED: f32 = 0.25;
const MAX_GRAIN: f32 = 0.2;

/// None = rejected; else higher is better (contrast + music lock + some motion).
pub fn verdict(m: &Metrics) -> Option<f32> {
    let ok = (MEAN.0..=MEAN.1).contains(&m.mean)
        && m.contrast >= MIN_CONTRAST
        && (MOTION.0..=MOTION.1).contains(&m.motion)
        && m.clipped <= MAX_CLIPPED
        && m.grain <= MAX_GRAIN;
    ok.then(|| m.contrast + 2.0 * m.kick_sync.max(0.0) + 4.0 * m.motion.min(0.05))
}
```

- [ ] **Step 2: `Judge`**, a headless renderer. It owns a device/queue (from `engine::gpu`), one `Engine` at 160×90 `Quality::Draft`, an `Rgba8Unorm` output texture with `RENDER_ATTACHMENT | COPY_SRC`, and a `MAP_READ | COPY_DST` buffer with `bytes_per_row` padded to 256.

```rust
pub struct Judge { /* device, queue, engine, out_tex, buf, padded */ }
impl Judge {
    pub fn new(scenes: &[crate::scenes::Scene], scenes_dir: &std::path::Path, cpu: bool) -> anyhow::Result<Judge>;
    /// Compiles `scene` (Err = doesn't compile; message is the compiler's) and scores `secs` seconds of it
    /// from `t0`, using palette 0, params at range midpoints, no mods.
    pub fn judge(&mut self, scene: crate::scenes::Scene, analysis: &Analysis, t0: f64, secs: f64) -> anyhow::Result<Option<f32>>;
}
```

`judge` does the following:
1. `engine.add_scene(scene)`, then `engine.compile(&id)?`.
2. Build a one-segment `Plan` (start 0, end `analysis.duration`, one `Variation` at 0 with midpoint params, CUT, no mods).
3. `engine.reset()`, then pre-roll 2 s.
4. For each output frame at 30 fps: two `engine.step(&fs, ..)` calls, with `Some(1.0)` on the second.
5. Every 3rd frame: `engine.finish` into `out_tex` with grain off and debug off, `copy_texture_to_buffer`, `map_async` + `device.poll(wgpu::PollType::wait_indefinitely())`, convert RGBA8 rows to luma (`0.2126 r + 0.7152 g + 0.0722 b`, /255), and record `fs.a.beat[0]` as the kick.
6. Return `verdict(&metrics(..))`.

Copy the readback pattern from `render_chunk`/`flush` in `src/render/mod.rs`.

- [ ] **Step 3:** In `engine.rs`, add:

```rust
    /// Registers a scene created after construction (generated scenes); compile it with `compile`.
    pub fn add_scene(&mut self, scene: Scene) {
        if !self.scenes.iter().any(|s| s.id == scene.id) {
            self.scenes.push(scene);
        }
    }
```

- [ ] **Step 4: Tests** (`score.rs`):

```rust
    #[test]
    fn verdict_rejects_black_static_and_noise_accepts_a_pulse() {
        let w = 16;
        let flat = |v: f32| vec![v; w * 9];
        let kicks = vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        assert!(verdict(&metrics(&vec![flat(0.0); 6], w, &kicks)).is_none(), "black");
        assert!(verdict(&metrics(&vec![flat(0.4); 6], w, &kicks)).is_none(), "static");
        let noise: Vec<Vec<f32>> = (0..6).map(|i| (0..w * 9).map(|p| ((p * 7 + i * 13) % 2) as f32).collect()).collect();
        assert!(verdict(&metrics(&noise, w, &kicks)).is_none(), "tv static");
        // Gradient that brightens on kicks: contrasty, moving, kick-locked.
        let pulse: Vec<Vec<f32>> = kicks.iter().map(|k| (0..w * 9).map(|p| 0.1 + 0.3 * (p % w) as f32 / w as f32 + 0.2 * k).collect()).collect();
        assert!(verdict(&metrics(&pulse, w, &kicks)).is_some(), "pulse");
    }

    /// Manual calibration: every hand-made scene should pass on real music.
    #[test]
    #[ignore]
    fn calibrate() {
        let a: Analysis = serde_json::from_str(&std::fs::read_to_string("testdata/dnb5.analysis.json").expect("run dnb5 once first")).unwrap();
        let dir = std::path::Path::new("scenes");
        let scenes = crate::scenes::load_all(dir).unwrap();
        let mut j = Judge::new(&scenes, dir, false).unwrap();
        for s in &scenes {
            println!("{:24} {:?}", s.id, j.judge(s.clone(), &a, 60.0, 4.0).unwrap());
        }
    }
```

### Task 5.2: `evolve.rs`

**Files:** Create `src/evolve.rs`. Add to `src/genome.rs`.

- [ ] **Step 1:** Add `mutate` to `genome.rs`:

```rust
/// One random change: swap motion, emitter or colour, or take another palette.
pub fn mutate(g: &Genome, blocks: &Blocks, r: u64) -> Genome {
    let mut out = g.clone();
    let k = (r >> 48) as usize;
    let pick = |n: usize| (r & 0xffff_ffff) as usize % n;
    match r % 4 {
        0 => out.motion = blocks.motion[pick(blocks.motion.len())].id.clone(),
        1 => out.emitter = blocks.emitter[pick(blocks.emitter.len())].id.clone(),
        2 => out.colour = blocks.colour[pick(blocks.colour.len())].id.clone(),
        _ => out.palette = blocks.palettes[k % blocks.palettes.len()].clone(),
    }
    out
}
```

- [ ] **Step 2: `evolve.rs`**

```rust
//! Picks a generated scene for one segment: score a small population against the segment's
//! own music, mutate the best, keep the winner. Budget = candidates rendered.
use crate::genome::{self, Blocks, Genome};
use crate::score::Judge;
use crate::types::*;

pub fn best(judge: &mut Judge, blocks: &Blocks, gen_dir: &std::path::Path, a: &Analysis, start: f64, end: f64, seed: u64, seeds: &[Genome], budget: usize) -> Option<Genome> {
    // Score window: 4 s, 30 s into the segment (or its middle if shorter).
    let t0 = (start + 30.0).min((start + end) / 2.0);
    let mut score = |g: &Genome| -> Option<f32> {
        let sc = genome::write(g, blocks, gen_dir).ok()?;
        judge.judge(sc, a, t0, 4.0).ok().flatten()
    };
    let first = (budget * 2 / 5).max(2);
    let mut pop: Vec<Genome> = seeds.iter().take(first).cloned().collect();
    let mut k = 0u64;
    while pop.len() < first {
        pop.push(genome::random(blocks, hash64(seed, k)));
        k += 1;
    }
    let mut scored: Vec<(f32, Genome)> = pop.into_iter().filter_map(|g| Some((score(&g)?, g))).collect();
    scored.sort_by(|x, y| y.0.total_cmp(&x.0));
    let parents: Vec<Genome> = scored.iter().take(2).map(|x| x.1.clone()).collect();
    let per = (budget - first) / parents.len().max(1);
    for (pi, p) in parents.iter().enumerate() {
        for j in 0..per {
            let child = genome::mutate(p, blocks, hash64(seed, 1000 + (pi * 100 + j) as u64));
            if let Some(s) = score(&child) {
                scored.push((s, child));
            }
        }
    }
    scored.into_iter().max_by(|x, y| x.0.total_cmp(&y.0)).map(|x| x.1)
}

fn hash64(a: u64, b: u64) -> u64 {
    fnv1a(&[a.to_le_bytes(), b.to_le_bytes()].concat())
}
```

- [ ] **Step 3: main.rs wiring.**
  - Add `mod score; mod evolve;`.
  - Change the `--generated` default to `0.5`.
  - When `generated > 0` and a new plan is needed, create `let judge = RefCell::new(Judge::new(&scenes, &scenes_dir, cli.cpu)?)` and pass:

```rust
genome_for: Some(&|start, end, r| {
    eprintln!("generating a scene for {}..{}", hms(start as u64, false), hms(end as u64, false));
    evolve::best(&mut judge.borrow_mut(), &blocks, &gen_dir, &analysis, start, end, r, &[], if cli.quality == Quality::Draft { 10 } else { 20 })
})
```

  - The judge is created lazily, so no GPU is touched when the plan is reused.

- [ ] **Step 4: `--keep <segment>`** adds a generated look to the library:

```rust
    /// Copy segment N's generated scene into scenes/ as a hand-made one (keep a look you love).
    #[arg(long)]
    keep: Option<usize>,
```

  Add `genome::export(g, blocks, scenes_dir) -> Result<String>`. It writes `scenes/<id>.wgsl` (full composed body) and `scenes/<id>.toml` with `name`, `palettes = [[the 4 hex]]` and the 8 `[[param]]` entries (`min`/`max`/`fixed`), formatted by hand with `format!`. It returns the id. `--keep` runs after the plan loads, prints `kept as scenes/<id>`, and exits.
- [ ] **Step 5: Wizard.** Add `generated: Option<f32>` to `Answers` and the question "Visual style?": "Mix of hand-made and freshly generated (recommended)" → None, "Hand-made only" → `0`, "All freshly generated" → `1`. `to_args` emits `--generated <v>`. Extend the round-trip test with `generated: Some(1.0)` → `--generated 1`.
- [ ] **Step 6:** `cargo build`. Commit: `git commit -am "feat(evolve): scored generation of scenes, --keep"`

---

## Phase 6: AI (bring your own key: Anthropic or Amazon Bedrock)

**Verified 2026-10-10** (AWS and Anthropic docs). A **Bedrock API key** (not AWS access keys) works with the Anthropic-native Messages API:
- URL `https://bedrock-mantle.{region}.api.aws/anthropic/v1/messages`
- headers `x-api-key: <bedrock key>`, `anthropic-version: 2023-06-01`
- same request body as api.anthropic.com
- model ids carry an `anthropic.` prefix, e.g. `anthropic.claude-sonnet-5-5` (Sonnet 5.5 needs model access enabled in the Bedrock console; `anthropic.claude-sonnet-5` is open to everyone)

**Structured outputs are NOT supported on Bedrock**, so use **forced tool use** (`tool_choice: {type: "tool"}`), which works on both providers.

### Task 6.1: `llm.rs` + `--setup-ai`  ·  [opus]

**Files:** Create `src/llm.rs`. Modify `Cargo.toml`. **Do not edit `main.rs`**: report the flag wiring for Wave 5.

- [ ] **Step 1:** `cargo add ureq --features json` and `cargo add keyring --features windows-native`.
- [ ] **Step 2: The module**

```rust
//! Optional AI: bring your own Anthropic or Amazon Bedrock API key. The key lives in Windows
//! Credential Manager (or env vars) and is never printed, logged or written to project files.
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AiConfig {
    /// "anthropic" or "bedrock".
    pub provider: String,
    pub key: String,
    /// Bedrock region, e.g. "us-east-1".
    #[serde(default)]
    pub region: Option<String>,
    /// Override the default model id.
    #[serde(default)]
    pub model: Option<String>,
}

const SERVICE: &str = "visualiser";
const ACCOUNT: &str = "ai";

/// (url, model) for a config.
pub fn endpoint(c: &AiConfig) -> (String, String) {
    let model = std::env::var("VISUALISER_AI_MODEL").ok().or_else(|| c.model.clone());
    match c.provider.as_str() {
        "bedrock" => (
            format!("https://bedrock-mantle.{}.api.aws/anthropic/v1/messages", c.region.as_deref().unwrap_or("us-east-1")),
            model.unwrap_or_else(|| "anthropic.claude-sonnet-5-5".into()),
        ),
        _ => ("https://api.anthropic.com/v1/messages".into(), model.unwrap_or_else(|| "claude-sonnet-5-5".into())),
    }
}

/// Env vars first (ANTHROPIC_API_KEY, or AWS_BEARER_TOKEN_BEDROCK + AWS_REGION), then Credential Manager.
pub fn load() -> Option<AiConfig> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        return Some(AiConfig { provider: "anthropic".into(), key, region: None, model: None });
    }
    if let Ok(key) = std::env::var("AWS_BEARER_TOKEN_BEDROCK") {
        return Some(AiConfig { provider: "bedrock".into(), key, region: std::env::var("AWS_REGION").ok(), model: None });
    }
    let s = keyring::Entry::new(SERVICE, ACCOUNT).ok()?.get_password().ok()?;
    serde_json::from_str(&s).ok()
}

fn save(c: &AiConfig) -> Result<()> {
    keyring::Entry::new(SERVICE, ACCOUNT)?.set_password(&serde_json::to_string(c)?)?;
    Ok(())
}

pub fn forget() -> Result<()> {
    match keyring::Entry::new(SERVICE, ACCOUNT)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// POST a Messages request; returns the JSON body or an error with the API's message (never the key).
fn post(c: &AiConfig, body: &Value) -> Result<Value> {
    let (url, _) = endpoint(c);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(90)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .post(&url)
        .header("x-api-key", &c.key)
        .header("anthropic-version", "2023-06-01")
        .send_json(body)
        .map_err(|e| anyhow!("AI request failed: {e}"))?;
    let status = resp.status().as_u16();
    let v: Value = resp.body_mut().read_json().unwrap_or(Value::Null);
    if status != 200 {
        let msg = v.pointer("/error/message").or(v.pointer("/message")).and_then(Value::as_str).unwrap_or("no details");
        bail!("AI service returned {status}: {msg}");
    }
    Ok(v)
}

/// One forced tool call; returns the tool input JSON.
pub fn call_tool(c: &AiConfig, system: &str, user: &str, tool: &str, description: &str, schema: Value, max_tokens: u32) -> Result<Value> {
    let (_, model) = endpoint(c);
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system,
        "messages": [{"role": "user", "content": user}],
        "tools": [{"name": tool, "description": description, "input_schema": schema}],
        "tool_choice": {"type": "tool", "name": tool},
    });
    tool_input(&post(c, &body)?, tool)
}

/// The `input` of the first tool_use block named `tool`.
pub fn tool_input(resp: &Value, tool: &str) -> Result<Value> {
    resp["content"]
        .as_array()
        .and_then(|a| a.iter().find(|b| b["type"] == "tool_use" && b["name"] == tool))
        .map(|b| b["input"].clone())
        .context("AI reply had no tool call")
}

/// Interactive: choose provider, paste key (hidden), test it, store it.
pub fn setup() -> Result<()> {
    use inquire::{Password, PasswordDisplayMode, Select, Text};
    println!("AI features let you describe the visuals you want in words.\nYour key is stored in Windows Credential Manager and only sent to the AI service.\n");
    let providers = vec!["Anthropic API key", "Amazon Bedrock API key"];
    let bedrock = Select::new("Which key do you have?", providers).raw_prompt()?.index == 1;
    let key = Password::new("Paste your key:").with_display_mode(PasswordDisplayMode::Masked).without_confirmation().prompt()?;
    let region = if bedrock { Some(Text::new("AWS region:").with_default("us-east-1").prompt()?) } else { None };
    let c = AiConfig { provider: (if bedrock { "bedrock" } else { "anthropic" }).into(), key: key.trim().to_string(), region, model: None };
    let (_, model) = endpoint(&c);
    print!("Testing the key with {model} ... ");
    post(&c, &json!({"model": model, "max_tokens": 8, "messages": [{"role": "user", "content": "Say ok"}]}))?;
    println!("works.");
    save(&c)?;
    println!("Saved. In guided mode you'll now be asked to describe the vibe; on the command line use --vibe \"...\".");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_and_tool_parsing() {
        let b = AiConfig { provider: "bedrock".into(), key: "k".into(), region: Some("eu-west-1".into()), model: None };
        assert_eq!(endpoint(&b).0, "https://bedrock-mantle.eu-west-1.api.aws/anthropic/v1/messages");
        assert!(endpoint(&b).1.starts_with("anthropic.claude-"));
        let a = AiConfig { provider: "anthropic".into(), ..b };
        assert_eq!(endpoint(&a).0, "https://api.anthropic.com/v1/messages");
        let resp = json!({"content": [{"type": "text", "text": "hi"}, {"type": "tool_use", "name": "submit", "input": {"x": 1}}]});
        assert_eq!(tool_input(&resp, "submit").unwrap(), json!({"x": 1}));
        assert!(tool_input(&json!({"content": []}), "submit").is_err());
    }
}
```

> Check `ureq` 3's builder API against docs.rs for the version `cargo add` installed (`config_builder`, `http_status_as_error`, `body_mut().read_json()`). Adjust names if they differ; keep the behaviour.

- [ ] **Step 3:** `cargo build`. Report to the orchestrator the `main.rs` wiring for Wave 5: `mod llm;`, the flags `--setup-ai` and `--forget-ai` (both `bool`, handled at the top of `run` before the audio check: call `llm::setup()` / `llm::forget()` and return).

### Task 6.2: Vibe → genomes + wiring  ·  [opus]

**Files:** Create `src/vibe.rs`. Modify `src/main.rs`, `src/wizard.rs`, `installer/visualiser.iss`.

- [ ] **Step 1: `vibe.rs`**

```rust
//! "Describe the vibe" → starting genomes for the evolution. The AI only picks from the block
//! menu (ids are an enum in the schema; every answer is validated), so it can't break a render.
//! Cached in <name>.vibe.json: same vibe text = no new call, identical re-renders.
use crate::genome::{Blocks, Genome};
use crate::llm::{self, AiConfig};
use crate::types::Analysis;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Cache {
    vibe: String,
    genomes: Vec<Genome>,
}

pub fn genomes(c: &AiConfig, blocks: &Blocks, a: &Analysis, vibe: &str, cache: &Path) -> Result<Vec<Genome>> {
    if let Some(hit) = std::fs::read_to_string(cache).ok().and_then(|s| serde_json::from_str::<Cache>(&s).ok()).filter(|h| h.vibe == vibe) {
        return Ok(hit.genomes);
    }
    let menu = |list: &[crate::genome::Block]| list.iter().map(|b| format!("- {}: {}", b.id, b.description)).collect::<Vec<_>>().join("\n");
    let ids = |list: &[crate::genome::Block]| list.iter().map(|b| b.id.clone()).collect::<Vec<_>>();
    let system = format!(
        "You design music visuals by combining building blocks. Each look = one motion + one emitter + one colour block + a 4-colour palette \
         (sRGB hex; first colour dark, the background). Dark backgrounds with vivid accents read best on video.\n\nMotion blocks:\n{}\n\nEmitter blocks:\n{}\n\nColour blocks:\n{}",
        menu(&blocks.motion), menu(&blocks.emitter), menu(&blocks.colour)
    );
    let user = format!(
        "Vibe: {vibe}\nMusic: {:.0} BPM, {:.0} minutes, {} drops, {} track changes.\nGive 6 distinct looks that fit this vibe.",
        a.bpm, a.duration / 60.0, a.drops.len(), a.track_changes.len()
    );
    let schema = json!({
        "type": "object",
        "properties": {"genomes": {"type": "array", "minItems": 1, "maxItems": 8, "items": {
            "type": "object",
            "properties": {
                "motion": {"type": "string", "enum": ids(&blocks.motion)},
                "emitter": {"type": "string", "enum": ids(&blocks.emitter)},
                "colour": {"type": "string", "enum": ids(&blocks.colour)},
                "palette": {"type": "array", "minItems": 4, "maxItems": 4, "items": {"type": "string", "pattern": "^#[0-9a-fA-F]{6}$"}}
            },
            "required": ["motion", "emitter", "colour", "palette"]
        }}},
        "required": ["genomes"]
    });
    let input = llm::call_tool(c, &system, &user, "submit_looks", "Submit the looks", schema, 2000)?;
    let out = parse(&input, blocks);
    std::fs::write(cache, serde_json::to_string_pretty(&Cache { vibe: vibe.into(), genomes: out.clone() })?)?;
    Ok(out)
}

/// Valid genomes only: known block ids and parseable palettes; the rest are dropped.
fn parse(input: &serde_json::Value, blocks: &Blocks) -> Vec<Genome> {
    input["genomes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| serde_json::from_value::<Genome>(g.clone()).ok())
        .filter(|g| blocks.parts(g).is_ok() && crate::scenes::palette(&g.palette).is_ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keeps_only_valid_genomes() {
        let blocks = Blocks::load(Path::new("scenes/blocks")).unwrap();
        let (m, e, c) = (&blocks.motion[0].id, &blocks.emitter[0].id, &blocks.colour[0].id);
        let pal = json!(["#000000", "#ff0000", "#00ff00", "#0000ff"]);
        let input = json!({"genomes": [
            {"motion": m, "emitter": e, "colour": c, "palette": pal},
            {"motion": "nope", "emitter": e, "colour": c, "palette": pal},
            {"motion": m, "emitter": e, "colour": c, "palette": ["red", "#ff0000", "#00ff00", "#0000ff"]},
        ]});
        assert_eq!(parse(&input, &blocks).len(), 1);
    }
}
```

- [ ] **Step 2: main.rs.**
  - Add `mod llm; mod vibe;` and the flags from 6.1's report, plus:

```rust
    /// Describe the visuals in words (needs an AI key: see --setup-ai).
    #[arg(long)]
    vibe: Option<String>,
```

  - If `vibe` is set and `generated == 0`, treat `generated` as `1.0`.
  - If `vibe` is set: `match llm::load()`. On `None`, warn "no AI key set up (run visualiser --setup-ai); continuing without the vibe". On `Some(c)`, call `vibe::genomes(&c, &blocks, &analysis, v, &sibling(&audio, "vibe.json"))`. On `Err`, warn with the message and continue with no seeds.
  - Pass the seeds to `evolve::best(..., &seeds, ...)`, rotating them so each segment starts from a different seed: `seeds[k % n..]` chained with `seeds[..k % n]`, where `k` = segment counter in a `Cell`.
  - Add the vibe text to the plan-reuse check (`Plan.vibe: Option<String>`, `#[serde(default)]`).
  - **AI failure never fails a render.**
- [ ] **Step 3: Wizard.** Add `vibe: Option<String>` to `Answers`. If `llm::load().is_some()`: `Text::new("Describe the vibe (optional, Enter to skip):")`, and an empty answer means None. Otherwise print one dim line: `Tip: run "visualiser --setup-ai" to describe visuals in words.` `to_args` emits `--vibe <text>`. Extend the round-trip test with `vibe: Some("dark and liquid".into())`.
- [ ] **Step 4: Installer.** Add to `visualiser.iss`:

```iss
[Tasks]
Name: setupai; Description: "Set up AI features now (needs an Anthropic or Amazon Bedrock API key; you can do it later)"; Flags: unchecked

[Run]
Filename: "{app}\visualiser.exe"; Parameters: "--setup-ai"; Description: "Set up AI features"; Tasks: setupai; Flags: waituntilterminated

[UninstallRun]
Filename: "{app}\visualiser.exe"; Parameters: "--forget-ai"; RunOnceId: "ForgetAI"; Flags: runhidden
```

  Also add a Start Menu icon: `Name: "{group}\Set up AI"; Filename: "{app}\visualiser.exe"; Parameters: "--setup-ai"`. `--forget-ai` must not pause when hidden: in `main`, skip the "Press Enter" pause when the only argument is `--forget-ai`.
- [ ] **Step 5:** Also wire P7's flag (see Task 7, Step 2). Then `cargo build`. Commit after Wave 5: `git commit -am "feat(ai): bring-your-own-key vibe to genomes (Anthropic or Bedrock)"`

---

## Phase 7: `--author` (dev-only scene writer)  ·  [opus]

**Files:** Create `src/author.rs`. The flag goes into `main.rs` via 6.2's agent.

- [ ] **Step 1: `author.rs`**

```rust
//! Dev tool: describe a scene, the AI writes a full scene (.wgsl + .toml); it is compiled,
//! errors are fed back (3 tries), scored on the given audio, then saved to scenes/.
pub fn run(c: &crate::llm::AiConfig, judge: &mut crate::score::Judge, scenes_dir: &std::path::Path, analysis: &crate::types::Analysis, prompt: &str) -> anyhow::Result<String>;
```

  - **System prompt:** the text of `scenes/_prelude.wgsl`, plus `scenes/liquid_tunnel.wgsl` and `.toml` as the worked example, plus these rules: react to kick/drops, not every band; `P()` never multiplies `time()`; soft luminance cap; ≤ 8 params; palettes are 4 sRGB hex values with a dark first colour.
  - **Tool** `submit_scene`, schema `{id: string, pattern "^[a-z][a-z0-9_]{2,30}$"; wgsl: string; toml: string}`.
  - **Loop up to 3 attempts:**
    1. Reject an id that already exists in `scenes_dir`.
    2. Write both files to `std::env::temp_dir().join("visualiser_author")` and copy `_prelude.wgsl` there.
    3. Parse with `scenes::load_all(temp)`.
    4. `judge.judge(scene, analysis, 60.0f64.min(analysis.duration / 2.0), 4.0)`.
    5. On a compile or parse `Err`, append `"Your previous attempt failed: <error>. Fix it and resubmit."` to the user message and retry.
    6. A `None` score counts as a failure too, with feedback "renders too dark, static, flooded or noisy".
  - **On success:** copy both files to `scenes_dir` and return the id.
- [ ] **Step 2: Flag** (added to `main.rs` by 6.2's agent):

```rust
    /// Dev: have the AI write a new hand-editable scene from a description (needs --setup-ai), then audition it with --dev --scene <id>.
    #[arg(long)]
    author: Option<String>,
```

  Run it after the analysis loads and before planning. On success print `new scene: scenes/<id>.wgsl  →  try: visualiser <audio> --dev --scene <id>` and return.
- [ ] **Step 3: Test** (`author.rs`): a unit test for the id validation helper (`fn valid_id(id: &str, scenes_dir: &Path) -> bool`), covering a good id, a bad pattern and an existing id (`liquid_tunnel`). The live AI loop is covered by the gate smoke run.

---

## Gate M2

- [ ] `cargo build --release` and `cargo test`. Expected: all pass, including the all-combinations compile in `engine_runs`, the score, vibe and llm tests, and the wizard round-trip.
- [ ] `cargo test calibrate -- --ignored --nocapture`. Expected: every hand-made scene prints `Some(..)`. If not, adjust the `score.rs` thresholds (calibration knobs) and re-run only this.
- [ ] Smoke: `target/release/visualiser testdata/dnb5.mp3 --replan --generated 1 --preview 1:00 --quality draft`. Expected: "generating a scene" lines, `testdata/dnb5.gen/*.wgsl`, and a preview MP4. **Ask the user to watch it.**
- [ ] AI smoke, with the user's Bedrock key. **Ask the user** to run `visualiser --setup-ai` (choose Bedrock) or to set `AWS_BEARER_TOKEN_BEDROCK` (+ `AWS_REGION`) in their shell. Then run `target/release/visualiser testdata/dnb5.mp3 --replan --vibe "dark underwater bioluminescence" --preview 1:00 --quality draft`. Expected: `testdata/dnb5.vibe.json` with valid genomes and a preview. If you get 403/404 on the model, set `VISUALISER_AI_MODEL=anthropic.claude-sonnet-5` or enable Sonnet 5.5 access in the Bedrock console.
- [ ] `target/release/visualiser testdata/dnb5.mp3 --author "slow aurora curtains that flare on the kick"`. Expected: a new `scenes/<id>.wgsl`. **Ask the user** to audition it with `--dev --scene <id>`, and delete it if unwanted.
- [ ] `--keep 0` on a generated segment. Expected: `scenes/gen_*.wgsl/.toml` appears and `engine_runs` still passes.
- [ ] `/simplify` → `/review`. Fix the findings, re-run only the affected tests.
- [ ] Docs [sonnet]: `docs/USAGE.md` sections "Generated scenes (`--generated`, `--keep`)", "AI features (`--setup-ai`, `--vibe`, `--forget-ai`, Bedrock/Anthropic keys, env vars, `VISUALISER_AI_MODEL`)", "Writing scenes with `--author`", "Scene blocks" (link to `scenes/blocks/CONTRACT.md`). Update the `package.ps1` staging check: `scenes/blocks/` is copied (it's inside `scenes/`, so already covered). Verify.
- [ ] Commit: `git commit -am "docs: generated scenes, AI features, author tool"`
- [ ] Report: tier T2. List what was built, what was checked, and what was skipped: no `/security-review` (the user chose T2). Recommend running it on `src/llm.rs` before sharing the AI build widely. The branch is ready for the user to merge into `main`.

---

## Self-review notes

- **Spec coverage:**
  - wizard (P1)
  - installer with PATH, shortcuts, right-click menu and static CRT (P2)
  - modulation matrix (P3)
  - layered composition (P4)
  - C evolution + scoring + keep favourites (P5)
  - D bring-your-own-key, with an installer page offering setup now or later, Anthropic default plus Bedrock API key (P6)
  - LLM proposes, C refines: the vibe genomes seed the evolution (P6.2 → P5)
  - dev authoring tool (P7)
  - "LLM writes genome JSON, never WGSL", except dev-only `--author` (P6/P7)
- **Deliberately skipped (ponytail):**
  - 3×3 breeding grid in `--dev`: `--keep` covers "favourite this take". Add the grid if curating via renders gets tedious.
  - OpenAI or other providers.
  - Code signing: friends click "More info → Run anyway".
  - Auto-update.
- **Known ceilings:**
  - GPU float differences can make the scorer pick a different genome on another PC. The saved plan stores genomes, so re-renders on the same plan are identical.
  - Scoring cost is about 20 small renders per generated segment (seconds on a GPU, slow on `--cpu`).
