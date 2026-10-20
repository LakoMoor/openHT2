//! The AI world of the original (`Ai_main.cpp`): the vehicle catalogue and what the start of a
//! new game (`0x434780`) sets up from it.

use crate::road::{AiRng, Parking};
use ht2_formats::sexpr::Sexp;

/// A vehicle of the catalogue (`evehicles.sch`):
/// `( vehicle ID "model" colour kind roles price power ( trailers ) ... )`.
#[derive(Debug, Clone)]
pub struct Vehicle {
    /// `+8`: the identifier (`KAMAZ_LORRY1`).
    pub id: String,
    /// `+0x14`: the model; the colours of a model share it.
    pub model: String,
    pub colour: i32,
    /// `+0x20`: 1 car, 2 lorry, 3 tractor, 4 semitrailer.
    pub kind: u32,
    /// `+0x24`: 0 none, 2 trucker, 5 police, 6 mafia, 7 dumb.
    pub roles: Vec<u32>,
    /// `+0x30`.
    pub price: f64,
    /// `+0x38`.
    pub power: f64,
    /// The trailers a tractor can pull.
    pub trailers: Vec<String>,
}

fn kind_code(s: &str) -> u32 {
    match s {
        "car" => 1,
        "lorry" => 2,
        "tractor" => 3,
        "semitrailer" => 4,
        _ => 0,
    }
}

fn role_code(s: &str) -> u32 {
    match s {
        "trucker" => 2,
        "police" => 5,
        "mafia" => 6,
        "dumb" => 7,
        _ => 0,
    }
}

impl Vehicle {
    /// The vehicles of a decrypted catalogue, in its order (their index is their number, `+0`).
    pub fn parse(forms: &[Sexp]) -> Vec<Vehicle> {
        let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
        tops.into_iter()
            .filter_map(|f| f.form("vehicle"))
            .map(|r| {
                let text = |k: usize| r.get(k).and_then(|s| s.as_text()).unwrap_or_default();
                let roles = match r.get(4) {
                    Some(Sexp::List(l)) => l.iter().filter_map(|s| s.as_text()).map(role_code).collect(),
                    Some(s) => vec![role_code(s.as_text().unwrap_or_default())],
                    None => Vec::new(),
                };
                Vehicle {
                    id: text(0).to_string(),
                    model: text(1).to_string(),
                    colour: text(2).parse().unwrap_or(0),
                    kind: kind_code(text(3)),
                    roles,
                    price: text(5).parse().unwrap_or(0.0),
                    power: text(6).parse().unwrap_or(0.0),
                    trailers: r.get(7).and_then(|s| s.as_list()).map_or(Vec::new(), |l| l.iter().filter_map(|s| s.as_text()).map(str::to_string).collect()),
                }
            })
            .collect()
    }

    /// The catalogue of the game folder (`evehicles.sch`).
    pub fn read(game_dir: &std::path::Path) -> Vec<Vehicle> {
        let Ok(data) = std::fs::read(game_dir.join("evehicles.sch")) else { return Vec::new() };
        let Ok(plain) = ht2_formats::sdte::decrypt(&data) else { return Vec::new() };
        ht2_formats::sexpr::parse(&ht2_formats::sdte::cp1251_to_string(&plain)).map_or(Vec::new(), |f| Vehicle::parse(&f))
    }
}

