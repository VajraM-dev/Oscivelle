//! GPU feedback engine shared by the offline renderer and the preview window.
//!
//! Per SIM_HZ step: scene A -> fb[next] (sampling fb[cur]); during a morph scene B -> scratch,
//! blended onto fb[next] with a constant-blend copy. The step can also be accumulated into
//! `accum` (weight 1/i gives the average of i sub-frames = motion blur).
//! Finish: bloom chain from `accum`, then finish.wgsl into the caller's target.

use crate::scenes::{self, Scene};
use crate::types::*;
use anyhow::{bail, Context, Result};
use bytemuck::{Pod, Zeroable};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const POST_WGSL: &str = concat!(include_str!("common.wgsl"), include_str!("post.wgsl"));
pub const FINISH_WGSL: &str = concat!(include_str!("common.wgsl"), include_str!("finish.wgsl"));

/// Picks the adapter (high performance, else software fallback; `cpu` forces WARP/fallback).
pub fn gpu(
    instance: &wgpu::Instance,
    cpu: bool,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let req = |fallback: bool| {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: fallback,
            compatible_surface: surface,
            ..Default::default()
        }))
    };
    let adapter = if cpu { req(true) } else { req(false).or_else(|_| req(true)) }
        .context("no GPU adapter found (not even the software fallback)")?;
    let info = adapter.get_info();
    eprintln!("GPU: {} ({:?})", info.name, info.backend);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    Ok((adapter, device, queue))
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FinishUniforms {
    a: [f32; 4],
    b: [f32; 4],
    r: [f32; 4],
    d0: [f32; 4],
    d1: [f32; 4],
}

pub struct Engine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    scenes: Vec<Scene>,
    scenes_dir: PathBuf,
    pub sim: (u32, u32),
    out: (u32, u32),
    final_q: bool,
    scene_layout: wgpu::PipelineLayout,
    pipelines: HashMap<String, wgpu::RenderPipeline>,
    failed: HashSet<String>,
    fb: [wgpu::TextureView; 2],
    cur: usize,
    scratch: wgpu::TextureView,
    accum: wgpu::TextureView,
    bloom: Vec<wgpu::TextureView>,
    ubuf: [wgpu::Buffer; 2],
    /// [uniforms a/b][source fb]
    scene_bg: [[wgpu::BindGroup; 2]; 2],
    copy_pl: wgpu::RenderPipeline,
    down_first_pl: wgpu::RenderPipeline,
    down_pl: wgpu::RenderPipeline,
    up_pl: wgpu::RenderPipeline,
    finish_pl: wgpu::RenderPipeline,
    fb_bg: [wgpu::BindGroup; 2],
    scratch_bg: wgpu::BindGroup,
    accum_bg: wgpu::BindGroup,
    bloom_bg: Vec<wgpu::BindGroup>,
    fbuf: wgpu::Buffer,
    finish_bg: wgpu::BindGroup,
}

