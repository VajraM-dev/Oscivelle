# Manual test checklist: `feature/dynamic-visuals`

Things only a human can check. The automated part is green: `cargo test` passes (14 tests), plus the ignored `calibrate` test, which was run by hand.
Run everything from the repo root in PowerShell. Build first: `cargo build --release`.

## 1. AI setup (OpenAI-compatible)

Settings stored by the old Anthropic/Bedrock setup no longer load. Run setup again.

- [ ] `target
eleaseisualiser.exe --setup-ai`. Enter the base URL, key and a model that supports tool calling (e.g. `https://api.openai.com/v1` + `gpt-5-mini`, or `http://localhost:11434/v1` + empty key + `qwen3`). Expect `works.` and `Saved.`
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

## Known gaps (not bugs, decided)

- A saved plan that names a block that has since been deleted stops with an error; `--replan` fixes it.
- A different GPU may evolve a different genome on replan. Re-renders from the same plan are identical.
- Unsigned installer: SmartScreen → "More info → Run anyway".
