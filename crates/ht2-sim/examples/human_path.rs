//! A drive of the human player for comparing with the original: from the start of `GAME.CNF`
//! along the roads at 15 m/s, to one node after another, one point a tick (0.1 s), written as
//! single-precision coordinates (what the game hands the AI in the vehicle's matrix, `0x43ee70`).
//!
//!     cargo run -p ht2-sim --example human_path -- "<game>" ENV <ticks> <out.txt>
use ht2_sim::road::{RoadNet, Spot};

fn main() {
    let game = std::path::PathBuf::from(std::env::args().nth(1).expect("game folder"));
    let map = game.join(std::env::args().nth(2).unwrap_or_else(|| "ENV".into()));
    let ticks: usize = std::env::args().nth(3).and_then(|t| t.parse().ok()).unwrap_or(1000);
    let out = std::env::args().nth(4).expect("output file");
    let mut files: Vec<_> = std::fs::read_dir(&map).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("way"))).collect();
    files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().to_string());
    let ways: Vec<_> = files.iter().map(|f| ht2_formats::way::rooms(&ht2_formats::way::parse(&std::fs::read(f).unwrap()).unwrap())).collect();
    let net = RoadNet::build(&ways);
    let start = [-3020.0, -4625.0, 20.8];
    let mut place = net.nearest(start).expect("a road");
    let goals = [600usize, 100, 900, 300, 1100, 50];
    let mut g = 0;
    let mut route = Vec::new();
    let mut lines = String::new();
    for _ in 0..ticks {
        if route.is_empty() {
            while route.is_empty() {
                let to = goals[g % goals.len()];
                g += 1;
                if let Some((r, _)) = net.route_from(&Spot::Road(place), &Spot::Node(to), 0) {
                    route = r;
                }
            }
            if let Spot::Road(p) = net.locate(&route, net.coordinates(&place), &Spot::Road(place)) {
                place = p;
            }
        }
        let mut d = 1.5;
        net.advance_route(&mut place, &mut d, &route);
        if d != 0.0 || place.node == route.last().map(|s| s.node) {
            route.clear();
        }
        let c = net.coordinates(&place);
        lines += &format!("{} {} {}\n", c[0] as f32, c[1] as f32, c[2] as f32);
    }
    std::fs::write(out, lines).unwrap();
}
