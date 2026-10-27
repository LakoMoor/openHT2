//! The player's vehicle run entirely by the code ported from the original: the constructor
//! `_Car_V::_Car_V` (`0x4ebd00`) and the step `_Car_V::run1` (`0x4fdc80`), on a flat memory image
//! with the original's field offsets. The world they collide with is the scene graph of the
//! map's rooms ([`crate::world`]); a scene without rooms falls back to the drawn ground
//! ([`MeshWorld`]).

#[cfg(test)]
mod mode_integration_tests {
    use super::*;
    #[test]
    #[ignore="requires HT2_GAME installed game data"]
    fn current_physics_pose_drives_installed_event_query() {
        let game=std::path::PathBuf::from(std::env::var("HT2_GAME").unwrap());
        let world=crate::load_world(&game,"ENV",None).unwrap();
        let (_,_,files)=crate::load_scenes(&game,"ENV").unwrap();
        let root=&files[0].nodes[0].name;
        let events=crate::event_scene::EventScene::from_map_files(&game,&files,root).unwrap();
        let params=&world.truck.params;let data=std::fs::read(game.join("vehicle.tech")).unwrap();
        let mut car=SimCar::new(&data,params.record,&params.wheel_nodes,&params.add_nodes,&world.ground,world.spawn.0+Vec3::Z*world.spawn.2,world.spawn.1).unwrap();
        let wrapper=0x1a000000;car.bind_event_scene(&events,wrapper,root).unwrap();
        struct Callback;
        impl ht2_sim::scene_events::EventHooks for Callback {
            fn event(&mut self,_:&mut Mem,callback:u32,_:u32,user:u32){assert_eq!(callback,0x4ce300);assert_eq!(user,0x1a000000);}
        }
        for _ in 0..12 {
            car.step(&world.ground,1./60.,0.,0.,0);
            car.query_scene_events(wrapper,&mut Callback);
            for i in 0..3 {assert_eq!(car.m.u32(0x721298+i*4),car.m.u32(CAR+0x34+i*4));}
        }
        assert!(car.pos().is_finite());
    }
    #[test]
    #[ignore = "requires HT2_GAME installed game data"]
    fn physical_mode_runs_existing_driver_on_installed_scene() {
        let game=std::path::PathBuf::from(std::env::var("HT2_GAME").expect("HT2_GAME"));
        let world=crate::load_world(&game,"ENV",None).unwrap();
        let params=&world.truck.params;let data=std::fs::read(game.join("vehicle.tech")).unwrap();
        let mut car=SimCar::new(&data,params.record,&params.wheel_nodes,&params.add_nodes,&world.ground,
            world.spawn.0+Vec3::Z*world.spawn.2,world.spawn.1).unwrap();
        let wrapper=0x1a000000;let room=car.room();
        let matrix=car.m.read(CAR+0x10,48);car.m.write(wrapper+0x204,&matrix);
        car.m.set_u32(wrapper+0x5460,CAR);car.m.set_u32(wrapper+0x330,room);
        for (i,value) in [0.,3.,0.,0.,0.,0.1].into_iter().enumerate(){car.m.set_f32(wrapper+0x1b0+i as u32*4,value);}
        car.m.set_u32(wrapper+0x2aac,2);
        with_world(&world.ground,car.rooms,|contact|{
            let mut backend=ht2_sim::ai_scene::PhysicalMode{world:contact,diagnostics:vec![]};
            ht2_sim::ai_scene::vehicle_mode(&mut car.m,wrapper,1,&mut backend);
            assert!(backend.diagnostics.is_empty());
        });
        assert_eq!(car.m.u32(wrapper+0x2aac),1);assert_eq!(car.m.u32(CAR+0x2638),3);
        assert_eq!([0,4,8].map(|o|car.m.f32(CAR+0x40+o)),[0.,3.,0.]);
        assert_eq!([0,4,8].map(|o|car.m.f32(CAR+0x58+o)),[0.,0.,0.1]);
        for _ in 0..12 {car.step(&world.ground,1./60.,0.,0.,0);}
        assert!(car.pos().is_finite());assert!(car.body().is_finite());
        with_world(&world.ground,car.rooms,|contact|{
            let mut backend=ht2_sim::ai_scene::PhysicalMode{world:contact,diagnostics:vec![]};
            ht2_sim::ai_scene::vehicle_mode(&mut car.m,wrapper,2,&mut backend);
        });
        assert_eq!(car.m.u32(CAR+0x2638),0);
        for offset in [0x50d0,0x50e8] {assert_eq!(car.m.read(wrapper+offset,12),vec![0;12]);}

        // Compose distance decisions, transition setters, mode owner and driver.
        // Scene/AI query fixtures are explicit; collision world and physics are real.
        struct Owner<'a>{backend:ht2_sim::ai_scene::PhysicalMode<'a>,pose:[f32;12],human:[f32;12]}
        impl ht2_sim::ai_scene::TrajectoryHooks for Owner<'_> {
            fn mode(&mut self,m:&mut Mem,v:u32,value:i32){ht2_sim::ai_scene::vehicle_mode(m,v,value,&mut self.backend);}
            fn world_transform(&mut self,_:&mut Mem,body:u32)->[f32;12] {if body==0x1a000010 {self.pose}else{assert_eq!(body,0x1b000010);self.human}}
            fn publish_flags(&mut self,_:&mut Mem,player:u32,_:u32){assert_eq!(player,0x12345678);}
        }
        impl ht2_sim::ai_scene::PhysicsHooks for Owner<'_> {
            fn player_speed(&mut self,_:&mut Mem,player:u32)->f64 {assert_eq!(player,0x12345678);3.}
            fn player_direction(&mut self,_:&mut Mem,player:u32)->[f32;3] {assert_eq!(player,0x12345678);[0.,1.,0.]}
        }
        let pose: [f32;12]=std::array::from_fn(|i|car.m.f32(CAR+0x10+i as u32*4));
        let flags=wrapper+0x10000;let scenario=0x1c000000;
        car.m.set_u32(0x6d2078,scenario);car.m.set_u32(scenario+0x268,0x1b000000);
        car.m.set_u32(flags,0);
        with_world(&world.ground,car.rooms,|contact|{
            let mut human=pose;human[9]+=30.;
            let mut owner=Owner{backend:ht2_sim::ai_scene::PhysicalMode{world:contact,diagnostics:vec![]},pose,human};
            ht2_sim::ai_scene::update_physics_mode(&mut car.m,wrapper,0x12345678,flags,&mut owner);
            assert_eq!(car.m.u32(flags)&1,1);assert_eq!(car.m.u32(CAR+0x2638),3);
            assert_eq!(car.m.u32(CAR+0x4078),1);
            owner.human[9]=pose[9]+400.;
            ht2_sim::ai_scene::update_physics_mode(&mut car.m,wrapper,0x12345678,flags,&mut owner);
            assert_eq!(car.m.u32(flags)&1,0);assert_eq!(car.m.u32(CAR+0x2638),0);
            assert!(owner.backend.diagnostics.is_empty());
        });
    }
}

