//! OWNER: render agent. Contract signatures below must stay.
pub mod engine;

use crate::director;
use crate::scenes::{self, Scene};
use crate::types::*;
use anyhow::{bail, Context, Result};
use engine::Engine;
use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Instant;

pub struct RenderJob<'a> {
    pub audio: PathBuf,
    pub out: PathBuf,
    pub plan: &'a Plan,
    pub analysis: &'a Analysis,
    pub scenes: &'a [Scene],
    pub scenes_dir: PathBuf,
    pub quality: Quality,
    /// Output size (e.g. 1920x1080).
    pub width: u32,
    pub height: u32,
    /// Window to render, seconds (whole file = 0..duration).
    pub start: f64,
    pub end: f64,
    pub titles: bool,
    pub grain: bool,
    /// Draw analysis overlay (kick/beat/energy/tension bars + phrase/drop/track markers).
    pub debug_overlay: bool,
    /// Force the software adapter (WARP on Windows) for benchmarking / no-GPU testing.
    pub cpu: bool,
}

const FPS: u64 = OUT_FPS as u64;
/// Sim steps per output frame.
const SUB: u64 = (SIM_HZ / OUT_FPS) as u64;
/// Chunk length (s). Long, so the preroll overhead (PREROLL_S / CHUNK_S, ~6.7%) stays small.
const CHUNK_S: u64 = 300;
/// Sim seconds re-run before each chunk so its feedback matches a continuous render.
/// The slowest-decaying scene (ink_in_water, 0.994/step) leaves 0.994^(20*SIM_HZ) ~ 7e-4 of the reset.
const PREROLL_S: i64 = 20;
const CHUNK_FRAMES: u64 = CHUNK_S * FPS;
const PREROLL_STEPS: i64 = PREROLL_S * SIM_HZ as i64;
const SLOTS: usize = 3;
const TITLE_LEN: f64 = 5.0;

