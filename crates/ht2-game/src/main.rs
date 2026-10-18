mod ground;
mod vehicle;

use glam::{Mat4, Vec3};
use ground::Ground;
use ht2_render::{request_device, save_png, scene, ObjectId, Renderer, Scene};
use std::{collections::HashSet, path::PathBuf, process::ExitCode, sync::Arc, time::Instant};
use vehicle::{Input, Vehicle};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const USAGE: &str = "usage: ht2-game <game dir> [--map MENV/dr] [--truck Kamaz] [--shot out.png [--seconds N]]";
const STEP: f32 = 1.0 / 120.0;

struct World {
    ground: Ground,
    vehicle: Vehicle,
    world_scene: Scene,
    truck_scene: Scene,
    spawn: (Vec3, f32, f32),
}

fn load_world(dir: &PathBuf, map: &str, truck: &str) -> Result<World, String> {
    let world_scene = scene::load(&dir.join(format!("{map}.b3d")), None)?;
    let truck_scene = scene::load(&dir.join("COMMON/TRUCKS.B3D"), Some(truck))?;
    let ground = Ground::new(&world_scene);
    let (pos, heading) = Ground::spawn_on(&world_scene, "road").ok_or("no road found to spawn on")?;
    let low = truck_scene.batches.iter().flat_map(|b| b.verts.iter()).map(|v| v.pos[2]).fold(f32::MAX, f32::min);
    let ride = -low;
    println!("ground: {} triangles, spawn at {pos:?}, heading {heading:.2}, ride height {ride:.2}", ground.triangle_count());
    let vehicle = Vehicle::new(pos, heading, ride);
    Ok(World { ground, vehicle, world_scene, truck_scene, spawn: (pos, heading, ride) })
}

fn chase_view_proj(v: &Vehicle, aspect: f32) -> Mat4 {
    let fwd = v.forward();
    let eye = v.pos - fwd * 16.0 + Vec3::Z * 6.5;
    let target = v.pos + fwd * 10.0 + Vec3::Z * 1.5;
    Mat4::perspective_rh(60f32.to_radians(), aspect, 0.3, 4000.0) * Mat4::look_at_rh(eye, target, Vec3::Z)
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
    let mut dir = None;
    let (mut map, mut truck, mut shot, mut seconds) = ("MENV/dr".to_string(), "Kamaz".to_string(), None, 10.0f32);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(USAGE.to_string());
        match a.as_str() {
            "--map" => map = val()?,
            "--truck" => truck = val()?,
            "--shot" => shot = Some(PathBuf::from(val()?)),
            "--seconds" => seconds = val()?.parse().map_err(|_| USAGE)?,
            _ => dir = Some(PathBuf::from(a)),
        }
    }
    let world = load_world(&dir.ok_or(USAGE)?, &map, &truck)?;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

    match shot {
        Some(path) => {
            // Headless demo: full throttle with a gentle weave, then one frame.
            let size = (1024, 768);
            let (_, device, queue) = pollster::block_on(request_device(&instance, None))?;
            let mut game = Game::new(Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm), world);
            let mut t = 0.0;
            while t < seconds {
                game.world.vehicle.step(Input { steer: (t * 0.6).sin() * 0.25, throttle: 1.0, brake: 0.0 }, &game.world.ground, STEP);
                t += STEP;
            }
            println!("after {seconds}s: pos {:?}, speed {:.1} m/s", game.world.vehicle.pos, game.world.vehicle.speed);
            game.renderer.set_model(game.truck, game.world.vehicle.model());
            let rgba = game.renderer.screenshot(size, chase_view_proj(&game.world.vehicle, size.0 as f32 / size.1 as f32));
            save_png(&path, size, &rgba)?;
            println!("wrote {}", path.display());
            Ok(())
        }
        None => {
            let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
            let mut app = App { instance, world: Some(world), gfx: None, keys: HashSet::new(), last: Instant::now(), acc: 0.0 };
            event_loop.run_app(&mut app).map_err(|e| e.to_string())
        }
    }
}

