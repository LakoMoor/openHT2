//! Compare the road boxes (`RoadNet::road_box`) with a dump of the original's (`road + 0x88`,
//! lines `index flags x0 y0 z0 x1 y1 z1`).
//!
//!     cargo run -p ht2-sim --example road_boxes -- "<game>/ENV" boxes.txt
use ht2_sim::road::RoadNet;

fn main() {
    let dir = std::env::args().nth(1).expect("map folder");
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("way"))).collect();
    files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().to_string());
    let ways: Vec<_> = files.iter().map(|f| ht2_formats::way::rooms(&ht2_formats::way::parse(&std::fs::read(f).unwrap()).unwrap())).collect();
    let net = RoadNet::build(&ways);
    let (mut same, mut differ) = (0, 0);
    for line in std::fs::read_to_string(std::env::args().nth(2).expect("dump")).unwrap().lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        let Ok(i) = w[0].parse::<usize>() else { continue };
        let want: Vec<f64> = w[2..8].iter().map(|v| v.parse().unwrap()).collect();
        let b = net.road_box(i);
        let ours = [b[0][0], b[0][1], b[0][2], b[1][0], b[1][1], b[1][2]];
        if ours.iter().zip(&want).all(|(a, b)| a.to_bits() == b.to_bits()) {
            same += 1;
        } else {
            differ += 1;
            if differ <= 10 {
                println!("road {i}: ours {ours:?}\n          want {want:?}");
                let r = &net.roads[i];
                println!("  start {:?} end {:?}", net.nodes[r.start].pos, net.nodes[r.end].pos);
                for &sg in &r.segments {
                    println!("  seg {sg} {:?}", net.segments[sg].shape);
                }
            }
        }
    }
    println!("same {same} differ {differ}");
}