use crate::ground::Ground;
use std::collections::HashMap;
use glam::{Mat4, Vec3};
use ht2_sim::contact::World;
use ht2_sim::gearbox::TECH_TABLE;
use ht2_sim::mem::Mem;
use ht2_sim::{ai, contact, ctor, globals, loader, step, tech_rec::RECORD_LEN, tech_system as ts};

const CAR: u32 = 0x1000_0000;
/// The computer vehicles follow the player's in memory, this far apart.
const CAR_STRIDE: u32 = 0x10000;
const CLOCK: u32 = 0x1800_0000;
/// Stands for the room the vehicle is in (the mesh world has only one).
const ROOM: u32 = 0x1900_0000;
/// The surface record returned when the scene has no collision meshes and the drawn ground is
/// used instead: type 1, kind 1 (dry asphalt), no parameters.
const SURFACE: u32 = 0x1900_0100;
/// A stand-in for the scene collision when the map has no rooms: the nearest crossing of the
/// query segment with the drawn ground, all of it dry asphalt.
pub struct MeshWorld<'a> {
    pub ground: &'a Ground,
}

impl World for MeshWorld<'_> {
    fn collide(&mut self, m: &mut Mem, _obj: u32, mode: u32, rec: u32) -> u32 {
        let v = |a: u32| Vec3::new(m.f32(a), m.f32(a + 4), m.f32(a + 8));
        if mode & 4 == 0 {
            return 0;
        }
        let (a, b) = (v(rec + 0x4c), v(rec + 0x58));
        let Some((p, n)) = self.ground.segment_hit(a, b) else { return 0 };
        let surface = SURFACE;
        for (i, c) in p.to_array().iter().enumerate() {
            m.set_f32(rec + 0x34 + 4 * i as u32, *c);
        }
        for (i, c) in n.to_array().iter().enumerate() {
            m.set_f32(rec + 0x40 + 4 * i as u32, *c);
        }
        m.set_u32(rec + 0x64, surface);
        1
    }
}

