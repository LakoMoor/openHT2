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
    let mut world = World::new(&net, &lots, &vehicles, &cats, &names, rng, 0);
    let mut counts = [0usize; 12];
    counts[2] = 50;
    counts[5] = 15;
    counts[6] = 8;
    let live = vehicles.iter().position(|v| v.model == "ZilR" && v.colour == 1);
    world.create_players_regular(&counts, live);
    world.create_dealers([25, 5, 5]);
    world.hire_out(20);
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