/// Renders to MP4 (H.264 + AAC from source), chunked with resume. Prints progress + ETA.
pub fn render_video(job: &RenderJob) -> Result<()> {
    let ffmpeg = find_ffmpeg();
    let parts = job.out.with_extension("parts");
    let meta = serde_json::json!({ "hash": format!("{:016x}", job_hash(job)?) }).to_string();
    let meta_path = parts.join("job.json");
    if std::fs::read_to_string(&meta_path).ok().as_deref() != Some(meta.as_str()) && parts.exists() {
        eprintln!("settings changed, discarding old chunks in {}", parts.display());
        std::fs::remove_dir_all(&parts)?;
    }
    std::fs::create_dir_all(&parts)?;
    std::fs::write(&meta_path, &meta)?;

    let instance = wgpu::Instance::default();
    let (_adapter, device, queue) = engine::gpu(&instance, job.cpu, None)?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut eng = Engine::new(&device, &queue, job.scenes, &job.scenes_dir, job.quality, job.width, job.height, format);
    eprintln!("sim {}x{} -> output {}x{} ({:?})", eng.sim.0, eng.sim.1, job.width, job.height, job.quality);
    let out_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("out"),
        size: wgpu::Extent3d { width: job.width, height: job.height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let out_view = out_tex.create_view(&Default::default());
    let padded = (job.width * 4).div_ceil(256) * 256;
    let slots: Vec<wgpu::Buffer> = (0..SLOTS)
        .map(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: padded as u64 * job.height as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
        .collect();
    let gpu = Gpu { device: &device, queue: &queue, out_tex: &out_tex, out_view: &out_view, slots: &slots, padded };

    let total = ((job.end - job.start) * FPS as f64).round().max(0.0) as u64;
    let mut progress = Progress::new(total);
    let mut chunks = Vec::new();
    for k in 0..total.div_ceil(CHUNK_FRAMES) {
        let frames = (k * CHUNK_FRAMES)..((k + 1) * CHUNK_FRAMES).min(total);
        let name = format!("chunk_{k:04}.mp4");
        if !parts.join(&name).exists() {
            let tmp = format!("chunk_{k:04}.tmp.mp4");
            render_chunk(job, &mut eng, &gpu, &ffmpeg, &parts, &tmp, frames.clone(), &mut progress)?;
            std::fs::rename(parts.join(&tmp), parts.join(&name))?;
        } else {
            progress.skip(frames.end - frames.start);
        }
        chunks.push(name);
    }
    eprintln!();

    // Concat chunks + source audio.
    let list: String = chunks.iter().map(|c| format!("file '{c}'\n")).collect();
    let list_path = parts.join("list.txt");
    std::fs::write(&list_path, list)?;
    let status = Command::new(&ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args(["-ss", &format!("{:.3}", job.start), "-t", &format!("{:.3}", job.end - job.start), "-i"])
        .arg(&job.audio)
        .args(["-map", "0:v:0", "-map", "1:a:0", "-c:v", "copy"])
        .args(["-c:a", "aac", "-b:a", "384k", "-ar", "48000", "-movflags", "+faststart"])
        .arg(&job.out)
        .status()
        .context("running ffmpeg (mux)")?;
    if !status.success() {
        bail!("ffmpeg mux failed ({status})");
    }
    std::fs::remove_dir_all(&parts)?;
    eprintln!("wrote {}", job.out.display());
    Ok(())
}

struct Gpu<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    out_tex: &'a wgpu::Texture,
    out_view: &'a wgpu::TextureView,
    slots: &'a [wgpu::Buffer],
    padded: u32,
}

type Pending = (usize, wgpu::SubmissionIndex, mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>);

#[allow(clippy::too_many_arguments)]
fn render_chunk(
    job: &RenderJob,
    eng: &mut Engine,
    gpu: &Gpu,
    ffmpeg: &Path,
    parts: &Path,
    tmp: &str,
    frames: std::ops::Range<u64>,
    progress: &mut Progress,
) -> Result<()> {
    let t0 = job.start + frames.start as f64 / FPS as f64;
    let len = (frames.end - frames.start) as f64 / FPS as f64;
    let (sw, sh) = eng.sim;
    let fs_at = |t: f64| director::frame_state(job.plan, job.analysis, job.scenes, t, sw, sh, job.quality);

    let mut cmd = Command::new(ffmpeg);
    cmd.current_dir(parts)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgba"])
        .args(["-s", &format!("{}x{}", job.width, job.height), "-r", &OUT_FPS.to_string(), "-i", "-"])
        .args(["-vf", &video_filter(job, parts, t0, len)?]);
    let (b, max, buf) = if job.height >= 1440 { ("16M", "20M", "32M") } else { ("8M", "10M", "16M") };
    cmd.args(["-c:v", "libx264", "-profile:v", "high", "-preset", "medium"])
        .args(["-b:v", b, "-maxrate", max, "-bufsize", buf, "-g", "15", "-flags", "+cgop", "-bf", "2"])
        .args(["-color_primaries", "bt709", "-color_trc", "bt709", "-colorspace", "bt709", "-an", tmp])
        .stdin(Stdio::piped());
    let mut child = cmd.spawn().context("starting ffmpeg (is it next to the exe or on PATH?)")?;
    let mut stdin = child.stdin.take().unwrap();

    eng.reset();
    // Pre-roll whenever there's audio before t0 (later chunks, or --preview starting mid-file),
    // so the feedback doesn't start from black. Capped at t=0: never steps before the file start.
    if t0 > 0.0 {
        for n in -PREROLL_STEPS.min((t0 * SIM_HZ as f64) as i64)..0 {
            eng.step(&fs_at(t0 + n as f64 / SIM_HZ as f64), None)?;
        }
    }
    let mut pending: VecDeque<Pending> = VecDeque::new();
    let write_res = (|| -> Result<()> {
        for f in 0..(frames.end - frames.start) {
            let mut fs = None;
            for j in 0..SUB {
                let s = fs_at(t0 + (SUB * f + j) as f64 / SIM_HZ as f64);
                let w = match job.quality {
                    Quality::Final => Some(1.0 / (j + 1) as f32),
                    Quality::Draft => (j == SUB - 1).then_some(1.0),
                };
                eng.step(&s, w)?;
                fs = Some(s);
            }
            let slot = (f as usize) % SLOTS;
            if pending.len() == SLOTS {
                flush(gpu, pending.pop_front().unwrap(), job, &mut stdin)?;
            }
            let mut enc = gpu.device.create_command_encoder(&Default::default());
            eng.finish(&mut enc, gpu.out_view, fs.as_ref().unwrap(), job.grain, job.debug_overlay);
            enc.copy_texture_to_buffer(
                gpu.out_tex.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &gpu.slots[slot],
                    layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(gpu.padded), rows_per_image: None },
                },
                wgpu::Extent3d { width: job.width, height: job.height, depth_or_array_layers: 1 },
            );
            let sub = gpu.queue.submit([enc.finish()]);
            let (tx, rx) = mpsc::channel();
            gpu.slots[slot].map_async(wgpu::MapMode::Read, .., move |r| {
                let _ = tx.send(r);
            });
            pending.push_back((slot, sub, rx));
            progress.tick();
        }
        while let Some(p) = pending.pop_front() {
            flush(gpu, p, job, &mut stdin)?;
        }
        Ok(())
    })();
    drop(stdin);
    let status = child.wait()?;
    write_res.context("rendering/encoding chunk")?;
    if !status.success() {
        bail!("ffmpeg encode failed ({status})");
    }
    Ok(())
}