/// Wheel `Space` transforms of a model (12 floats: three axes, then the origin).
pub type WheelNodes = [Option<[f32; 12]>; 8];

pub struct SimCar {
    m: Mem,
    time: f64,
    /// Whether the scene graph (rooms) is used; otherwise the drawn ground.
    rooms: bool,
    /// Computer vehicles: their address and their driver.
    traffic: Vec<(u32, ai::Driver)>,
    vid: i32,
    /// The player's vehicle driven by the ported computer driver (for demos).
    autopilot: Option<ai::Driver>,
    /// The road module's random generator (`0x68b9c8`), shared by all drivers.
    rng: ht2_sim::road::AiRng,
    /// The AI world of the map (players on the road network); with it, the computer vehicles
    /// are a pool of slots taken by the players whose rooms the AI has loaded.
    ai_world: Option<ht2_sim::world::World>,
    /// Original callback requests awaiting the application's media/menu layer.
    game_actions: Vec<ht2_sim::game_message::Action>,
    slot_player: Vec<Option<usize>>,
    /// The model prefix of the vehicle in each slot, and the `vehicle.tech` record of each
    /// model prefix (set up with its wheel nodes, as the game does for every vehicle type when
    /// it loads them).
    slot_model: Vec<Option<String>>,
    records: HashMap<String, i32>,
}


/// What is needed to draw a vehicle.
pub struct Pose {
    pub body: Mat4,
    pub steer: f32,
    pub wheel_angle: [f32; 8],
    pub wheel_drop: [f32; 8],
    /// The model prefix of the vehicle (`None`: the player's).
    pub model: Option<String>,
    /// Not in the scene (a free slot of the pool).
    pub hidden: bool,
}

fn with_world<R>(ground: &Ground, rooms: bool, f: impl FnOnce(&mut dyn World) -> R) -> R {
    if rooms {
        f(&mut crate::world::SceneWorld { scene: &ground.scene })
    } else {
        f(&mut MeshWorld { ground })
    }
}

/// Gear request codes the original's gearbox understands (`+0x2746`).
const REQUEST_REVERSE: i16 = 0x14;
const REQUEST_NEUTRAL: i16 = 0x15;
const REQUEST_DRIVE: i16 = 0x16;

impl SimCar {
    /// Build vehicle `record` of `tech_file` (the contents of `vehicle.tech`) at `pos`, facing
    /// `heading` (0 is +Y, positive turns left).
    pub fn new(tech_file: &[u8], record: usize, wheels: &WheelNodes, add: &WheelNodes, ground: &Ground, pos: Vec3, heading: f32) -> Result<Self, String> {
        let name_len = u32::from_le_bytes(tech_file.get(0..4).ok_or("short vehicle.tech")?.try_into().unwrap()) as usize;
        let hdr = 4 + name_len;
        let rec_len = u32::from_le_bytes(tech_file[hdr + 4..hdr + 8].try_into().unwrap()) as usize;
        let count = u32::from_le_bytes(tech_file[hdr + 8..hdr + 12].try_into().unwrap()) as usize;
        if rec_len != RECORD_LEN || record >= count {
            return Err("unexpected vehicle.tech layout".into());
        }
        let mut m = Mem::new();
        for i in 0..count {
            m.write(TECH_TABLE + (i * RECORD_LEN) as u32, &tech_file[57 + i * rec_len..57 + (i + 1) * rec_len]);
        }
        globals::install_clock(&mut m, CLOCK, 0.0, 0.0);
        globals::install_constants(&mut m);
        contact::install_surface_tables(&mut m);
        for i in 0..3 {
            m.set_f32(0x6974a8 + 4 * i, 1.0);
        }
        m.set_i16(0x696bda, 1); // wheel queries go through the room (set at start-up)
        m.set_u32(globals::RAND_SEED, 1);
        for (o, v) in [(0u32, 1u32), (4, 1), (8, 0), (12, 0)] {
            m.set_u32(SURFACE + o, v);
        }
        ground.scene.install(&mut m);
        let vid = record as i32;
        loader::init_vehicle_nodes(&mut m, vid, wheels, add);
        let (c, s) = (heading.cos(), heading.sin());
        let mat = [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0, pos.x, pos.y, pos.z];
        // The room of the road place nearest to the start (as the road module places players),
        // else the room whose road is below.
        let named = ground.roads.nearest([pos.x as f64, pos.y as f64, pos.z as f64]).and_then(|p| ground.scene.room_named(ground.roads.room_of(&p)));
        let start_room = named.or_else(|| ground.scene.room_at(&mut m, pos.to_array()));
        let rooms = start_room.is_some();
        let room = start_room.unwrap_or(ROOM);
        with_world(ground, rooms, |w| ctor::fn_004ebd00(&mut m, w, CAR, room, mat, vid));
        // The player starts with the engine running, automatic gearbox, keyboard control.
        m.set_i32(CAR + 0x2774, 1);
        m.set_i32(CAR + 0x2760, 1);
        m.set_i32(CAR + 0x1b24, 2);
        m.set_i16(CAR + 0x2704, 1);
        m.set_i16(CAR + 0x2706, 1);
        ht2_sim::gearbox::fn_004ff770(&mut m, CAR, 1);
        Ok(SimCar { m, time: 0.0, rooms, traffic: Vec::new(), vid, autopilot: None, rng: Default::default(), ai_world: None, game_actions: Vec::new(), slot_player: Vec::new(), slot_model: Vec::new(), records: HashMap::new() })
    }

