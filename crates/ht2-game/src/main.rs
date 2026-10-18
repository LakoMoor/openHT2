mod ground;
mod vehicle;

use glam::{Mat4, Vec3};
use ground::Ground;
use ht2_formats::{b3d, sdte, sexpr, tech};
use ht2_render::{request_device, save_png, scene, ObjectId, Renderer, Scene};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::Arc,
    time::Instant,
};
use vehicle::{Input, Params, Vehicle};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const USAGE: &str = "usage: ht2-game <game dir> [--map MENV/dr | ENV] [--truck Kamaz] [--shot out.png [--seconds N] [--overview]]";
const STEP: f32 = 1.0 / 120.0;

struct World {
    ground: Ground,
    vehicle: Vehicle,
    world_scenes: Vec<Scene>,
    truck_scene: Scene,
    spawn: (Vec3, f32, f32),
}

/// What a `GAME.CNF` says about where and as what the player starts.
#[derive(Default)]
struct Scenario {
    start: Option<Vec3>,
    truck: Option<String>,
}

fn read_scenario(dir: &Path) -> Scenario {
    let Ok(raw) = fs::read(dir.join("GAME.CNF")) else { return Scenario::default() };
    let Ok(plain) = sdte::decrypt(&raw) else { return Scenario::default() };
    let Ok(top) = sexpr::parse(&sdte::cp1251_to_string(&plain)) else { return Scenario::default() };
    let Some(root) = top.first() else { return Scenario::default() };
    let start = root.find_form("startposition").and_then(|v| match v {
        [x, y, z] => Some(Vec3::new(x.as_f32()?, y.as_f32()?, z.as_f32()?)),
        _ => None,
    });
    let truck = root
        .find_form("playertypes")
        .and_then(|l| l.iter().find_map(|c| c.form("live")))
        .and_then(|live| live.get(1)?.as_text().map(str::to_owned));
    Scenario { start, truck }
}

/// `map` is either a model path without extension (one scene) or a folder holding many.
fn load_scenes(dir: &Path, map: &str) -> Result<(Vec<Scene>, PathBuf), String> {
    let path = dir.join(map);
    if path.is_dir() {
        let mut files: Vec<_> = fs::read_dir(&path)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("b3d")))
            .collect();
        files.sort();
        let mut scenes = Vec::new();
        for f in files {
            match scene::load(&f, None) {
                Ok(s) => scenes.push(s),
                Err(e) => eprintln!("skipping {}: {e}", f.display()),
            }
        }
        println!("loaded {} scenes from {}", scenes.len(), path.display());
        Ok((scenes, path))
    } else {
        let scene = scene::load(&path.with_extension("b3d"), None)?;
        let folder = path.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.to_path_buf());
        Ok((vec![scene], folder))
    }
}

