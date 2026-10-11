# Manual test checklist: `feature/dynamic-visuals`

Things only a human can check. The automated part is green: `cargo test` passes (14 tests), plus the ignored `calibrate` test, which was run by hand.
Run everything from the repo root in PowerShell. Build first: `cargo build --release`.

## 1. AI setup (OpenAI-compatible)

Settings stored by the old Anthropic/Bedrock setup no longer load. Run setup again.

- [ ] `target\release\visualiser.exe --setup-ai`. Enter the base URL, key and a model that supports tool calling (e.g. `https://api.openai.com/v1` + `gpt-5-mini`, or `http://localhost:11434/v1` + empty key + `qwen3`). Expect `works.` and `Saved.`
- [ ] If it fails, note the exact error.
- [ ] Alternative without setup: `$env:OPENAI_API_KEY="..."; $env:VISUALISER_AI_MODEL="..."` (plus `$env:OPENAI_BASE_URL="..."` for a non-OpenAI service)

## 2. AI features (needs step 1)

- [ ] Vibe: `target\release\visualiser testdata\dnb5.mp3 --replan --vibe "dark underwater bioluminescence" --preview 1:00 --quality draft`
  Expect a `vibe:` note line (plus any `new block` lines), `vibe.json` in the printed `project data:` folder, "generating a scene" lines, and a preview mp4. Re-run the same command and confirm no new AI call is made (the cache is used).
- [ ] Vibe without a key (run `--forget-ai` first, or test in a fresh shell): expect a warning, and the render still completes.
- [ ] Author: `target\release\visualiser testdata\dnb5.mp3 --author "slow aurora curtains that flare on the kick"`
  Expect `new scene: scenes\<id>.wgsl`. Audition it with `--dev --scene <id>`, and **delete both files** if you don't want to keep it.
- [ ] `target\release\visualiser --forget-ai`: no "Press Enter" pause, and the key is gone from Credential Manager.

## 3. Guided mode (wizard)

- [ ] Double-click `target\release\visualiser.exe` (or run it with no args). Pick `testdata\dnb5.mp3`, choose "Quick 30-second test", and finish.
  Expect the printed equivalent command, then a render, then "Press Enter to close".
- [ ] Press Esc on the first question: it exits quietly with no error.
- [ ] Cancel the file dialog: it falls back to "type the path".
- [ ] With a key set up: the vibe question appears, but not after choosing "Hand-made only".
- [ ] Drag an audio file onto the exe: it renders directly and the console stays open.

## 4. Generated scenes + modulation (watch the videos)

- [ ] `target\release\visualiser testdata\dnb5.mp3 --replan --generated 1 --preview 1:00 --quality draft`, then **watch** `testdata\dnb5.preview-60s.mp4`. Is the generated look good? Does it react to kicks?
- [ ] Hand-made scenes with modulation: `target\release\visualiser testdata\dnb5.mp3 --replan --generated 0 --preview 1:00 --quality draft`. Do the scene settings move with the music in a way that looks good rather than jittery? Check the `mods` in `plan.json` in the printed `project data:` folder.
- [ ] Mix with segments: `target\release\visualiser testdata\mix.wav --markers testdata\mix.cue --replan --preview 0:30 --quality draft`
- [ ] Keep a look: `target\release\visualiser testdata\mix.wav --keep 1` → `scenes\gen_*.wgsl/.toml`. Then `cargo test engine_runs` should still pass. Delete the kept files afterwards unless you want them.
- [ ] Re-run with no flags on an old project: the saved plan should be reused ("using saved plan").

## 5. Installer

