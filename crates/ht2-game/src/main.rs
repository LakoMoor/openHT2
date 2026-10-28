mod ground;
mod world;
mod powertrain;
mod simcar;
mod vehicle;
mod audio;
mod movie;
mod menu;
mod camera;
mod cockpit;
mod event_scene;
mod event_heap;
mod particle_memory;

use glam::{Mat4, Vec3};
use ground::Ground;
use ht2_formats::{b3d, sdte, sexpr, tech};
use ht2_render::{request_device, save_png, scene, ObjectId, Renderer, Scene, TexturesId};
use std::{
    collections::{HashMap, HashSet},
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

const USAGE: &str = "usage: ht2-game [<game dir>] [--map ENV | MENV/dr] [--truck Kamaz] [--shot out.png [--seconds N] [--overview | --cab]]
\n  the game dir defaults to $HT2_GAME, else a folder \"Hard Truck 2 King of the Road\" next to the current one or its parents";
const STEP: f32 = 1.0 / 120.0;

struct World {
    game_dir: PathBuf,
    ground: Ground,
    vehicle: Vehicle,
    world_scenes: Vec<Scene>,
    models: Models,
    truck: Truck,
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
/// Also returns the parsed models (for their collision).
fn load_scenes(dir: &Path, map: &str) -> Result<(Vec<Scene>, PathBuf, Vec<b3d::B3d>), String> {
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
        let mut coll = Vec::new();
        for f in files {
            match scene::load_assets(&f) {
                Ok((model, mut s)) => {
                    let batches = model.triangles();
                    if batches.is_empty() {
                        eprintln!("skipping {}: model has no triangles", f.display());
                        continue;
                    }
                    s.set_batches(batches);
                    scenes.push(s);
                    coll.push(model);
                }
                Err(e) => eprintln!("skipping {}: {e}", f.display()),
            }
        }
        println!("loaded {} scenes from {}", scenes.len(), path.display());
        Ok((scenes, path, coll))
    } else {
        let (model, mut scene) = scene::load_assets(&path.with_extension("b3d"))?;
        scene.set_batches(model.triangles());
        let folder = path.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.to_path_buf());
        Ok((vec![scene], folder, vec![model]))
    }
}

/// The road network of the `.way` files next to the models: all of them when `all`, else the
/// one of the model `map`.
fn load_roads(folder: &Path, map: &str, all: bool) -> ht2_sim::road::RoadNet {
    let one = Path::new(map).file_name().map(|n| n.to_string_lossy().to_lowercase());
    let mut files: Vec<_> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("way")))
        .filter(|p| all || p.file_stem().map(|n| n.to_string_lossy().to_lowercase()) == one)
        .collect();
    files.sort();
    let mut ways = Vec::new();
    for f in files {
        let Ok(data) = fs::read(&f) else { continue };
        match ht2_formats::way::parse(&data) {
            Ok(chunks) => ways.push(ht2_formats::way::rooms(&chunks)),
            Err(e) => eprintln!("skipping {}: {e}", f.display()),
        }
    }
    let mut net = ht2_sim::road::RoadNet::build(&ways);
    let scripts = ht2_sim::road::read_map_scripts(folder);
    net.mark_towns(&ht2_sim::road::Town::from_scripts(&scripts));
    net.mark_parkings(&ht2_sim::road::Parking::from_scripts(&scripts));
    net
}

/// One wheel: where it sits on the body and whether the steering turns it.
struct Wheel {
    /// Space of the wheel in body coordinates (axes are the columns).
    mount: Mat4,
    steered: bool,
    /// +1 if the wheel axle points along the body X axis, -1 if the mount mirrors it.
    spin_sign: f32,
}

/// A vehicle model split into the body and one part per wheel.
struct Truck {
    params: Params,
    body: Vec<b3d::Batch>,
    wheel_parts: Vec<Vec<b3d::Batch>>,
    wheels: Vec<Wheel>,
    ride_height: f32,
    /// Where the driver's eyes are in the model (stand-in: the original's cab view, with the
    /// interiors of `COMMON/CABINES.B3D`, is not ported): left of the middle, 0.5 m under the
    /// roof of the front 3 m of the body, 1.2 m behind its front.
    cab_eye: Vec3,
    cab_tilt:f32,
}

/// The vehicle models (`COMMON/TRUCKS.B3D`, all of them in one file with their textures) and
/// the records of `vehicle.tech`, whose model prefixes name the blocks of each vehicle.
struct Models {
    b3d: b3d::B3d,
    /// The textures and materials, no geometry.
    assets: Scene,
    techs: Vec<tech::Tech>,
    fallback_cabin:Option<String>,
}

