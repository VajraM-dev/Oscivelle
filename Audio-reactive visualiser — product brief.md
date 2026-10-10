# Audio-reactive visualiser — product brief

Oct 10, 2026 · @Prathamesh Kulkarni

## Summary

A single Windows program turns an hour-long DJ mix or production into an MP4 with trippy, music-aware visuals, with no manual work needed.

It listens to the whole file first, then plans the visuals around the music's structure: the kick drives the pulse, phrases drive variation, and drops and track changes drive scene changes. The look is MilkDrop-style liquid feedback with a modern finish. It renders on any PC; a GPU makes it fast, and without one it is slow but works.

## Problem

Sets and productions need visuals for uploads and projection, and making them by hand for an hour of audio is not practical.

Earlier attempts, built in HTML, failed on quality for three reasons:

- **Too basic.** Simple lines, dots and flat colours, with no depth or finish.
- **Reacted to everything.** Every sound moved something, so the result was noise. A DJ watches the 4/4 kick, not every hi-hat.
- **Weak rendering platform.** A browser canvas limited both effects and render quality.

The lesson that shapes this brief: visual quality is the main risk, so it is tested first.

## Goals and non-goals

Version 1 makes one thing well: an offline render of a long audio file into a striking, music-aware video.

**Goals**

- Visuals good enough to put on top of your own uploads without embarrassment.
- Visual changes that follow the music's structure, not just its loudness.
- Hands-off by default: drop in a file, get a video.
- Hand it to friends as one download that runs on Windows with or without a dedicated GPU.

**Not in version 1**

- Live or real-time visuals, including output to Resolume or OBS.
- Stem separation (splitting drums, bass and vocals). A candidate for later.
- An AI or LLM "director". Plain rules come first.
- Running existing MilkDrop presets through projectM.
- A full graphical editor. Markers come from files, not a timeline UI.
- macOS and Linux builds. The code should not rule them out.

## Users and workflow

Two users: you, on a Windows PC with a GPU, and friends who may have only integrated graphics or none at all.

The default workflow needs no decisions:

1. Drag an audio file onto the program.
2. It analyses the audio, plans the visuals and renders, showing progress and time remaining.
3. An MP4 with the original audio appears next to the source file.

Optional choices, all with defaults: the visual mode, a marker file, quality (draft or final), resolution and a seed. The same file, settings and seed always give the same video; a new seed gives a different take.

## Features

### Input

- WAV, FLAC, MP3 and AIFF, up to about 2 hours.
- Mixed DJ sets and single long productions.

### Analysis

- **Kick and the 4/4 pulse first.** Detect kicks specifically, not just overall loudness. This is the main driver of all motion.
- **Beat grid.** Tempo, beats, downbeats, and phrases of 8, 16 and 32 bars.
- **Bands.** Energy in sub, bass, mids and highs, smoothed and levelled per track.
- **Energy and drops.** A continuous tension curve plus drop detection, which is what the director actually uses. Named section labels (intro, breakdown, outro) come later.
- **Track changes** in mixes, each with a confidence score.
- **Debug output.** A click track and an overlay video, so the analysis can be checked by ear and eye.

### Visual modes

| Mode | What changes the scene | Use it for |
| --- | --- | --- |
| Auto (default) | Each detected track change; drops add bigger moments within a scene | Mixes, hands-off |
| Single | Never; one scene varies on phrases and intensifies on drops | A consistent look, single productions |
| Markers | Your timestamps, snapped to the nearest downbeat or phrase | Full control over where changes land |

Markers can be loaded from a CUE file, an Audacity label file, or a plain text list of times. A marker can optionally name the scene to use. When a mix has a tracklist, Markers is the reliable route; Auto detection is a convenience for when it doesn't.

### Output

- MP4 (H.264, encoded with x264) at 1080p30 by default, with a 1440p option.
- YouTube's recommended encode settings built in: H.264 High, closed GOP, about 8 Mbps at 1080p and 16 Mbps at 1440p, BT.709 colour tags. Audio is re-encoded to AAC-LC at 48 kHz, 384 kbps; the source file is untouched.
- A YouTube chapters file from the detected track changes or markers (first chapter at 00:00, at least 3 chapters, each at least 10 s), ready to paste into the description.
- Track titles: a lower-third caption (artist and title) fades in for a few seconds at each track change. Names come from the CUE or marker file, so titles appear only when names exist; one setting turns them off.
- Draft quality: lower resolution and lighter effects, for quick checks and slow machines.
- Preview: render any 30-second window before committing to the full hour.
- Renders resume after a crash or shutdown instead of starting over.

