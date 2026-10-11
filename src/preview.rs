//! OWNER: render agent. Contract signatures below must stay.
use crate::director;
use crate::render::engine::{self, Engine};
use crate::scenes::{self, Scene};
use crate::types::*;
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const DT: f64 = 1.0 / SIM_HZ as f64;
const MAX_STEPS: u32 = 4;

/// Developer preview window: plays the audio and renders live, reloading scene shaders on save.
/// Keys: Esc quit, Space pause, R restart, D toggle debug overlay.
pub fn run(audio: &Path, analysis: &Analysis, plan: &Plan, scenes: &[Scene], scenes_dir: &Path, quality: Quality, start: f64) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let plates = crate::plates::load_all(&plan.plates);
    let mut app = App { audio, analysis, plan, scenes, scenes_dir, quality, start, plates, live: None, err: None };
    event_loop.run_app(&mut app)?;
    app.err.map_or(Ok(()), Err)
}

struct App<'a> {
    audio: &'a Path,
    analysis: &'a Analysis,
    plan: &'a Plan,
    scenes: &'a [Scene],
    scenes_dir: &'a Path,
    quality: Quality,
    start: f64,
    /// Decoded once; reused when a resize rebuilds the engine.
    plates: Vec<Vec<u8>>,
    live: Option<Live>,
    err: Option<anyhow::Error>,
}

struct Live {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    engine: Engine,
    _sink: rodio::MixerDeviceSink,
    player: rodio::Player,
    sim_t: f64,
    paused: bool,
    debug: bool,
    fs: Option<FrameState>,
    /// A sim step happened since the last draw.
    dirty: bool,
    mtimes: Vec<Option<SystemTime>>,
    last_check: Instant,
    title: String,
}