fn load_models(dir: &Path) -> Result<Models, String> {
    let data=fs::read(dir.join("vehicle.tech")).map_err(|e|format!("vehicle.tech: {e}"))?;
    let techs = tech::parse(&data).map_err(|e| e.to_string())?;
    let fallback_cabin=tech::cabin_fallback_index(&data).ok().and_then(|index|techs.iter().find(|t|t.index==index)).map(|t|t.cabin.clone());
    let (b3d, assets) = scene::load_assets(&dir.join("COMMON/TRUCKS.B3D"))?;
    Ok(Models { b3d, assets, techs, fallback_cabin })
}

impl Models {
    fn tech(&self, prefix: &str) -> Result<&tech::Tech, String> {
        self.techs.iter().find(|t| t.prefix.eq_ignore_ascii_case(prefix)).ok_or_else(|| format!("no vehicle with model prefix {prefix:?} in vehicle.tech"))
    }

    /// The transforms of the `<prefix>Wheel<k>Space` and `<prefix>AddWheel<k>Space` nodes.
    fn nodes(&self, prefix: &str) -> (simcar::WheelNodes, simcar::WheelNodes) {
        let node = |name: String| {
            self.b3d.space(&name).map(|(axes, pos)| {
                let mut v = [0.0f32; 12];
                for (i, a) in axes.iter().enumerate() {
                    v[3 * i..3 * i + 3].copy_from_slice(&<[f32; 3]>::from(*a));
                }
                v[9..].copy_from_slice(&<[f32; 3]>::from(pos));
                v
            })
        };
        (core::array::from_fn(|i| node(format!("{prefix}Wheel{i}Space"))), core::array::from_fn(|i| node(format!("{prefix}AddWheel{i}Space"))))
    }

    /// The wheel nodes of every record of `vehicle.tech` that has a model, by record.
    fn all_nodes(&self) -> Vec<(usize, String, simcar::WheelNodes, simcar::WheelNodes)> {
        self.techs
            .iter()
            .filter(|t| self.b3d.space(&format!("{}Wheel0Space", t.prefix)).is_some())
            .map(|t| {
                let (w, a) = self.nodes(&t.prefix);
                (t.index, t.prefix.clone(), w, a)
            })
            .collect()
    }

    fn truck(&self, prefix: &str) -> Result<Truck, String> {
        let t = self.tech(prefix)?;
        let model = &self.b3d;
        // Wheels are separate groups placed by `<prefix>Wheel<N>Space` links.
        let mut wheels = Vec::new();
        let mut wheel_groups = Vec::new();
        let mut wheel_parts = Vec::new();
        for i in 0..8 {
            let space = format!("{}Wheel{i}Space", t.prefix);
            let (Some((axes, pos)), Some(group)) = (model.space(&space), model.linked_group(&space)) else { break };
            let axes = axes.map(Vec3::from);
            wheel_parts.push(model.group_triangles(group));
            wheel_groups.push(group.to_owned());
            wheels.push(Wheel {
                mount: Mat4::from_cols(axes[0].extend(0.0), axes[1].extend(0.0), axes[2].extend(0.0), Vec3::from(pos).extend(1.0)),
                // the original turns the first two wheels with the steering (0x547030)
                steered: i < 2,
                spin_sign: if axes[0].x < 0.0 { -1.0 } else { 1.0 },
            });
        }
        if wheels.len() < 2 {
            return Err(format!("could not find the wheels of {prefix:?}"));
        }
        let skip: Vec<&str> = wheel_groups.iter().map(String::as_str).collect();
        let body = model.triangles_skipping(Some(&t.prefix), &skip);
        if body.is_empty() {
            return Err(format!("no model blocks named {:?}", t.prefix));
        }

        // The lowest point of a wheel in body coordinates sets the ride height.
        let low = wheel_parts
            .iter()
            .zip(&wheels)
            .flat_map(|(p, w)| p.iter().flat_map(|b| &b.verts).map(|v| w.mount.transform_point3(Vec3::from(v.pos)).z))
            .fold(f32::MAX, f32::min);

        let positions: Vec<Vec3> = wheels.iter().map(|w| w.mount.w_axis.truncate()).collect();
        let (wheel_nodes, add_nodes) = self.nodes(&t.prefix);
        let params = truck_params(t, &positions, wheel_nodes, add_nodes)?;
        let cab_eye=Vec3::from(t.driver_seat);
        Ok(Truck { params, body, wheel_parts, wheels, ride_height: -low, cab_eye, cab_tilt:t.driver_tilt })
    }
}