    /// One step of `dt` seconds with the pedals in `0..=1` and the steering keys (-1 right, 1 left).
    /// Let the computer driver drive the player's vehicle from its place on the nearest road.
    pub fn set_autopilot(&mut self, ground: &Ground) {
        let net = &ground.roads;
        let p = self.pos();
        let Some(mut place) = net.nearest([p.x as f64, p.y as f64, p.z as f64]) else { return };
        let t = net.direction(&place, false);
        let fwd = self.body().y_axis;
        if t[0] * (fwd.x as f64) + t[1] * (fwd.y as f64) < 0.0 {
            place.reversed = true;
        }
        let m = &mut self.m;
        ai::init_driver(m, CAR, true);
        m.set_u32(CAR + 0x4078, 1);
        m.set_i16(CAR + 0x2704, 1);
        m.set_i16(CAR + 0x2706, 1);
        m.set_i16(CAR + 0x2746, REQUEST_DRIVE);
        self.autopilot = Some(ai::Driver::new(place));
    }

    pub fn step(&mut self, ground: &Ground, dt: f32, throttle: f32, brake: f32, steer: i32) {
        let m = &mut self.m;
        self.time += dt as f64;
        globals::install_clock(m, CLOCK, self.time, dt as f64);
        if let Some(drv) = &mut self.autopilot {
            let others: Vec<u32> = std::iter::once(CAR).chain(self.traffic.iter().map(|(c, _)| *c)).collect();
            ai::drive(m, CAR, &ground.roads, &mut self.rng, drv, 0, dt as f64, &others);
            with_world(ground, self.rooms, |w| step::fn_004fdc80(m, w, CAR));
            self.step_traffic(ground, dt);
            return;
        }
        m.wr8(CAR + 0x2702, (steer > 0) as u8);
        m.wr8(CAR + 0x2703, (steer < 0) as u8);
        let speed = m.f32(CAR + 0xdc8);
        let gear = m.i32(CAR + 0x1c1c);
        let shifting = m.i32(CAR + 0x1c24) != 0;
        // Drive or reverse the way the game's keyboard control picks them: brake at standstill
        // engages reverse, where the brake key is the gas pedal.
        let reversing = gear == 0 && throttle == 0.0;
        let request = if shifting {
            0
        } else if throttle > 0.0 && gear < 2 && speed > -0.5 {
            REQUEST_DRIVE
        } else if brake > 0.0 && throttle == 0.0 && speed < 0.5 && gear != 0 {
            REQUEST_REVERSE
        } else if throttle == 0.0 && brake == 0.0 && speed.abs() < 0.2 && gear != 1 {
            REQUEST_NEUTRAL
        } else {
            0
        };
        m.set_i16(CAR + 0x2746, request);
        let (pedal, braking) = if reversing { (brake, 0.0) } else { (throttle, brake) };
        // Keyboard control as the game has it (+0x2704 = 0, 0x4ff800): +0x2714 is the gas key
        // and +0x2716 the brake key; both ramp up while held. (+0x270c and +0x2710 are the
        // pedals of analog control.)
        m.set_i16(CAR + 0x2704, 0);
        m.set_f32(CAR + 0x270c, 0.0);
        m.set_f32(CAR + 0x2710, 0.0);
        m.set_i16(CAR + 0x2714, (pedal > 0.0) as i16);
        m.set_i16(CAR + 0x2716, (braking > 0.0) as i16);
        with_world(ground, self.rooms, |w| step::fn_004fdc80(m, w, CAR));
        self.step_traffic(ground, dt);
    }

