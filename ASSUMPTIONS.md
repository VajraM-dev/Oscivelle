# Assumptions made while building v1

The brief was implemented end to end in one pass. I didn't stop to ask about anything below; I picked the option shown. Correct any that are wrong.

## Scope and process
- **Tier: T2 Standard** (friends use it, nothing at stake). The review pass is `/simplify` then `/review`. No security review.
- **No web frontend.** The UI is drag-and-drop or the CLI, plus a native developer preview window, so the "design materials" gate doesn't apply.
- **All phases are built as code, but the phase *gates* are left to you.** They need your judgement and your data: side-by-side visual quality against MDropDX12/projectM, YouTube upload tests, the 5-mix answer key, and a clean-VM test. None of that can be automated here.

## Analysis
- **beat-this is not used yet.** It needs a one-off model conversion in Python plus an 83 MB model. v1 ships the brief's own fallback instead: a custom onset-based beat tracker built on the kick detector (tempo by autocorrelation, then dynamic-programming beat tracking). The analysis interface (`src/analysis.rs`) is the swap point if beat-this passes Phase 1 later.
- Tempo search range is 70–180 BPM, with a preference for 115–150 (electronic music).
- Downbeat = the beat phase (of 4) with the strongest kicks and bass changes. Phrases = every 8 bars from the first downbeat, re-anchored at strong novelty points.
- Track changes come from bar-level feature self-similarity with a checkerboard novelty kernel (FMP-style), plus tempo jumps. Each gets a 0..1 confidence and a Cut or Blend classification.
- Tempo range is actually 70–200 BPM. It is scored on the full onset envelope with 2- and 4-beat support, so DnB reads ~174, not half-time. (This replaces the 70–180 above.)
- A track change only counts if the music changes **key (chroma over 32 bars either side) or tempo**. Energy changes alone (breakdowns, drops) score 0. Known limit: a long in-key, beatmatched blend can score below 0.35 and be missed. Use Markers for those mixes.
- Drops are detected two ways: a bass jump after a low-kick build, or a filter-sweep drop (highs jump after rising while the kicks continue).
- Analysis runs on mono audio downsampled for features. It streams, so a 2-hour file doesn't need gigabytes of RAM.

## Director
- Per-segment seed = hash(plan seed, segment index). Re-rolling one segment changes only that segment, plus a neighbour's scene choice if needed to avoid a back-to-back repeat.
- "No repeats" = never the same scene twice in a row, and a recently used scene is avoided while others are available.
- Markers snap to the nearest downbeat when it is within 1 bar, and to a phrase start when one is within 2 bars.
- In Auto mode, track changes with confidence < 0.35 are ignored.
- Chapters are titled from marker/CUE names when they exist, otherwise "Track N". No chapters file is written if the YouTube rules can't be met (≥3 chapters, each ≥10 s).

## Rendering
- The simulation always steps at 60 Hz and the output is 30 fps. Final quality averages the 2 sub-frames per output frame (that is the motion blur). Draft shows the last sub-frame only. Draft and final therefore animate identically.
- Final quality supersamples at 1.5× (2× is too slow for an hour of 1080p). Draft defaults to 720p output, with no supersampling and lighter bloom (3 levels, not 5).
- Bloom, chromatic aberration, vignette and grain amounts are compiled into the exe (`src/render/finish.wgsl`), so changing them needs a rebuild. Only scene shaders hot-reload.
- The preview window can't play AIFF (rodio has no AIFF decoder). Rendering AIFF works.
- Resume: the video renders in 300 s chunks to `<name>.parts/`. A finished chunk is never re-rendered. Each chunk pre-rolls 20 s of simulation without output (never before the file start), so a resumed render matches a fresh one and chunk seams don't pop: the slowest feedback (ink_in_water, 0.994/step at 60 Hz) fades to 0.994^1200 ~ 7e-4 in 20 s. Preroll overhead is about 6.7%. Chunks are then concatenated and muxed with the source audio.
- Track titles are drawn by ffmpeg's `drawtext` (Segoe UI from `C:\Windows\Fonts`), not on the GPU. That's simpler, and ffmpeg is already in the loop.
- ffmpeg is called directly with `std::process::Command` over a stdin pipe, not through the `ffmpeg-sidecar` crate. It does the same job with one less dependency. The program looks for ffmpeg next to the exe first, then on `PATH`.
- Encode settings: libx264 High, `-g 15` with closed GOP (half the frame rate, as YouTube recommends), 8 Mbps at 1080p / 16 Mbps at 1440p, BT.709 tags, AAC-LC 48 kHz 384 kbps.
- Grain is on by default (`--no-grain` turns it off) so it can be A/B tested for Phase 0.

## Scenes
- 7 scenes ship: liquid tunnel, kaleidoscope bloom, fractal zoom, ink in water, neon geometry trails, plasma field and starfield warp. Their tuning is a first pass and *will* need your eye in the preview window (`visualiser dev <file>`).

## Files written next to the source audio
- `<name>.analysis.json`: reused on the next run if present (delete it to re-analyse).
- `<name>.plan.json`: reused when the mode, seed and markers are unchanged. Edit it to tweak, set `"locked": true` to keep a segment, and use `--reroll N` to re-roll one segment.
- `<name>.mp4`, `<name>.chapters.txt`, and `<name>.click.wav` (with `--debug`).

## Packaging
- Packaging (Phase 3) is a `package.ps1` script that zips the exe, `scenes/` and `ffmpeg.exe`. ffmpeg itself is not bundled in the repo: you drop a GPL static build next to the exe with its licence.

## Measured on this machine (RTX 3060 laptop), Oct 2026
- GPU final 1080p (1.5× supersampling plus motion blur): about 48–63 fps, which is 1.6–2× realtime. Draft 720p: about 98 fps.
- WARP software renderer at 720p draft: about 3.3 fps, so a 1-hour mix takes about 9 h. That is just inside the assumed <10 h target, and it was measured while other renders were loading the CPU, so it should be re-measured on a clean machine.
- Analysis of a 2–3 minute file takes under 1 s.
- Test tracks used: dnb5/dnb8 (your DnB productions, ~174.5 BPM detected) and `testdata/make_mix.py`, a synthetic 2-track mix with known drops and a known cut.

## Deferred, not built (pick up when needed)
- beat-this model (Phase 1 validation against your 5-mix answer key).
- Doing the RGB→YUV conversion on the GPU and adding a writer thread for the ffmpeg pipe. That would reduce CPU load and help the WARP tier most.
- A single GPU submit per frame. Trimming the STFT loops in analysis (results are cached anyway).
- The phase gates that need you: side-by-side comparison with MDropDX12/projectM, YouTube upload tests (1080p vs 1440p, grain on/off), a clean-VM run of the zip.