fn truck_params(t: &tech::Tech, wheels: &[Vec3], wheel_nodes: simcar::WheelNodes, add_nodes: simcar::WheelNodes) -> Result<Params, String> {
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
    Ok(Params::from_tech(t, wheels.len(), front_y, rear_y, half_track, wheel_nodes, add_nodes))
}

fn load_world(dir: &Path, map: &str, truck: Option<&str>) -> Result<World, String> {
    let (world_scenes, folder, coll) = load_scenes(dir, map)?;
    let scenario = read_scenario(&folder);
    let truck = truck.map(str::to_owned).or(scenario.truck).unwrap_or_else(|| "Kamaz".into());
    let models = load_models(dir)?;
    let truck_model = models.truck(&truck)?;
    let mut ground = Ground::new(&world_scenes);
    ground.scene = world::Scene::new(&coll);
    println!("collision: {} rooms", ground.scene.room_count());
    if let Ok(ids) = std::env::var("HT2_PORTALS") {
        for id in ids.split(',').filter_map(|v| v.parse::<u32>().ok()) {
            for l in ground.scene.debug_portals(id) {
                println!("room {id}: {l}");
            }
        }
    }
    ground.roads = std::rc::Rc::new(load_roads(&folder, map, dir.join(map).is_dir()));
    println!("roads: {} nodes, {} roads", ground.roads.nodes.len(), ground.roads.roads.len());
    // The scenario start is only usable if there is ground under it (the menu scenes have none).
    let start = scenario.start.and_then(|p| ground.height_at(p.x, p.y, p.z + 200.0, 0.0).map(|z| Vec3::new(p.x, p.y, z)));
    let (pos, heading) = match start {
        Some(p) => (p, Ground::road_heading_near(&world_scenes, p)),
        None => Ground::spawn_on_road(&world_scenes).ok_or("no road found to spawn on")?,
    };
    let ride = truck_model.ride_height;
    println!("{truck}: ground {} triangles, spawn at {pos:?}, heading {heading:.2}, ride height {ride:.2}", ground.triangle_count());
    let tech_file = std::rc::Rc::new(fs::read(dir.join("vehicle.tech")).map_err(|e| format!("vehicle.tech: {e}"))?);
    let vehicle = Vehicle::new(truck_model.params.clone(), tech_file, pos, heading, ride, &ground);
    println!("vehicle physics: {}", if vehicle.is_ported() { "ported from the original" } else { "stand-in" });
    let mut vehicle = vehicle;
    let traffic = std::env::var("HT2_TRAFFIC").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
    let map_dir = dir.join(map);
    if map_dir.join("GAME.CNF").is_file() && std::env::var_os("HT2_OLD_TRAFFIC").is_none() {
        // the AI world of the map: its players drive on the road network, and those near the
        // player get vehicles
        let human = ground.roads.nearest([pos.x as f64, pos.y as f64, pos.z as f64]).map(ht2_sim::road::Spot::Road);
        let ai = ht2_sim::world::World::new_game(dir, &map_dir, ground.roads.clone(), 1.0, human);
        println!("AI world: {} players", ai.players.len());
        vehicle.set_ai_world(ai, traffic.max(12), &models.all_nodes());
    } else {
        vehicle.add_traffic(&ground, traffic, 0x2545_f491);
    }
    println!("traffic: {} vehicle slots", vehicle.traffic().len());
    Ok(World { game_dir: dir.to_path_buf(), ground, vehicle, world_scenes, models, truck: truck_model, spawn: (pos, heading, ride) })
}

fn chase_view_proj(v: &Vehicle, size: (u32,u32)) -> camera::Frame {
    chase_from(v.pos, v.forward(), size)
}

