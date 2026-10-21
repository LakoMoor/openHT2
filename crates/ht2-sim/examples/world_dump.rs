//! Print what the start of a new game sets up in the AI world (for comparing with the original in
//! the emulator, `tools/emu/newgame_steps.py`).
//!
//!     cargo run -p ht2-sim --example world_dump -- "<game>" ENV
use ht2_sim::road::{read_map_scripts, AiRng, Parking};
use ht2_sim::world::{market_offers, sort_vehicles, DriverName, Vehicle, World};

fn main() {
    let game = std::path::PathBuf::from(std::env::args().nth(1).expect("game folder"));
    let map = std::env::args().nth(2).unwrap_or_else(|| "ENV".into());
    let scripts = read_map_scripts(&game.join(&map));
    let lots = Parking::from_scripts(&scripts);
    let vehicles = Vehicle::read(&game);
    let mut rng = AiRng::default();
    let offers = market_offers(&lots, &vehicles, &mut rng);
    for (k, lot) in offers.iter().enumerate() {
        for (s, o) in lot.iter().enumerate() {
            println!("{k} {s} {:?} {}", lots[k].level, o.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "));
        }
    }
    let cats = sort_vehicles(&vehicles, &mut rng, 1.0);
    for k in [2, 7, 8, 9] {
        println!("category {k} {}", cats.lists[k].iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "));
    }
    println!("rng {rng:?}");
    // the players of createPlayersRegular (types of GAME.CNF: aitruck 2, police 5, mafia 6)
    let dir = game.join(&map);
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("way"))).collect();
    files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().to_string());
    let ways: Vec<_> = files.iter().map(|f| ht2_formats::way::rooms(&ht2_formats::way::parse(&std::fs::read(f).unwrap()).unwrap())).collect();
    let mut net = ht2_sim::road::RoadNet::build(&ways);
    net.mark_towns(&ht2_sim::road::Town::from_scripts(&scripts));
    net.mark_parkings(&lots);
    let names = DriverName::read(&game);
    let mut world = World::new(std::rc::Rc::new(net), lots.clone(), vehicles.clone(), cats.clone(), names.clone(), rng, 0);
    let mut counts = [0usize; 12];
    counts[2] = 50;
    counts[5] = 15;
    counts[6] = 8;
    let live = vehicles.iter().position(|v| v.model == "ZilR" && v.colour == 1);
    world.create_players_regular(&counts, live);
    // the human player at the start position of GAME.CNF, as the original finds it (0x436900)
    world.players[0].place = Some(ht2_sim::road::Spot::Road(ht2_sim::road::Place { road: 31, node: None, t: 0.8333333333333334, piece: 0, segment: 0, reversed: true }));
    world.create_dealers([25, 5, 5]);
    world.hire_out(20);
    if let Ok(path) = std::env::var("OBJECTS") {
        // compare the objects of the new game with tools/emu/objects_check.py
        let text = std::fs::read_to_string(path).unwrap();
        let line = text.lines().find(|l| l.starts_with("rng ")).expect("rng line");
        let v: Vec<u32> = line.split_whitespace().skip(1).map(|x| x.parse().unwrap()).collect();
        let theirs = AiRng::from_state(v[..55].try_into().unwrap(), v[55] as usize, v[56] as usize);
        println!("rng same as the original's after the players: {}", format!("{:?}", world.rng) == format!("{theirs:?}"));
        world.rng = theirs;
        world.human_pos = Some([-3020.0, -4625.0, 20.0]);
        world.towns = ht2_sim::road::Town::from_scripts(&scripts);
        world.mark_liquid_towns(&ht2_sim::objects::liquid_goods(&game));
        world.presents = ht2_sim::objects::Present::read(&game);
        world.object_want = ht2_sim::objects::ObjectCounts { fuel: 10, semitrailers: 28, missions: 10, containers: 10, parkings: 40 };
        world.create_objects(0.0);
        for (k, o) in world.objects.iter().enumerate() {
            let pl = o.place.map_or("-".to_string(), |q| format!("{} -1 {:?} {} {} 0", q.road, q.t, q.piece, q.segment));
            println!("object {k} flags {:#x} node {} pos {:?} {:?} {:?} place {pl} veh {} x6c {}", o.flags, o.node, o.pos[0], o.pos[1], o.pos[2], o.vehicle.map_or(-1, |v| v as i64), o.present.map_or(-1, |v| v as i64));
        }
        return;
    }
    if let Ok(path) = std::env::var("XLOAD") {
        // compare the scene loads and the ghosts with tools/emu/xload_check.py: start from the
        // AI generator it prints, the human player where the original has him, ticks of 0.1 s
        let text = std::fs::read_to_string(path).unwrap();
        let line = text.lines().find(|l| l.starts_with("rng ")).expect("rng line");
        let v: Vec<u32> = line.split_whitespace().skip(1).map(|x| x.parse().unwrap()).collect();
        world.rng = AiRng::from_state(v[..55].try_into().unwrap(), v[55] as usize, v[56] as usize);
        world.human_pos = Some([-3020.0, -4625.0, 20.0]);
        world.ghost_want = 25;
        world.create_tasks(1.0);
        let budget: usize = std::env::var("BUDGET").ok().and_then(|b| b.parse().ok()).unwrap_or(8);
        let ticks: u32 = std::env::var("TICKS").ok().and_then(|t| t.parse().ok()).unwrap_or(400);
        // the generator at the ghost task of each tick (tasks not ported draw from it too)
        let mut at_ghosts = std::collections::HashMap::new();
        let mut at_moves = std::collections::HashMap::new();
        let mut tick = 0u32;
        for l in text.lines() {
            if let Some(t) = l.strip_prefix("tick ") {
                tick = t.parse().unwrap();
            } else if let Some(r) = l.strip_prefix("moves ") {
                let v: Vec<u32> = r.split_whitespace().map(|x| x.parse().unwrap()).collect();
                at_moves.insert(tick, AiRng::from_state(v[..55].try_into().unwrap(), v[55] as usize, v[56] as usize));
            } else if let Some(r) = l.strip_prefix("ghosts ") {
                let v: Vec<u32> = r.split_whitespace().map(|x| x.parse().unwrap()).collect();
                at_ghosts.insert(tick, AiRng::from_state(v[..55].try_into().unwrap(), v[55] as usize, v[56] as usize));
            }
        }
        let live = |w: &World, k: usize| w.players[..k].iter().filter(|p| p.kind != 0).count();
        println!("players {}", world.players.len());
        let mut time = 0.0;
        for tick in 1..=ticks {
            time += 0.1;
            println!("tick {tick}");
            if let Some(r) = at_ghosts.get(&tick) {
                world.rng = r.clone();
            }
            world.debug_rng_at_moves = at_moves.get(&tick).cloned();
            let before: Vec<u32> = world.players.iter().map(|p| p.kind).collect();
            if std::env::var_os("XLOAD_VEC").is_some() {
                world.debug_wants = Some(Vec::new());
            }
            let changes = world.tick(time, &mut |loaded, _| loaded.len() < budget);
            if let Some(w) = world.debug_wants.take() {
                for (k, a, b) in w {
                    println!("want {} {:?} {:?} {:?} {:?} {:?} {:?}", live(&world, k), a[0], a[1], a[2], b[0], b[1], b[2]);
                }
            }
            let mut gone = 0;
            for k in 0..before.len() {
                if before[k] != 0 && world.players[k].kind == 0 {
                    println!("delete {}", world.players[..k].iter().zip(&before).filter(|(_, b)| **b != 0).count() - gone);
                    gone += 1;
                }
            }
            for c in changes {
                match c {
                    ht2_sim::world::SceneChange::Load(k) => println!("load {}", live(&world, k)),
                    ht2_sim::world::SceneChange::Unload(k) if world.players[k].kind != 0 => println!("unload {}", live(&world, k)),
                    _ => {}
                }
            }
            for k in before.len()..world.players.len() {
                if let Some(ht2_sim::road::Spot::Road(q)) = world.players[k].place {
                    println!("ghost {} {} {} {:?} {} {} {}", live(&world, k), q.road, q.node.map_or(-1, |n| n as i64), q.t, q.piece, q.segment, q.reversed as u32);
                }
            }
        }
        if std::env::var_os("XLOAD_SPEED").is_some() {
            let only: Option<usize> = std::env::var("XLOAD_SPEED").ok().and_then(|v| v.parse().ok());
            for (k, p) in world.players.iter().enumerate().filter(|(k, _)| only.is_none_or(|o| o == *k)).take(6) {
                let t = match p.place {
                    Some(ht2_sim::road::Spot::Road(q)) => q.t,
                    _ => 1.0,
                };
                println!("speed {k} {:?} {:?} {:?} last {:?} vel {:?}", p.speed, p.speed2, t, p.last_pos, p.velocity);
            }
        }
        return;
    }
    for tick in 1..=std::env::var("TICKS").ok().and_then(|t| t.parse().ok()).unwrap_or(0u32) {
        world.tick(tick as f64, &mut |_, _| true);
        for (k, p) in world.players.iter().enumerate() {
            if let Some(ht2_sim::road::Spot::Road(q)) = p.place {
                println!("tick {tick} player {k} place {} {} {:?} {} {} {}", q.road, q.node.map_or(-1, |n| n as i64), q.t, q.piece, q.segment, q.reversed as u32);
            }
        }
    }
    for (k, p) in world.players.iter().enumerate() {
        let pl = match p.place {
            None => "-".to_string(),
            Some(ht2_sim::road::Spot::Node(n)) => format!("-1 {n} 0.0 0 0 0"),
            Some(ht2_sim::road::Spot::Road(q)) => format!("{} {} {:?} {} {} {}", q.road, q.node.map_or(-1, |n| n as i64), q.t, q.piece, q.segment, q.reversed as u32),
        };
        println!("player {k} type {} veh {} trailer {} name {} place {pl} route {}", p.kind, p.vehicle.map_or(-1, |v| v as i64), p.trailer.map_or(-1, |v| v as i64), p.name.map_or(-1, |v| v as i64), p.route.len());
        println!("route {k} {}", p.route.iter().map(|s| format!("{}:{}", s.node, s.road.map_or(-1, |r| r as i64))).collect::<Vec<_>>().join(" "));
    }
}