## How the dynamic visuals work

The visuals feel designed because each timescale in the music drives a different kind of change, and the whole file is known before rendering starts.

| Timescale | Detected | Visual response |
| --- | --- | --- |
| Beat | Kick, 4/4 pulse | Pulse: zoom kick, brightness flash, warp push |
| Phrase (8–32 bars) | Phrase boundaries, fills | Variation: camera move, palette shift, a layer in or out |
| Section | Builds and drops | Tension and release within the scene |
| Track or marker | Track change, your marker | New scene, with a transition style that fits |

The director turns the analysis into a render plan using these rules:

- **Kick first.** The kick is the main driver. Each scene links at most 2–3 parameters to audio, and hats or mids are used sparingly.
- **Anticipation.** A build is known in advance, so tension rises across it: zoom pushes in, colour drains, feedback tightens. Everything releases exactly on the drop.
- **Contrast.** Breakdowns go calm so drops land harder. Intensity follows energy, levelled per track.
- **Transitions follow the music.** A drop gets a hard cut or flash; a long blend gets a slow morph between scenes.
- **No repeats.** Recent scenes are remembered, and each track gets its own palette and character.
- **Repeatable.** A seed fixes every random choice.

The plan is saved as a readable file. You can inspect or tweak it, and re-render without re-analysing. You can also lock a scene or re-roll the seed for a single track without changing the rest.

## Visual quality

The look is MilkDrop's liquid feedback, finished to a modern standard. Quality comes from the engine's finish and from hand-tuned scenes, never from randomness.

### The engine

- **Feedback core.** Every frame, the previous frame is warped through a mesh (zoom, rotate, swirl), its colours fade slightly, and new shapes are drawn on top. Fade and colour behaviour are tuned per scene, so trails never wash out to grey or white.
- **Smooth colour.** Colours are stored at high precision throughout, which removes banding (visible steps in gradients). Proper tonemapping makes bright areas roll off softly instead of clipping.
- **Curated palettes**, designed for each scene rather than picked at random.
- **Finish pass.** Bloom, gentle chromatic aberration, vignette, light grain and dithering.
- **Final quality only.** Supersampling for clean edges, and real motion blur from sub-frames.
- **Optional geometric layer.** AVS-style lines and shapes drawn into the feedback.

### Scene library

The library sets the quality ceiling, so it is curated like synth presets. Each scene ships with ranges that are known to look good, and variation stays inside them.

Version 1 aims for 6–8 scenes. Candidates:

- Liquid tunnel
- Kaleidoscope bloom
- Fractal zoom
- Ink in water
- Neon geometry trails
- Plasma field
- Starfield warp

### Surviving YouTube

- Export at YouTube's recommended bitrates (see Output).
- Offer 1440p. An older test found 1440p uploads got a better codec, but this is unverified for 2026; Phase 0 tests it.
- Test grain on and off in Phase 0. Fine, fast-changing detail is the hardest thing to compress, so grain may be smeared away or waste bitrate.
- Check finished renders after a test upload, not only the local file.

## Architecture

One Rust program does everything, shipped as a single zip. There is no Python and no Docker.

&#91;embedded content: render pipeline · 4 stages, 2 saved files\]

A marker file only changes the Direct stage, so trying new markers or a new seed re-renders without re-analysing.

- **Decoding and analysis:** `symphonia` decodes the audio; beats and downbeats come from beat-this; the rest is custom (see Building blocks).
- **Rendering:** `wgpu` with WGSL shaders. Frames stream straight to the encoder, with checkpoints so a render can resume.
- **Device fallback:** `wgpu` picks a dedicated GPU, then integrated graphics, then Windows' software renderer (WARP). The same code runs everywhere; only the speed differs.
- **Scenes as files:** each scene is a WGSL shader plus a small settings file, loaded at start-up. New scenes need no recompile.
- **Why not Docker:** on Windows, a container only reaches the GPU through WSL2 with an NVIDIA card, so most friends would get the slowest path. A native program is simpler and faster.
- **Package:** the program, the encoder and a scenes folder in one zip. Unzip and drag a file onto it.