/// The 2D layer (stand-in for the original's interface): the instruments at the bottom right
/// and the keys at the top left.
fn hud(v: &Vehicle, size: (u32, u32), cab: bool) -> Vec<ht2_render::hud::Rect> {
    use ht2_render::hud::{text, text_width, Rect};
    let mut out = Vec::new();
    let (w, h) = (size.0 as f32, size.1 as f32);
    let k = (h / 480.0).max(1.0).floor();
    let white = [1.0, 1.0, 1.0, 1.0];
    let amber = [1.0, 0.75, 0.2, 1.0];
    let gear = match v.gear { 0 => "R".to_string(), 1 => "N".to_string(), g => (g - 1).to_string() };
    let speed = format!("{:.0}", (v.speed * 3.6).abs());
    let lines = [format!("GEAR {gear}"), format!("RPM {:.0}", v.rpm()), format!("FUEL {:.0} L", v.fuel)];
    let pw = 150.0 * k;
    let ph = 92.0 * k;
    let (px, py) = (w - pw - 12.0 * k, h - ph - 12.0 * k);
    out.push(Rect { x: px, y: py, w: pw, h: ph, color: [0.0, 0.0, 0.0, 0.55] });
    let big = 4.0 * k;
    text(&mut out, &speed, px + pw - 10.0 * k - text_width(&speed, big) - text_width(" KM/H", k), py + 8.0 * k, big, amber);
    text(&mut out, " KM/H", px + pw - 10.0 * k - text_width(" KM/H", k), py + 8.0 * k + 21.0 * k, k, white);
    for (i, l) in lines.iter().enumerate() {
        text(&mut out, l, px + 10.0 * k, py + 42.0 * k + i as f32 * 15.0 * k, 2.0 * k * 0.5 + k * 0.5, white);
    }
    let keys = format!("W/S GAS/BRAKE  A/D STEER  C {}  R RESTART  ESC EXIT", if cab { "CHASE VIEW" } else { "CAB VIEW" });
    out.push(Rect { x: 8.0 * k, y: 8.0 * k, w: text_width(&keys, k) + 12.0 * k, h: 7.0 * k + 12.0 * k, color: [0.0, 0.0, 0.0, 0.45] });
    text(&mut out, &keys, 14.0 * k, 14.0 * k, k, white);
    out
}

/// Stand-in seat pose, looking ahead along the body. Matrix preparation and
/// projection use the original path; the original camera controller is pending.
fn cab_world_transform(v:&Vehicle,eye:Vec3,tilt:f32)->[f32;12] {
    use ht2_sim::{mem::Mem,matrix,instruments};
    let mut memory=Mem::new();ht2_sim::globals::install_constants(&mut memory);
    let (scenario,vehicle,local,body,out)=(0x2a000000,0x2a010000,0x2a020000,0x2a020100,0x2a020200);
    memory.set_u32(0x6d2078,scenario);memory.set_u32(scenario+0x268,vehicle);memory.set_u32(0x6cec68,1);
    memory.set_f32(scenario+200,tilt);matrix::put12(&mut memory,scenario+0x5f0,&matrix::IDENTITY);
    // Initial view: no look offset, height offset or suspension-filter state.
    // Dynamic channels and the original per-frame update remain pending.
    instruments::cabin_view(&mut memory,local,false);
    for (i,value) in eye.to_array().into_iter().enumerate(){memory.set_f32(local+36+i as u32*4,value);}
    let model=v.model();let axes=[model.x_axis,model.y_axis,model.z_axis,model.w_axis];
    matrix::put12(&mut memory,body,&std::array::from_fn(|i|axes[i/3][i%3]));
    matrix::mat_mul(&mut memory,out,local,body);
    std::array::from_fn(|i|memory.f32(out+i as u32*4))
}
fn cab_view_proj(v: &Vehicle, eye: Vec3, tilt:f32, size: (u32,u32)) -> camera::Frame {
    // Original cabin setup 0x4de2a0 sets +0x58=1.6, +0x54=1.2.
    camera::from_transform(cab_world_transform(v,eye,tilt),size,[1.6,1.2])
}

