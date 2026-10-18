mod render;
mod scene;

use render::{Camera, Renderer};
use std::{path::PathBuf, process::ExitCode, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const USAGE: &str = "usage: ht2-view <model.b3d> [--shot out.png] [--yaw DEG] [--pitch DEG] [--size WxH] [--only BLOCK]";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut model = None;
    let mut only = None;
    let (mut shot, mut yaw, mut pitch, mut size) = (None, 35.0f32, 25.0f32, (1024u32, 768u32));
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(USAGE.to_string());
        match a.as_str() {
            "--shot" => shot = Some(PathBuf::from(val()?)),
            "--only" => only = Some(val()?),
            "--yaw" => yaw = val()?.parse().map_err(|_| USAGE)?,
            "--pitch" => pitch = val()?.parse().map_err(|_| USAGE)?,
            "--size" => {
                let v = val()?;
                let (w, h) = v.split_once('x').ok_or(USAGE)?;
                size = (w.parse().map_err(|_| USAGE)?, h.parse().map_err(|_| USAGE)?);
            }
            _ => model = Some(PathBuf::from(a)),
        }
    }
    let scene = scene::load(&model.ok_or(USAGE)?, only.as_deref())?;
    let cam = Camera { yaw: yaw.to_radians(), pitch: pitch.to_radians(), dist: scene.radius * 2.6 };

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    match shot {
        Some(path) => {
            let mut r = pollster::block_on(init_gpu(&instance, None, &scene, wgpu::TextureFormat::Rgba8Unorm))?.0;
            let rgba = r.screenshot(size, cam);
            let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size.0, size.1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().and_then(|mut w| w.write_image_data(&rgba)).map_err(|e| e.to_string())?;
            println!("wrote {}", path.display());
            Ok(())
        }
        None => {
            let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
            let mut app = App { instance, scene: Some(scene), cam, gfx: None, drag: None };
            event_loop.run_app(&mut app).map_err(|e| e.to_string())
        }
    }
}

async fn init_gpu(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    scene: &scene::Scene,
    format: wgpu::TextureFormat,
) -> Result<(Renderer, wgpu::Adapter), String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: surface,
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        })
        .await
        .ok_or("no suitable GPU adapter")?;
    let (device, queue) =
        adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await.map_err(|e| e.to_string())?;
    Ok((Renderer::new(device, queue, format, scene), adapter))
}

struct Gfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

struct App {
    instance: wgpu::Instance,
    scene: Option<scene::Scene>,
    cam: Camera,
    gfx: Option<Gfx>,
    drag: Option<(f64, f64)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let window = Arc::new(
            el.create_window(Window::default_attributes().with_title("openHT2 viewer")).expect("create window"),
        );
        let surface = self.instance.create_surface(window.clone()).expect("create surface");
        let scene = self.scene.take().expect("scene loaded");
        let size = window.inner_size();
        // The surface format is only known once an adapter is chosen, so probe it first.
        let adapter = pollster::block_on(self.instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter");
        let format = surface.get_capabilities(&adapter).formats[0];
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).expect("device");
        let renderer = Renderer::new(device, queue, format, &scene);
        let config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).expect("surface config");
        surface.configure(&renderer.device, &config);
        self.gfx = Some(Gfx { window, surface, config, renderer });
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(g) = self.gfx.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                g.config.width = s.width;
                g.config.height = s.height;
                g.surface.configure(&g.renderer.device, &g.config);
                g.window.request_redraw();
            }
            WindowEvent::MouseInput { button: MouseButton::Left, state, .. } => {
                self.drag = (state == ElementState::Pressed).then_some((f64::NAN, 0.0));
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(last) = self.drag {
                    if !last.0.is_nan() {
                        self.cam.yaw -= ((position.x - last.0) * 0.01) as f32;
                        self.cam.pitch = (self.cam.pitch + ((position.y - last.1) * 0.01) as f32).clamp(-1.5, 1.5);
                        g.window.request_redraw();
                    }
                    self.drag = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.cam.dist = (self.cam.dist * 0.9f32.powf(dy)).clamp(g.renderer.radius * 0.05, g.renderer.radius * 30.0);
                g.window.request_redraw();
            }
            WindowEvent::RedrawRequested => match g.surface.get_current_texture() {
                Ok(frame) => {
                    let view = frame.texture.create_view(&Default::default());
                    g.renderer.draw(&view, (g.config.width, g.config.height), self.cam);
                    g.window.pre_present_notify();
                    frame.present();
                }
                Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => g.surface.configure(&g.renderer.device, &g.config),
                Err(e) => eprintln!("surface error: {e}"),
            },
            _ => {}
        }
    }
}