impl Engine {
    /// `out_format` is the finish target format (Rgba8Unorm for video, the surface format for preview).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scenes: &[Scene],
        scenes_dir: &Path,
        quality: Quality,
        width: u32,
        height: u32,
        out_format: wgpu::TextureFormat,
    ) -> Self {
        let final_q = quality == Quality::Final;
        let ss = if final_q { 1.5 } else { 1.0 };
        let sim = ((width as f32 * ss).round() as u32, (height as f32 * ss).round() as u32);
        let d = device;

        let tex = |w: u32, h: u32| {
            d.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HDR,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default())
        };
        let fb = [tex(sim.0, sim.1), tex(sim.0, sim.1)];
        let scratch = tex(sim.0, sim.1);
        let accum = tex(sim.0, sim.1);
        let levels = if final_q { 5 } else { 3 };
        let bloom: Vec<_> = (1..=levels).map(|i| tex(sim.0 >> i, sim.1 >> i)).collect();

        let sampler = |mode: wgpu::AddressMode| {
            d.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: mode,
                address_mode_v: mode,
                address_mode_w: mode,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
        };
        let mirror = sampler(wgpu::AddressMode::MirrorRepeat);
        let clamp = sampler(wgpu::AddressMode::ClampToEdge);

        let frag = wgpu::ShaderStages::FRAGMENT;
        let uniform_e = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let tex_e = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: frag,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let samp_e = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: frag,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let bgl = |entries: &[wgpu::BindGroupLayoutEntry]| {
            d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries })
        };
        let scene_bgl = bgl(&[uniform_e(0), tex_e(1), samp_e(2)]);
        let tex_bgl = bgl(&[tex_e(0), samp_e(1)]);
        let finish_bgl = bgl(&[uniform_e(0), tex_e(1), tex_e(2), samp_e(3)]);
        let layout = |l: &wgpu::BindGroupLayout| {
            d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(l)],
                immediate_size: 0,
            })
        };
        let scene_layout = layout(&scene_bgl);
        let tex_layout = layout(&tex_bgl);
        let finish_layout = layout(&finish_bgl);

        let buf = |size: usize| {
            d.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: size as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let ubuf = [buf(size_of::<SceneUniforms>()), buf(size_of::<SceneUniforms>())];
        let fbuf = buf(size_of::<FinishUniforms>());

        let bg = |l: &wgpu::BindGroupLayout, res: &[wgpu::BindingResource]| {
            let entries: Vec<_> = res
                .iter()
                .enumerate()
                .map(|(i, r)| wgpu::BindGroupEntry { binding: i as u32, resource: r.clone() })
                .collect();
            d.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: l, entries: &entries })
        };
        use wgpu::BindingResource as R;
        let scene_bg = [0, 1].map(|u| {
            [0, 1].map(|f| bg(&scene_bgl, &[ubuf[u].as_entire_binding(), R::TextureView(&fb[f]), R::Sampler(&mirror)]))
        });
        let tex_bg = |v: &wgpu::TextureView| bg(&tex_bgl, &[R::TextureView(v), R::Sampler(&clamp)]);
        let fb_bg = [tex_bg(&fb[0]), tex_bg(&fb[1])];
        let scratch_bg = tex_bg(&scratch);
        let accum_bg = tex_bg(&accum);
        let bloom_bg = bloom.iter().map(tex_bg).collect();
        let finish_bg = bg(
            &finish_bgl,
            &[fbuf.as_entire_binding(), R::TextureView(&accum), R::TextureView(&bloom[0]), R::Sampler(&clamp)],
        );

        let post = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(POST_WGSL.into()),
        });
        let finish = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("finish"),
            source: wgpu::ShaderSource::Wgsl(FINISH_WGSL.into()),
        });
        let comp = |src, dst| wgpu::BlendComponent { src_factor: src, dst_factor: dst, operation: wgpu::BlendOperation::Add };
        let constant = comp(wgpu::BlendFactor::Constant, wgpu::BlendFactor::OneMinusConstant);
        let add = comp(wgpu::BlendFactor::One, wgpu::BlendFactor::One);
        let copy_pl = pipeline(d, &post, &tex_layout, "fs_copy", HDR, Some(wgpu::BlendState { color: constant, alpha: constant }));
        let down_first_pl = pipeline(d, &post, &tex_layout, "fs_down_first", HDR, None);
        let down_pl = pipeline(d, &post, &tex_layout, "fs_down", HDR, None);
        let up_pl = pipeline(d, &post, &tex_layout, "fs_up", HDR, Some(wgpu::BlendState { color: add, alpha: add }));
        let finish_pl = pipeline(d, &finish, &finish_layout, "fs_main", out_format, None);

        let e = Self {
            device: device.clone(),
            queue: queue.clone(),
            scenes: scenes.to_vec(),
            scenes_dir: scenes_dir.to_path_buf(),
            sim,
            out: (width, height),
            final_q,
            scene_layout,
            pipelines: HashMap::new(),
            failed: HashSet::new(),
            fb,
            cur: 0,
            scratch,
            accum,
            bloom,
            ubuf,
            scene_bg,
            copy_pl,
            down_first_pl,
            down_pl,
            up_pl,
            finish_pl,
            fb_bg,
            scratch_bg,
            accum_bg,
            bloom_bg,
            fbuf,
            finish_bg,
        };
        e.reset();
        e
    }

    /// Clears the feedback (start of a chunk / restart).
    pub fn reset(&self) {
        let mut enc = self.device.create_command_encoder(&Default::default());
        for v in [&self.fb[0], &self.fb[1], &self.accum] {
            begin(&mut enc, v, true);
        }
        self.queue.submit([enc.finish()]);
    }

    /// Ids of scenes compiled (or attempted) so far.
    pub fn loaded(&self) -> Vec<String> {
        self.pipelines.keys().chain(&self.failed).cloned().collect()
    }

    /// (Re)compiles a scene from disk. On failure an existing pipeline is kept.
    pub fn compile(&mut self, id: &str) -> Result<()> {
        match self.try_compile(id) {
            Ok(pl) => {
                self.pipelines.insert(id.to_string(), pl);
                self.failed.remove(id);
                Ok(())
            }
            Err(e) => {
                self.failed.insert(id.to_string());
                Err(e)
            }
        }
    }

    fn try_compile(&self, id: &str) -> Result<wgpu::RenderPipeline> {
        let scene = self.scenes.iter().find(|s| s.id == id).with_context(|| format!("unknown scene '{id}'"))?;
        let src = scenes::shader_source(&self.scenes_dir, scene)?;
        let internal = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(id),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });
        let pl = pipeline(&self.device, &module, &self.scene_layout, "fs_main", HDR, None);
        let errs = [pollster::block_on(validation.pop()), pollster::block_on(internal.pop())];
        if let Some(e) = errs.into_iter().flatten().next() {
            bail!("scene '{id}': {e}");
        }
        Ok(pl)
    }

    /// One SIM_HZ simulation step. `accum_weight`: blend this step into the output accumulator
    /// with that weight (1/i for the i-th sub-frame; None = don't accumulate).
    /// Returns Err the first time a scene fails to compile (that scene is then skipped).
    pub fn step(&mut self, fs: &FrameState, accum_weight: Option<f32>) -> Result<()> {
        let mut res = Ok(());
        for id in std::iter::once(&fs.scene_a).chain(fs.b.as_ref().map(|b| &b.0)) {
            if !self.pipelines.contains_key(id) && !self.failed.contains(id) {
                if let Err(e) = self.compile(id) {
                    res = Err(e);
                }
            }
        }
        let Some(pa) = self.pipelines.get(&fs.scene_a) else { return res };
        let (cur, next) = (self.cur, 1 - self.cur);
        self.queue.write_buffer(&self.ubuf[0], 0, bytemuck::bytes_of(&fs.a));
        let mut enc = self.device.create_command_encoder(&Default::default());
        draw(&mut enc, &self.fb[next], true, pa, &self.scene_bg[0][cur], 1.0);
        if let Some((id_b, ub)) = &fs.b {
            if let Some(pb) = self.pipelines.get(id_b) {
                self.queue.write_buffer(&self.ubuf[1], 0, bytemuck::bytes_of(ub));
                draw(&mut enc, &self.scratch, true, pb, &self.scene_bg[1][cur], 1.0);
                draw(&mut enc, &self.fb[next], false, &self.copy_pl, &self.scratch_bg, fs.mix as f64);
            }
        }
        if let Some(w) = accum_weight {
            draw(&mut enc, &self.accum, false, &self.copy_pl, &self.fb_bg[next], w as f64);
        }
        self.queue.submit([enc.finish()]);
        self.cur = next;
        res
    }

    /// Records bloom + finish into `enc`, drawing into `target`. Submit before the next `finish`.
    pub fn finish(&self, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, fs: &FrameState, grain: bool, debug: bool) {
        let n = self.bloom.len();
        draw(enc, &self.bloom[0], true, &self.down_first_pl, &self.accum_bg, 1.0);
        for i in 1..n {
            draw(enc, &self.bloom[i], true, &self.down_pl, &self.bloom_bg[i - 1], 1.0);
        }
        for i in (1..n).rev() {
            draw(enc, &self.bloom[i - 1], false, &self.up_pl, &self.bloom_bg[i], 1.0);
        }
        let a = &fs.a;
        let u = FinishUniforms {
            a: [fs.exposure, fs.flash, fs.saturation, grain as u32 as f32],
            // ponytail: bloom strength / tap offset are fixed constants; expose if tuning needs it.
            b: [a.res_time[2], 0.18 / n as f32, if self.final_q { 0.25 } else { 0.0 }, debug as u32 as f32],
            r: [self.out.0 as f32, self.out.1 as f32, (a.res_time[2] * OUT_FPS as f32).floor(), 0.0],
            d0: [a.beat[0], a.shape[0], a.shape[1], a.bands[1]],
            d1: [a.beat[1], a.beat[2], a.beat[3], a.shape[2]],
        };
        self.queue.write_buffer(&self.fbuf, 0, bytemuck::bytes_of(&u));
        draw(enc, target, true, &self.finish_pl, &self.finish_bg, 1.0);
    }
}