/// Stand-in chase pose; original distance, interpolation and collision are pending.
fn chase_from(pos: Vec3, fwd: Vec3, size: (u32,u32)) -> camera::Frame {
    let fwd = fwd.with_z(0.0).normalize_or(Vec3::Y);
    let eye = pos - fwd * 16.0 + Vec3::Z * 6.5;
    let target = pos + fwd * 10.0 + Vec3::Z * 1.5;
    camera::from_pose(eye,target,Vec3::Z,size)
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

/// Where the original game is when it is not given: `HT2_GAME`, else a folder
/// `Hard Truck 2 King of the Road` beside the current folder or one of its parents.
fn find_game_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("HT2_GAME") {
        return Some(PathBuf::from(d));
    }
    let cwd = std::env::current_dir().ok()?;
    cwd.ancestors().map(|a| a.join("Hard Truck 2 King of the Road")).find(|d| d.join("king.exe").exists() || d.join("ENV").is_dir())
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut dir = None;
    let (mut map, mut truck, mut shot, mut seconds, mut overview) = ("ENV".to_string(), None, None, 10.0f32, false);
    let mut cab = false;
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(USAGE.to_string());
        match a.as_str() {
            "--map" => map = val()?,
            "--truck" => truck = Some(val()?),
            "--shot" => shot = Some(PathBuf::from(val()?)),
            "--seconds" => seconds = val()?.parse().map_err(|_| USAGE)?,
            "--overview" => overview = true,
            "--cab" => cab = true,
            _ => dir = Some(PathBuf::from(a)),
        }
    }
    let dir = dir.or_else(find_game_dir).ok_or(USAGE)?;
    let world = load_world(&dir, &map, truck.as_deref())?;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

    match shot {
        Some(path) => {
            // Headless demo: some seconds of driving, then one frame.
            let size = (1024, 768);
            let (_, device, queue) = pollster::block_on(request_device(&instance, None))?;
            let mut game = Game::new(Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm), world);
            // The demo is driven by the ported computer driver (HT2_WEAVE=1: full throttle with a
            // gentle weave instead).
            let weave = std::env::var_os("HT2_WEAVE").is_some();
            // HT2_INPUT_TEST=1: the keyboard as a player would use it: gas for 10 s, then brake.
            if std::env::var_os("HT2_INPUT_TEST").is_some() {
                let mut t = 0.0;
                let mut next = 0.0;
                while t < 16.0 {
                    let (throttle, brake) = if t < 10.0 { (1.0, 0.0) } else { (0.0, 1.0) };
                    game.world.vehicle.step(Input { steer: 0.0, throttle, brake }, &game.world.ground, STEP);
                    t += STEP;
                    if t >= next {
                        let v = &game.world.vehicle;
                        println!("t={t:5.2} {} speed {:5.1} km/h gear {}", if brake > 0.0 { "brake" } else { "gas  " }, v.speed * 3.6, v.gear);
                        next += 1.0;
                    }
                }
                return Ok(());
            }
            if !weave {
                game.world.vehicle.set_autopilot(&game.world.ground);
            }
            let mut t = 0.0;
            let trace = std::env::var_os("HT2_TRACE").is_some();
            let mut next_print = 0.0;
            while t < seconds {
                game.world.vehicle.step(Input { steer: (t * 0.6).sin() * 0.25, throttle: 1.0, brake: 0.0 }, &game.world.ground, STEP);
                t += STEP;
                if trace && t >= next_print {
                    let below = game.world.vehicle.room_below(&game.world.ground);
                    println!("  room below {below:?}");
                    let v = &game.world.vehicle;
                    println!("t={t:5.2} room={:?} pos={:6.1?} speed={:5.1} gear={} tgt={} shift={} rev={:5.1} thr={:.2} F={:8.0} fuel={:.2}", v.room(), v.pos, v.speed, v.gear, v.last.map_or(0, |o| o.target_gear), v.last.map_or(0, |o| o.shift_code), v.engine_rev, v.last.map_or(0.0, |o| o.throttle_eff), v.last.map_or(0.0, |o| o.drive_force), v.fuel);
                    for k in 0.. {
                        let Some(st) = game.world.vehicle.traffic_state(k) else { break };
                        println!("  {st}");
                    }
                    next_print += 0.25;
                }
            }
            let v = &game.world.vehicle;
            println!("after {seconds}s: pos {:?}, speed {:.1} m/s, gear {}, {:.0} rpm, fuel {:.2} l", v.pos, v.speed, v.gear, v.rpm(), v.fuel);
            game.place_truck();
            let v = &game.world.vehicle;
            let aspect = size.0 as f32 / size.1 as f32;
            // HT2_FOLLOW=k looks at the k-th computer vehicle in the scene instead.
            let follow = std::env::var("HT2_FOLLOW").ok().and_then(|k| k.parse::<usize>().ok()).and_then(|k| v.traffic().into_iter().filter(|p| !p.hidden).nth(k));
            let cockpit_visible=cab && !overview && follow.is_none();
            let vp = match follow {
                Some(p) => chase_from(p.body.w_axis.truncate(), p.body.y_axis.truncate(), size).combined(),
                None if overview => overview_view_proj(v, aspect, 1500.0),
                None if cab => cab_view_proj(v, game.world.truck.cab_eye,game.world.truck.cab_tilt, size).combined(),
                None => chase_view_proj(v, size).combined(),
            };
            game.place_cockpit(cockpit_visible);
            game.renderer.set_hud(if cockpit_visible && game.cockpit.is_some() {Vec::new()}else{hud(&game.world.vehicle, size, cab)});
            let rgba = game.renderer.screenshot(size, vp);
            save_png(&path, size, &rgba)?;
            println!("wrote {}", path.display());
            Ok(())
        }
        None => {
            let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
            let mut app = App { instance, world: Some(world), gfx: None, keys: HashSet::new(), last: Instant::now(), acc: 0.0, cab: false };
            event_loop.run_app(&mut app).map_err(|e| e.to_string())
        }
    }
}