- [ ] Put a GPL static ffmpeg build in `.\ffmpeg\` (`ffmpeg.exe`, LICENSE, README). `package.ps1` fails without it; that requirement predates this branch.
- [ ] `winget install JRSoftware.InnoSetup`
- [ ] `powershell -File package.ps1` → `dist\visualiser-setup-0.1.0.exe`. The `.iss` script has never been compiled.
- [ ] Install it, then in a **new** terminal: `visualiser --version`. Right-click a `.wav` → "Make visuals". Start Menu "Visualiser" opens the wizard; "Set up AI" runs setup.
- [ ] Package contents: `dist\visualiser\scenes\blocks\` has no `emit_clouds.*` (or any other uncommitted block).
- [ ] Render a track, then check `%LOCALAPPDATA%\Visualiser\projects\<name>-<hash>\` holds `analysis.json`, `plan.json` and the `.parts\` folder, and only the mp4 sits next to the audio.
- [ ] Upgrade: put a dummy `scenes\blocks\emit_test.toml` + `.wgsl` in the install folder, install again over it. The dummy block stays; built-in ones are refreshed.
- [ ] Uninstall, answer **No**: the PATH entry, the right-click menu and the stored AI settings are gone; `%LOCALAPPDATA%\Visualiser\` and the added block remain.
- [ ] Reinstall, uninstall, answer **Yes**: `%LOCALAPPDATA%\Visualiser\` and `%LOCALAPPDATA%\Programs\Visualiser\` are both gone. Videos next to the audio are untouched.
- [ ] Silent uninstall (`unins000.exe /SILENT`): no question, data kept.
- [ ] Optional: try it on a friend's PC with no Rust or VC++ installed.

## 6. Before merging

- [ ] Optional: `/security-review` on `src/llm.rs`. It was skipped because the branch was built at tier T2.
- [ ] Merge `feature/dynamic-visuals` → `main` (21 commits after `b47bce5`).

## 7. Motion layers (`feature/motion-layers`)

- [ ] Layers: `target\release\visualiser testdata\dnb5.mp3 --replan --layers --preview 1:00 --quality draft`, then **watch** the preview. Backdrops behind the visuals, a flash on drops, a morph on build-ups?
- [ ] Picture + VHS: `target\release\visualiser testdata\dnb5.mp3 --replan --layers --image clouds.jpg --look vhs --preview 1:00 --quality draft` (any png/jpeg for `clouds.jpg`). Is the picture shown, with wobble and colour bleed?
- [ ] Mono: `target\release\visualiser testdata\dnb5.mp3 --replan --layers --look mono --preview 1:00 --quality draft`. Whole video tinted?
- [ ] Default output unchanged: render `target\release\visualiser testdata\dnb5.mp3 --replan --generated 0 --preview 1:00 --quality draft` on `main` (copy the mp4 aside as `main1.mp4`), again on `main` (`main2.mp4`), then on this branch (`branch.mp4`). Compare pairs with `ffmpeg -i a.mp4 -i b.mp4 -lavfi psnr -f null -`.
  The render is not bit-deterministic run to run on `main` (GPU), so `framemd5` can't be used. Instead the average PSNR of `branch.mp4` vs `main1.mp4` should match that of `main2.mp4` vs `main1.mp4` (about 42 dB observed).

## 8. AI pictures and storyboard (M2, costs money)

- [ ] `target\release\visualiser --setup-ai`, then enter `gpt-image-1` at the image-model prompt. Expect `Image model gpt-image-1 saved` and `Saved.`
- [ ] `target\release\visualiser testdata\dnb8.wav --replan --vibe "90s CGI, chrome pyramid on a mirror sea, VHS" --preview 1:00 --quality draft`. Watch the preview: a chrome pyramid, a mirror sea, VHS wobble? Do the shots change at drops and build-ups?
- [ ] `target\release\visualiser testdata\dnb8.wav --replan --vibe "green hologram brain, HUD rings, mono" --preview 1:00 --quality draft`. Expect a cut-out plate, `back_sprite` and `over_hud`, in green mono.
- [ ] Run the previous command again: no `vibe: painting` lines (pictures come from the cache in `plates\` in the data folder).
- [ ] With no image model set (`--forget-ai`, then `--setup-ai` and press Enter at the image prompt): a vibe still works and requests no pictures.

## 9. Engine 2 (`feature/core-variety`)

Build first: `cargo build --release`. Copy `dnb8.wav` from `C:\Prathamesh\Ableton\Music\dnb5` into `testdata\` if it's missing.

- [ ] Seed smokes: `target\release\visualiser testdata\dnb8.wav --engine 2 --generated 1 --replan --preview 1:00 --quality draft --seed N` for N = 1, 2, 3, 4. **Watch** each `testdata\dnb8.preview-60s.mp4`. Look for:
  - Each seed is structurally different: another fold, mood or blend, not just another colour.
  - Drop anticipation: the picture darkens before the drop, then explodes on it.
  - No flashes faster than once per bar.
- [ ] Engine 1 is byte-identical to `main`. Build `main` in a separate git worktree (`git worktree add ..\rv-main main`, then `cargo build --release` there). Give each tree its own copy of the wav in its own folder, so the two projects don't share a data folder. Project data is keyed by the wav's absolute path (`%LOCALAPPDATA%\Visualiser\projects\<stem>-<hash>`), so each tree needs its own wav copy. For `--replan` runs use a scratch copy, not your real project wav.
  **Caveat:** libx264 multithreaded encoding is not repeatable run to run. In BOTH trees, temporarily add `-threads 1` to the x264 args in `src\render\mod.rs` (~line 167), rebuild, then revert it afterwards.
  - [ ] Run `visualiser <wav> --replan --preview 1:00 --quality draft --seed 1` in each tree (an explicit seed: a new project otherwise gets a random one), adding `--engine 1` in BOTH trees (`main` now defaults new projects to engine 2 too), then `ffmpeg -i <preview>.mp4 -f framemd5 -an <main|branch>.md5`. The two `.md5` files must be equal.
  - [ ] Repeat with `--layers` added. On the `main` run, move the 6 untracked `scenes\blocks\*runner*` / `*trippy*` files aside first (main loads them, the branch pins them out) and put them back afterwards.
  - [ ] Revert the `-threads 1` change in both trees.

## 10. Engine 2 shows (phase 2)

Build first: `cargo build --release`. Use a scratch copy of `dnb8.wav` in `testdata\`.

- [ ] Smoke: open the project's `analysis.json` and note the first drop time `D`. For N = 1, 2, 3, 4 run `target\release\visualiser testdata\dnb8.wav --engine 2 --generated 1 --replan --preview <D-8s> --quality draft --seed N` and **watch** each preview. Look for:
  - The drop goes black and a new look rebuilds.
  - Transitions are styled (wipe, iris, melt or tiles, not only crossfades).
  - At least one phrase variant inside the 30 s: a fold, framing or palette change.
  - No flashes faster than once per bar.
- [ ] Grep the four `plan.json` files for `"template"` and `"comp"`. Try seeds 5-12 until `comp_slitscan`, `comp_echo`, `stateless` and `dual` have each appeared, then watch them: slit-scan rows lag in time, the echo leaves ghost trails.
- [ ] Saved plan is unchanged by the branch. Build the base commit in a worktree (`git worktree add ..\rv-base 231afa2`, `cargo build --release`), with `-threads 1` added to the x264 args in `src\render\mod.rs` in both trees (see section 9; revert afterwards). Run the base with `--engine 2 --replan --preview 1:00 --quality draft` on its own wav copy. Copy its `plan.json` into the branch project's folder (the branch wav copy's `projects\<stem>-<hash>`), then run the branch with no `--replan`. It must print "using saved plan". `ffmpeg -i <preview>.mp4 -f framemd5 -an <x>.md5` for both: the files must be equal.
- [ ] Planning time: time the `--replan` run on base and on the branch (same wav, `--engine 2 --generated 1`). The branch plans one more judged look per drop, so it is a bit slower. Flag it if it takes more than 2x the base.

## 11. Engine 2 grammars

Build first: `cargo build --release`. Use a scratch copy of `dnb8.wav` in `testdata\`. `BASE` is commit `7fdc5cc` (before grammars).

- [ ] Smoke, one per grammar (radial, flow, grid, scroll, still): run `target\release\visualiser testdata\dnb8.wav --engine 2 --generated 1 --replan --preview 1:00 --quality draft --seed N` for N = 1, 2, 3, ... (or add `--plan-only` to find seeds quickly). After each run, grep the project's `plan.json` for `"grammar"` (under each segment's `genome`). Stop when all five have a render (cap 25 seeds), then **watch** them. Look for:
  - flow, grid, scroll and still do not read as tunnels, rotation or a centred fold.
  - still holds its camera.
  - Every segment has a `genome` (no grammar fell back to a hand-made scene).
- [ ] `motion_sink` seams: in a look that uses it (grep `plan.json` for `motion_sink`), check for visible seams or tearing along the edges as it sinks.
- [ ] Engine 1 is byte-identical to `main`: use the method in section 9 (worktree, `-threads 1` in both trees, own wav copy each) with `target\release\visualiser <wav> --engine 1 --replan --preview 1:00 --quality draft --seed 1` in both, then compare the `ffmpeg -i <preview>.mp4 -f framemd5 -an <x>.md5` files. They must be equal.
- [ ] Saved engine-2 plan is unchanged: build `BASE` in a worktree (`git worktree add ..\rv-base 7fdc5cc`) with the same `-threads 1` patch. Run it with `--engine 2 --generated 1 --replan --preview 1:00 --quality draft --seed 3` on its own wav copy, copy its `plan.json` into the branch project's folder, then run the branch with the same args and no `--replan`. It must print "using saved plan", and the framemd5 files must be equal. Repeat with `--seed 5`. Revert `-threads 1` and remove the worktrees afterwards.
- [ ] Planning time: time the `--replan` run on base and on the branch (same wav, `--engine 2 --generated 1`). Flag it if the branch takes more than 2x the base.

## Known gaps (not bugs, decided)

- A saved plan that names a block that has since been deleted stops with an error; `--replan` fixes it.
- A different GPU may evolve a different genome on replan. Re-renders from the same plan are identical.
- Unsigned installer: SmartScreen → "More info → Run anyway".
