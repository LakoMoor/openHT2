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
    let mut net = RoadNet::build(&ways);
    // the towns of the map scripts mark their nodes at the start of a game (0x433f90)
    let scripts = ht2_sim::road::read_map_scripts(std::path::Path::new(&dir));
    let towns = ht2_sim::road::Town::from_scripts(&scripts);
    net.mark_towns(&towns);
    let lots = ht2_sim::road::Parking::from_scripts(&scripts);
    net.mark_parkings(&lots);
    if std::env::var_os("PARKING").is_some() {
        for (i, l) in lots.iter().enumerate() {
            let nodes: Vec<i64> = l.nodes.iter().map(|(n, _, _)| net.nodes.iter().position(|x| &x.name == n).map_or(-1, |k| k as i64)).collect();
            println!("{i} id {} nodes {nodes:?}", l.number);
        }
        return;
    }
    if std::env::var_os("TOWNS").is_some() {
        for (i, t) in towns.iter().enumerate() {
            let nodes: Vec<_> = t.nodes.iter().map(|(n, f)| (net.nodes.iter().position(|x| &x.name == n).map_or(-1, |k| k as i64), *f)).collect();
            println!("{i} {} {nodes:?}", t.id);
        }
        // with the tables of the original (town index, then the costs to every town): compare
        if let Some(file) = std::env::args().nth(2) {
            let ours = net.town_distances(&towns);
            let (mut same, mut differ) = (0, 0);
            for line in std::fs::read_to_string(file).unwrap().lines() {
                let w: Vec<&str> = line.split_whitespace().collect();
                let i: usize = w[0].parse().unwrap();
                for (j, v) in w[1..].iter().enumerate() {
                    let want: f64 = v.parse().unwrap();
                    if (ours[i][j] - want).abs() <= 1e-9 * want.abs().max(1.0) { same += 1 } else {
                        differ += 1;
                        if differ <= 8 { println!("{i}->{j}: original {want} ours {}", ours[i][j]); }
                    }
                }
            }
            println!("town distances: {same} same, {differ} differ");
        }
        return;
    }
    // with a file of routes found by the original (tools/emu/route_check.py): compare
    // with the generator table and draws of the original (tools/emu/rng_check.py): compare
    if let Some(road) = std::env::var("ARC").ok().and_then(|r| r.parse::<usize>().ok()) {
        for (k, &s) in net.roads[road].segments.iter().enumerate() {
            let seg = &net.segments[s];
            println!("segment {k} {:?} length {:?}", seg.shape, seg.length());
        }
        return;
    }
    // with the road costs of the original (tools/emu/road_cost_check.py): compare bit for bit
    if let Some(file) = std::env::args().nth(2).filter(|f| f.contains("road_costs")) {
        let (mut same, mut differ, mut worst) = (0, 0, 0.0f64);
        for line in std::fs::read_to_string(file).unwrap().lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            let (road, mode, against): (usize, u32, u32) = (w[0].parse().unwrap(), w[1].parse().unwrap(), w[2].parse().unwrap());
            let want: f64 = w[3].parse().unwrap();
            let got = net.road_cost(mode, road, against != 0);
            if got.to_bits() == want.to_bits() {
                same += 1;
            } else {
                differ += 1;
                worst = worst.max(((got - want) / want).abs());
                if differ <= 5 {
                    println!("road {road} mode {mode} against {against}: original {want:?} ours {got:?} ({} segments)", net.roads[road].segments.len());
                }
            }
        }
        println!("road costs: {same} same bit for bit, {differ} differ (worst relative {worst:e})");
        return;
    }
    // with routes between places by the original (tools/emu/route_from_check.py): compare
    if let Some(file) = std::env::args().nth(2).filter(|f| f.contains("routes_from")) {
        let spot = |w: &[&str]| {
            let road: i64 = w[0].parse().unwrap();
            let node: i64 = w[1].parse().unwrap();
            if road < 0 {
                ht2_sim::road::Spot::Node(node as usize)
            } else {
                ht2_sim::road::Spot::Road(ht2_sim::road::Place {
                    road: road as usize,
                    node: (node >= 0).then_some(node as usize),
                    t: w[2].parse().unwrap(),
                    piece: w[3].parse().unwrap(),
                    segment: w[4].parse().unwrap(),
                    reversed: w[5].parse::<u32>().unwrap() & 1 != 0,
                })
            }
        };
        let (mut same, mut differ) = (0, 0);
        for line in std::fs::read_to_string(file).unwrap().lines() {
            let parts: Vec<&str> = line.split('|').map(str::trim).collect();
            if parts.len() < 5 {
                continue;
            }
            let mode: u32 = parts[0].parse().unwrap();
            let a: Vec<&str> = parts[1].split_whitespace().collect();
            let b: Vec<&str> = parts[2].split_whitespace().collect();
            let cost: f64 = parts[3].parse().unwrap();
            let theirs: Vec<&str> = parts[4].split_whitespace().collect();
            let (ours, oc) = match net.route_from(&spot(&a), &spot(&b), mode) {
                Some((r, c)) => (r.iter().map(|s| format!("{}:{}", s.node, s.road.map_or(-1, |r| r as i64))).collect::<Vec<_>>(), c),
                None => (Vec::new(), f64::MAX),
            };
            if ours == theirs && (oc - cost).abs() <= 1e-6 * cost.abs().max(1.0) {
                same += 1;
            } else {
                differ += 1;
                if differ <= 6 {
                    println!("{line}\n    ours {oc} {}", ours.join(" "));
                }
            }
        }
        println!("routes from places: {same} same, {differ} differ");
        return;
    }
    // with random start places of the original (tools/emu/place_check.py): compare
    if let Some(file) = std::env::args().nth(2).filter(|f| f.contains("places")) {
        let mut rng = ht2_sim::road::AiRng::default();
        let (mut same, mut differ) = (0, 0);
        for line in std::fs::read_to_string(file).unwrap().lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            let p = net.random_start(&mut rng).unwrap();
            let ours = format!("{} {} {} {} {} {}", p.road, p.node.map_or(-1, |n| n as i64), p.t, p.piece, p.segment, p.reversed as u32);
            let t: f64 = w[2].parse().unwrap();
            if w[0] == p.road.to_string() && w[1] == p.node.map_or(-1, |n| n as i64).to_string() && t.to_bits() == p.t.to_bits() && w[3] == p.piece.to_string() && w[4] == p.segment.to_string() && w[5] == (p.reversed as u32).to_string() {
                same += 1;
            } else {
                differ += 1;
                if differ <= 5 {
                    println!("original {line}
    ours {ours}");
                }
            }
        }
        println!("places: {same} same, {differ} differ");
        return;
    }
    if let Some(draws) = std::env::args().nth(2).filter(|f| f.contains("draws")) {
        let text = std::fs::read_to_string(draws).unwrap();
        let mut rng = ht2_sim::road::AiRng::default();
        let probe = rng.clone();
        for line in text.lines() {
            if let Some(t) = line.strip_prefix("table ") {
                let want: Vec<&str> = t.split_whitespace().collect();
                println!("table {}", if format!("{probe:?}").contains(&want[..55].join(", ")) { "same" } else { "DIFFERS" });
            }
        }
        let (mut same, mut differ) = (0, 0);
        for line in text.lines().filter(|l| l.starts_with("draw ")) {
            let want: i64 = line[5..].trim().parse().unwrap();
            let got = net.random_destination(&mut rng, 0, 0).map_or(-1, |n| n as i64);
            if got == want { same += 1 } else { differ += 1 }
        }
        println!("draws: {same} same, {differ} differ");
        return;
    }
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
    if std::env::var_os("NODE_FLAGS").is_some() {
        for (i, n) in net.nodes.iter().enumerate() {
            println!("node {i} raw {} flags {} {}", n.flag_raw, n.flags, n.name);
        }
        return;
    }
    for (i, r) in net.roads.iter().enumerate() {
        println!("road {i} start {} end {} segments {} length {:.6} cumulative {:.6}", r.start, r.end, r.segments.len(), r.length, r.cumulative);
    }
}
