# AI features

Optional. Everything else in `visualiser` works without them. For the short version see [USAGE.md](USAGE.md#ai-features---setup-ai---vibe---forget-ai). This page explains how the features work and what to do when they misbehave.

- [Overview](#overview)
- [Provider setup](#provider-setup)
- [How requests work](#how-requests-work)
- [The vibe pipeline](#the-vibe-pipeline)
- [Example: "clouds, atmos"](#example-clouds-atmos)
- [Cost and limits](#cost-and-limits)
- [--author](#--author-dev-tool)
- [Where files live](#where-files-live)
- [Troubleshooting](#troubleshooting)

---

## Overview

| Option | What it does |
|---|---|
| `--vibe "<text>"` | You describe the look in words. The AI designs 1 to 6 "looks" (a motion block, an emitter block, a colour block, a palette and tuned param values). The generated-scene evolution then starts from those looks and stays inside them. If no block can express something you asked for, the AI writes a new one. |
| `--author "<text>"` | Dev tool. The AI writes one complete scene (`.wgsl` + `.toml`) into `scenes\`. Nothing is rendered. |

**Principle: the block library is a starting vocabulary, not a limit.** The AI is told to add a new block rather than settle for a poor match. New blocks are compiled and test-rendered before they are saved, so a bad answer cannot break a render.

A vibe only steers generated scenes. If the generated share would be 0, `--vibe` raises it to 1.

---

## Provider setup

Only OpenAI-compatible services are supported: anything that serves `/chat/completions` (OpenAI, OpenRouter, Ollama, LM Studio, vLLM, llama.cpp server, Groq, Together, ...). Anthropic and Amazon Bedrock support was removed. If you had one of those stored, the program warns that the stored settings are from an older version. Run `--setup-ai` again.

The model must support structured output or tool calling. Most current models do.

### `--setup-ai`

Prompts for three things:

| Prompt | Notes |
|---|---|
| API base URL | Default `https://api.openai.com/v1`. Must start with `http://` or `https://`. You may paste the full `.../chat/completions` URL too. |
| API key | Hidden as you type. Press Enter for none (local servers). |
| Model id | Required. |

It then makes a test call through the same structured-output path the real features use, so a model that can't do structured output or tool calling fails here, not mid-render. On success it saves the settings to Windows Credential Manager (service `visualiser`, account `ai`). The key is sent only to that service. It is never printed or written to project files.

`--forget-ai` deletes the stored entry.

### Examples

| Service | Base URL | Key | Model |
|---|---|---|---|
| OpenAI | `https://api.openai.com/v1` | your `sk-...` key | e.g. `gpt-5` |
| OpenRouter | `https://openrouter.ai/api/v1` | your OpenRouter key | e.g. `anthropic/claude-sonnet-5` |
| Ollama | `http://localhost:11434/v1` | empty | a pulled model, e.g. `qwen3:14b` |
| LM Studio | `http://localhost:1234/v1` | empty | the loaded model's id |

Model names are examples. Use whatever the service lists.

### Environment variables

Used only when nothing is stored.

| Variable | Use |
|---|---|
| `VISUALISER_AI_MODEL` | Model id. Required for the env setup. **Always overrides the model**, including a stored one. |
| `OPENAI_API_KEY` | Key. |
| `OPENAI_BASE_URL` | Base URL, default `https://api.openai.com/v1`. |

The env setup is active when `VISUALISER_AI_MODEL` is set together with `OPENAI_API_KEY` or `OPENAI_BASE_URL`. If a key or URL is set without a model, the program warns and ignores them. Stored settings win over env vars, and are never mixed with them: a global `OPENAI_API_KEY` can't redirect a stored Ollama setup.

---

## How requests work

All calls go through one function in `src\llm.rs`.

1. **Strict structured output.** The request carries `response_format` with `type: json_schema` and `strict: true`. The server constrains decoding to the schema.
2. **Schema normalisation.** Strict mode has rules, so the schema is rewritten before sending: every object is closed (`additionalProperties: false`), every property is required, and keywords some providers reject (`pattern`, `format`, `minItems`, `maxItems`, `minLength`, `maxLength`, `minimum`, `maximum`) are removed. The limits those keywords expressed are checked in code on the reply instead.
3. **Fallback to tool calling.** If the server answers 400 or 422 to the structured request, the same request is sent as a tool call. Once a tool call works, it is used for the rest of the run. (The first error is kept and shown if the tool call fails too, since the 400 may not have been about structured output.) `tool_choice` is not forced, because some models reject it and Ollama ignores it. The system prompt asks for the tool instead.
4. **Token limit name.** Requests use `max_tokens`. If a server answers 400 asking for `max_completion_tokens` (OpenAI reasoning models), the request is retried with that name, and the choice is remembered for the run.
5. **Reply parsing.** Tool-call arguments are read first (as a JSON string or an object). Otherwise the reply text is tried as pure JSON. Failing that, everything up to the last `</think>` is dropped and the text from the first `{` to the last `}` is parsed. That handles fences and prose around the JSON, and reasoning models that print their thinking.
6. **Timeout.** 600 seconds per request, because `--author` and block writing ask for up to 8000 tokens and local models can be slow.
7. **Errors.** A failed call shows the server's own message (`AI service returned 401: ...`). If the body is not JSON, the first 200 characters are shown. The key is never in any message.

The answer is still validated by the caller. Structured output guarantees shape, not sense.

---

## The vibe pipeline

`--vibe` runs once, before the plan is made, and only when a new plan is being generated.

### 1. Design call

One request. The AI sees:

- `scenes\blocks\CONTRACT.md` (the block format and rules).
- Every block's `.toml`: id, kind, description, and each param's name and range.
- The palette bank.
- Music stats: BPM, length in minutes, number of drops, number of track changes.
- The instruction that blocks are a starting vocabulary and params should be tuned to the vibe.

It returns:

| Field | Meaning |
|---|---|
| `note` | One plain sentence describing what the visuals will look like. Printed to the console. |
| `new_blocks` | 0 to 3 blocks to write: `id`, `kind` (`motion`, `emitter`, `colour`), `brief`. |
| `looks` | 1 to 6 looks, best first, only ones that fit. Each has `motion`, `emitter`, `colour`, a 4-colour `palette` (first colour dark) and `params` as `[{key, value}]`, with keys like `motion_ink.persistence`. |

Fewer looks is fine. Params are a list of pairs because strict schemas have no free-form objects.

### 2. Writing new blocks

For each requested block (up to 3 per vibe):

- The id must match `^(motion|emit|colour)_[a-z0-9_]{2,30}$`. Bad ids are skipped with a warning.
- An id that already exists in `scenes\blocks\` is skipped. **Existing blocks are never overwritten.**
- The AI gets the prelude, the contract, one worked example of the same kind, the brief and the vibe, and returns `wgsl` and `toml`. Up to 3 attempts. After each failure the error is fed back (parse error, wrong kind, compile error, or judge rejection).
- A candidate is parsed, composed into a scene with the first look that uses it, compiled, and auditioned by the judge: 4 seconds rendered small from mid-song (at most 60 s in). It is rejected if it renders too dark, static, flooded or noisy.
- Passing blocks are saved as `scenes\blocks\<id>.wgsl` and `.toml` and join the library for later runs.
- If a block fails 3 times, a warning is printed and the looks that use it are dropped.

### 3. Validation

Looks are kept only if every block they name exists and the palette parses as 4 hex colours. Params with unknown keys or non-finite values are pruned. If no look survives, the vibe fails (and the run continues without it).

### 4. How tuned params work

A look's param value does not pin the param. The block's range is narrowed to the value plus or minus 15% of the range width (clamped to the original range). Audio modulation still moves the param inside that narrowed range. Params marked `fixed` are pinned to the exact value.

### 5. How evolution stays on-vibe

The looks become the seeds of the usual evolution, with these changes:

- `Blocks::subset`: random picks and mutations only use the blocks and palettes that appear in the looks.
- Mutations can also nudge one tuned param by 15% of its range, up or down. When a block is swapped, tuned params for the old block are dropped.
- Each generated segment starts from a different seed look, rotating through the list.
- The music-fit judge (scored on that segment's own music) still picks the winner.

### 6. Caching

The result (vibe text, note, genomes) is saved to `vibe.json` in the [data folder](#where-files-live). Same vibe text plus all genomes still valid against the current library means no AI call, so re-renders are identical. A different text asks again. A cached look whose block was deleted triggers a fresh call.

If the plan itself is reused (same seed, mode, generated share and vibe), the vibe step is skipped entirely.

---

## Example: "clouds, atmos"

**First attempt (earlier design).** The result was a kaleidoscope plus Lissajous scene, not clouds. Causes:

- The prompt demanded 6 distinct looks, so the AI padded with poor matches.
- The AI could not tune params or write blocks.
- Evolution ignored the vibe and picked from the whole library.

**After the change.** For `--vibe "clouds, atmos"` the AI:

- Wrote a new emitter, `emit_clouds`, because no existing emitter drew clouds.
- Chose `motion_ink` and `colour_iridescent`.
- Tuned `motion_ink.current_speed` to 0.005, `motion_ink.persistence` to 0.97, `motion_ink.current_scale` to 1.35, `colour_iridescent.sheen` to 0.25 and `colour_iridescent.hue_offset` to 0.08.
- Used the palette `#050914 #344b70 #8b83a8 #e4c9bd`.
- Printed the note: "Slow, softly billowing clouds drift through a deep blue-violet atmosphere, with muted lavender and warm pearl light along their wispy edges."

The result was soft clouds.

**Observed gap.** The render looked cyan rather than the note's blue-violet. The iridescent colour block shifts hue with intensity, so the palette is not shown as written. The note describes intent, not a guarantee. If the colour is wrong, try a vibe that names the colour treatment ("flat colour, no iridescence"), which can lead the AI to write or choose a different colour block.

---

## Cost and limits

- **Calls per new vibe:** 1 design call, plus up to 3 per new block, with up to 3 new blocks. That is at most 10 calls. A cached vibe makes none.
- **The judge can reject very soft visuals.** The thresholds are hand-set (average brightness 0.04 to 0.75, contrast at least 0.04, frame-to-frame motion 0.002 to 0.25, at most 25% clipped pixels, grain at most 0.2). A very faint, slow vibe can be rejected as "too dark" or "static". Rejected attempts cost a call each.
- **One-look vibes give little variety.** Evolution only uses the looks' blocks, so a single look means every segment uses the same blocks and palette. Only tuned-param nudges differ. Ask for a broader vibe if you want variety.
- **AI-written blocks live in `scenes\blocks\`.** To remove one, delete its `.wgsl` and `.toml`. They are not committed to git by default; add them yourself if you want to keep them.
- **AI trouble never fails a render.** No service set up, a failed call, or an unusable answer prints a warning and the scenes are evolved without the vibe.

---

## `--author` (dev tool)

```
visualiser.exe track.wav --author "slow aurora curtains that flare on the kick"
```

The AI writes a full scene (`id`, `wgsl`, `toml`) in the format described in [Writing scenes](USAGE.md#writing-scenes). It uses the same retry helper as block writing: up to 3 attempts with the error fed back, and the same audition (4 s scored on your audio). A scene id must match `^[a-z][a-z0-9_]{2,30}$` and not already exist.

On success the files are saved to `scenes\<id>.wgsl` and `.toml` and the program prints the `--dev --scene <id>` command to audition it. Nothing is rendered. Delete the two files to discard it. It needs the audio file for scoring.

---

## Where files live

Videos go next to the audio. Everything else goes to the per-track data folder `%LOCALAPPDATA%\Visualiser\projects\<name>-<hash>\` (the hash comes from the audio file's full path). Each run prints `project data: <folder>`.

| File | Written by AI features |
|---|---|
| `vibe.json` (data folder) | The cached vibe answer. |
| `plan.json` (data folder) | The plan, including each generated segment's recipe. |
| `gen\` (data folder) | Generated scene shaders. |
| `scenes\blocks\*.wgsl`, `*.toml` | New AI-written blocks. |
| `scenes\<id>.wgsl`, `.toml` | `--author` output. |

Stored credentials are in Windows Credential Manager, not in any file.

---

Uninstalling always forgets the stored AI settings. It also asks whether to delete the project data folder and the scenes and blocks added after install (AI-written blocks included).

## Troubleshooting

| Message / symptom | Cause and fix |
|---|---|
| `warning: no AI service set up (run visualiser --setup-ai); continuing without the vibe` | Nothing stored and no usable env vars. Run `--setup-ai`. |
| `OPENAI_API_KEY/OPENAI_BASE_URL is set but VISUALISER_AI_MODEL isn't` | The env vars are ignored without a model. Set `VISUALISER_AI_MODEL`, or use `--setup-ai`. |
| `stored AI settings are from an older version` | Settings from the Anthropic/Bedrock version. Run `--setup-ai` again. |
| `AI service returned 401` | Wrong or missing key. Run `--setup-ai` again. |
| `AI service returned 404` | Wrong base URL (often missing `/v1`) or unknown model id. Check both. Ollama and LM Studio need `/v1`. |
| `AI service returned 400/422` after a structured-output attempt | The model supports neither structured output nor tool calling. Pick another model. |
| `AI reply had no tool call` | The model answered in plain text with no JSON. Use a model that supports tool calling or structured output. |
| `new block ... failed ...; dropping the looks that use it` | The block failed 3 attempts (compile error or judge rejection). Run again, use a stronger model, or reword the vibe. |
| `the AI gave no usable looks` | None of the looks named existing blocks. Try again or reword. |
| Very slow or timeout (600 s) | Slow local model. Use a smaller or faster one. Block and scene writing request up to 8000 tokens. |
| Visuals don't match the note | The note describes intent. See the colour gap in the [example](#example-clouds-atmos). Reword the vibe (a new text asks the AI again). |
