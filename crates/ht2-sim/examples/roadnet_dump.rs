//! Print the road network built from the `.way` files of a map folder (for comparing with what
//! the original builds, `tools/emu/dump_ai_world.py`).
//!
//!     cargo run -p ht2-sim --example roadnet_dump -- "<game>/MENV"
use ht2_sim::road::RoadNet;

fn main() {
    let dir = std::env::args().nth(1).expect("map folder");
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("way"))).collect();
    files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().to_string());
    let ways: Vec<_> = files.iter().map(|f| ht2_formats::way::rooms(&ht2_formats::way::parse(&std::fs::read(f).unwrap()).unwrap())).collect();
    let net = RoadNet::build(&ways);
    println!("{} nodes {} roads", net.nodes.len(), net.roads.len());
    for (i, r) in net.roads.iter().enumerate() {
        println!("road {i} start {} end {} segments {} length {:.6} cumulative {:.6}", r.start, r.end, r.segments.len(), r.length, r.cumulative);
    }
}
