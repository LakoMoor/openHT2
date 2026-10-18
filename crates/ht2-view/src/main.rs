use glam::{Mat4, Vec3};
use ht2_render::{request_device, save_png, scene, ObjectId, Renderer, Scene};
use std::{path::PathBuf, process::ExitCode, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const USAGE: &str = "usage: ht2-view <model.b3d> [--shot out.png] [--yaw DEG] [--pitch DEG] [--size WxH] [--only BLOCK]";

#[derive(Clone, Copy)]
struct Camera {
    yaw: f32,
    pitch: f32,
    dist: f32,
    center: Vec3,
    radius: f32,
}

impl Camera {
    /// The game world is Z-up.
    fn view_proj(&self, aspect: f32) -> Mat4 {
        let dir = Vec3::new(self.yaw.sin() * self.pitch.cos(), self.yaw.cos() * self.pitch.cos(), self.pitch.sin());
        let eye = self.center + dir * self.dist;
        Mat4::perspective_rh(50f32.to_radians(), aspect, self.radius * 0.01, self.radius * 50.0)
            * Mat4::look_at_rh(eye, self.center, Vec3::Z)
    }
}

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
    let cam = Camera {
        yaw: yaw.to_radians(),
        pitch: pitch.to_radians(),
        dist: scene.radius * 2.6,
        center: Vec3::from(scene.center),
        radius: scene.radius,
    };

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    match shot {
        Some(path) => {
            let (_, device, queue) = pollster::block_on(request_device(&instance, None))?;
            let mut r = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
            r.add(&scene);
            let rgba = r.screenshot(size, cam.view_proj(size.0 as f32 / size.1 as f32));
            save_png(&path, size, &rgba)?;
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

struct Gfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    _object: ObjectId,
}

struct App {
    instance: wgpu::Instance,
    scene: Option<Scene>,
    cam: Camera,
    gfx: Option<Gfx>,
    drag: Option<(f64, f64)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let window =
            Arc::new(el.create_window(Window::default_attributes().with_title("openHT2 viewer")).expect("create window"));
        let surface = self.instance.create_surface(window.clone()).expect("create surface");
        let scene = self.scene.take().expect("scene loaded");
        let size = window.inner_size();
        let (adapter, device, queue) = pollster::block_on(request_device(&self.instance, Some(&surface))).expect("GPU");
        let format = surface.get_capabilities(&adapter).formats[0];
        let mut renderer = Renderer::new(device, queue, format);
        let object = renderer.add(&scene);
        let config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).expect("surface config");
        surface.configure(&renderer.device, &config);
        self.gfx = Some(Gfx { window, surface, config, renderer, _object: object });
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
                self.cam.dist = (self.cam.dist * 0.9f32.powf(dy)).clamp(self.cam.radius * 0.05, self.cam.radius * 30.0);
                g.window.request_redraw();
            }
            WindowEvent::RedrawRequested => match g.surface.get_current_texture() {
                Ok(frame) => {
                    let view = frame.texture.create_view(&Default::default());
                    let size = (g.config.width, g.config.height);
                    g.renderer.draw(&view, size, self.cam.view_proj(size.0 as f32 / size.1 as f32));
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