struct Game {
    cockpit:Option<cockpit::Cockpit>,
    #[cfg(windows)]
    radar_sound:Option<audio::RadarSound>,
    movie: Option<movie::Player>,
    movie_mode: Option<u32>,
    menu_mode: Option<u32>,
    shown_menu: Option<u32>,
    menu_buttons: menu::Buttons,
    action_index: usize,
    media_error: Option<String>,
    renderer: Renderer,
    textures: TexturesId,
    /// Body first, then one object per wheel.
    truck_objects: Vec<ObjectId>,
    /// The models of the computer vehicles, made when first needed (`None`: not in the file).
    trucks: HashMap<String, Option<Truck>>,
    /// The same objects for each computer vehicle, per model it has had.
    traffic_objects: Vec<HashMap<String, Vec<ObjectId>>>,
    world: World,
}

fn truck_parts(t: &Truck) -> Vec<&[b3d::Batch]> {
    std::iter::once(&t.body[..]).chain(t.wheel_parts.iter().map(|p| &p[..])).collect()
}

/// Place the body and wheels of `t` at pose `p`.
fn place_parts(renderer: &mut Renderer, ids: &[ObjectId], t: &Truck, p: &simcar::Pose) {
    renderer.set_model(ids[0], p.body);
    for (i, w) in t.wheels.iter().enumerate() {
        let steer = if w.steered { Mat4::from_rotation_z(p.steer) } else { Mat4::IDENTITY };
        let spin = Mat4::from_rotation_x(-p.wheel_angle[i] * w.spin_sign);
        let drop = Mat4::from_translation(-Vec3::Z * p.wheel_drop[i]);
        renderer.set_model(ids[i + 1], p.body * drop * w.mount * steer * spin);
    }
}