impl App<'_> {
    fn init(&self, event_loop: &ActiveEventLoop) -> Result<Live> {
        let attrs = Window::default_attributes()
            .with_title("visualiser preview")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720))
            .with_resizable(false);
        let window = Arc::new(event_loop.create_window(attrs)?);
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone())?;
        let (adapter, device, queue) = engine::gpu(&instance, false, Some(&surface))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface not supported by adapter")?;
        // Finish writes sRGB-encoded values itself, so prefer a non-sRGB swapchain format.
        let caps = surface.get_capabilities(&adapter);
        config.format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(config.format);
        surface.configure(&device, &config);
        // ponytail: layer files aren't hot-reloaded; restart --dev after editing a back_/over_ block.
        let engine = Engine::new(&device, &queue, self.scenes, self.scenes_dir, self.quality, config.width, config.height, config.format, &self.plates);

        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| anyhow!("audio output: {e}"))?;
        sink.log_on_drop(false);
        let player = rodio::Player::connect_new(sink.mixer());
        let file = std::fs::File::open(self.audio).with_context(|| format!("opening {}", self.audio.display()))?;
        player.append(rodio::Decoder::try_from(file).map_err(|e| anyhow!("preview can't decode audio: {e}"))?);
        player.try_seek(Duration::from_secs_f64(self.start)).map_err(|e| anyhow!("seek: {e}"))?;

        Ok(Live {
            window,
            surface,
            config,
            device,
            queue,
            engine,
            _sink: sink,
            player,
            sim_t: self.start,
            paused: false,
            debug: false,
            fs: None,
            dirty: false,
            mtimes: mtimes(self.scenes, self.scenes_dir),
            last_check: Instant::now(),
            title: String::new(),
        })
    }

    fn frame(&mut self) {
        let Some(l) = self.live.as_mut() else { return };

        // Hot reload.
        if l.last_check.elapsed() >= Duration::from_millis(500) {
            l.last_check = Instant::now();
            let now = mtimes(self.scenes, self.scenes_dir);
            let prelude_changed = now[0] != l.mtimes[0];
            let mut ids: Vec<String> = if prelude_changed { l.engine.loaded() } else { Vec::new() };
            for (i, s) in self.scenes.iter().enumerate() {
                if now[i + 1] != l.mtimes[i + 1] && !ids.contains(&s.id) {
                    ids.push(s.id.clone());
                }
            }
            l.mtimes = now;
            for id in ids {
                match l.engine.compile(&id) {
                    Ok(()) => eprintln!("reloaded {id}"),
                    Err(e) => eprintln!("{e:#}\n(keeping previous version of {id})"),
                }
            }
        }

        // Advance the SIM_HZ sim to the audio clock.
        if !l.paused {
            let audio_t = l.player.get_pos().as_secs_f64();
            if audio_t < l.sim_t - 0.25 || audio_t > l.sim_t + 1.0 {
                l.sim_t = audio_t; // seek or long stall: jump
            }
            let mut n = 0;
            while l.sim_t + DT <= audio_t && n < MAX_STEPS {
                l.sim_t += DT;
                let fs = director::frame_state(self.plan, self.analysis, self.scenes, l.sim_t, l.engine.sim.0, l.engine.sim.1, self.quality);
                if let Err(e) = l.engine.step(&fs, Some(1.0 / (n + 1) as f32)) {
                    eprintln!("{e:#}");
                }
                l.fs = Some(fs);
                n += 1;
            }
            l.dirty |= n > 0;
            if audio_t - l.sim_t > MAX_STEPS as f64 * DT {
                l.sim_t = audio_t - DT; // can't keep up: drop steps rather than drift
            }
        }

        // Nothing new to show (paused, or the display outruns the sim): skip bloom + finish.
        if !l.dirty {
            return;
        }
        let Some(fs) = &l.fs else { return };
        let title = format!("{:02}:{:04.1}  {}{}", (l.sim_t / 60.0) as u32, l.sim_t % 60.0, fs.scene_a,
            fs.b.as_ref().map(|b| format!(" -> {}", b.0)).unwrap_or_default());
        if title != l.title {
            l.window.set_title(&title);
            l.title = title;
        }

        let frame = match l.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                l.surface.configure(&l.device, &l.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = l.device.create_command_encoder(&Default::default());
        l.engine.finish(&mut enc, &view, fs, true, l.debug);
        l.queue.submit([enc.finish()]);
        l.queue.present(frame);
        l.dirty = false;
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.live.is_some() {
            return;
        }
        match self.init(event_loop) {
            Ok(l) => {
                l.window.request_redraw();
                self.live = Some(l);
            }
            Err(e) => {
                self.err = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(l) = self.live.as_mut() {
                    if size.width == 0 || size.height == 0 || (size.width, size.height) == (l.config.width, l.config.height) {
                        return; // minimised, or no change
                    }
                    l.config.width = size.width;
                    l.config.height = size.height;
                    l.surface.configure(&l.device, &l.config);
                    // Sim/out textures and finish uniforms are sized at construction: rebuild (feedback restarts; sim_t kept).
                    l.engine = Engine::new(&l.device, &l.queue, self.scenes, self.scenes_dir, self.quality, size.width, size.height, l.config.format, &self.plates);
                    l.fs = None; // stale: carries the old sim size
                    l.dirty = false;
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && !event.repeat => {
                let Some(l) = self.live.as_mut() else { return };
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::Space) => {
                        l.paused = !l.paused;
                        if l.paused { l.player.pause() } else { l.player.play() }
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("r") => {
                        if let Err(e) = l.player.try_seek(Duration::from_secs_f64(self.start)) {
                            eprintln!("seek: {e}");
                        }
                        l.sim_t = self.start;
                        l.engine.reset();
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("d") => {
                        l.debug = !l.debug;
                        l.dirty = true;
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.frame();
                if let Some(l) = &self.live {
                    l.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// [prelude, scene 0, scene 1, ...] modification times.
fn mtimes(scenes: &[Scene], dir: &Path) -> Vec<Option<SystemTime>> {
    let mt = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    std::iter::once(mt(&scenes::prelude_path(dir))).chain(scenes.iter().map(|s| mt(&s.wgsl_path))).collect()
}