/// `0x434270` (new game): what the markets offer. For every place of a parking place with a
/// level above 0, `level` times: among the vehicles for sale that are left (trucker vehicles and
/// the `BMW_M5_1..4`, `0x457820`), one is drawn with weight 1 / (|price − level · 50000/11| + 1000)
/// and all colours of its model go to the place. Returns, per parking place and per place, the
/// numbers of the vehicles offered (`slot + 0x1c`).
pub fn market_offers(lots: &[Parking], vehicles: &[Vehicle], rng: &mut AiRng) -> Vec<Vec<Vec<usize>>> {
    let for_sale: Vec<usize> = (0..vehicles.len())
        .filter(|&k| vehicles[k].roles.contains(&2) || ["BMW_M5_1", "BMW_M5_2", "BMW_M5_3", "BMW_M5_4"].contains(&vehicles[k].id.as_str()))
        .collect();
    lots.iter()
        .map(|lot| {
            lot.nodes
                .iter()
                .map(|_| {
                    let mut offered = Vec::new();
                    let mut left = for_sale.clone();
                    let level = lot.level;
                    let mut drawn = 0;
                    while 0.0 < level && (drawn as f64) < level && !left.is_empty() {
                        let target = level * 4545.454545454546;
                        let weights: Vec<f64> = left.iter().map(|&k| 1.0 / ((vehicles[k].price - target).abs() + 1000.0)).collect();
                        let total: f64 = weights.iter().fold(0.0, |t, w| w + t);
                        let chosen = if left.len() == 1 {
                            Some(0)
                        } else {
                            let x = (rng.next() % 1_000_000) as f64 * total * 1e-6;
                            let mut acc = 0.0;
                            let mut found = None;
                            for (i, w) in weights.iter().enumerate() {
                                if acc <= x && x <= acc + w {
                                    found = Some(i);
                                    break;
                                }
                                acc += w;
                            }
                            found
                        };
                        let Some(c) = chosen else { break };
                        let model = vehicles[left[c]].model.clone();
                        while let Some(i) = left.iter().position(|&k| vehicles[k].model == model) {
                            offered.push(left.remove(i));
                        }
                        drawn += 1;
                    }
                    offered
                })
                .collect()
        })
        .collect()
}

/// The vehicle lists the AI draws from (`0x68cbf8`, twelve lists); `0x42fa90` (new game) fills
/// four of them.
#[derive(Debug, Clone, Default)]
pub struct Categories {
    pub lists: [Vec<usize>; 12],
}

/// `0x42fa90`: sort the catalogue into the lists. 8: four rounds over the vehicles with role dumb,
/// 8 or trucker, cars three times, then shuffled; 2: the trucker vehicles, even numbers first;
/// 7: cars and other vehicles of those roles taken in turn so that cars are `car_share` · 20/21
/// of the others (`0x66c6f0`: the traffic setting of the options, `trafs`, in percent × 0.01, which
/// the game passes to the new game, `0x572653`), as many as the two counts
/// multiplied; 9: list 7 shuffled.
pub fn sort_vehicles(vehicles: &[Vehicle], rng: &mut AiRng, car_share: f64) -> Categories {
    let has = |k: usize, r: u32| vehicles[k].roles.contains(&r);
    let n = vehicles.len();
    let mut c = Categories::default();
    for _ in 0..4 {
        for k in 0..n {
            if has(k, 7) || has(k, 8) || has(k, 2) {
                c.lists[8].push(k);
                if vehicles[k].kind == 1 {
                    c.lists[8].push(k);
                    c.lists[8].push(k);
                }
            }
        }
    }
    crate::stl::random_shuffle(&mut c.lists[8], |m| rng.next() % m);
    for start in [0, 1] {
        c.lists[2].extend((start..n).step_by(2).filter(|&k| has(k, 2)));
    }
    let road = |k: usize| has(k, 7) || has(k, 2);
    let cars: Vec<usize> = (0..n).filter(|&k| road(k) && vehicles[k].kind == 1).collect();
    let others: Vec<usize> = [1, 0].iter().flat_map(|&s| (s..n).step_by(2)).filter(|&k| road(k) && vehicles[k].kind != 1).collect();
    let ratio = car_share * 0.9523809523809523;
    let (mut ncars, mut nothers) = (0.0, 0.0);
    let (mut ic, mut io) = (0usize, 0usize);
    for _ in 0..cars.len() * others.len() {
        if ncars < nothers * ratio {
            c.lists[7].push(cars[ic % cars.len()]);
            ncars += 1.0;
            ic += 1;
        } else {
            c.lists[7].push(others[io % others.len()]);
            nothers += 1.0;
            io += 1;
        }
    }
    c.lists[9] = c.lists[7].clone();
    crate::stl::random_shuffle(&mut c.lists[9], |m| rng.next() % m);
    c
}