fn pipeline(
    d: &wgpu::Device,
    module: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    fs: &str,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(fs),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fs),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn begin<'e>(enc: &'e mut wgpu::CommandEncoder, view: &wgpu::TextureView, clear: bool) -> wgpu::RenderPass<'e> {
    let load = if clear { wgpu::LoadOp::Clear(wgpu::Color::BLACK) } else { wgpu::LoadOp::Load };
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

/// Fullscreen triangle into `target`. `blend` sets the blend constant (used by the copy pipeline).
fn draw(
    enc: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    clear: bool,
    pl: &wgpu::RenderPipeline,
    bg: &wgpu::BindGroup,
    blend: f64,
) {
    let mut p = begin(enc, target, clear);
    p.set_pipeline(pl);
    p.set_bind_group(0, bg, &[]);
    p.set_blend_constant(wgpu::Color { r: blend, g: blend, b: blend, a: blend });
    p.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: post/finish shaders + a temp scene compile and run a few steps on the real GPU;
    /// a broken scene returns Err instead of panicking.
    #[test]
    fn engine_runs() {
        let dir = std::env::temp_dir().join("visualiser_engine_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy("scenes/_prelude.wgsl", dir.join("_prelude.wgsl")).unwrap();
        std::fs::write(dir.join("ok.wgsl"), "fn scene(uv: vec2<f32>) -> vec4<f32> { return vec4<f32>(prev(warp(uv, 1.01, 0.01, 0.0, vec2<f32>(0.0))) * 0.95 + pal(uv.x) * kick(), 1.0); }").unwrap();
        std::fs::write(dir.join("bad.wgsl"), "fn scene(uv: vec2<f32>) -> vec4<f32> { return nope; }").unwrap();
        let scene = |id: &str| Scene { id: id.into(), params: vec![], palettes: vec![[[1.0; 3]; 4]], wgsl_path: dir.join(format!("{id}.wgsl")) };
        let scenes = [scene("ok"), scene("bad")];

        let instance = wgpu::Instance::default();
        let (_a, device, queue) = gpu(&instance, false, None).unwrap();
        for q in [Quality::Draft, Quality::Final] {
            let mut e = Engine::new(&device, &queue, &scenes, &dir, q, 320, 180, wgpu::TextureFormat::Rgba8Unorm);
            let mut u = SceneUniforms::default();
            u.res_time = [e.sim.0 as f32, e.sim.1 as f32, 0.0, 1.0 / SIM_HZ as f32];
            u.beat[0] = 1.0;
            u.pal = [[1.0, 0.5, 0.2, 1.0]; 4];
            let fs = FrameState { scene_a: "ok".into(), a: u, b: Some(("ok".into(), u)), mix: 0.5, flash: 0.2, exposure: 1.0, saturation: 1.0 };
            for i in 0..4 {
                e.step(&fs, Some(1.0 / (i % 2 + 1) as f32)).unwrap();
            }
            let bad = FrameState { scene_a: "bad".into(), b: None, ..fs.clone() };
            assert!(e.step(&bad, None).is_err());
            assert!(e.step(&bad, None).is_ok(), "failure reported once, then skipped");
            let out = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d { width: 320, height: 180, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let mut enc = device.create_command_encoder(&Default::default());
            e.finish(&mut enc, &out.create_view(&Default::default()), &fs, true, true);
            queue.submit([enc.finish()]);
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        }

        // Every shipped scene compiles against the real prelude + common.wgsl.
        let dir = Path::new("scenes");
        let real = scenes::load_all(dir).unwrap();
        let mut e = Engine::new(&device, &queue, &real, dir, Quality::Draft, 64, 36, wgpu::TextureFormat::Rgba8Unorm);
        for s in &real {
            e.compile(&s.id).unwrap();
        }
    }
}
