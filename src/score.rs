//! Judge for generated scenes: render a few seconds small, reject the dull, the flooded,
//! the noisy and the deaf. Catches bad, not great; humans keep the favourites (--keep).
use crate::render::engine::{self, Engine};
use crate::scenes::Scene;
use crate::types::*;
use anyhow::Result;

pub struct Metrics {
    pub mean: f32,      // average luma
    pub contrast: f32,  // average per-frame luma std-dev
    pub motion: f32,    // average |frame - previous frame|
    pub clipped: f32,   // share of pixels near white
    pub grain: f32,     // average horizontal neighbour difference (TV-static detector)
    pub kick_sync: f32, // correlation of kick envelope with frame change
    pub symmetry: f32,      // 1 = left-right mirror image, 0 = unrelated halves
    pub density: f32,       // share of pixels with luma > 0.1
    pub colourfulness: f32, // Hasler–Süsstrunk, set by the judge from rgb (0 in `metrics`)
}

/// `frames`: luma 0..1, row-major, width `w`; `kicks[i]` = kick envelope at frame i.
pub fn metrics(frames: &[Vec<f32>], w: usize, kicks: &[f32]) -> Metrics {
    let n = frames.len().max(1) as f32;
    let px = frames.first().map_or(1, |f| f.len()).max(1) as f32;
    let mean_of = |f: &Vec<f32>| f.iter().sum::<f32>() / px;
    let mean = frames.iter().map(mean_of).sum::<f32>() / n;
    let contrast = frames
        .iter()
        .map(|f| {
            let m = mean_of(f);
            (f.iter().map(|v| (v - m).powi(2)).sum::<f32>() / px).sqrt()
        })
        .sum::<f32>()
        / n;
    let clipped = frames.iter().map(|f| f.iter().filter(|&&v| v > 0.98).count() as f32 / px).sum::<f32>() / n;
    let grain = frames
        .iter()
        .map(|f| f.chunks(w).map(|r| r.windows(2).map(|p| (p[1] - p[0]).abs()).sum::<f32>()).sum::<f32>() / px)
        .sum::<f32>()
        / n;
    let diffs: Vec<f32> =
        frames.windows(2).map(|p| p[0].iter().zip(&p[1]).map(|(a, b)| (a - b).abs()).sum::<f32>() / px).collect();
    let motion = diffs.iter().sum::<f32>() / diffs.len().max(1) as f32;
    let symmetry = frames
        .iter()
        .map(|f| {
            let d = f.chunks(w).map(|r| (0..w / 2).map(|x| (r[x] - r[w - 1 - x]).abs()).sum::<f32>()).sum::<f32>() / (px / 2.0);
            (1.0 - d / (2.0 * mean_of(f) + 1e-3)).clamp(0.0, 1.0)
        })
        .sum::<f32>()
        / n;
    let density = frames.iter().map(|f| f.iter().filter(|&&v| v > 0.1).count() as f32 / px).sum::<f32>() / n;
    Metrics {
        mean,
        contrast,
        motion,
        clipped,
        grain,
        kick_sync: pearson(&kicks[1.min(kicks.len())..], &diffs),
        symmetry,
        density,
        colourfulness: 0.0,
    }
}

/// Hasler–Süsstrunk colourfulness of linear-ish 0..1 rgb pixels (0 = grey; ~0.3+ = vivid).
pub fn colourfulness(px: &[[f32; 3]]) -> f32 {
    let n = px.len().max(1) as f32;
    let (rg, yb): (Vec<f32>, Vec<f32>) = px.iter().map(|c| (c[0] - c[1], 0.5 * (c[0] + c[1]) - c[2])).unzip();
    let stat = |v: &[f32]| {
        let m = v.iter().sum::<f32>() / n;
        (m, v.iter().map(|x| (x - m).powi(2)).sum::<f32>() / n)
    };
    let ((mr, vr), (my, vy)) = (stat(&rg), stat(&yb));
    (vr + vy).sqrt() + 0.3 * (mr * mr + my * my).sqrt()
}

/// Where a look sits in "look space", each 0..1: brightness, motion, symmetry, colourfulness, density, fine detail.
pub fn descriptor(m: &Metrics) -> [f32; 6] {
    [(m.mean / 0.5).min(1.0), (m.motion / 0.05).min(1.0), m.symmetry, (m.colourfulness / 0.4).min(1.0), m.density, (m.grain / 0.2).min(1.0)]
}

