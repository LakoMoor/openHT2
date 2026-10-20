//! Print what the start of a new game sets up in the AI world (for comparing with the original in
//! the emulator, `tools/emu/newgame_steps.py`).
//!
//!     cargo run -p ht2-sim --example world_dump -- "<game>" ENV
use ht2_sim::road::{read_map_scripts, AiRng, Parking};
use ht2_sim::world::{market_offers, Vehicle};

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
    println!("rng {rng:?}");
}
