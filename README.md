# visualiser

Turns a long audio file (DJ mix or production) into an MP4 with music-aware, MilkDrop-style visuals.

## Use

**Full command reference: [docs/USAGE.md](docs/USAGE.md)**: every option, marker formats, plan editing, preview keys, writing scenes, troubleshooting.

**Guided mode:** run `visualiser.exe` with no arguments (double-click or Start Menu) and answer a few questions. It prints the equivalent command, then renders.

**Installer:** `package.ps1` also builds `dist\visualiser-setup-<version>.exe` when Inno Setup 6 is installed: per-user, adds to `PATH`, Start Menu, optional "Make visuals" right-click entry.

**Modulation:** each segment routes kick, bands, energy, tension or drop onto 1-2 scene params (`mods` in the plan file; `fixed = true` in a scene's `.toml` opts a param out).

**Generated scenes:** `--generated <0..1>` makes some scenes from a menu of motion, emitter and colour blocks, each scored on its segment's music. `--keep N` copies one into `scenes/`.

**AI vibe (optional, bring your own Anthropic or Bedrock key):** `visualiser --setup-ai`, then `--vibe "dark and liquid"` steers the generated scenes. Never required, never fails a render.

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
visualiser.exe mix.flac --debug                      # + click track (.click.wav) and analysis overlay video (.debug.mp4)
visualiser.exe mix.flac --cpu                        # force the software renderer (no-GPU benchmark)
visualiser.exe mix.flac --dev [--scene ink_in_water] [--start 1:00]   # live preview window, shaders reload on save
```

Plain-text marker file: one line per marker, `mm:ss Artist - Title`. Add `@scene_id` to force a scene.

## Files written next to the audio

| File | What |
|---|---|
| `<name>.mp4` | the video (H.264 High, AAC 384k, BT.709) |
| `<name>.analysis.json` | analysis, reused next run (`--reanalyse` to redo) |
| `<name>.plan.json` | render plan: edit it, set `"locked": true` on a segment to keep it |
| `<name>.chapters.txt` | YouTube chapters, when there are ≥3 sections |
| `<name>.parts/` | in-progress chunks; a killed render resumes from here |

## Scenes

`scenes/<id>.wgsl` + `scenes/<id>.toml`. The shader contract is in `scenes/_prelude.wgsl`. New scenes need no recompile.

## Build

```
cargo build --release
powershell -File package.ps1   # zip + installer (needs Inno Setup 6), with ffmpeg (put an ffmpeg build + licence in .\ffmpeg\ first)
```

`ffmpeg` must sit next to the exe or be on `PATH`. Design decisions and assumptions: see `ASSUMPTIONS.md`.