### Building blocks

Existing crates cover decoding, beat tracking and encoding; sections, track changes and the whole visual engine are built from scratch.

| Need | Use | Status |
| --- | --- | --- |
| Audio decoding | [symphonia](https://crates.io/crates/symphonia) | Established |
| Spectra | rustfft or realfft | Established |
| Beats, downbeats, BPM | [beat-this](https://docs.rs/crate/beat-this/0.3.0): Rust port of the Beat This! AI tracker, pure-Rust model runtime, MIT | v1.1.0, young; validate in Phase 1 |
| Kick detection | Custom, onsets in the low band | Build |
| Sections and track changes | Custom, ported from the [FMP notebooks](https://audiolabs-erlangen.de/resources/MIR/FMP/C4/C4.html) | Build; no Rust crate exists |
| Visual engine | Custom on wgpu | Build; no MilkDrop-style engine exists in Rust |
| Encoding | [ffmpeg-sidecar](https://docs.rs/crate/ffmpeg-sidecar) driving a separate ffmpeg program with x264 | Established |

Notes:

- beat-this needs a one-off model conversion with Python at development time; the converted model ships with the program (about 83 MB standard, 10 MB small). Neither it nor the original reports accuracy on electronic music or DJ mixes.
- Fallback if beat-this fails Phase 1: a custom onset-based beat tracker, built on the kick detector.
- Rejected: [beat-detector](https://github.com/phip1611/beat-detector) (its author says it struggles with complex songs) and [projectm-rs](https://rust-digger.code-maven.com/crates/projectm-rs) (LGPL bindings, last updated April 2023).

## Phases and success criteria

Each phase ends with a check that must pass before the next one starts. Phase 0 tests the biggest risk, visual quality, before any analysis is built.

1. **Phase 0: visual quality test.** Build the feedback engine, the finish pass and 3–4 scenes, driven by simple kick detection on a 2-minute clip. Include a developer preview window that plays the clip with shaders reloading on save, so scenes can be tuned without rendering an MP4 each time.
   - Pass: you would put these clips on a real upload, judged side by side with recordings from MDropDX12 (a current DirectX 12 MilkDrop rebuild) and projectM.
   - Pass: on your GPU, draft quality renders 1080p30 at least in real time; final quality speed (supersampling and motion blur on) is measured and recorded.
   - Benchmark: the same clip on WARP (no GPU), so the CPU tier is designed now rather than discovered in Phase 3.
   - Test: the clip uploaded to YouTube at 1080p and 1440p, with grain on and off.
2. **Phase 1: analysis.** Beats and downbeats from beat-this; kick detection, phrases, sections and track changes built on top; click track and overlay for checking.
   - Test set: 5 of your mixes, with track changes and drops marked by you as the answer key. Compare beat-this's standard and small models, and time both on CPU.
   - Pass: kicks and downbeats sound locked to the music when the click track plays against the mix.
   - Pass: at least 80% of track changes found within 8 bars; hard cuts within 1 bar.
   - Pass: at least 80% of drops found within 1 bar.
3. **Phase 2: director and modes.** Render plan, anticipation, transitions, all three modes, marker import and snapping.
   - Pass: one full 1-hour mix renders end to end in each mode.
   - Pass: markers land on the music; scenes don't repeat back to back; the same seed gives the same video; locking or re-rolling one track changes only that track.
4. **Phase 3: hand-off.** Packaging, CPU fallback, draft quality, resume.
   - Pass: the zip runs on a clean Windows machine with no dedicated GPU (a virtual machine is a good test).
   - Pass: on that machine, a 1-hour mix renders overnight (under 10 hours, an assumed target) at 720p draft; a killed render resumes.
5. **Phase 4: later.** Stem separation, named section labels, short vertical clips, a hardware-encoder fast mode, more scenes, a simple GUI, macOS and Linux builds.

## Risks

| Risk | Effect | Mitigation |
| --- | --- | --- |
| Visuals still not impressive | Product fails its purpose | Phase 0 gate before anything else; curated scenes; restrained reactivity; preview window for fast tuning |
| Copyright claims on uploaded mixes | YouTube mutes, blocks or monetises the mix, whatever the visuals | Own productions are the main use case; chapters file; consider Mixcloud for full mixes; short clips as a later option |
| Track changes missed in long blends | Scenes change in the wrong place | Markers from a tracklist as the reliable route; confidence scores; snap to phrases |
| Slow renders without a GPU | Hours per mix on friends' PCs | WARP benchmarked in Phase 0; a 720p draft CPU tier; resume; honest time estimate |
| Software rendering fails | Program won't start on some PCs | wgpu supports WARP on Windows 10 1709 or later; test on a clean VM in Phase 3 |
| YouTube compression | Blocky detail after upload | Recommended bitrates; 1080p vs 1440p and grain tested in Phase 0 |
| ffmpeg licensing | Problems if shipped wrongly | ffmpeg ships as a separate program with its licence and source link (see decisions) |
| Scene library too small | Long mixes feel repetitive | Phrase-level variation; per-track palettes; add scenes over time |

## Assumptions and decisions

**Assumed. Correct any that are wrong:**

- MilkDrop-style feedback is the base look, with AVS-style geometry as an optional layer. The reference was unclear on this.
- Version 1 is drag-and-drop plus command-line options, with no settings window.
- Markers come from CUE files, Audacity labels or plain text, not a built-in editor.
- Your own productions are the main upload use case; full mixes on YouTube carry copyright risk the tool can't fix.

**Open:**

- [x] **Video encoder: x264, via a separate ffmpeg program.** It gives the best quality on every machine, needs no GPU, and makes identical files everywhere. ffmpeg ships as its own exe with its licence and a link to its source; because the Rust program only runs it over a pipe and doesn't link it, the program keeps its own licence (a common reading of the GPL, not legal advice). On a no-GPU PC, rendering is the bottleneck, so x264's CPU cost doesn't matter; on a GPU PC, it encodes while the GPU renders. Hardware encoders (NVENC, AMF, QSV) can be added later as an optional fast mode.
- [x] **On-screen track titles: yes,** as described under Output.
- [x] **Short vertical clips: later** (Phase 4).
- [ ] **Scene list.** Pick the 3–4 scenes for Phase 0 from the candidates above.
- [ ] **Test mixes.** Choose the 5 mixes for the Phase 1 answer key, including at least one with long blends.
- [ ] **No-GPU target.** Confirm "under 10 hours at 720p draft" once WARP is benchmarked in Phase 0.

## Sources

- [beat-this on docs.rs](https://docs.rs/crate/beat-this/0.3.0)
- [Beat This! (CPJKU)](https://github.com/CPJKU/beat_this)
- [ffmpeg-sidecar on docs.rs](https://docs.rs/crate/ffmpeg-sidecar)
- [beat-detector README](https://github.com/phip1611/beat-detector/blob/main/README.md)
- [projectm-rs](https://rust-digger.code-maven.com/crates/projectm-rs)
- [FMP notebooks: music structure analysis](https://audiolabs-erlangen.de/resources/MIR/FMP/C4/C4.html)

* [YouTube: recommended upload encoding settings](https://support.google.com/youtube/answer/1722171)
* [YouTube: video chapters](https://support.google.com/youtube/answer/9884579)
* [FFmpeg: legal and licensing](https://www.ffmpeg.org/legal.html)
* [Microsoft: DirectX WARP](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/directx-warp)
* [wgpu DXGI adapter handling](https://github.com/gfx-rs/wgpu/blob/trunk/wgpu-hal/src/auxil/dxgi/factory.rs)
* [Digital DJ Tips: uploading mixes to YouTube](https://www.digitaldjtips.com/questions-legally-upload-mixes-youtube/)
* [DJ.Studio: music visualizer](https://dj.studio/blog/music-visualizer)
* [Videobolt reviews (Capterra)](https://www.capterra.com/p/180330/Videobolt/reviews/)
* [Filmdaft: 1440p and YouTube codecs](https://filmdaft.com/how-to-get-better-image-quality-on-your-youtube-videos/)
* [MDropDX12](https://github.com/shanevbg/mdropdx12)