/// Waits for a readback slot and streams its rows to ffmpeg.
fn flush(gpu: &Gpu, (slot, sub, rx): Pending, job: &RenderJob, out: &mut impl Write) -> Result<()> {
    gpu.device.poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None })?;
    let r = match rx.try_recv() {
        Ok(r) => r,
        Err(_) => {
            gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
            rx.recv()?
        }
    };
    r?;
    let buf = &gpu.slots[slot];
    {
        let data = buf.get_mapped_range(..)?;
        let row = job.width as usize * 4;
        let padded = gpu.padded as usize;
        if row == padded {
            out.write_all(&data)?;
        } else {
            for y in 0..job.height as usize {
                out.write_all(&data[y * padded..y * padded + row])?;
            }
        }
    }
    buf.unmap();
    Ok(())
}

/// Colour conversion (+ lower-third titles for segments starting in/near this chunk; times chunk-local).
fn video_filter(job: &RenderJob, parts: &Path, t0: f64, len: f64) -> Result<String> {
    let mut vf = Vec::new();
    if job.titles {
        for (i, seg) in job.plan.segments.iter().enumerate() {
            let Some(title) = &seg.title else { continue };
            let l = seg.start - t0;
            if l >= len || l + TITLE_LEN <= 0.0 {
                continue;
            }
            // Text via file (ffmpeg runs in `parts`), so titles need no filtergraph escaping.
            let file = format!("title_{i:04}.txt");
            std::fs::write(parts.join(&file), title)?;
            let l = format!("({l:.3})");
            let size = (job.height as f64 * 0.042).round();
            vf.push(format!(
                "drawtext=fontfile='C\\:/Windows/Fonts/segoeui.ttf':textfile='{file}':expansion=none\
                 :fontcolor=white:fontsize={size}:x={x}:y={y}:shadowcolor=black@0.6:shadowx=2:shadowy=2\
                 :alpha='if(lt(t,{l}+0.7),(t-{l})/0.7,if(lt(t,{l}+4.3),1,({l}+5-t)/0.7))'\
                 :enable='between(t,{l},{l}+5)'",
                x = (job.width as f64 * 0.06).round(),
                y = (job.height as f64 * 0.80).round(),
            ));
        }
    }
    vf.push("scale=out_color_matrix=bt709:out_range=tv,format=yuv420p".into());
    Ok(vf.join(","))
}

fn find_ffmpeg() -> PathBuf {
    beside_exe("ffmpeg.exe").unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

/// FNV-1a over plan + settings + analysis + used shader sources (stable across builds, unlike DefaultHasher).
fn job_hash(job: &RenderJob) -> Result<u64> {
    let mut data = serde_json::to_vec(job.plan)?;
    let settings = format!(
        "{:?} {} {} {} {} {} {} {}",
        job.quality, job.width, job.height, job.start, job.end, job.titles, job.grain, job.debug_overlay
    );
    data.extend(settings.bytes());
    data.extend(serde_json::to_vec(job.analysis)?);
    data.extend(engine::POST_WGSL.bytes());
    data.extend(engine::FINISH_WGSL.bytes());
    // Only scenes this plan uses, so editing another scene doesn't throw away finished chunks.
    for s in job.scenes.iter().filter(|s| job.plan.segments.iter().any(|g| g.scene == s.id)) {
        data.extend(scenes::shader_source(&job.scenes_dir, s)?.bytes());
    }
    Ok(fnv1a(&data))
}

struct Progress {
    total: u64,
    done: u64,
    rendered: u64,
    started: Instant,
    last: Instant,
}

impl Progress {
    fn new(total: u64) -> Self {
        Self { total, done: 0, rendered: 0, started: Instant::now(), last: Instant::now() }
    }
    fn skip(&mut self, n: u64) {
        self.done += n;
    }
    fn tick(&mut self) {
        self.done += 1;
        self.rendered += 1;
        if self.last.elapsed().as_secs_f64() < 1.0 && self.done < self.total {
            return;
        }
        self.last = Instant::now();
        let fps = self.rendered as f64 / self.started.elapsed().as_secs_f64().max(1e-6);
        let eta = (self.total - self.done) as f64 / fps.max(1e-6);
        eprint!(
            "\rrendering {:5.1}%  {:6.1} fps  ETA {}   ",
            100.0 * self.done as f64 / self.total.max(1) as f64,
            fps,
            hms(eta as u64, true)
        );
    }
}
