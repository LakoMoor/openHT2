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
    let mut w = World::new_game(&game, &map, std::rc::Rc::new(net), 1.0, human);
    let rng = |w: &World| {
        let s = format!("{:?}", w.rng);
        let table = s.split('[').nth(1).unwrap().split(']').next().unwrap().replace(',', "");
        let idx: Vec<&str> = s.split(": ").skip(2).map(|x| x.trim_end_matches(['}', ' ', ',']).split(',').next().unwrap()).collect();
        format!("rng {table} {} {}", idx[0], idx[1])
    };
    println!("{}", rng(&w));
    let mut time = 0.0;
    for tick in 1..=ticks {
        time += 0.1;
        // as the emulator has it: an empty scene with no memory to spare
        w.tick(time, &mut |_, _| false);
        println!("tick {tick}");
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
}