/// Quality-diversity cell (0..27): brightness × motion × colourfulness, 3 bins each.
pub fn cell(d: &[f32; 6]) -> usize {
    let b = |x: f32| ((x * 3.0) as usize).min(2);
    b(d[0]) + 3 * b(d[1]) + 9 * b(d[3])
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

/// Judge acceptance ranges (engine 2: one set per regime).
#[derive(Clone, Copy, Debug)]
pub struct Thresholds {
    pub mean: (f32, f32),
    pub min_contrast: f32,
    pub motion: (f32, f32),
    pub max_clipped: f32,
    pub max_grain: f32,
}

// ponytail: hand-set thresholds. Calibrate with `cargo test calibrate -- --ignored --nocapture`
// (every hand-made scene must pass); a learned scorer only if these keep misjudging.
pub const V1: Thresholds = Thresholds { mean: (0.04, 0.75), min_contrast: 0.04, motion: (0.002, 0.25), max_clipped: 0.25, max_grain: 0.2 };

/// Not black, static, flooded or noisy by `th`.
pub fn passes(m: &Metrics, th: &Thresholds) -> bool {
    (th.mean.0..=th.mean.1).contains(&m.mean)
        && m.contrast >= th.min_contrast
        && (th.motion.0..=th.motion.1).contains(&m.motion)
        && m.clipped <= th.max_clipped
        && m.grain <= th.max_grain
}

/// None = rejected; else higher is better (contrast + music lock + some motion).
pub fn verdict(m: &Metrics) -> Option<f32> {
    passes(m, &V1).then(|| m.contrast + 2.0 * m.kick_sync.max(0.0) + 4.0 * m.motion.min(0.05))
}

const W: u32 = 160;
const H: u32 = 90;

/// Headless 160x90 draft renderer that scores scenes.
pub struct Judge {
    device: wgpu::Device,
    queue: wgpu::Queue,
    engine: Engine,
    out_tex: wgpu::Texture,
    out_view: wgpu::TextureView,
    buf: wgpu::Buffer,
    padded: u32,
}

impl Judge {
    pub fn new(scenes: &[Scene], scenes_dir: &std::path::Path, cpu: bool) -> Result<Judge> {
        let instance = wgpu::Instance::default();
        let (_adapter, device, queue) = engine::gpu(&instance, cpu, None)?;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut engine = Engine::new(&device, &queue, scenes, scenes_dir, Quality::Draft, W, H, format, &[]);
        engine.layers = false; // Backdrops are art, not music: score the feedback only (layer passes still compile).
        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("judge"),
            size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&Default::default());
        let padded = (W * 4).div_ceil(256) * 256;
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: padded as u64 * H as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Judge { device, queue, engine, out_tex, out_view, buf, padded })
    }

    /// Compiles `scene` (Err = doesn't compile; message is the compiler's) and scores `secs` seconds of it
    /// from `t0`, using palette 0, params at range midpoints, no mods. The scene is dropped from the
    /// judge afterwards, so the judge doesn't grow and a later scene with the same id compiles afresh.
    pub fn judge(&mut self, scene: Scene, analysis: &Analysis, t0: f64, secs: f64) -> Result<Option<f32>> {
        Ok(verdict(&self.measure(scene, analysis, t0, secs)?))
    }

    /// The raw metrics of `secs` seconds from `t0` (engine 2 gates by regime and picks by descriptor).
    /// Like `judge`, the scene is dropped from the judge afterwards.
    pub fn measure(&mut self, scene: Scene, analysis: &Analysis, t0: f64, secs: f64) -> Result<Metrics> {
        let id = scene.id.clone();
        let r = self.score(scene, analysis, t0, secs);
        self.engine.remove_scene(&id);
        r
    }

    /// The check new AI scenes and blocks must pass: 4 s from mid-song (at most 60 s in) scores at all.
    pub fn audition(&mut self, scene: Scene, analysis: &Analysis) -> Result<()> {
        if self.judge(scene, analysis, 60.0f64.min(analysis.duration / 2.0), 4.0)?.is_none() {
            anyhow::bail!("renders too dark, static, flooded or noisy");
        }
        Ok(())
    }

    /// `audition` with the layer pass drawn: for new backdrop/overlay blocks, which normal judging never draws.
    pub fn audition_layered(&mut self, scene: Scene, analysis: &Analysis) -> Result<()> {
        self.engine.layers = true;
        let r = self.audition(scene, analysis);
        self.engine.layers = false;
        r
    }

    fn score(&mut self, scene: Scene, analysis: &Analysis, t0: f64, secs: f64) -> Result<Metrics> {
        let id = scene.id.clone();
        let mid: Vec<f32> = scene.params.iter().map(|p| (p.min + p.max) / 2.0).collect();
        let scenes = [scene];
        self.engine.add_scene(scenes[0].clone());
        self.engine.compile(&id)?;
        let plan = Plan {
            version: 1,
            source: analysis.source.clone(),
            seed: 0,
            mode: Mode::Single,
            generated: 0.0,
            vibe: None,
            plates: vec![],
            post: None,
            layers: false,
            engine: 1,
            pool: Default::default(),
            segments: vec![Segment {
                start: 0.0,
                end: analysis.duration,
                scene: id,
                seed: 0,
                locked: false,
                palette: 0,
                title: None,
                transition_in: Transition { kind: TransitionKind::Cut, duration: 0.0 },
                variations: vec![Variation { time: 0.0, params: mid, hue_shift: 0.0 }],
                mods: vec![],
                genome: None,
                variant: false,
            }],
        };
        let (sw, sh) = self.engine.sim;
        let fs_at = |t: f64| crate::director::frame_state(&plan, analysis, &scenes, t, sw, sh, Quality::Draft);

        self.engine.reset();
        let pre = (2.0f64.min(t0) * SIM_HZ as f64) as i64;
        for n in -pre..0 {
            self.engine.step(&fs_at(t0 + n as f64 / SIM_HZ as f64), None)?;
        }
        let sub = (SIM_HZ / OUT_FPS) as u64;
        let (mut frames, mut kicks) = (Vec::new(), Vec::new());
        let mut cols = Vec::new();
        for f in 0..(secs * OUT_FPS as f64) as u64 {
            let mut fs = None;
            for j in 0..sub {
                let s = fs_at(t0 + (sub * f + j) as f64 / SIM_HZ as f64);
                self.engine.step(&s, (j == sub - 1).then_some(1.0))?;
                fs = Some(s);
            }
            if f % 3 != 0 {
                continue;
            }
            let fs = fs.unwrap();
            let mut enc = self.device.create_command_encoder(&Default::default());
            self.engine.finish(&mut enc, &self.out_view, &fs, false, false);
            enc.copy_texture_to_buffer(
                self.out_tex.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &self.buf,
                    layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(self.padded), rows_per_image: None },
                },
                wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
            );
            self.queue.submit([enc.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            self.buf.map_async(wgpu::MapMode::Read, .., move |r| {
                let _ = tx.send(r);
            });
            self.device.poll(wgpu::PollType::wait_indefinitely())?;
            rx.recv()??;
            {
                let data = self.buf.get_mapped_range(..)?;
                let mut luma = Vec::with_capacity((W * H) as usize);
                let mut rgb = Vec::with_capacity((W * H) as usize);
                for y in 0..H as usize {
                    let row = &data[y * self.padded as usize..][..W as usize * 4];
                    luma.extend(row.chunks(4).map(|p| (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0));
                    rgb.extend(row.chunks(4).map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]));
                }
                frames.push(luma);
                cols.push(colourfulness(&rgb));
            }
            self.buf.unmap();
            kicks.push(fs.a.beat[0]);
        }
        let mut m = metrics(&frames, W as usize, &kicks);
        m.colourfulness = cols.iter().sum::<f32>() / cols.len().max(1) as f32;
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let pulse: Vec<Vec<f32>> = kicks
            .iter()
            .map(|k| (0..w * 9).map(|p| 0.1 + 0.3 * (p % w) as f32 / w as f32 + 0.2 * k).collect())
            .collect();
        assert!(verdict(&metrics(&pulse, w, &kicks)).is_some(), "pulse");
    }

    #[test]
    fn descriptors_see_symmetry_density_colour_and_cells() {
        let w = 16;
        let sym: Vec<f32> = (0..w * 9).map(|p| 0.1 + 0.05 * ((p % w) as f32 - 7.5).abs()).collect();
        let ramp: Vec<f32> = (0..w * 9).map(|p| (p % w) as f32 / w as f32).collect();
        let kicks = vec![0.0; 4];
        let (ms, mr) = (metrics(&vec![sym; 4], w, &kicks), metrics(&vec![ramp; 4], w, &kicks));
        assert!(ms.symmetry > 0.99 && mr.symmetry < 0.6, "{} {}", ms.symmetry, mr.symmetry);
        assert!(ms.density > 0.99 && (0.8..0.9).contains(&mr.density), "{}", mr.density);
        assert!(colourfulness(&[[0.5; 3]; 8]) < 1e-6);
        assert!(colourfulness(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].repeat(4)) > 0.3);
        let d = descriptor(&ms);
        assert!(d.iter().all(|x| (0.0..=1.0).contains(x)) && cell(&d) < 27);
    }

    /// Manual calibration: every hand-made scene should pass on real music.
    #[test]
    #[ignore]
    fn calibrate() {
        let a: Analysis =
            serde_json::from_str(&std::fs::read_to_string("testdata/dnb5.analysis.json").expect("run dnb5 once first")).unwrap();
        let dir = std::path::Path::new("scenes");
        let scenes = crate::scenes::load_all(dir).unwrap();
        let mut j = Judge::new(&scenes, dir, false).unwrap();
        for s in &scenes {
            println!("{:24} {:?}", s.id, j.judge(s.clone(), &a, 60.0, 4.0).unwrap());
        }
    }
}