fn truck_params(dir: &Path, prefix: &str) -> Result<Params, String> {
    let techs = tech::parse(&fs::read(dir.join("vehicle.tech")).map_err(|e| format!("vehicle.tech: {e}"))?)
        .map_err(|e| e.to_string())?;
    let t = techs
        .iter()
        .find(|t| t.prefix.eq_ignore_ascii_case(prefix))
        .ok_or_else(|| format!("no vehicle with model prefix {prefix:?} in vehicle.tech"))?;
    let model = b3d::parse(&fs::read(dir.join("COMMON/TRUCKS.B3D")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let wheels: Vec<Vec3> = (0..8).map_while(|i| model.space_origin(&format!("{}Wheel{i}Space", t.prefix)).map(Vec3::from)).collect();
    if wheels.len() < 2 {
        return Err(format!("could not find the wheel positions of {prefix:?}"));
    }
    let front_y = wheels.iter().map(|w| w.y).fold(f32::MIN, f32::max);
    let rear: Vec<f32> = wheels.iter().map(|w| w.y).filter(|y| *y < front_y - 0.3).collect();
    let rear_y = if rear.is_empty() { front_y - 3.0 } else { rear.iter().sum::<f32>() / rear.len() as f32 };
    let half_track = wheels.iter().map(|w| w.x.abs()).fold(0.0, f32::max);
    println!(
        "{}: {:.0} kg, {} hp, {} gears, {:.0} N*m, wheelbase {:.2} m, track {:.2} m",
        t.name,
        t.mass,
        t.horse_power,
        t.num_gears,
        t.max_torque,
        front_y - rear_y,
        half_track * 2.0
    );
    Ok(Params::from_tech(t, front_y, rear_y, half_track))
}

fn load_world(dir: &Path, map: &str, truck: Option<&str>) -> Result<World, String> {
    let (world_scenes, folder) = load_scenes(dir, map)?;
    let scenario = read_scenario(&folder);
    let truck = truck.map(str::to_owned).or(scenario.truck).unwrap_or_else(|| "Kamaz".into());
    let truck_scene = scene::load(&dir.join("COMMON/TRUCKS.B3D"), Some(&truck))?;
    let params = truck_params(dir, &truck)?;
    let ground = Ground::new(&world_scenes);
    let (pos, heading) = match scenario.start {
        Some(p) => {
            let z = ground.height_at(p.x, p.y, p.z + 200.0, 0.0).unwrap_or(p.z);
            (Vec3::new(p.x, p.y, z), Ground::road_heading_near(&world_scenes, p))
        }
        None => Ground::spawn_on_road(&world_scenes).ok_or("no road found to spawn on")?,
    };
    let low = truck_scene.batches.iter().flat_map(|b| b.verts.iter()).map(|v| v.pos[2]).fold(f32::MAX, f32::min);
    let ride = -low;
    println!("{truck}: ground {} triangles, spawn at {pos:?}, heading {heading:.2}, ride height {ride:.2}", ground.triangle_count());
    let vehicle = Vehicle::new(params, pos, heading, ride);
    Ok(World { ground, vehicle, world_scenes, truck_scene, spawn: (pos, heading, ride) })
}

fn chase_view_proj(v: &Vehicle, aspect: f32) -> Mat4 {
    let fwd = v.forward();
    let eye = v.pos - fwd * 16.0 + Vec3::Z * 6.5;
    let target = v.pos + fwd * 10.0 + Vec3::Z * 1.5;
    Mat4::perspective_rh(60f32.to_radians(), aspect, 0.3, 6000.0) * Mat4::look_at_rh(eye, target, Vec3::Z)
}

/// Looking straight down at the vehicle from `height` metres.
fn overview_view_proj(v: &Vehicle, aspect: f32, height: f32) -> Mat4 {
    let eye = v.pos + Vec3::Z * height;
    Mat4::perspective_rh(50f32.to_radians(), aspect, 1.0, height * 4.0) * Mat4::look_at_rh(eye, v.pos, Vec3::Y)
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
    let (mut map, mut truck, mut shot, mut seconds, mut overview) = ("MENV/dr".to_string(), None, None, 10.0f32, false);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(USAGE.to_string());
        match a.as_str() {
            "--map" => map = val()?,
            "--truck" => truck = Some(val()?),
            "--shot" => shot = Some(PathBuf::from(val()?)),
            "--seconds" => seconds = val()?.parse().map_err(|_| USAGE)?,
            "--overview" => overview = true,
            _ => dir = Some(PathBuf::from(a)),
        }
    }
    let dir = dir.ok_or(USAGE)?;
    let world = load_world(&dir, &map, truck.as_deref())?;
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
            let v = &game.world.vehicle;
            println!("after {seconds}s: pos {:?}, speed {:.1} m/s, gear {}, {:.0} rpm", v.pos, v.speed, v.gear, v.rpm());
            game.renderer.set_model(game.truck, v.model());
            let aspect = size.0 as f32 / size.1 as f32;
            let vp = if overview { overview_view_proj(v, aspect, 1500.0) } else { chase_view_proj(v, aspect) };
            let rgba = game.renderer.screenshot(size, vp);
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
        for scene in &world.world_scenes {
            renderer.add(scene);
        }
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
                                g.game.world.vehicle = Vehicle::new(g.game.world.vehicle.params.clone(), p, h, r);
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
                        let v = &g.game.world.vehicle;
                        g.window.set_title(&format!("openHT2 - {:.0} km/h, gear {}, {:.0} rpm", v.speed * 3.6, v.gear, v.rpm()));
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
