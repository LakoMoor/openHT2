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
    // with a file of routes found by the original (tools/emu/route_check.py): compare
    if let Some(routes) = std::env::args().nth(2) {
        let (mut same, mut differ) = (0, 0);
        for line in std::fs::read_to_string(routes).unwrap().lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            if w.len() < 4 || w[3] == "error" {
                continue;
            }
            let (src, dst, mode): (usize, usize, u32) = (w[0].parse().unwrap(), w[1].parse().unwrap(), w[2].parse().unwrap());
            let cost: f64 = w[3].parse().unwrap();
            let theirs: Vec<String> = w[4..].iter().map(|s| s.to_string()).collect();
            let (ours, oc) = match net.route_between(src, dst, mode) {
                Some((r, c)) => (r.iter().map(|s| format!("{}:{}", s.node, s.road.map_or(-1, |r| r as i64))).collect::<Vec<_>>(), c),
                None => (Vec::new(), f64::MAX),
            };
            if ours == theirs && (oc - cost).abs() <= 1e-6 * cost.abs().max(1.0) {
                same += 1;
            } else {
                differ += 1;
                if differ <= 5 {
                    println!("{src}->{dst} mode {mode}: original {cost} {theirs:?}
    ours {oc} {ours:?}");
                }
            }
        }
        println!("routes: {same} same, {differ} differ");
        return;
    }
    println!("{} nodes {} roads", net.nodes.len(), net.roads.len());
    for (i, r) in net.roads.iter().enumerate() {
        println!("road {i} start {} end {} segments {} length {:.6} cumulative {:.6}", r.start, r.end, r.segments.len(), r.length, r.cumulative);
    }
}