struct Game {
    renderer: Renderer,
    truck: ObjectId,
    world: World,
}

impl Game {
    fn new(mut renderer: Renderer, world: World) -> Self {
        renderer.add(&world.world_scene);
        let truck = renderer.add(&world.truck_scene);
        renderer.set_model(truck, world.vehicle.model());
        Game { renderer, truck, world }
    }
}

struct Gfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    game: Game,
}

struct App {
    instance: wgpu::Instance,
    world: Option<World>,
    gfx: Option<Gfx>,
    keys: HashSet<KeyCode>,
    last: Instant,
    acc: f32,
}

impl App {
    fn input(&self) -> Input {
        let k = |a: KeyCode, b: KeyCode| self.keys.contains(&a) || self.keys.contains(&b);
        let axis = |neg: bool, pos: bool| f32::from(pos) - f32::from(neg);
        Input {
            steer: axis(k(KeyCode::KeyD, KeyCode::ArrowRight), k(KeyCode::KeyA, KeyCode::ArrowLeft)),
            throttle: f32::from(k(KeyCode::KeyW, KeyCode::ArrowUp)),
            brake: f32::from(k(KeyCode::KeyS, KeyCode::ArrowDown) || self.keys.contains(&KeyCode::Space)),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let window = Arc::new(el.create_window(Window::default_attributes().with_title("openHT2")).expect("create window"));
        let surface = self.instance.create_surface(window.clone()).expect("create surface");
        let size = window.inner_size();
        let (adapter, device, queue) = pollster::block_on(request_device(&self.instance, Some(&surface))).expect("GPU");
        let format = surface.get_capabilities(&adapter).formats[0];
        let game = Game::new(Renderer::new(device, queue, format), self.world.take().expect("world loaded"));
        let config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).expect("surface config");
        surface.configure(&game.renderer.device, &config);
        window.request_redraw();
        self.gfx = Some(Gfx { window, surface, config, game });
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state.is_pressed() {
                        self.keys.insert(code);
                        if code == KeyCode::Escape {
                            el.exit();
                        }
                        if code == KeyCode::KeyR {
                            if let Some(g) = self.gfx.as_mut() {
                                let (p, h, r) = g.game.world.spawn;
                                g.game.world.vehicle = Vehicle::new(p, h, r);
                            }
                        }
                    } else {
                        self.keys.remove(&code);
                    }
                }
            }
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                if let Some(g) = self.gfx.as_mut() {
                    g.config.width = s.width;
                    g.config.height = s.height;
                    g.surface.configure(&g.game.renderer.device, &g.config);
                }
            }
            WindowEvent::RedrawRequested => {
                let input = self.input();
                let now = Instant::now();
                self.acc += (now - self.last).as_secs_f32().min(0.1);
                self.last = now;
                let Some(g) = self.gfx.as_mut() else { return };
                while self.acc >= STEP {
                    g.game.world.vehicle.step(input, &g.game.world.ground, STEP);
                    self.acc -= STEP;
                }
                g.game.renderer.set_model(g.game.truck, g.game.world.vehicle.model());
                match g.surface.get_current_texture() {
                    Ok(frame) => {
                        let view = frame.texture.create_view(&Default::default());
                        let size = (g.config.width, g.config.height);
                        let vp = chase_view_proj(&g.game.world.vehicle, size.0 as f32 / size.1 as f32);
                        g.game.renderer.draw(&view, size, vp);
                        g.window.set_title(&format!("openHT2 - {:.0} km/h", g.game.world.vehicle.speed * 3.6));
                        g.window.pre_present_notify();
                        frame.present();
                    }
                    Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => g.surface.configure(&g.game.renderer.device, &g.config),
                    Err(e) => eprintln!("surface error: {e}"),
                }
                g.window.request_redraw();
            }
            _ => {}
        }
    }
}
