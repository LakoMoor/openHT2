//! A new game of the AI world and its first ticks (0.1 s each), printed as
//! `tools/emu/formorders_check.py` prints the original's: the orders of the towns' store nodes
//! and the AI generator after each tick.
//!
//!     cargo run -p ht2-sim --example newgame_ticks -- "<game>" ENV <ticks>
use ht2_sim::road::{Parking, Place, RoadNet, Spot, Town};
use ht2_sim::world::World;

fn main() {
    let game = std::path::PathBuf::from(std::env::args().nth(1).expect("game folder"));
    let map = game.join(std::env::args().nth(2).unwrap_or_else(|| "ENV".into()));
    let ticks: u32 = std::env::args().nth(3).and_then(|t| t.parse().ok()).unwrap_or(1);
    let mut files: Vec<_> = std::fs::read_dir(&map).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("way"))).collect();
    files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().to_string());
    let ways: Vec<_> = files.iter().map(|f| ht2_formats::way::rooms(&ht2_formats::way::parse(&std::fs::read(f).unwrap()).unwrap())).collect();
    let scripts = ht2_sim::road::read_map_scripts(&map);
    let mut net = RoadNet::build(&ways);
    net.mark_towns(&Town::from_scripts(&scripts));
    net.mark_parkings(&Parking::from_scripts(&scripts));
    // the human player at the start position of GAME.CNF, as the original finds it (0x436900)
    let human = Some(Spot::Road(Place { road: 31, node: None, t: 0.8333333333333334, piece: 0, segment: 0, reversed: true }));
    if let Ok(list) = std::env::var("NAMES") {
        for k in list.split(',').filter_map(|k| k.parse::<usize>().ok()) {
            println!("node {k} {}", net.nodes[k].name);
        }
        for (k, n) in net.nodes.iter().enumerate() {
            if n.name.starts_with("check_point") || n.name.starts_with("finish") || n.name.starts_with("start") {
                println!("node {k} {}", n.name);
            }
        }
        return;
    }
    let mut w = World::new_game(&game, &map, std::rc::Rc::new(net), 1.0, human);
    w.debug_arrivals = std::env::var_os("ARRIVALS").is_some();
    let rng = |w: &World| {
        let s = format!("{:?}", w.rng);
        let table = s.split('[').nth(1).unwrap().split(']').next().unwrap().replace(',', "");
        let idx: Vec<&str> = s.split(": ").skip(2).map(|x| x.trim_end_matches(['}', ' ', ',']).split(',').next().unwrap()).collect();
        format!("rng {table} {} {}", idx[0], idx[1])
    };
    println!("{}", rng(&w));
    let mut time = 0.0;
    // HUMAN=<file>: the human player drives, as the game reports it (0x43ee70: his vehicle's
    // point, and his place on the roads from it, 0x43e610 -> 0x436900 without a route)
    let path: Vec<[f64; 3]> = std::env::var("HUMAN").ok().map_or(Vec::new(), |f| {
        std::fs::read_to_string(f).unwrap().lines().map(|l| {
            let v: Vec<f64> = l.split_whitespace().map(|x| x.parse::<f32>().unwrap() as f64).collect();
            [v[0], v[1], v[2]]
        }).collect()
    });
    for tick in 1..=ticks {
        time += 0.1;
        if let Some(&q) = path.get(tick as usize - 1) {
            w.human_pos = Some(q);
            let h = w.players.iter().position(|p| p.kind == 1).unwrap();
            if let Some(pl) = w.net.nearest(q) {
                w.players[h].place = Some(Spot::Road(pl));
                let flags = if pl.reversed { 1 } else { 0 };
                println!("hplace {} {} {} {} {} {}", pl.road, pl.node.map_or(-1, |n| n as i64), pl.t.to_bits(), pl.piece, pl.segment, flags);
            }
        }
        // as the emulator has it: an empty scene with no memory to spare
        w.tick(time, &mut |_, _| false);
        println!("tick {tick}");
        if let Some(k) = std::env::var("WATCH").ok().and_then(|k| k.parse::<usize>().ok()) {
            let p = &w.players[k];
            if let Some(l) = p.loads.first() { println!("load {k} {:?}", l); }
            println!("rating {k} earning {:?} share {:?} fleet {:?} money {:?} loads {} rating {:?}", p.earning, p.share, p.fleet, p.money, p.loads.len(), p.rating);
            println!("watch {k} {:?} {:?} route {:?} to {:?} laps {} state {:#x} flags {:#x}", p.place, p.place.map(|s| w.net.spot_coordinates(&s)), p.route.iter().map(|s| s.node).collect::<Vec<_>>(), p.route.last().map(|s| s.node), p.laps, p.state, p.flags);
        }
        for (t, nodes) in w.town_slots.iter().enumerate() {
            for (k, slots) in nodes.iter().enumerate() {
                for (s, o) in slots.iter().enumerate() {
                    if let Some(o) = o {
                        println!("slot {tick} {t} {k} {s} {} -1 -1 {} {:?} {:?}", o.merchandise, o.to.map_or(-1, |x| x as i64), o.deadline, o.fee);
                    }
                }
            }
        }
        println!("{}", rng(&w));
    }
    if std::env::var_os("LOTS").is_some() {
        for (li, slots) in w.slots.iter().enumerate() {
            let occ: Vec<String> = slots.iter().map(|o| o.map_or(".".to_string(), |k| k.to_string())).collect();
            println!("lot {li} used {} {:?}", w.lot_used[li], occ);
        }
    }
    if let Some(k) = std::env::var("ADV").ok().and_then(|k| k.parse::<usize>().ok()) {
        let p = &w.players[k];
        if let Some(ht2_sim::road::Spot::Road(mut q)) = p.place {
            let r = &w.net.roads[q.road];
            println!("road {} start {} end {} segments {}", q.road, r.start, r.end, r.segments.len());
            println!("route head {:?}", &p.route[..3.min(p.route.len())]);
            let mut d = 10.0;
            w.net.advance_route(&mut q, &mut d, &p.route);
            println!("after {:?} left {d}", q);
        }
    }
}
