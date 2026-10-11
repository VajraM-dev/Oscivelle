# visualiser: command reference

`visualiser` turns an audio file (a DJ mix or a single production) into an MP4 with music-aware, MilkDrop-style visuals. It listens to the whole file first, plans the visuals around the music's structure, then renders.

- [Quick start](#quick-start)
- [Guided mode](#guided-mode)
- [Installer](#installer)
- [How a run works](#how-a-run-works)
- [Options](#options): [input](#input), [mode and markers](#mode-and-markers), [output](#output), [variation](#variation-and-repeatability), [re-use](#re-using-saved-work), [developer](#developer-tools), [hardware](#hardware)
- [Recipes](#recipes)
- [Marker files](#marker-files)
- [Editing the plan file](#editing-the-plan-file)
- [Files written](#files-written)
- [Preview window](#preview-window---dev)
- [Writing scenes](#writing-scenes)
- [Generated scenes](#generated-scenes---generated---keep)
- [Motion graphics](#motion-graphics---layers---image---look)
- [Engine 2](#engine-2---engine-2)
- [AI features](#ai-features---setup-ai---vibe---forget-ai)
- [Writing scenes with --author](#writing-scenes-with---author)
- [Scene blocks](#scene-blocks)
- [Troubleshooting](#troubleshooting)

---

## Quick start

**Guided mode:** run `visualiser.exe` with no arguments (double-click it, or use the Start Menu entry) and answer a few questions. See [Guided mode](#guided-mode).

**Drag and drop:** drop a WAV, FLAC, MP3 or AIFF file onto `visualiser.exe`. A console window shows progress and time remaining, and `<name>.mp4` appears next to the audio. When it finishes, the window waits for Enter so you can read any messages.

**Command line:**

```
visualiser.exe <AUDIO> [OPTIONS]
```

```
visualiser.exe "D:\Mixes\set.flac"
```

That runs with all defaults: Auto mode, final quality, 1080p at 30 fps, seed 1, titles and grain on.

`visualiser.exe --help` lists every option. `--version` prints the version.

---

## Guided mode

Run `visualiser.exe` with no arguments, by double-clicking it or from the Start Menu. It asks a few questions: the audio file (a file dialog opens; if you cancel it, type or drag in a path), an optional tracklist, auto or single mode, the visual style, motion graphics, who designs the visuals (core engine or AI), your own pictures, the finishing look, what to make (live preview, full video or a 30-second test file), resolution, and optionally seed, film grain and (with a tracklist) titles. It then prints the equivalent command line, so you can copy it for next time, and renders.

- **Live preview in a window** (the first choice) asks a start time (default 1:00) and opens the [preview window](#preview-window---dev) at draft quality (`--dev --start <time> --quality draft`). Nothing is saved except the plan. Keys: `Space` pause, `R` restart, `D` debug overlay, `Esc` close. When the window closes, a menu offers:
  - **Render the full video with these settings**: asks the resolution, then renders. It reuses the previewed plan (same seed and settings), so the video matches what you saw and the AI vibe isn't paid for again.
  - **Render a 30-second test file**: a draft `--preview` file from the same start time.
  - **Try another take (new seed)**: adds 1 to the seed, shows it, and previews again. The vibe is cached, so a new take costs no AI credit.
  - **Change settings**: asks the questions again with your previous answers selected (the audio file is kept unless you choose another), then previews again, or renders if you pick full video or a test file.
  - **Quit** (or `Esc`).
- Choosing **Full video** or **Quick 30-second test file** directly renders straight away, with no menu.

- **What kind of visuals?** comes first:
  - **Variety** (recommended, `--engine 2`): a new world on each drop, looks that develop with the music, styled transitions. See [Engine 2](#engine-2---engine-2) and [Shows](#shows).
  - **Classic** (`--engine 1`): the original looks.
  - **AI-designed** (only when an AI service is set up): asks for a description (`--vibe`) and uses your AI credit, more when an image model paints pictures. See [AI features](#ai-features---setup-ai---vibe---forget-ai). Without an AI service the wizard prints a tip to run `visualiser --setup-ai`.
- **Scenes?**: freshly generated (`--generated 1`), a mix of hand-made and freshly generated (`--generated 0.5`), or hand-made only (`--generated 0`, not offered for AI-designed). "(recommended)" is on freshly generated for Variety and on the mix for Classic and AI. See [Generated scenes](#generated-scenes---generated---keep).
- **Motion graphics** (generated scenes only): Yes adds scenery behind the visuals (sky, 3D sea world, your pictures) and sometimes a HUD/grid/text on top (`--layers`). No keeps abstract visuals only. With Classic and AI it also adds a new shot at each drop; Variety cuts to a new world on every drop either way, except in Single mode.
- **Your own pictures** is asked after motion graphics or an AI description: pick PNG/JPG files (`--image`, up to 8). Full-frame photos become backdrops, transparent PNGs float as cut-outs.
- Dragging a file onto the exe, or using the "Make visuals" right-click entry, skips the questions and runs with that file.
- In both cases the console stays open when the run ends ("Press Enter to close") so you can read the result or any error.
- `Esc` quits at any question.
- When stdin isn't a terminal (scripts, redirected input) there are no questions and no pause: no arguments behaves like any other command line.

---

## Installer

`package.ps1` builds the release and writes `dist\visualiser.zip`. If Inno Setup 6 is installed (`winget install JRSoftware.InnoSetup`) it also writes `dist\visualiser-setup-<version>.exe`. Without Inno Setup it skips the installer with a note.

```
powershell -File package.ps1
```

Populate `.\ffmpeg\` first (an ffmpeg build plus its licence), as for the zip.

Only scenes committed to git are packaged (`git ls-files scenes`). Blocks the AI wrote on your machine and scenes you're still trying out stay out of the zip and the installer until you commit them.

The installer:

- Installs per user, with no admin prompt, to `%LOCALAPPDATA%\Programs\Visualiser`.
- Adds that folder to your user `PATH` (open a new terminal to use `visualiser` from anywhere) and creates a Start Menu entry.
- Offers an optional desktop shortcut (you can drop audio files on it) and an optional "Make visuals" right-click entry for `.wav`, `.flac`, `.mp3`, `.aiff` and `.aif` files.
- Offers an optional (unchecked) "Set up AI features now" step that runs `--setup-ai` at the end of the install, and adds a "Set up AI" Start Menu entry for doing it later.
- Uninstalls cleanly: the `PATH` entry and the right-click menu are removed, and the stored AI settings are forgotten (`--forget-ai`).
- The uninstaller asks whether to delete your data too: the project data in `%LOCALAPPDATA%\Visualiser\` (analyses, plans, AI answers) and any scenes or blocks you or the AI added. Your videos are never touched. A silent uninstall keeps the data.
- Installing over an older version replaces the built-in scenes and blocks and keeps the ones you or the AI added. Your project data is untouched.

The C runtime is linked statically, so no Visual C++ redistributable is needed. The installer is unsigned: Windows SmartScreen may warn. Choose "More info", then "Run anyway".

---

## How a run works

```
audio ──► Analyse ──► analysis.json ──► Plan ──► plan.json ──► Render ──► <name>.mp4
          (kicks, beats,                        (scenes, palettes,          (GPU, chunked,
           phrases, drops,                       transitions,                resumable)
           track changes)                        phrase variations)
```

Only the videos go next to the audio. `analysis.json`, `plan.json` and the other working files live in the per-track [data folder](#files-written), and every run prints `project data: <folder>`.

1. **Analyse.** Decodes the audio and finds kicks, tempo, beats, downbeats, 8/16/32-bar phrases, band energy, the tension (build-up) curve, drops and track changes. The result is saved and reused next time.
2. **Plan.** Splits the file into segments (one per track or marker), picks a scene, palette and transition for each, and schedules a variation at each phrase and drop. The result is saved and reused next time. You can edit the plan by hand.
3. **Render.** Steps the simulation 60 times a second and writes 30 fps video. Work is saved in 5-minute chunks, so an interrupted render picks up where it stopped. At the end the chunks are joined and the original audio is added.

Because each stage is saved, changing markers, the seed or a single segment re-plans without re-analysing. A render that crashed resumes without re-rendering finished chunks.

---

## Options

### Input

| Argument | Description |
|---|---|
| `<AUDIO>` | **Required.** WAV, FLAC, MP3 or AIFF, up to about 2 hours. Videos are written next to it; working files go to the [data folder](#files-written). |

### Mode and markers

| Option | Values | Default | Description |
|---|---|---|---|
| `--mode <MODE>` | `auto`, `single`, `markers` | `markers` if `--markers` is given, else `auto` | Decides what changes the scene. |
| `--markers <FILE>` | path | none | A CUE sheet, Audacity label file or plain-text list of times. See [Marker files](#marker-files). |

**Modes:**

| Mode | Scene changes | Variation within a scene | Use it for |
|---|---|---|---|
| `auto` | At each detected track change (confidence ≥ 0.35, segments ≥ 30 s) | On 16/32-bar phrases and drops | Mixes, hands-off |
| `single` | Never | On every 8-bar phrase, plus drops | Single productions, one consistent look |
| `markers` | At your marker times, snapped to the nearest downbeat (within 1 bar) or phrase start (within 2 bars) | On 16/32-bar phrases and drops | Full control; the reliable route for mixes with a tracklist |

Auto mode detects a track change when the **key or tempo** changes. Breakdowns and drops inside one track don't count. A long, in-key, beatmatched blend can be missed, so use markers for those mixes.

**Transitions** are chosen from the music:

| Situation | Transition |
|---|---|
| Hard cut | Cut |
| Hard cut on a drop | White flash |
| Long blend | Slow morph between the two scenes, lasting as long as the blend (2–16 s) |
| Marker with no detected change nearby | 4 s morph, or a flash if it's on a drop |

### Output

| Option | Values | Default | Description |
|---|---|---|---|
| `--quality <Q>` | `draft`, `final` | `final` | **final** renders at 1.5× resolution and scales down (clean edges), blends sub-frames for motion blur, and uses full bloom. **draft** renders at output resolution with no motion blur and lighter bloom. Both animate identically. |
| `--resolution <H>` | `720`, `1080`, `1440` | `1080` (`720` with draft) | Output height. Width is 16:9: 1280, 1920 or 2560. |
| `--preview <TIME>` | seconds or `mm:ss` / `h:mm:ss` | none | Render only a **30-second window** starting at TIME. It writes `<name>.preview-<start>s.mp4` and leaves the full video alone. The simulation warms up before the window, so it looks exactly like the same stretch of a full render. |
| `--no-titles` | flag | titles on | Turns off the lower-third "Artist - Title" caption that fades in for a few seconds at each track change. Titles only appear when the marker or CUE file supplies names. |
| `--no-grain` | flag | grain on | Turns off film grain. Fine, fast-changing grain is hard to compress, so A/B it after a test upload. |

**Encode settings** (fixed, following YouTube's recommendations):

- Video: H.264 High, closed GOP of 15 frames, yuv420p, BT.709 tags. About 8 Mbps at 720p/1080p, or 16 Mbps at 1440p.
- Audio: AAC-LC, 48 kHz, 384 kbps, re-encoded from the source. The source file is never modified.

### Variation and repeatability

| Option | Values | Default | Description |
|---|---|---|---|
| `--seed <N>` | integer | saved plan's, else random | Fixes every random choice. Same file + same settings + same seed = the same video. Unset: a plain re-run (and `--replan`/`--reroll`) keeps the saved plan's seed; a new project gets a fresh random seed, printed as `seed N (--seed N makes this take again)`. The guided mode starts every session and every "Try another take" on a fresh random seed. |
| `--reroll <INDEX>` | segment index (0-based, as in `plan.json`) | none | Gives one segment a new take: scene, palette and variations. Every other segment stays the same. Repeat it for several segments: `--reroll 2 --reroll 5`. Re-rolling the same segment again gives yet another take. |

To keep a segment you like, set `"locked": true` on it in the plan file. See [Editing the plan file](#editing-the-plan-file).

### Re-using saved work

| Option | Description |
|---|---|
| `--reanalyse` | Ignores the saved `analysis.json` and analyses again. Also re-plans. Use it after updating the program or when the audio file changed. |
| `--replan` | Ignores the saved plan's **unlocked** segments and plans again from the analysis. Locked segments are kept. |
| `--plan-only` | Stops after writing the analysis, plan and chapters. Doesn't render. Use it to inspect or edit the plan first. |

**When is the saved plan reused?** When all of these are true: the seed, mode and `--pool` match the saved plan, and none of `--markers`, `--reroll`, `--replan`, `--reanalyse` or `--scene` are given. Otherwise a new plan is made. Locked segments always carry over.

### Developer tools

| Option | Values | Description |
|---|---|---|
| `--dev` | flag | Opens the live [preview window](#preview-window---dev) instead of rendering. The audio plays and the visuals render in real time. Scene shaders reload when you save them. |
| `--start <TIME>` | seconds or `mm:ss` | With `--dev`: where playback starts. |
| `--scene <ID>` | scene id (file name without extension) | Forces **one scene for the whole file**, to audition it. Works with `--dev` and with renders (`--preview` gives `<name>.preview-<id>.<start>s.mp4`). The saved plan is **not** overwritten. |
| `--debug` | flag | Also writes `click.wav` (in the data folder; the audio at −6 dB with a click on every kick and a higher click on every downbeat) and `<name>.debug.mp4` (next to the audio; a 720p draft render with an analysis overlay). Use them to check the analysis by ear and by eye. |
| `--scenes <DIR>` | folder | Loads scenes from another folder. Default: `scenes\` next to the program, else `.\scenes`, else the source tree's `scenes\`. |

**Debug overlay** (in `.debug.mp4` and the `D` key in `--dev`): bars for kick, energy, tension, bass, phrase position and drop, boxes for the beat within the bar, and a beat-phase line.

### Hardware

| Option | Description |
|---|---|
| `--cpu` | Forces the software renderer (WARP on Windows). Use it to benchmark or test a no-GPU machine. |

The renderer picks a dedicated GPU, then integrated graphics, then the software renderer automatically. The adapter in use is printed at start-up (for example `GPU: NVIDIA GeForce RTX 3060 Laptop GPU (Vulkan)`).

Measured speeds (RTX 3060 laptop):

| Setting | Speed | One hour of audio |
|---|---|---|
| GPU, final 1080p | 48–63 fps | about 30–40 min |
| GPU, draft 720p | about 98 fps | about 20 min |
| Software (WARP), draft 720p | about 3.3 fps | about 9 h |

---

## Recipes

**Hands-off mix:**
```
visualiser.exe set.flac
```

**Mix with a tracklist (recommended for mixes):** scene changes land on your track times and titles come from the CUE.
```
visualiser.exe set.flac --markers set.cue
```

**Single production, one consistent look:**
```
visualiser.exe track.wav --mode single
```

**Quick check before a long render:** check the drop at 12:30, or check the whole thing at low quality.
```
visualiser.exe set.flac --preview 12:30
visualiser.exe set.flac --quality draft
```

**Try another take:** one segment, or the whole file.
```
visualiser.exe set.flac --reroll 4
visualiser.exe set.flac --seed 7
```

**Lock what you like, then re-roll the rest:**
1. `visualiser.exe set.flac --plan-only`
2. In `plan.json` (in the data folder; the run prints `project data: <folder>`), set `"locked": true` on the segments you like.
3. `visualiser.exe set.flac --seed 8`. Locked segments stay, everything else changes.

**Best quality for YouTube:**
```
visualiser.exe track.wav --resolution 1440
```

**Check the analysis:** listen to `click.wav` (in the data folder) and watch `track.debug.mp4` (next to the audio).
```
visualiser.exe track.wav --debug --preview 1:00
```

**Tune a scene live:**
```
visualiser.exe track.wav --dev --scene ink_in_water --start 1:05
```
Then edit `scenes\ink_in_water.wgsl` and save. The window reloads it.

**Benchmark a no-GPU machine:**
```
visualiser.exe track.wav --cpu --quality draft --preview 0
```

**Resume after a crash or shutdown:** run the same command again. Finished 5-minute chunks are skipped.

---

## Marker files

The format is detected from the content. Every format supports `@scene_id` anywhere in a name to force that scene for the segment, for example `@ink_in_water`. The tag is removed from the displayed title. Unknown scene ids are ignored with a warning.

A marker at or near 0:00 names the opening segment. Markers within 1 s of the previous one, or within 1 s of the end, are dropped.

Files may be UTF-8 (with or without a BOM) or Latin-1.

### CUE sheet

Detected by `TRACK` lines. Titles are `PERFORMER - TITLE` (the track's performer, else the file-level one).

```
PERFORMER "Various"
FILE "set.wav" WAVE
  TRACK 01 AUDIO
    PERFORMER "Artist A"
    TITLE "Opener"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    PERFORMER "Artist B"
    TITLE "Second One @kaleidoscope_bloom"
    INDEX 01 04:31:37
```

`INDEX 01 mm:ss:ff` uses CUE frames, 75 per second.

### Audacity label file

From Audacity: *File → Export → Export Labels*. Each line is `start<TAB>end<TAB>label`, in seconds.

```
0.000000	0.000000	Artist A - Opener
271.493000	271.493000	Artist B - Second One
```

### Plain text

One marker per line: a time, then an optional name. Lines starting with `#` are comments.

```
# my tracklist
0:00      Artist A - Opener
4:31      Artist B - Second One
[9:02]    Artist C - Third @plasma_field
1:02:15.5 Artist D - Closer
754.2
```

Accepted times are `h:mm:ss`, `mm:ss`, `m:ss.s` or plain seconds (`754.2`), optionally in `[ ]`.

---

## Editing the plan file

`plan.json` is plain JSON and is meant to be read and edited. It is in the per-track data folder; every run prints `project data: <folder>`. After editing, run the same command again (same seed and mode, no `--replan`) and it is used as written.

```jsonc
{
  "version": 1,
  "source": "set.flac",
  "seed": 1,
  "mode": "Markers",
  "segments": [
    {
      "start": 0.0, "end": 271.49,       // seconds; segments are contiguous
      "scene": "liquid_tunnel",          // any id from scenes\
      "seed": 1234567890,                // per-segment seed (changed by --reroll)
      "locked": false,                   // true = kept through --replan, re-seeds and re-rolls
      "palette": 2,                      // index into the scene's palettes
      "title": "Artist A - Opener",       // caption text, or null for none
      "transition_in": { "kind": "Cut", "duration": 0.0 },   // Cut | Flash | Morph (duration s)
      "variations": [                    // phrase-level changes inside the segment
        { "time": 0.0, "params": [0.004, 0.12], "hue_shift": 0.0 }
      ],
      "mods": [                          // audio modulation of params, on top of the variations
        { "param": 0, "source": "Kick", "depth": 0.3 }
      ]
    }
  ]
}
```

What you can change safely:

- **scene**: swap in any scene id.
- **palette**: pick another palette of that scene, `0` to (number of palettes − 1).
- **title**: change or remove the caption.
- **transition_in**: for example, change a `Morph` to a `Flash`.
- **locked**: protect a segment.
- **variations[].params**: stay inside the ranges in the scene's `.toml`.
- **variations[].hue_shift**: in turns, 0..1.
- **mods**: see below.

### Audio modulation (`mods`)

Each segment has a list of `mods` that tie scene params to the music. The director picks one or two per segment at plan time, on random params that aren't `fixed`. Each entry has:

| Field | Meaning |
|---|---|
| `param` | Which param, as an index in the scene's `.toml` order (`0` = the first `[[param]]`). |
| `source` | `Kick`, `Bass`, `Mid`, `High`, `Energy`, `Tension` or `Drop`. |
| `depth` | How far the param moves, as a fraction of its `min`..`max` range at a source value of 1. Negative inverts. |

The movement is added to the param's current value (including its phrase variation) and clamped to the range. `Bass`, `Mid`, `High` and `Energy` are levels centred on their midpoint, so they push the param both ways around its value. `Kick`, `Tension` and `Drop` are envelopes that rest at 0 and only push one way (up for positive depth). Mods on a `fixed` param, or on a param index the scene doesn't have, are ignored. Remove an entry, or set `"mods": []`, to turn modulation off for that segment.

Plans saved by older versions have no `mods` and render without modulation. Run with `--replan` to get them (locked segments keep their old settings).

Leave `start` and `end` contiguous. To move a boundary, prefer a marker file.

A change to the plan, the settings, the analysis or a used scene's shader automatically invalidates old render chunks.

---

## Files written

**Next to the audio** only videos are written:

| File | When | What |
|---|---|---|
| `<name>.mp4` | full render | The video. |
| `<name>.preview-<start>s.mp4` | `--preview` | The 30-second preview (`<name>.preview-<scene>.<start>s.mp4` when `--scene` is used). |
| `<name>.debug.mp4` | `--debug` | Analysis overlay video. |

**Everything else** goes to the per-track data folder `%LOCALAPPDATA%\Visualiser\projects\<name>-<hash>\`. The hash is taken from the audio file's full path, so same-named files in different folders don't collide. Every run prints `project data: <folder>`. Files inside have no name prefix:

| File | When | What |
|---|---|---|
| `analysis.json` | always | Saved analysis, reused next run. |
| `plan.json` | always (except with `--scene`) | Saved plan, editable. |
| `chapters.txt` | at least 3 sections, each at least 10 s | YouTube chapters (`00:00 Title` lines), first at 00:00, ready to paste into the description. |
| `click.wav` | `--debug` | Click track. |
| `gen\` | plan with generated scenes | The generated scenes' shaders. Rebuilt from the plan; a new plan removes the ones it no longer uses. |
| `vibe.json` | `--vibe` | The AI's answer for your vibe text, cached. |
| `<video name>.parts\` | during a render | Finished chunks. Deleted after a successful render. Safe to delete by hand to force a clean render. |

Files from older versions next to the audio (`<name>.analysis.json`, `<name>.plan.json`, `<name>.parts\` and so on) are not used any more. Delete them.

---

## Preview window (`--dev`)

A 1280×720 window that plays the audio and renders live. The title bar shows the current time and the scene (and the incoming scene during a morph).

| Key | Action |
|---|---|
| `Space` | Pause / resume (audio and visuals) |
| `R` | Restart from `--start` |
| `D` | Toggle the debug overlay |
| `Esc` | Quit |

**Hot reload:** saving any `scenes\*.wgsl` file, or `scenes\_prelude.wgsl`, recompiles it within half a second. If the shader has an error, the error is printed to the console and the last working version keeps running.

Notes:

- Bloom, aberration, vignette and grain amounts are compiled into the program (`src\render\finish.wgsl`), so changing them needs a rebuild.
- The preview can't play AIFF. Rendering AIFF works.
- Resizing the window restarts the trails.

---

## Writing scenes

A scene is two files in `scenes\` with the same name: `<id>.wgsl` and `<id>.toml`. Scenes are loaded at start-up, so a new scene needs no recompile.

**`<id>.toml`:**

```toml
name = "Liquid tunnel"          # label only
palettes = [                    # 1+ palettes, exactly 4 sRGB hex colours each
  ["#05010f", "#3a0ca3", "#f72585", "#4cc9f0"],
]

[[param]]                       # up to 8; order = P(0)..P(7) in the shader
name = "trail_drift"            # label only
min = 0.0                       # the director picks values inside [min, max]
max = 0.01                      # and morphs between them on phrases. Keep both ends good-looking.
# fixed = true                  # optional: the director never audio-modulates this param
```

Set `fixed = true` on params that would look wrong if they moved with the music: integer params (the shader uses `floor` or `round`), params that multiply `time()` (the phase would jump), and hue offsets. Other params may be modulated by the plan's [`mods`](#audio-modulation-mods).

**`<id>.wgsl`:** define exactly one entry point:

```wgsl
fn scene(uv: vec2<f32>) -> vec4<f32> {
    // uv: 0..1, (0,0) = top-left. Return the NEW feedback colour (linear HDR; alpha ignored).
    let back = prev(warp(uv, 1.01, 0.002, 0.0, vec2<f32>(0.0))) * 0.96;   // warp + fade previous frame
    let p = to_centered(uv);
    let ring = line_glow(p, vec2<f32>(-0.5, 0.0), vec2<f32>(0.5, 0.0), 0.01 + 0.02 * kick());
    return vec4<f32>(back + pal(time() * 0.05) * ring, 1.0);
}
```

It runs 60 times a second on an HDR feedback buffer. The finish pass does tonemapping, bloom, aberration, vignette, grain and dithering, so don't do those in the scene.

**Available from `_prelude.wgsl`:**

| Group | Functions |
|---|---|
| Time | `time()`, `dt()`, `resolution()`, `aspect()`, `quality()` (0 draft / 1 final), `seed()` (0..1 per segment) |
| Beat | `kick()` (decaying kick envelope 0..1), `since_kick()` (s), `beat_phase()`, `bar_phase()`, `phrase_phase()` (0..1) |
| Bands | `sub()`, `bass()`, `mid()`, `high()` (0..1, levelled per track) |
| Shape | `energy()`, `tension()` (rises across a build, 0 at the drop), `drop_env()` (1 at a drop, decays), `since_drop()` (s) |
| Params | `P(i)`: the scene's params 0..7 |
| Feedback | `prev(uv)`, `warp(uv, zoom, angle, swirl, center)` |
| Geometry | `to_centered(uv)`, `to_uv(p)`, `rot2(a)`, `line_glow(p, a, b, width)` |
| Colour | `pal(t)` (cyclic 4-colour palette with the phrase hue shift), `hue_rotate(c, turns)` |
| Noise | `hash21(p)`, `noise2(p)`, `fbm(p)` |

**House rules** (these are why the visuals don't look like noise):

- The kick is the main driver. Link at most 2–3 things to audio, and use `high()`/`mid()` sparingly.
- Use `tension()` for anticipation (push in, tighten, calm the colour) and `drop_env()` for the release.
- Low `energy()` (breakdowns) means calmer motion.
- Balance fade against emission so trails never wash to a flat fill or die to black.
- Mark params that must never be audio-modulated with `fixed = true` (see below).
- Don't multiply `time()` by a param. Params morph, so the phase would jump. Apply speeds per step (`* dt()`) instead.

Audition with `--dev --scene <id>`, or render `--scene <id> --preview <time>` and look at frames.

**Included scenes:** `liquid_tunnel`, `kaleidoscope_bloom`, `fractal_zoom`, `ink_in_water`, `neon_geometry`, `plasma_field`, `starfield_warp`.

---

## Generated scenes (`--generated`, `--keep`)

Besides the hand-made scenes, the program can build new ones from a library of parts: a **motion** (how the picture flows), an **emitter** (what is drawn), a **colour block** and a palette. This needs no AI and no key.

| Option | Values | Default | Description |
|---|---|---|---|
| `--generated <SHARE>` | `0` to `1` | the saved plan's value, else `1` for new engine-2 projects (`0.5` for engine 1) | Share of the auto-picked segments that get a generated scene. `0` = only hand-made scenes, `1` = all generated. Saved projects keep their own value. |
| `--keep <N>` | segment index (0-based, as in `plan.json`) | none | Copies segment N's generated scene into `scenes\` as a normal scene. Doesn't render. |

- Each generated scene is chosen by scoring candidates on **that segment's own music**. The judge renders a few seconds of each candidate small and rejects the dull, the flooded (washed to white), the noisy (TV static) and the deaf (not reacting to the kick). It catches bad ones, not great ones, so you pick favourites with `--keep`.
- The scenes are written to `gen\` in the data folder. The plan stores each scene's recipe, so re-rendering from the saved plan gives the identical video.
- A marker's `@scene_id` and `--scene` always win over generated scenes.
- `--generated` outside 0..1 is an error. A plain re-run reuses the saved plan's share; a different `--generated` makes a new plan.
- If the judge can't start (for example no GPU), the plan falls back to hand-made scenes with a warning.

**Keep a look you love:** `--keep` acts on the **saved plan** (the one your video was rendered from) and never re-plans. It errors if there is no saved plan, the segment doesn't exist, or that segment isn't generated.
```
visualiser.exe set.flac --keep 3
```
The scene is written to `scenes\gen_<id>.wgsl` and `.toml`. After that it is a normal hand-editable scene (see [Writing scenes](#writing-scenes)) you can use with `--scene`, `@scene_id` or in the plan.

**Auto picks stay on the shipped scenes.** The director only picks the 7 hand-made scenes on its own (Auto, Single and `--reroll`), so a seed gives the same video however many scenes you keep or author. Kept and `--author` scenes are reachable by name (`--scene`, `@scene_id`, the plan). To let the director pick them too, add `--pool all`; this changes which scenes your seeds pick, is saved in the plan (a plain re-run keeps it) and `--pool builtin` switches back.

**Known limit:** scoring runs on your GPU, so a different GPU may pick a different scene when it plans again. A saved plan always renders the same.

---

## Motion graphics: `--layers`, `--image`, `--look`

Layers put motion graphics on top of the generated scenes' feedback visuals. Everything is built in: no AI, no key, no download.

| Option | Values | Default | Description |
|---|---|---|---|
| `--layers` | flag | off | Dresses generated scenes with a **backdrop**: `back_sky` (gradient sky), `back_sea` (raymarched sea world) or `back_plate` / `back_sprite` (show your `--image`). Some also get an **overlay**: `over_hud` (rings), `over_grid` (retro grid) or `over_title` (pixel text). Adds a new shot on each drop (a flash) and each build-up (a morph), snapped to phrases. Turns generated scenes on (share `1`) if `--generated` would be `0`. |
| `--image <FILE>` | png or jpeg path | none | A picture the backdrops can show. Repeatable, up to 8; extra ones are dropped with a warning. Only used with `--layers` (a warning says so otherwise). Each picture is centre-cropped and fitted to 3:2 (1536x1024). An unreadable file stays blank, with a warning. |
| `--look <LOOK>` | `clean`, `vhs`, `crt`, `mono` | none (same as `clean`) | Finishing look for the whole video: VHS wobble and colour bleed, CRT scanlines, or a mono tint. |

- `--look` is a render setting like `--no-grain`: set it on every run, it is not saved in the plan or reused.
- `--layers` and the `--image` list are part of the plan: changing either makes a new plan. A plain re-run with the same ones reuses the saved plan.
- Without these flags nothing changes: the default render is the same as before.
- `--keep N` exports a layered look too: `scenes\gen_<id>.layers.wgsl` is copied next to the `.wgsl` and `.toml`.

```
visualiser.exe song.wav --replan --layers --preview 1:00 --quality draft
visualiser.exe song.wav --replan --layers --image clouds.jpg --look vhs --preview 1:00 --quality draft
visualiser.exe song.wav --replan --layers --look mono --preview 1:00 --quality draft
```

---

## Engine 2: `--engine 2`

The default generator for new projects. It makes every `--seed` a structurally different piece. Without AI, each retry changes the fold, motion, emitter, blend, mood and palette together.

| Option | Values | Default | Description |
|---|---|---|---|
| `--engine <N>` | `1`, `2` | the saved plan's value, else `2` | `2` evolves generated scenes with the engine-2 parts below. `1` = the original (classic) looks. |

What it adds:

- **Space folds:** the plane is folded (mirrors, kaleidoscopes, hyperbolic folds) before the motion and the emitter see it, so trails and shapes inherit the symmetry.
- **Blend modes:** how the new emission is laid onto the fading previous frame.
- **Six moods:** `calm`, `dark`, `mono`, `sparse`, `glitch` and `maximal`. A mood sets how fast trails fade, how hard the emitter is driven and how the finish pass looks. The judge uses each mood's own brightness and motion limits, so a `dark` look isn't rejected for being dark.
- **19 new blocks** in `sceneslocks\`: space folds, feedback operators and emitters. See [Scene blocks](#scene-blocks).
- **Onset hits:** snares and kicks drive the visuals directly (a snare envelope, a sharp kick punch, time since the last snare), not only the smoothed kick.
- **Drop build-ups:** the picture darkens half a bar before a drop and explodes on it.
- **Glitch looks:** strobe, RGB split, tear and invert, on moods `glitch` and `maximal` only. They are gated on downbeats and snares.

Notes:

- `--generated` still sets how many segments are generated. Use `--generated 1` to make every segment engine 2.
- **New projects use engine 2.** Projects with a saved plan keep their engine (plans saved before engine 2 existed are engine 1). `--engine 1` forces the classic looks.
- The engine is saved in the plan, so a plain re-run (and a render from the saved plan) keeps using it. Changing `--engine` makes a new plan.
- The guided mode's first question, **What kind of visuals?**, picks the engine (Variety = engine 2, Classic = engine 1).
- **Engine 1 is unchanged.** `--engine 1` output is the same as before. New blocks, AI blocks and `--keep` scenes never alter engine-1 seeds.
- **Re-analysis:** after upgrading, each project is analysed once again (the analysis format gained snare onsets). This applies to all runs, not just engine 2.
- **Flash limit:** strobe and invert flashes are limited to at most one per bar, under the 3 Hz limit in WCAG.
- `--vibe` keeps engine-1 looks.

### Shows

Engine 2 also plans a whole show, not only single looks:

- **Drop resets:** at a drop the picture goes black, then a new look rebuilds from nothing. Single mode (`--mode single`) keeps one look for the whole track, so it gets drop resets only with `--layers`.
- **Phrase variants:** every 16 or 32 bars a look develops into a variant (a changed fold, framing or palette). Variants are not chapters. They only vary over the frozen engine-2 block library, so new block files on disk don't change seeds, and they only apply to engine-2 looks. `--reroll N` of a variant re-rolls its parent segment. Locking a generated segment, or any of its variants, locks the whole look including its variants.
- **Transition styles:** crossfade, luma wipe, iris zoom-through, melt or tiles. The seed picks.
- **Comp framings:** mirror, kaleido, slit-scan and time echo, applied to the display only (the feedback loop doesn't see them). Slit-scan rows lag in time; the echo leaves ghost trails.
- **Recipes:** `stateless` (no feedback buffer, the picture is drawn fresh each frame) and `dual` (two emitters at once).
- **Default share:** `--generated` defaults to `1` for new engine-2 projects (`0.5` for engine 1). Saved projects keep theirs.
- **Memory:** a history ring of past frames feeds slit-scan and echo, about 133 MB at 1080p and 236 MB at 1440p. It is only allocated when a look uses one, and is cleared on a reset cut and on preview seek.
- **Planning takes a bit longer:** one judged look per drop.

```
visualiser.exe song.wav --engine 2 --generated 1 --replan --preview 1:00 --quality draft --seed 3
```

---

## AI features (`--setup-ai`, `--vibe`, `--forget-ai`)

Optional. Full details: [AI.md](AI.md). You bring your own **OpenAI-compatible API**: OpenAI, OpenRouter, Groq, Together, a local Ollama / LM Studio / vLLM server, or anything else that serves `/chat/completions`. Answers are requested as strict structured output (`response_format` JSON schema), so the output always has the expected shape; servers without it get a tool call instead. The model needs one of the two (most current models have both). Without one, everything else works as before.

| Option | Description |
|---|---|
| `--setup-ai` | Asks for the API base URL (default `https://api.openai.com/v1`; e.g. `https://openrouter.ai/api/v1`, `http://localhost:11434/v1` for Ollama), the key (hidden as you type; leave empty for local servers that need none) the model id and, optionally, an image model id (e.g. `gpt-image-1`, OpenAI image models only; leave empty for no AI pictures). It tests the model with a tiny request (not the image model, which costs money) and stores them in **Windows Credential Manager**. The installer's "Set up AI features now" checkbox and the Start Menu "Set up AI" entry run the same thing. |
| `--vibe "<TEXT>"` | Describe the visuals in your own words, for example `--vibe "dark and liquid, slow"`. The AI tunes the blocks' settings to the vibe, picks looks that the generated scenes then evolve from (still scored on your music), and writes new shader blocks when the library can't express what you asked for. If the share of generated scenes would be 0, `--vibe` makes it 1, since a vibe only steers generated scenes. |
| `--forget-ai` | Removes the stored settings. Uninstalling does this too. |

**Environment variables** are used when nothing is stored (`VISUALISER_AI_MODEL` also overrides a stored model):

| Variable | Use |
|---|---|
| `VISUALISER_AI_MODEL` | The model id. Required for the env-var setup; on its own it overrides the stored model. |
| `OPENAI_API_KEY` | The key. |
| `OPENAI_BASE_URL` | The base URL (default `https://api.openai.com/v1`). |
| `VISUALISER_IMAGE_MODEL` | The image model id. Overrides a stored one. |

With nothing stored, the env vars are used when `VISUALISER_AI_MODEL` is set together with `OPENAI_API_KEY` or `OPENAI_BASE_URL`.

- The blocks are a starting vocabulary, not a limit. The AI tunes their settings to the vibe, keeps the evolution to looks that fit, and writes new shader blocks (motion, emitter or colour) when the library can't express what you asked for.
- New blocks are compile-checked and test-rendered (3 attempts each). They are saved in `scenes\blocks\` and become part of the library for later renders. Delete a block's `.wgsl` and `.toml` to remove it.
- After the AI call it prints one line about what the visuals will look like, and any new blocks it wrote.
- The music still picks the winner among the on-vibe looks.
- The answer is cached in `vibe.json` in the data folder (the note and the looks). The same vibe text on the same file makes no new call, so re-renders are identical. A new text asks again.
- Cost: the first use of a new vibe can make several AI calls, one design call plus up to 3 per new block.
- AI trouble never fails a render. With no key, or if the call fails, you get a warning and the scenes are evolved without the vibe.
- The key is never printed and never written to project files. It is sent only to the AI service.

---

## Writing scenes with `--author`

A developer tool. Describe a scene and the AI writes a complete, hand-editable scene (`.wgsl` + `.toml`, the format in [Writing scenes](#writing-scenes)). It needs an AI service and the audio file (for scoring).

```
visualiser.exe track.wav --author "slow aurora curtains that flare on the kick"
```

- The scene is compile-checked, with errors fed back for up to 3 tries, then scored on your audio by the same judge as [generated scenes](#generated-scenes---generated---keep). Too dark, static, flooded or noisy counts as a failed try.
- On success it is saved to `scenes\<id>.wgsl` and `.toml`, and the program prints the command to audition it:
  ```
  visualiser.exe track.wav --dev --scene <id>
  ```
- Nothing is rendered. If you don't like the result, delete the two files.
- Like `--keep` scenes, it is only auto-picked with `--pool all` (see [Generated scenes](#generated-scenes---generated---keep)).
- With no key set up, or after 3 failed tries, it stops with an error.

---

## Scene blocks

The parts generated scenes are built from live in `scenes\blocks\`. A motion, emitter or colour block is `<id>.wgsl` + `<id>.toml` there. The format, the functions each kind must define and the rules are in [`scenes/blocks/CONTRACT.md`](../scenes/blocks/CONTRACT.md).

To add a block: copy an existing block of the kind you want, rename it and prefix its helper functions with the new id, then edit the `.toml` (`kind`, a one-line `description` the AI sees, at most 3 / 3 / 2 params for motion / emitter / colour). Blocks load at start-up, so no recompile is needed. With `--vibe`, the AI can also write blocks here; they follow the same format.

---

## Troubleshooting

| Message / symptom | Fix |
|---|---|
| `scenes folder ...: cannot find the path` | Run from the folder containing `scenes\`, keep `scenes\` next to the exe, or pass `--scenes <DIR>`. |
| `ffmpeg` not found / mux failed | Put `ffmpeg.exe` next to `visualiser.exe` or on `PATH`. |
| `markers mode needs --markers <file>` | `--mode markers` was given without a marker file. |
| `unknown scene X` | Check the id against the file names in `scenes\` (no extension). |
| `--preview start ... is at or past the end` | Pick an earlier start. The window is 30 s, cut short at the end of the file. |
| `resolution must be 720, 1080 or 1440` | Only these heights are supported. |
| Edited the plan but nothing changed | Run with the same `--seed` and `--mode`, and without `--replan`, `--reroll` or `--markers`. Otherwise the plan is regenerated (only locked segments survive). |
| Scene changes in the wrong places in a mix | Use a marker file (CUE from your DJ software, or a text list). Auto detection misses in-key blends. |
| Analysis looks wrong after an update | `--reanalyse`. |
| Render is very slow | Check the adapter line at start-up. `Microsoft Basic Render Driver` / WARP means no GPU, so use `--quality draft`. |
| Want a clean re-render | Delete the `<video name>.parts\` folder in the data folder (and `plan.json` there too for a fresh plan). The path is printed as `project data: <folder>` on every run. |