impl Game {
    fn finish_movie(&mut self) {
        let mode = self.movie_mode.take();
        self.movie = None; // stops audio and joins decoder workers before clearing video
        self.renderer.clear_movie();
        if let Some(menu) = mode.and_then(ht2_sim::game_message::movie_menu) {
            self.menu_mode = Some(menu);
        } else {
            self.media_error = Some("movie completion mode is not ported yet".into());
        }
    }
    fn movie_character(&mut self, ch: u16) {
        if let Some(mode) = self.movie_mode {
            let (_, finish) = ht2_sim::game_message::movie_character(mode, 0x102, ch);
            if finish { self.finish_movie(); }
        }
    }
    /// Consume only successfully started requests. Unsupported menu transitions remain
    /// pending and stop simulation rather than pretending the end-of-game UI is complete.
    fn update_media(&mut self, size: (u32, u32)) -> bool {
        if self.movie.is_none() && self.media_error.is_none() && self.menu_mode.is_none() {
            if let Some(action) = self.world.vehicle.pending_game_actions().get(self.action_index).copied() {
                let result = match action {
                    ht2_sim::game_message::Action::PlayMovie { path, mode } => {
                        self.movie_mode = Some(mode);
                        let exe = std::env::var_os("HT2_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
                        let path = self.world.game_dir.join(path.trim_start_matches(['\\', '/']));
                        movie::Player::open(&exe, &path)
                    },
                    ht2_sim::game_message::Action::Menu { mode } => {
                        self.menu_mode = Some(mode);
                        self.action_index += 1;
                        return true;
                    },
                };
                match result {
                    Ok(player) => { self.movie = Some(player); self.action_index += 1; },
                    Err(e) => { self.movie_mode = None; eprintln!("media request remains pending: {e}"); self.media_error = Some(e); },
                }
            }
        }
        if let Some(player) = &mut self.movie {
            if let Err(e) = player.update(&mut self.renderer) {
                eprintln!("movie playback failed: {e}");
                self.media_error = Some(e);
                self.movie = None;
                self.movie_mode = None;
                self.action_index = self.action_index.saturating_sub(1);
            }
        }
        if self.movie.as_ref().is_some_and(|p| p.finished) { self.finish_movie(); }
        if self.media_error.is_none() && self.menu_mode != self.shown_menu {
            if let Some(mode) = self.menu_mode {
                // The renderer uses full-colour output. Family 16 is provisional until
                // the original graphics-config choice (0x568fd0) is exposed in settings.
                let result = menu::screen_input(&self.world.game_dir, mode, 16, size, self.world.vehicle.rating_world(),Some(&self.menu_buttons))
                    .and_then(|rgba| self.renderer.set_movie_frame(size, &rgba));
                match result {
                    Ok(()) => self.shown_menu = Some(mode),
                    Err(e) => { eprintln!("menu background: {e}"); self.media_error = Some(e); },
                }
            }
        }
        self.movie.is_some() || self.media_error.is_some() || self.menu_mode.is_some()
    }

    fn new(mut renderer: Renderer, world: World) -> Self {
        for scene in &world.world_scenes {
            renderer.add(scene);
        }
        let textures = renderer.upload_textures(&world.models.assets);
        let truck_objects = renderer.add_parts(textures, &world.models.assets, &truck_parts(&world.truck));
        let traffic_objects = world.vehicle.traffic().iter().map(|_| HashMap::new()).collect();
        let cockpit=world.models.techs.iter().find(|t|t.index==world.vehicle.params.record)
            .and_then(|t|match cockpit::Cockpit::load(&mut renderer,&world.game_dir,&t.cabin,world.models.fallback_cabin.as_deref(),t.tank){Ok(c)=>Some(c),Err(e)=>{eprintln!("cockpit: {e}");None}});
        #[cfg(windows)]
        let radar_sound=match audio::RadarSound::load(&world.game_dir){Ok(s)=>Some(s),Err(e)=>{eprintln!("radar sound: {e}");None}};
        let mut game = Game { cockpit,#[cfg(windows)] radar_sound,renderer, textures, truck_objects, trucks: HashMap::new(), traffic_objects, world, movie: None, movie_mode: None, menu_mode: None, shown_menu: None, menu_buttons: Default::default(), action_index: 0, media_error: None };
        game.place_truck();
        game
    }

    /// Move the body and wheels to where the vehicle currently is.
    fn place_cockpit(&mut self,cab:bool) {
        for id in &self.truck_objects {self.renderer.set_visible(*id,!cab || self.cockpit.is_none());}
        let Some(cockpit)=&self.cockpit else{return};
        if !cab {cockpit.hide(&mut self.renderer);return;}
        let v=&self.world.vehicle;
        let t=cab_world_transform(v,self.world.truck.cab_eye,self.world.truck.cab_tilt);
        let mount=Mat4::from_cols(glam::Vec4::new(t[0],t[1],t[2],0.),glam::Vec4::new(t[3],t[4],t[5],0.),glam::Vec4::new(t[6],t[7],t[8],0.),glam::Vec4::new(t[9],t[10],t[11],1.));
        cockpit.update(&mut self.renderer,mount,v.dashboard_speed()*3.6,v.engine_rev,v.fuel,v.steering_position(),v.radar_equipped(),v.radar_lamp());
    }

    fn place_truck(&mut self) {
        let v = &self.world.vehicle;
        let me = simcar::Pose { body: v.model(), steer: v.steer_angle(), wheel_angle: core::array::from_fn(|i| v.wheel_angle(i)), wheel_drop: core::array::from_fn(|i| v.wheel_drop(i)), model: None, hidden: false };
        place_parts(&mut self.renderer, &self.truck_objects, &self.world.truck, &me);
        for (k, p) in self.world.vehicle.traffic().into_iter().enumerate() {
            let objects = &mut self.traffic_objects[k];
            for ids in objects.values() {
                for id in ids {
                    self.renderer.set_visible(*id, false);
                }
            }
            if p.hidden {
                continue;
            }
            let Some(model) = &p.model else {
                // the vehicles of the stand-in traffic are the player's model
                let ids = objects.entry(String::new()).or_insert_with(|| self.renderer.add_parts(self.textures, &self.world.models.assets, &truck_parts(&self.world.truck)));
                ids.iter().for_each(|id| self.renderer.set_visible(*id, true));
                place_parts(&mut self.renderer, ids, &self.world.truck, &p);
                continue;
            };
            let models = &self.world.models;
            let Some(t) = self.trucks.entry(model.clone()).or_insert_with(|| models.truck(model).map_err(|e| eprintln!("{model}: {e}")).ok()) else { continue };
            let ids = objects.entry(model.clone()).or_insert_with(|| self.renderer.add_parts(self.textures, &models.assets, &truck_parts(t)));
            ids.iter().for_each(|id| self.renderer.set_visible(*id, true));
            place_parts(&mut self.renderer, ids, t, &p);
        }
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
    /// C switches between the chase and the cab view.
    cab: bool,
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
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(g) = self.gfx.as_mut().filter(|g|g.game.menu_mode.is_some()) {
                    g.game.menu_buttons.cursor=Some((position.x as i32,position.y as i32));
                    let mode=g.game.menu_mode.unwrap();
                    if let Ok(rects)=menu::button_rects(&g.game.world.game_dir,mode,16,(g.config.width,g.config.height)) {
                        g.game.menu_buttons.event(&rects,0x200);
                        g.game.shown_menu=None;
                    }
                }
            }
            WindowEvent::MouseInput { state, button: winit::event::MouseButton::Left, .. } => {
                if let Some(g) = self.gfx.as_mut().filter(|g|g.game.menu_mode.is_some()) {
                    let mode=g.game.menu_mode.unwrap();
                    if let Ok(rects)=menu::button_rects(&g.game.world.game_dir,mode,16,(g.config.width,g.config.height)) {
                        g.game.menu_buttons.event(&rects,if state.is_pressed(){0x201}else{0x202});
                        g.game.shown_menu=None;
                        if let Some(action)=g.game.menu_buttons.action.take() {
                            if action==0xac {
                                // 0x51fd70: continue closes the end screen and resumes
                                // the existing world; it does not create a new game.
                                g.game.menu_mode=None;
                                g.game.renderer.clear_movie();
                                g.game.menu_buttons=Default::default();
                                self.keys.clear();
                            } else {
                                g.game.menu_buttons.action=Some(action);
                                eprintln!("menu action {action:#x}: original transition not yet ported");
                            }
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if self.gfx.as_ref().is_some_and(|g|g.game.menu_mode.is_some()) {
                    self.keys.clear();
                    return;
                }
                if event.state.is_pressed() {
                    if let Some(g) = self.gfx.as_mut().filter(|g| g.game.movie.is_some()) {
                        let ch = event.text.as_ref().and_then(|s| s.chars().next()).and_then(|c| u16::try_from(c as u32).ok())
                            .or_else(|| (event.physical_key == PhysicalKey::Code(KeyCode::Escape)).then_some(27));
                        if let Some(ch) = ch { g.game.movie_character(ch); }
                        self.keys.clear();
                        return;
                    }
                }
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state.is_pressed() {
                        self.keys.insert(code);
                        if code == KeyCode::Escape {
                            el.exit();
                        }
                        if code == KeyCode::KeyC {
                            self.cab = !self.cab;
                        }
                        if code == KeyCode::KeyR {
                            if let Some(g) = self.gfx.as_mut() {
                                let (p, h, r) = g.game.world.spawn;
                                g.game.world.vehicle = g.game.world.vehicle.respawned(p, h, &g.game.world.ground);
                                let _ = r;
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
                    g.game.shown_menu = None;
                    g.surface.configure(&g.game.renderer.device, &g.config);
                }
            }
            WindowEvent::RedrawRequested => {
                let input = self.input();
                let now = Instant::now();
                self.acc += (now - self.last).as_secs_f32().min(0.1);
                self.last = now;
                let Some(g) = self.gfx.as_mut() else { return };
                if g.game.update_media((g.config.width, g.config.height)) {
                    #[cfg(windows)]
                    if let Some(sound)=&mut g.game.radar_sound {if let Err(e)=sound.update(None){eprintln!("radar sound: {e}");g.game.radar_sound=None;}}
                    self.acc = 0.0;
                } else {
                    while self.acc >= STEP {
                        g.game.world.vehicle.step(input, &g.game.world.ground, STEP);
                        #[cfg(windows)]
                        if let Some(sound)=&mut g.game.radar_sound {if let Err(e)=sound.update(g.game.world.vehicle.radar_sound()){eprintln!("radar sound: {e}");g.game.radar_sound=None;}}
                        self.acc -= STEP;
                    }
                }
                g.game.place_truck();
                g.game.place_cockpit(self.cab);
                match g.surface.get_current_texture() {
                    Ok(frame) => {
                        let view = frame.texture.create_view(&Default::default());
                        let size = (g.config.width, g.config.height);
                        let vp = if self.cab { cab_view_proj(&g.game.world.vehicle, g.game.world.truck.cab_eye,g.game.world.truck.cab_tilt, size) } else { chase_view_proj(&g.game.world.vehicle, size) };
                        g.game.renderer.set_hud(if self.cab && g.game.cockpit.is_some(){Vec::new()}else{hud(&g.game.world.vehicle, size, self.cab)});
                        g.game.renderer.draw_original_view(&view,size,&vp.view,&vp.projection,vp.viewport).expect("camera viewport must fit the surface");
                        let v = &g.game.world.vehicle;
                        let gear = match v.gear { 0 => "R".to_string(), 1 => "N".to_string(), g => (g - 1).to_string() };
                        g.window.set_title(&format!("openHT2 - {:.0} km/h, gear {gear}, {:.0} rpm, fuel {:.1} l", v.speed * 3.6, v.rpm(), v.fuel));
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
