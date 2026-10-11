# visualiser

Turns a long audio file (DJ mix or production) into an MP4 with music-aware, MilkDrop-style visuals.

## Use

**Full command reference: [docs/USAGE.md](docs/USAGE.md)**: every option, marker formats, plan editing, preview keys, writing scenes, troubleshooting.

**Guided mode:** run `visualiser.exe` with no arguments (double-click or Start Menu) and answer a few questions. It prints the equivalent command, then renders.

**Installer:** `package.ps1` also builds `dist\visualiser-setup-<version>.exe` when Inno Setup 6 is installed: per-user, adds to `PATH`, Start Menu, optional "Make visuals" right-click entry.

**Modulation:** each segment routes kick, bands, energy, tension or drop onto 1-2 scene params (`mods` in the plan file; `fixed = true` in a scene's `.toml` opts a param out).

**Generated scenes:** `--generated <0..1>` makes some scenes from a menu of motion, emitter and colour blocks, each scored on its segment's music. `--keep N` copies one into `scenes/`.

**AI vibe (optional, bring your own OpenAI-compatible API: OpenAI, OpenRouter, Ollama, ...):** `visualiser --setup-ai`, then `--vibe "dark and liquid"` steers the generated scenes. Never required, never fails a render. Details: [docs/AI.md](docs/AI.md).

**Dev:** `--author "<description>"` has the AI write a new hand-editable scene into `scenes/`; audition it with `--dev --scene <id>`.

Drag a WAV, FLAC, MP3 or AIFF file onto `visualiser.exe`. It analyses the audio, plans the visuals and renders. The MP4 appears next to the source file.

Options (command line):

```
visualiser.exe mix.flac                              # defaults: auto mode, final quality, 1080p30, seed 1
visualiser.exe mix.flac --markers mix.cue            # scene changes + titles from a CUE / Audacity labels / text list
visualiser.exe mix.flac --quality draft              # 720p, lighter effects, fast check
visualiser.exe mix.flac --preview 12:30              # render just 30 s from 12:30
visualiser.exe mix.flac --resolution 1440 --seed 7   # 1440p, a different take
visualiser.exe mix.flac --mode single                # one scene, varies on phrases
visualiser.exe mix.flac --reroll 3                   # new take for segment 3 only
visualiser.exe mix.flac --no-titles --no-grain
visualiser.exe mix.flac --debug                      # + click track (click.wav, in the data folder) and analysis overlay video (.debug.mp4)
visualiser.exe mix.flac --cpu                        # force the software renderer (no-GPU benchmark)
visualiser.exe mix.flac --dev [--scene ink_in_water] [--start 1:00]   # live preview window, shaders reload on save
```

Plain-text marker file: one line per marker, `mm:ss Artist - Title`. Add `@scene_id` to force a scene.

## Files written

Next to the audio: `<name>.mp4` (H.264 High, AAC 384k, BT.709), `<name>.preview-<start>s.mp4` (`--preview`) and `<name>.debug.mp4` (`--debug`).

Everything else goes to `%LOCALAPPDATA%\Visualiser\projects\<name>-<hash>\` (hash of the audio file's full path). Every run prints `project data: <folder>`.

| File (in the data folder) | What |
|---|---|
| `analysis.json` | analysis, reused next run (`--reanalyse` to redo) |
| `plan.json` | render plan: edit it, set `"locked": true` on a segment to keep it |
| `vibe.json` | cached AI answer for `--vibe` |
| `chapters.txt` | YouTube chapters, when there are ≥3 sections |
| `click.wav` | click track (`--debug`) |
| `gen\` | generated scene shaders |
| `<video name>.parts\` | in-progress chunks; a killed render resumes from here |

Files from older versions next to the audio are not used any more; delete them.

## Scenes

`scenes/<id>.wgsl` + `scenes/<id>.toml`. The shader contract is in `scenes/_prelude.wgsl`. New scenes need no recompile.

## Build

```
cargo build --release
powershell -File package.ps1   # zip + installer (needs Inno Setup 6), with ffmpeg (put an ffmpeg build + licence in .\ffmpeg\ first)
```

`ffmpeg` must sit next to the exe or be on `PATH`. Design decisions and assumptions: see `ASSUMPTIONS.md`.

Packages contain only committed scenes (`git ls-files scenes`), so uncommitted or AI-written blocks never ship by accident.

**Line endings:** the repo stores text files with CRLF (`core.autocrlf=true`). A tool that rewrites a file with different line endings makes some diff viewers show every line as deleted and re-added, even when only a few lines changed. `git diff` (or `git diff --ignore-cr-at-eol`) shows the real change. Write files back with the line endings they had.