    fn step_traffic(&mut self, ground: &Ground, dt: f32) {
        let m = &mut self.m;
        let active: Vec<bool> = (0..self.traffic.len()).map(|k| self.ai_world.is_none() || self.slot_player.get(k).copied().flatten().is_some()).collect();
        let others: Vec<u32> = std::iter::once(CAR).chain(self.traffic.iter().zip(&active).filter(|(_, a)| **a).map(|((c, _), _)| *c)).collect();
        for ((car, drv), on) in self.traffic.iter_mut().zip(&active) {
            if !on {
                continue;
            }
            ai::drive(m, *car, &ground.roads, &mut self.rng, drv, 0, dt as f64, &others);
            with_world(ground, self.rooms, |w| step::fn_004fdc80(m, w, *car));
        }
        self.sync_ai(ground);
    }

    /// Run the AI world with the time and the player's place, and put its players into the
    /// scene or take them out as the AI loads and unloads their objects (`xLoadEntities`): a
    /// moving player whose object is loaded gets a free slot of the pool, at its place, driving
    /// its route; the slot's vehicle reports its place back to the AI world.
    fn sync_ai(&mut self, ground: &Ground) {
        let Some(mut w) = self.ai_world.take() else { return };
        let net = &ground.roads;
        let me = self.pos();
        w.human_pos = Some([me.x as f64, me.y as f64, me.z as f64]);
        if let Some(p) = net.nearest([me.x as f64, me.y as f64, me.z as f64]) {
            w.place_human(ht2_sim::road::Spot::Road(p));
        }
        for (k, slot) in self.slot_player.iter().enumerate() {
            if let Some(pl) = slot {
                w.players[*pl].place = Some(ht2_sim::road::Spot::Road(self.traffic[k].1.place));
            }
        }
        let pool = self.slot_player.len();
        let changes = w.tick(self.time, &mut |loaded, _| loaded.len() < pool);
        for winner in std::mem::take(&mut w.victory_events) {
            let p = &w.players[winner];
            self.game_actions.push(ht2_sim::game_message::victory(w.is_human(winner), p.flags));
        }
        if !changes.is_empty() && std::env::var_os("HT2_AI_LOG").is_some() {
            let ghosts = w.players.iter().filter(|p| p.kind == 8).count();
            let active = self.slot_player.iter().filter(|s| s.is_some()).count();
            eprintln!("t={:.1} ghosts {ghosts} driven {active} loaded {} changes {:?}", self.time, w.loaded.len(), changes);
        }
        // out of the scene: players whose objects are no longer loaded, or who are gone
        for k in 0..self.slot_player.len() {
            if let Some(pl) = self.slot_player[k] {
                if w.players[pl].kind == 0 || !w.in_scene(&w.players[pl]) {
                    self.slot_player[k] = None;
                    w.players[pl].driven = false;
                }
            }
        }
        // into the scene: moving players whose objects are loaded (standing ones are not shown
        // yet)
        for pl in 0..w.players.len() {
            let p = &w.players[pl];
            if p.kind == 0 || p.kind == 1 || p.driven || p.state != 0 || p.route.is_empty() || !w.in_scene(p) {
                continue;
            }
            let Some(ht2_sim::road::Spot::Road(place)) = p.place else { continue };
            let Some(k) = self.slot_player.iter().position(|s| s.is_none()) else { break };
            let route = p.route.clone();
            // the vehicle's own model when it has one, else the player's
            let model = p.vehicle.map(|v| w.vehicles[v].model.clone()).filter(|m| self.records.contains_key(m));
            let vid = model.as_ref().map_or(self.vid, |m| self.records[m]);
            if self.place_car(ground, self.traffic[k].0, place, vid) {
                let mut d = ai::Driver::new(place);
                d.route = route;
                self.traffic[k].1 = d;
                self.slot_player[k] = Some(pl);
                self.slot_model[k] = model;
                w.players[pl].driven = true;
            }
        }
        self.ai_world = Some(w);
    }

