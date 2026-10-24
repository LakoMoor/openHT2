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
    // AMBPLACES=1: the ambushes as `ambush_places.py` prints the original's
    if std::env::var_os("AMBPLACES").is_some() {
        let pl = |p: &Option<Place>| p.map_or("none".to_string(), |p| format!("{} {} {} {} {} {}", p.road, p.node.map_or(-1, |n| n as i64), p.t.to_bits(), p.piece, p.segment, u8::from(p.reversed)));
        for (k, a) in w.ambushes.iter().enumerate() {
            println!("{k} {:?} {:?} {:?} {:?} {:?} {:?} {} {}", a.pos[0], a.pos[1], a.pos[2], a.dir[0], a.dir[1], a.dir[2], pl(&a.place), pl(&a.facing_place));
        }
        return;
    }
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
    // the human player's vehicle matrix as the emulator's new game has it (rows 0, the
    // translation of the game's start position); a driving one has the matrix the game reports
    w.human_matrix = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -3020.0, -4625.0, 20.0];
    // ORDERS=<file>: lines "<tick> <town index> <node index> <amount>": before that tick the
    // game hands the human player an order (message 0x11, loadOrder 0x443d50), as
    // formorders_check.py FO_ORDERS does; after each tick his money, +0x218, earnings and loads
    let orders: Vec<(u32, usize, usize, f64)> = std::env::var("ORDERS").ok().map_or(Vec::new(), |f| {
        std::fs::read_to_string(f).unwrap().lines().filter_map(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            (w.len() == 4).then(|| (w[0].parse().unwrap(), w[1].parse().unwrap(), w[2].parse().unwrap(), w[3].parse().unwrap()))
        }).collect()
    });
    for tick in 1..=ticks {
        time += 0.1;
        if let Some(&q) = path.get(tick as usize - 1) {
            w.human_pos = Some(q);
            w.human_matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, q[0], q[1], q[2]];
            let h = w.players.iter().position(|p| p.kind == 1).unwrap();
            if let Some(pl) = w.net.nearest(q) {
                let _ = h;
                w.place_human(Spot::Road(pl));
                let flags = if pl.reversed { 1 } else { 0 };
                println!("hplace {} {} {} {} {} {}", pl.road, pl.node.map_or(-1, |n| n as i64), pl.t.to_bits(), pl.piece, pl.segment, flags);
            }
        }
        for &(_, t, r, amount) in orders.iter().filter(|o| o.0 == tick) {
            let h = w.players.iter().position(|p| p.kind == 1).unwrap();
            let now = w.now;
            let (ret, units, index) = w.load_order_at(h, t, r, amount, now);
            println!("horder {tick} {t} {r} {amount:?} ret {ret} amount {units:?} index {index}");
        }
        // as the emulator has it: an empty scene with no memory to spare
        w.tick(time, &mut |_, _| false);
        println!("tick {tick}");
        // COMP=1: the human player and the competitors as formorders_check.py FO_COMP prints them
        if std::env::var_os("COMP").is_some() {
            let live = |k: usize| w.players[..k].iter().filter(|p| p.kind != 0).count();
            let h = w.players.iter().position(|p| p.kind == 1).unwrap();
            for &c in std::iter::once(&h).chain(w.competitor_list.iter()) {
                let p = &w.players[c];
                let (r, t, sg) = match p.place {
                    Some(Spot::Road(q)) => (q.road as i64, q.t.to_bits(), q.segment as i64),
                    _ => (-1, 0, 0),
                };
                println!("comp {tick} {} {r} {t} {sg} {:?} {} {}", live(c), p.advancement, u8::from(p.behind), u8::from(p.shadow));
            }
        }
        if !orders.is_empty() {
            let h = &w.players[w.players.iter().position(|p| p.kind == 1).unwrap()];
            let loads: Vec<String> = h.loads.iter().map(|l| format!("{} {} {:?} {:?} {:?} {:?}", l.merchandise, l.to, l.amount, l.deadline, l.time, l.condition)).collect();
            println!("hstate {tick} {:?} {} {:?} {}{}{}", h.money, h.wins, h.earned, loads.len(), if loads.is_empty() { "" } else { " " }, loads.join(" "));
        }
        // AMB=1: the ambushes 250..600 m from the human player (0x42b550)
        if std::env::var_os("AMB").is_some() {
            if let Some(h) = w.human_pos {
                for (k, a) in w.ambushes.iter().enumerate() {
                    let Some(n) = w.net.nodes.iter().position(|x| x.name == a.node) else { continue };
                    let p = w.net.nodes[n].pos;
                    let d = ((p[0] - h[0]).powi(2) + (p[1] - h[1]).powi(2) + (p[2] - h[2]).powi(2)).sqrt();
                    if (250.0..=600.0).contains(&d) {
                        eprintln!("amb {tick} {k} {} {} active {} d {d:.1}", a.id, a.kind, a.active);
                    }
                }
            }
        }
        if std::env::var_os("OBSTLOG").is_some() {
            let act: Vec<usize> = w.obstacles.iter().enumerate().filter(|(_, o)| o.active).map(|(k, _)| k).collect();
            eprintln!("obst {tick} {:?}", act);
        }
        if let Some(k) = std::env::var("WATCH").ok().and_then(|k| k.parse::<usize>().ok()) {
            let p = &w.players[k];
            if let Some(l) = p.loads.first() { println!("load {k} {:?}", l); }
            println!("rating {k} earning {:?} share {:?} fleet {:?} money {:?} loads {} rating {:?}", p.earning, p.share, p.fleet, p.money, p.loads.len(), p.rating);
            println!("watch {k} {:?} {:?} route {:?} to {:?} laps {} state {:#x} flags {:#x} wp {:?} adv {:?} behind {} shadow {}", p.place, p.place.map(|s| w.net.spot_coordinates(&s)), p.route.iter().map(|s| s.node).collect::<Vec<_>>(), p.route.last().map(|s| s.node), p.laps, p.state, p.flags, p.waypoints, p.advancement, p.behind, p.shadow);
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
    if std::env::var_os("OBST").is_some() {
        println!("obstacles {:?}", w.obstacles.iter().enumerate().map(|(k, o)| (k, o.active, o.road)).collect::<Vec<_>>());
    }
    if std::env::var_os("SEGFLAGS").is_some() {
        let f = w.net.seg_flags.borrow();
        for (r, road) in w.net.roads.iter().enumerate() {
            let v: Vec<String> = road.segments.iter().map(|&sg| format!("{:#x}", f[sg])).collect();
            println!("road {r} {}", v.join(" "));
        }
    }
    // PLAYERS=1: the players as `police_dump.py` prints the original's
    if std::env::var_os("PLAYERS").is_some() {
        for (k, p) in w.players.iter().enumerate() {
            let (r, n, t, pc, sg, f) = match p.place {
                Some(Spot::Road(q)) => (q.road as i64, q.node.map_or(-1, |n| n as i64), q.t, q.piece as i64, q.segment as i64, u8::from(q.reversed)),
                Some(Spot::Node(n)) => (-1, n as i64, 0.0, 0, 0, 0),
                None => (-1, -1, 0.0, 0, 0, 0),
            };
            println!("{k} {} {:#x} {r} {n} {t:?} {pc} {sg} {f} route {}", p.kind, p.flags, p.route.len());
        }
    }
    if std::env::var_os("EXPLORED").is_some() {
        let v: Vec<usize> = (0..w.net.roads.len()).filter(|&r| w.net.is_explored(r)).collect();
        println!("explored {} {:?}", v.len(), v);
    }
    if std::env::var_os("OBJ").is_some() {
        println!("objects made {:?} want {:?}", w.object_made, w.object_want);
        let mut c = std::collections::BTreeMap::new();
        for o in &w.objects {
            *c.entry(o.flags & 0xfff).or_insert(0) += 1;
        }
        println!("objects by flags {:?}", c);
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