    /// Give the vehicles to the AI world `w`: `pool` slots for its players. `models` are the
    /// `vehicle.tech` records with a model: record, model prefix and wheel nodes.
    pub fn set_ai_world(&mut self, w: ht2_sim::world::World, pool: usize, models: &[(usize, String, WheelNodes, WheelNodes)]) {
        for (record, prefix, wheels, add) in models {
            let vid = *record as i32;
            if vid != self.vid {
                loader::init_vehicle_nodes(&mut self.m, vid, wheels, add);
            }
            self.records.insert(prefix.clone(), vid);
        }
        self.slot_model = vec![None; pool];
        self.traffic.clear();
        for k in 0..pool {
            let car = CAR + CAR_STRIDE * (k as u32 + 1);
            self.traffic.push((car, ai::Driver::new(ht2_sim::road::Place { road: 0, node: None, t: 0.0, piece: 0, segment: 0, reversed: false })));
        }
        self.slot_player = vec![None; pool];
        self.ai_world = Some(w);
    }

    /// Build the computer vehicle at `car` at a road place, in the right-hand lane, with its
    /// driver set up (as `add_traffic` does). False when there is no ground there.
    fn place_car(&mut self, ground: &Ground, car: u32, place: ht2_sim::road::Place, vid: i32) -> bool {
        let net = &ground.roads;
        let c = net.coordinates(&place);
        let t = net.direction(&place, false);
        let l = (t[0] * t[0] + t[1] * t[1]).sqrt();
        if l < 1e-6 {
            return false;
        }
        let (fx, fy) = ((t[0] / l) as f32, (t[1] / l) as f32);
        let at = Vec3::new(c[0] as f32 + fy * 1.75, c[1] as f32 - fx * 1.75, c[2] as f32);
        let Some(z) = ground.height_at(at.x, at.y, at.z + 3.0, 0.0) else { return false };
        let pos = Vec3::new(at.x, at.y, z + 1.0);
        let named = ground.scene.room_named(net.room_of(&place));
        let Some(room) = named.or_else(|| ground.scene.room_at(&mut self.m, pos.to_array())).or((!self.rooms).then_some(ROOM)) else { return false };
        let heading = (-fx).atan2(fy);
        let (c, s) = (heading.cos(), heading.sin());
        let mat = [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0, pos.x, pos.y, pos.z];
        let m = &mut self.m;
                with_world(ground, self.rooms, |w| ctor::fn_004ebd00(m, w, car, room, mat, vid));
        m.set_i32(car + 0x2774, 1);
        m.set_i32(car + 0x2760, 1);
        m.set_i32(car + 0x1b24, 2);
        m.set_i16(car + 0x2704, 1);
        m.set_i16(car + 0x2706, 1);
        ht2_sim::gearbox::fn_004ff770(m, car, 1);
        ai::init_driver(m, car, true);
        m.set_u32(car + 0x4078, 1);
        m.set_i16(car + 0x2746, REQUEST_DRIVE);
        true
    }

    /// Put `count` computer vehicles of the player's type on the roads 60 to 400 m from the
    /// player, in the right-hand lane. Stand-in: the original places traffic from the road
    /// module's own rules (not ported) and with its own vehicle types.
    pub fn add_traffic(&mut self, ground: &Ground, count: usize, seed: u32) {
        let net = &ground.roads;
        let me = self.pos();
        let mut taken: Vec<Vec3> = vec![me];
        let mut rng = seed | 1;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        };
        for _ in 0..count * 200 {
            if self.traffic.len() >= count || net.roads.is_empty() {
                break;
            }
            let road = next() as usize % net.roads.len();
            let mut place = net.road_start(road);
            let mut d = (next() % 1000) as f64 * 0.001 * net.roads[road].length;
            net.advance(&mut place, &mut d);
            place.node = None;
            if next() & 1 == 1 {
                place.reversed = true;
            }
            let c = net.coordinates(&place);
            let t = net.direction(&place, false);
            let l = (t[0] * t[0] + t[1] * t[1]).sqrt();
            if l < 1e-6 {
                continue;
            }
            let (fx, fy) = ((t[0] / l) as f32, (t[1] / l) as f32);
            let at = Vec3::new(c[0] as f32 + fy * 1.75, c[1] as f32 - fx * 1.75, c[2] as f32);
            let dist = (at - me).truncate().length();
            if !(60.0..=400.0).contains(&dist) || taken.iter().any(|p| (*p - at).length() < 20.0) {
                continue;
            }
            let Some(z) = ground.height_at(at.x, at.y, at.z + 3.0, 0.0) else { continue };
            let pos = Vec3::new(at.x, at.y, z + 1.0);
            let car = CAR + CAR_STRIDE * (self.traffic.len() as u32 + 1);
            let named = ground.scene.room_named(net.room_of(&place));
            let Some(room) = named.or_else(|| ground.scene.room_at(&mut self.m, pos.to_array())).or((!self.rooms).then_some(ROOM)) else { continue };
            let heading = (-fx).atan2(fy);
            let (c, s) = (heading.cos(), heading.sin());
            let mat = [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0, pos.x, pos.y, pos.z];
            let m = &mut self.m;
            let vid = self.vid;
            with_world(ground, self.rooms, |w| ctor::fn_004ebd00(m, w, car, room, mat, vid));
            // Engine running, automatic gearbox, pedals and steering set by the driver.
            m.set_i32(car + 0x2774, 1);
            m.set_i32(car + 0x2760, 1);
            m.set_i32(car + 0x1b24, 2);
            m.set_i16(car + 0x2704, 1);
            m.set_i16(car + 0x2706, 1);
            ht2_sim::gearbox::fn_004ff770(m, car, 1);
            ai::init_driver(m, car, true);
            // Traffic follows the vehicle ahead (`0x4d13e1`; other roles of the original differ).
            m.set_u32(car + 0x4078, 1);
            m.set_i16(car + 0x2746, REQUEST_DRIVE);
            self.traffic.push((car, ai::Driver::new(place)));
            taken.push(pos);
        }
    }

    /// Requests remain pending until the application's movie/menu layer handles them.
    pub fn pending_game_actions(&self) -> &[ht2_sim::game_message::Action] {
        &self.game_actions
    }
    pub fn rating_world(&self) -> Option<&ht2_sim::world::World> { self.ai_world.as_ref() }

    /// The computer vehicles, for drawing.
    pub fn traffic(&self) -> Vec<Pose> {
        self.traffic
            .iter()
            .enumerate()
            .map(|(k, (car, _))| {
                if self.ai_world.is_some() && self.slot_player.get(k).copied().flatten().is_none() {
                    // a free slot of the pool: out of sight
                    Pose { body: Mat4::IDENTITY, steer: 0.0, wheel_angle: [0.0; 8], wheel_drop: [0.0; 8], model: None, hidden: true }
                } else {
                    Pose { model: self.slot_model.get(k).cloned().flatten(), ..self.pose(*car) }
                }
            })
            .collect()
    }

    fn pose(&self, car: u32) -> Pose {
        let m = &self.m;
        let g = |i: u32| m.f32(car + 0x10 + 4 * i);
        Pose {
            body: Mat4::from_cols(
                Vec3::new(g(0), g(1), g(2)).extend(0.0),
                Vec3::new(g(3), g(4), g(5)).extend(0.0),
                Vec3::new(g(6), g(7), g(8)).extend(0.0),
                Vec3::new(g(9), g(10), g(11)).extend(1.0),
            ),
            steer: m.f32(car + 0x1a20),
            wheel_angle: core::array::from_fn(|k| m.f32(car + 0xe9c + 4 * k as u32)),
            wheel_drop: core::array::from_fn(|k| m.f32(car + 0xdd8 + 4 * k as u32) - 0.75 * m.f32(car + 0x1f9c + 4 * k as u32)),
            model: None,
            hidden: false,
        }
    }

    /// For diagnostics: speed, mode, lane offset and inputs of computer vehicle `k`.
    pub fn traffic_state(&self, k: usize) -> Option<String> {
        let (car, d) = self.traffic.get(k)?;
        let m = &self.m;
        Some(format!(
            "car{k}: pos={:6.1?} v={:5.1} gear={} mode={} lane={} flags={:#x} slots={:09b} lat={:5.2} steer={:5.2} thr={:.2} brk={:.2} road={} t={:.3}",
            [m.f32(car + 0x34), m.f32(car + 0x38), m.f32(car + 0x3c)],
            m.f32(car + 0xdc8),
            m.i32(car + 0x1c1c),
            m.u32(car + ai::MODE),
            m.i32(car + ai::LANE),
            m.u32(car + ai::FLAGS),
            m.u32(car + 0x4064),
            m.f64(car + ai::LATERAL),
            m.f32(car + ai::STEER_INPUT),
            m.f32(car + 0x270c),
            m.f32(car + 0x2710),
            d.place.road,
            d.place.t
        ))
    }

    /// The body transform (the original's matrix: rows are the body axes, then the origin).
    pub fn body(&self) -> Mat4 {
        let g = |i: u32| self.m.f32(CAR + 0x10 + 4 * i);
        Mat4::from_cols(
            Vec3::new(g(0), g(1), g(2)).extend(0.0),
            Vec3::new(g(3), g(4), g(5)).extend(0.0),
            Vec3::new(g(6), g(7), g(8)).extend(0.0),
            Vec3::new(g(9), g(10), g(11)).extend(1.0),
        )
    }
    /// Query from the current physical body, with an explicit scene wrapper
    /// owned by the caller. Full original scene-vehicle scheduling is pending.
    pub fn bind_event_scene(&mut self,scene:&crate::event_scene::EventScene,vehicle:u32,root_name:&str)->Result<(),String> {
        scene.install(&mut self.m);scene.bind_vehicle(&mut self.m,vehicle,root_name)
    }
    pub fn query_scene_events(&mut self,vehicle:u32,hooks:&mut impl ht2_sim::scene_events::EventHooks) {
        struct Query<'a,H>{pose:[f32;12],vehicle:u32,hooks:&'a mut H}
        impl<H> ht2_sim::ai_scene::ObstacleSourceHooks for Query<'_,H> {
            fn player_kind(&mut self,_:&mut Mem,_:u32)->i32{unreachable!("query does not ask player kind")}
            fn world_transform(&mut self,_:&mut Mem,body:u32)->[f32;12]{assert_eq!(body,self.vehicle+0x10);self.pose}
        }
        impl<H:ht2_sim::scene_events::EventHooks> ht2_sim::scene_events::EventHooks for Query<'_,H> {
            fn child(&mut self,m:&mut Mem,node:u32,index:i32)->u32{self.hooks.child(m,node,index)}
            fn contains(&mut self,m:&mut Mem,volume:u32,point:u32)->bool{self.hooks.contains(m,volume,point)}
            fn event(&mut self,m:&mut Mem,callback:u32,command:u32,user:u32){self.hooks.event(m,callback,command,user)}
        }
        let pose=std::array::from_fn(|i|self.m.f32(CAR+0x10+i as u32*4));
        ht2_sim::ai_scene::query_vehicle_events(&mut self.m,vehicle,&mut Query{pose,vehicle,hooks});
    }

    pub fn pos(&self) -> Vec3 {
        let g = |i: u32| self.m.f32(CAR + 0x34 + 4 * i);
        Vec3::new(g(0), g(1), g(2))
    }

    /// Forward speed, m/s.
    pub fn speed(&self) -> f32 {
        self.m.f32(CAR + 0xdc8)
    }

    pub fn gear(&self) -> i32 {
        self.m.i32(CAR + 0x1c1c)
    }

    /// Engine speed, revolutions per second.
    pub fn engine_rev(&self) -> f32 {
        self.m.f32(CAR + 0x1c84)
    }

    pub fn fuel(&self) -> f32 {
        ts::value(&self.m, CAR + 0x3ef0)
    }

    /// Angle of the steered wheels, radians, positive to the left.
    pub fn steer_angle(&self) -> f32 {
        self.m.f32(CAR + 0x1a20)
    }
    /// Original normalized steering position used by the cabin wheel.
    pub fn steering_position(&self)->f32 {self.m.f32(CAR+0x29e0)}
    pub fn radar_equipped(&self)->bool {self.m.u32(CAR+0x5400)!=0}

    /// Rotation angle of wheel `k`, radians.
    pub fn wheel_angle(&self, k: usize) -> f32 {
        self.m.f32(CAR + 0xe9c + 4 * k as u32)
    }

    /// How far wheel `k` hangs below its rest position, m: `+0xdd8` is the distance from the mount
    /// down to the wheel centre (0.75 of the travel when the vehicle is built). For drawing the
    /// wheels only; the original's wheel rendering is not ported.
    /// The id of the room the vehicle is in (`+0xf4`), 0 for none.
    pub fn room(&self) -> u32 {
        self.m.u32(CAR + 0xf4)
    }

    /// For diagnostics: the room whose road is right below the vehicle.
    pub fn room_below(&mut self, ground: &Ground) -> Option<u32> {
        let p = self.pos().to_array();
        ground.scene.room_at(&mut self.m, p)
    }

    /// Whether the scene graph is used for collision.
    pub fn uses_rooms(&self) -> bool {
        self.rooms
    }

    pub fn wheel_drop(&self, k: usize) -> f32 {
        self.m.f32(CAR + 0xdd8 + 4 * k as u32) - 0.75 * self.m.f32(CAR + 0x1f9c + 4 * k as u32)
    }
}
