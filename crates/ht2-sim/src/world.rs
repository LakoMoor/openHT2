//! The AI world of the original (`Ai_main.cpp`): the vehicle catalogue and what the start of a
//! new game (`0x434780`) sets up from it.

use crate::road::{AiRng, Parking, RoadNet, RouteStep, Spot};
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
    /// `+4`: the number after the trailers (1 for the tank semitrailers: they carry liquids).
    pub liquid: bool,
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
                    liquid: text(8) == "1",
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

/// A driver name of `names.sch`: `( "NAME" gender role number ... )`.
#[derive(Debug, Clone)]
pub struct DriverName {
    pub name: String,
    /// `+0x18`: 1 trucker, 2 mafia, 3 police.
    pub role: u32,
    /// `+0x30`: how well the driver keeps his cargo (1 at the start; `damageCargo`).
    pub care: f64,
    /// `+0x28`: how likely he is to go anywhere rather than for an order (`0x4194e0`; 1 for
    /// every name of the shipped game).
    pub wander: f64,
}

impl DriverName {
    pub fn parse(forms: &[Sexp]) -> Vec<DriverName> {
        let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
        tops.into_iter()
            .filter_map(|f| f.as_list())
            .filter(|l| matches!(l.first(), Some(Sexp::Str(_))))
            .map(|l| DriverName {
                care: 1.0,
                wander: 1.0,
                name: l[0].as_text().unwrap_or_default().to_string(),
                role: match l.get(2).and_then(|s| s.as_text()) {
                    Some("police") => 3,
                    Some("mafia") => 2,
                    _ => 1,
                },
            })
            .collect()
    }

    /// The names of the game folder (`names.sch`).
    pub fn read(game_dir: &std::path::Path) -> Vec<DriverName> {
        let Ok(data) = std::fs::read(game_dir.join("names.sch")) else { return Vec::new() };
        let Ok(plain) = ht2_formats::sdte::decrypt(&data) else { return Vec::new() };
        ht2_formats::sexpr::parse(&ht2_formats::sdte::cp1251_to_string(&plain)).map_or(Vec::new(), |f| DriverName::parse(&f))
    }
}

/// A second-order low-pass filter of the AI (`0x4881f0` builds it for a frequency `w` times the
/// step, by the bilinear transform; `0x4881b0` steps it).
#[derive(Debug, Clone, Copy)]
pub struct SpeedFilter {
    a: f64,
    c: f64,
    b: f64,
    y1: f64,
    y2: f64,
}

impl SpeedFilter {
    pub fn new(w: f64) -> Self {
        let k = 0.7353265585246408;
        let d1 = w * -1.5 * k;
        let d2 = w * 0.866 * k;
        let d3 = 1.0 - d1;
        let d5 = -d2;
        let d1b = d1 + 1.0;
        let d4 = d3 * d3 + d5 * d5;
        let re = (d1b * d3 + d5 * d2) / d4;
        let im = (d2 * d3 - d5 * d1b) / d4;
        // 0x488120
        let c = re + re;
        let b = -re * re - -im * -im;
        let a = (1.0 - (re + re)) - b;
        SpeedFilter { a, c, b, y1: 0.0, y2: 0.0 }
    }

    /// The filter at rest at `y` (both outputs).
    pub fn at(mut self, y: f64) -> Self {
        self.y1 = y;
        self.y2 = y;
        self
    }

    pub fn step(&mut self, x: f64) -> f64 {
        let y2 = self.y2;
        self.y2 = self.y1;
        // (y1·c + y2·b) + x·a, in this order (0x4881b0)
        self.y1 = (self.y1 * self.c + y2 * self.b) + x * self.a;
        self.y1
    }
}

/// A player of the AI world (`createNewPlayer`, `0x415940`; the list `0x68b990`).
#[derive(Debug, Clone)]
pub struct AiPlayer {
    /// `+0x54`: 1 the human, 2 trucker, 5 police, 6 mafia, 7..9 traffic.
    pub kind: u32,
    /// `+0x1fc`: the vehicle (catalogue number) and, for a tractor, its semitrailer.
    pub vehicle: Option<usize>,
    pub trailer: Option<usize>,
    /// `+0x3d8`: the driver name.
    pub name: Option<usize>,
    /// `+0x20` (`PositionId`).
    pub place: Option<Spot>,
    /// `+0x10`: the route.
    pub route: Vec<RouteStep>,
    /// For a vehicle standing at a parking place: the place and the slot (`+0x78`, `+0x7c`;
    /// such players have the state 0x100, `+0x64`).
    pub parked: Option<(usize, usize)>,
    /// `+0x5c`: 0x100 for sale, 0x101 hired out.
    pub state: u32,
    /// `+0x118`: the coordinates at the last tick; `+0x148`: the speed, `+0x170` and `+0x198`
    /// its filters (`0x473420`: 1e-4 and 1e-3 of the step).
    pub last_pos: [f64; 3],
    pub speed: f64,
    /// `+0x150`: the output of the second filter; `+0x158`: the velocity of the last tick.
    pub speed2: f64,
    pub velocity: [f64; 3],
    pub filters: [SpeedFilter; 2],
    /// Driven by a vehicle in the game's scene, which reports its place (then the AI does not
    /// move it).
    pub driven: bool,
    /// `+0x3b0`: when the player was made (ghosts go after 72 s out of sight).
    pub born: f64,
    /// `+0x58`: 1 the human player, 4 standing (the vehicles for sale have 0x44), 0x80000
    /// (wanted more in the scene).
    pub flags: u32,
    /// `+0x4c`: the id of the player's object in the game's scene (`-1`: none). The game gives
    /// it when the player is made (event 1 of `0x4d76b0`: 100000 + a counter modulo 500).
    pub object: Option<u32>,
    /// `+0x1f0`: in the list `0x68a980` (`0x4175c0`), wanted more in the scene.
    pub tracked: bool,
    /// For a vehicle for sale: which of the three kinds (`forsale`, `forsale_stolen`,
    /// `forsale_used`) it counts with (`0x68ced0`).
    pub dealer: Option<usize>,
    /// In a race (state 5): its place in the list of racers (`+0x2d8`), how many check points
    /// it has passed (`+0x2d4`) and its standing (`+0x1f8`, minus the route cost left).
    pub race_index: usize,
    pub laps: u32,
    pub race_progress: f64,
    /// Whether it has somewhere to go (the list of waypoints at `+0x40` is not empty): set when
    /// a route is given, cleared when it stands at the route's end (`0x417300`).
    pub waypoint: bool,
    /// `+0x68`: until when it stands (flag 4, `0x477bd0`).
    pub until: f64,
    /// A trucker's town (`+0x80`, its number) and the node of it whose order it goes for
    /// (`+0x84`, -1 for none).
    pub town: i32,
    pub order: i32,
    /// What it carries (`+0x1c` of its semitrailer, else of its vehicle).
    pub loads: Vec<crate::orders::Load>,
    /// `+0x80`: its money (40000 at the start, `0x68cfe8`); `+0x2f8`: what it has earned;
    /// `+0x3c0`: its rating.
    pub money: f64,
    pub earned: f64,
    pub rating: f64,
    /// `+0x2d8`: what it earns, smoothed every 8 s (`0x4880e0`: earned · 1/256 + 255/256 · it;
    /// 2 at the start); `+0x2f8`: its share of the load capacity of all truckers and the human
    /// player, and `+0x300` its own (`updateRating`, `0x41fcf0`).
    pub earning: f64,
    pub share: f64,
    pub fleet: f64,
    /// `+0x90`: the times its marks run out (`0x4774e0`).
    pub marks: Vec<f64>,
    /// `+0x28` of its vehicle (flag 2: it may be alarmed, `0x442850`) and whether it is (`+0x108`).
    pub vehicle_flags: u32,
    pub alarmed: bool,
    /// For a player in the scene (`0x41a9d0`): `+0x1c0` the filter its speed goes through (at
    /// rest at 1), `+0x3c8` whether it is at full speed, `+0x3d0` how hard it slows down (its
    /// speed may not exceed the one it has, less v²/2/(this + 0.1) a second).
    pub ramp: SpeedFilter,
    pub full_speed: bool,
    pub drag: f64,
}

/// What the AI world asks the game to do with its scene (`xLoadEntities`, `0x424550`): the
/// vehicle of a player is put into the scene as an object (`+0x4c`), then the game drives it;
/// the others move only on the road network.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneChange {
    /// Load the object of the player (`0x4d8b20`).
    Load(usize),
    /// Unload it (`0x4d8ae0`).
    Unload(usize),
}

/// An ambush of the map scripts (`( ambush ID MAFIA|POLICE ... )`, list `0x68a970` in the
/// order of the scripts): `+0x24` 1 mafia, 2 police; `+0xf0` whether it is set now.
#[derive(Debug, Clone)]
pub struct Ambush {
    pub id: String,
    pub kind: u32,
    pub active: bool,
}

impl Ambush {
    pub fn from_scripts(forms: &[Sexp]) -> Vec<Ambush> {
        let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
        tops.into_iter()
            .filter_map(|f| f.form("ambush"))
            .map(|r| Ambush {
                id: r.first().and_then(|s| s.as_text()).unwrap_or_default().to_string(),
                kind: match r.get(1).and_then(|s| s.as_text()) {
                    Some("MAFIA") => 1,
                    Some("POLICE") => 2,
                    _ => 0,
                },
                active: false,
            })
            .collect()
    }
}

/// An obstacle of the map scripts (`( obstacle ID REGULAR module "key" "node1" "node2"
/// ["closed" "open" ( "works" ... )] )`, list `0x68cc18`): the road between its two nodes
/// (`+0x38`, none when they are not given), where it is (`+0x40`: the last works node, else
/// half way between the two nodes), the nodes road works are put at and whether it is set.
#[derive(Debug, Clone)]
pub struct Obstacle {
    pub id: String,
    pub road: Option<usize>,
    pub pos: [f64; 3],
    pub works: Vec<usize>,
    pub active: bool,
}

impl Obstacle {
    pub fn from_scripts(forms: &[Sexp], net: &RoadNet) -> Vec<Obstacle> {
        let node = |name: &str| net.nodes.iter().position(|n| n.name == name);
        let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
        tops.into_iter()
            .filter_map(|f| f.form("obstacle"))
            .map(|r| {
                let text = |k: usize| r.get(k).and_then(|s| s.as_text()).unwrap_or_default();
                let (a, b) = (node(text(4)), node(text(5)));
                let road = match (a, b) {
                    (Some(a), Some(b)) => net.roads.iter().position(|r| (r.start == a && r.end == b) || (r.start == b && r.end == a)),
                    _ => None,
                };
                let works: Vec<usize> = r.get(8).and_then(|s| s.as_list()).map_or(Vec::new(), |l| l.iter().filter_map(|s| s.as_text()).filter_map(node).collect());
                let pos = match (works.last(), a, b) {
                    (Some(&w), _, _) => net.nodes[w].pos,
                    (None, Some(a), Some(b)) => {
                        let (p, q) = (net.nodes[a].pos, net.nodes[b].pos);
                        [(p[0] + q[0]) * 0.5, (p[1] + q[1]) * 0.5, (p[2] + q[2]) * 0.5]
                    }
                    _ => [0.0; 3],
                };
                Obstacle { id: text(0).to_string(), road, pos, works, active: false }
            })
            .collect()
    }
}

/// A task of the game clock: what it does, when it runs next and how often.
#[derive(Debug, Clone)]
pub struct Task {
    pub name: &'static str,
    pub time: f64,
    pub period: f64,
}

/// What the start of a new game keeps between its steps.
pub struct World {
    pub net: std::rc::Rc<RoadNet>,
    pub lots: Vec<Parking>,
    /// Per parking place and slot, the player standing there (`slot + 8`), and how many do
    /// (`+0x28`).
    pub slots: Vec<Vec<Option<usize>>>,
    pub lot_used: Vec<usize>,
    pub vehicles: Vec<Vehicle>,
    pub lists: Categories,
    pub names: Vec<DriverName>,
    /// `+0x1c` / `+0x20` of the names: how many players of the regular types (1, 2, 5, 6) and of
    /// the others use each name.
    pub name_use: Vec<[u32; 2]>,
    /// `0x68a830`: per player type, where its vehicle search goes on.
    pub cursor: [usize; 12],
    pub rng: AiRng,
    /// The second generator (`0x68a870`).
    pub rng2: AiRng,
    pub players: Vec<AiPlayer>,
    /// `0x68d0f0`: the AI time of the last tick.
    pub last_tick: f64,
    /// `0x66c6a0`: the speed the players move at while their rooms are not loaded: that of the
    /// human player (20 m/s at the start).
    pub travel_speed: f64,
    /// `0x68d13c`: `xLoadEntities` runs every ninth tick; `0x68d128`: how far away a vehicle has
    /// to be to be put into the scene (0 the first time, then 150 m).
    pub load_counter: u32,
    pub load_distance: f64,
    /// `0x66c60a`: the first tick runs `xLoadEntities` before all else.
    pub first_load: bool,
    /// The objects the game has in its scene (`0x4d8a40`), and the counter of the ids it gives.
    pub loaded: std::collections::BTreeSet<u32>,
    pub object_counter: u32,
    /// `+0x268` of the human player: where the game has its vehicle (else: its road place).
    pub human_pos: Option<[f64; 3]>,
    /// For comparing with the original: when set, the vectors each player was weighed by in
    /// the last `xLoadEntities`.
    pub debug_wants: Option<Vec<(usize, [f64; 3], [f64; 3])>>,
    /// For comparing with the original while the tasks that draw from the generator are not
    /// ported: the generator to go on with when the moves of the next tick start.
    pub debug_rng_at_moves: Option<AiRng>,
    /// For comparing with the original: print each `actOnArrival` as `formorders_check.py` does.
    pub debug_arrivals: bool,
    /// `updateGhosts` (`0x420ed0`): how many ghosts (type 8) to keep around the human player; it
    /// does something only after `xLoadEntities` ran once (`0x68d130`).
    pub ghost_want: usize,
    pub loads_done: bool,
    /// The tasks of the game clock (`0x68b99c`), in their order.
    pub tasks: Vec<Task>,
    /// The towns (`0x68cec0`), the objects (`0x68cf6c`) and what they can be
    /// (`etrashboxes.sch`), how many are wanted and made, the weight of each vehicle as a
    /// semitrailer at a base (`+0x70`), and the nodes that have an object (flag 0x10).
    pub towns: Vec<crate::road::Town>,
    pub objects: Vec<crate::objects::WorldObject>,
    pub presents: Vec<crate::objects::Present>,
    pub merchandise: Vec<crate::objects::Merchandise>,
    pub object_want: crate::objects::ObjectCounts,
    pub object_made: crate::objects::ObjectCounts,
    pub vehicle_weight: Vec<f64>,
    pub taken: Vec<bool>,
    /// `town + 0x64`: the cost of the way between every two towns (`0x414670`).
    pub town_dist: Vec<Vec<f64>>,
    /// The orders of the towns' store nodes (two slots per node, `node + 8`), and `0x68d100`
    /// (−2000 at the start; `formOrders` works when it is 20 s or more from now).
    pub town_slots: Vec<Vec<[Option<crate::objects::TownOrder>; 2]>>,
    pub orders_time: f64,
    /// The ambushes of the map (`0x68a970`).
    pub ambushes: Vec<Ambush>,
    /// The obstacles of the map (`0x68cc18`).
    pub obstacles: Vec<Obstacle>,
    /// The race (`provideRacesRegular`): `0x66c6e0` its state (3 waiting, 0 gathering racers,
    /// 2 running, 1 over), `0x66c6e4` the track, `0x68d170` when the wait began, `0x66c6d8`
    /// when it started and `0x66c6f8` how long it lasts, `0x68cf60` the racers and `0x66c6d0`
    /// how many it takes.
    pub race_state: u32,
    pub race_track: u32,
    pub race_wait: f64,
    pub race_start: f64,
    pub race_duration: f64,
    pub racers: Vec<usize>,
    pub racers_want: usize,
    /// `0x68d150`: how many vehicles are hired out, `0x68d154` how many of them wait.
    pub hired_want: usize,
    pub hired_waiting: usize,
    /// `0x66c68c`: standing ends at once rather than when its time is over (set when the AI
    /// starts, `0x403940`; a new game clears it, `0x423c00`).
    pub stand_free: bool,
    /// `0x68d14c`: how many vehicles are hired out.
    pub hired_count: usize,
    /// `0x68d164`: how many competitors have arrived.
    pub competitors_arrived: u32,
    /// `vehicle.tech` (read by `0x508000` into `0x697868`).
    pub tech: Vec<ht2_formats::tech::Tech>,
    /// `0x66c6f0`: the traffic setting (1 for 100 %); `0x68d15c`, `0x68d160`: how many
    /// competitors the human player gets and how (`0x425610`).
    pub car_share: f64,
    pub competitors: u32,
    pub competition: u32,
    /// `0x68ced0`: of each kind of vehicle for sale, how many there are and are wanted.
    pub dealer_have: [usize; 3],
    pub dealer_want: [usize; 3],
    /// `0x66c6e8`, drawn at the start of a new game (600..1199 s).
    pub obstacle_time: f64,
}

impl World {
    pub fn new(net: std::rc::Rc<RoadNet>, lots: Vec<Parking>, vehicles: Vec<Vehicle>, lists: Categories, names: Vec<DriverName>, rng: AiRng, time: u32) -> Self {
        let slots: Vec<Vec<Option<usize>>> = lots.iter().map(|l| vec![None; l.nodes.len()]).collect();
        let lot_used = vec![0; lots.len()];
        let name_use = vec![[0, 0]; names.len()];
        let vehicle_weight = vec![1.0; vehicles.len()];
        let taken = vec![false; net.nodes.len()];
        World { net, lots, slots, lot_used, vehicles, lists, names, name_use, cursor: [0; 12], rng, rng2: AiRng::new(time ^ 161_803_398), players: Vec::new(), last_tick: time as f64, travel_speed: 20.0, load_counter: 0, load_distance: 0.0, first_load: true, loaded: Default::default(), object_counter: 0, human_pos: None, debug_wants: None, debug_rng_at_moves: None, debug_arrivals: false, ghost_want: 0, loads_done: false, tasks: Vec::new(), towns: Vec::new(), objects: Vec::new(), presents: Vec::new(), merchandise: Vec::new(), object_want: Default::default(), object_made: Default::default(), vehicle_weight, taken, hired_want: 0, hired_waiting: 0, stand_free: true, hired_count: 0, competitors_arrived: 0, tech: Vec::new(), ambushes: Vec::new(), obstacles: Vec::new(), race_state: 3, race_track: 1, race_wait: 0.0, race_start: 0.0, race_duration: 0.0, racers: Vec::new(), racers_want: 4, car_share: 1.0, competitors: 0, competition: 0, dealer_have: [0; 3], dealer_want: [0; 3], obstacle_time: 0.0, town_dist: Vec::new(), town_slots: Vec::new(), orders_time: -2000.0 }
    }

    /// `0x4148a0`: the name for a new player: among the names of its role (trucker for types 1,
    /// 2 and 7..9, mafia for 6, police for 5) the least used one, the first in the file of those
    /// (a stable sort by the count, `0x45c8d0`); the count of the regular types for 1, 2, 5, 6, the
    /// other one for the rest (`0x474160`).
    fn pick_name(&mut self, kind: u32) -> Option<usize> {
        let role = match kind {
            5 => 3,
            6 => 2,
            _ => 1,
        };
        let which = if matches!(kind, 1 | 2 | 5 | 6) { 0 } else { 1 };
        let k = (0..self.names.len()).filter(|&k| self.names[k].role == role).min_by_key(|&k| self.name_use[k][which])?;
        self.name_use[k][which] += 1;
        Some(k)
    }

    /// `createNewPlayer` (`0x415940`) for the types placed on the roads: the vehicle, a tractor's
    /// semitrailer and the driver name.
    fn new_player(&mut self, kind: u32) -> AiPlayer {
        let n = self.vehicles.len();
        let has = |v: usize, r: u32| self.vehicles[v].roles.contains(&r);
        let mut vehicle = None;
        let mut trailer = None;
        match kind {
            2 => {
                // along list 2 until a trucker vehicle
                let list = &self.lists.lists[2];
                let v = loop {
                    let v = list[self.cursor[2] % list.len()];
                    if has(v, 2) {
                        break v;
                    }
                    self.cursor[2] += 1;
                };
                self.cursor[2] += 1;
                vehicle = Some(v);
                let trailers = &self.vehicles[v].trailers;
                if !trailers.is_empty() {
                    let t = &trailers[(self.rng.next() % trailers.len() as u32) as usize];
                    trailer = self.vehicles.iter().position(|x| &x.id == t);
                }
            }
            8 => {
                // along list 8 until a vehicle with role 7, 8 or 2
                let list = &self.lists.lists[8];
                let v = loop {
                    let v = list[self.cursor[8] % list.len()];
                    if has(v, 7) || has(v, 8) || has(v, 2) {
                        break v;
                    }
                    self.cursor[8] += 1;
                };
                self.cursor[8] += 1;
                vehicle = Some(v);
            }
            9 => {
                // along list 9 until a vehicle with role 9 or 2 (or 7 when 0x68d1a0 is set)
                let list = &self.lists.lists[9];
                let v = loop {
                    let v = list[self.cursor[9] % list.len()];
                    if has(v, 9) || has(v, 2) {
                        break v;
                    }
                    self.cursor[9] += 1;
                };
                self.cursor[9] += 1;
                vehicle = Some(v);
            }
            5 | 6 => {
                // along the catalogue until a vehicle with the role of the type
                let mut v = self.cursor[kind as usize];
                while !has(v % n, kind) {
                    v = (v + 1) % n;
                }
                vehicle = Some(v % n);
                self.cursor[kind as usize] = (v + 1) % n;
            }
            _ => {}
        }
        let name = if kind == 1 { None } else { self.pick_name(kind) };
        AiPlayer { kind, vehicle, trailer, name, place: None, route: Vec::new(), parked: None, state: 0, last_pos: [0.0; 3], speed: 20.0, speed2: 0.0, velocity: [0.0; 3], filters: [SpeedFilter::new(1e-4), SpeedFilter::new(1e-3)], driven: false, born: 0.0, flags: u32::from(kind == 1), object: Some(self.next_object()), tracked: false, dealer: None, race_index: 0, laps: 0, race_progress: 0.0, waypoint: false, until: f64::MAX, town: -1, order: -1, loads: Vec::new(), money: 40000.0, earned: 0.0, earning: 2.0, share: 0.0, fleet: 0.0, rating: 10000.0, marks: Vec::new(), vehicle_flags: 0, alarmed: false, ramp: SpeedFilter::new(0.01).at(1.0), full_speed: true, drag: 1000.0 }
    }

    /// `traceToArbitraryDestination` (`0x417820`): random destinations (`0x456140`) until there
    /// is a route to one from the player's place (`0x478260`, mode 0).
    fn trace_to_random(&mut self, p: &mut AiPlayer) {
        let place = p.place.expect("placed");
        let which = match place {
            Spot::Node(n) => self.net.nodes[n].flags,
            Spot::Road(q) => q.node.map_or(0, |n| self.net.nodes[n].flags),
        };
        p.route.clear();
        while p.route.is_empty() {
            let Some(dst) = self.net.random_destination(&mut self.rng, which, p.kind) else { return };
            self.set_route(p, &Spot::Node(dst));
        }
    }

    /// `0x40a2a0`: a new route to a random destination other than the node nearest to the player
    /// (`0x405a60`, `0x4558e0` → `0x45b8b0`), drawn again until there is a route.
    fn trace_away(&mut self, p: &mut AiPlayer) {
        let Some(place) = p.place else { return };
        let c = self.net.spot_coordinates(&place);
        let mut near = 0;
        let mut best = 1e36;
        for (k, n) in self.net.nodes.iter().enumerate() {
            let d = [c[0] - n.pos[0], c[1] - n.pos[1], c[2] - n.pos[2]];
            let l = (d[0] * d[0] + d[2] * d[2] + d[1] * d[1]).sqrt();
            if l < best {
                best = l;
                near = k;
            }
        }
        let net = self.net.clone();
        let picks: Vec<usize> = (0..net.nodes.len())
            .filter(|&k| {
                let f = net.nodes[k].flags;
                if f & 8 != 0 || k == near {
                    return false;
                }
                if p.kind == 2 {
                    // trucks: other towns' nodes (flag 2 from a node with flag 1, else flag 1)
                    let from = net.nodes[near].flags & 1 != 0;
                    let other = net.nodes[near].town.map(|t| t.0) != net.nodes[k].town.map(|t| t.0);
                    other && if from { f & 2 != 0 } else { f & 1 != 0 }
                } else {
                    f & 0x40 != 0
                }
            })
            .collect();
        p.route.clear();
        while p.route.is_empty() {
            if picks.is_empty() {
                return;
            }
            let dst = picks[self.rng.next() as usize % picks.len()];
            self.set_route(p, &Spot::Node(dst));
        }
    }

    /// `0x478260` / `0x4872f0`: the route (mode 0) from the player's place to `to`; when there is
    /// one, the player's place is set again from its coordinates on it (`0x43dc60`).
    pub(crate) fn set_route(&self, p: &mut AiPlayer, to: &Spot) {
        let Some(from) = p.place else { return };
        if let Some((r, _)) = self.net.route_from(&from, to, 0) {
            p.route = r;
            p.waypoint = true;
            let q = self.net.spot_coordinates(&from);
            p.place = Some(self.net.locate(&p.route, q, &from));
        }
    }

    /// The node of a parking slot.
    fn slot_node(&self, lot: usize, slot: usize) -> Option<usize> {
        let name = &self.lots[lot].nodes[slot].0;
        self.net.nodes.iter().position(|n| &n.name == name)
    }

    /// `0x456460`: the free slots of a parking place for vehicles of `kind` (0 for sale: slots
    /// without the bits 0x2c, 1 stolen: bit 0x20, 2 used: bit 8).
    fn free_slots(&self, lot: usize, kind: u32) -> Vec<usize> {
        (0..self.lots[lot].nodes.len())
            .filter(|&k| {
                let bits = self.lots[lot].nodes[k].1;
                let fits = match kind {
                    0 => bits & 0x2c == 0,
                    1 => bits & 0x20 != 0,
                    _ => bits & 8 != 0,
                };
                fits && self.slots[lot][k].is_none()
            })
            .collect()
    }

    /// `findAppropriateParkingPlace` (`0x418c10`): of the parking places with free slots of the
    /// kind, the least used one (the first of those); there the free slot farthest from the
    /// taken ones (a stable sort by the distance to the nearest taken slot, the last one).
    fn parking_place(&self, kind: u32) -> Option<(usize, usize)> {
        let mut best: Option<usize> = None;
        let mut used = 1000;
        for lot in 0..self.lots.len() {
            if !self.free_slots(lot, kind).is_empty() && self.lot_used[lot] < used && self.lot_used[lot] < self.lots[lot].nodes.len() {
                best = Some(lot);
                used = self.lot_used[lot];
            }
        }
        let lot = best?;
        let pos = |k: usize| self.slot_node(lot, k).map_or([0.0; 3], |n| self.net.nodes[n].pos);
        // 0x418890: the distance to the nearest taken slot, (y² + z²) + x²
        let key = |k: usize| {
            let a = pos(k);
            let mut d = 1_000_000.0;
            for (j, taken) in self.slots[lot].iter().enumerate() {
                if taken.is_some() {
                    let b = pos(j);
                    let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
                    let l = (dy * dy + dz * dz + dx * dx).sqrt();
                    if l < d {
                        d = l;
                    }
                }
            }
            d
        };
        let mut free = self.free_slots(lot, kind);
        free.sort_by(|&a, &b| key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal));
        Some((lot, *free.last()?))
    }

    /// `createDealerForSale` (`0x4221d0`, `0x421d90`): vehicles standing for sale at the
    /// markets: `wants[kind]` of each kind (`forsale`, `forsale_stolen`, `forsale_used` of
    /// `GAME.CNF`), players of type 9 put on a slot (`0x421a20`), at its node on the first road
    /// leaving it (else arriving, `0x421690`), turned when that road's first segment is one-way
    /// against it (`0x421880`).
    pub fn create_dealers(&mut self, wants: [usize; 3]) {
        self.dealer_want = wants;
        self.refill_dealers();
    }

    /// `0x4221d0`: for each kind, dealers while there are fewer than wanted (100 tries at most).
    fn refill_dealers(&mut self) {
        for kind in 0..3 {
            for _ in 0..100 {
                if self.dealer_want[kind] <= self.dealer_have[kind] {
                    break;
                }
                let Some((lot, slot)) = self.parking_place(kind as u32) else { break };
                self.dealer_have[kind] += 1;
                let mut p = self.new_player(9);
                p.dealer = Some(kind);
                if let Some(n) = self.slot_node(lot, slot) {
                    let node = &self.net.nodes[n];
                    let road = node.out.first().or(node.inc.first()).copied();
                    p.place = Some(match road {
                        Some(road) => {
                            let first = &self.net.segments[self.net.roads[road].segments[0]];
                            Spot::Road(crate::road::Place { road, node: Some(n), t: 0.0, piece: 0, segment: 0, reversed: first.flags & 8 != 0 })
                        }
                        None => Spot::Node(n),
                    });
                }
                p.parked = Some((lot, slot));
                p.state = 0x100;
                p.flags = 0x44;
                self.slots[lot][slot] = Some(self.players.len());
                self.lot_used[lot] += 1;
                self.players.push(p);
            }
        }
    }

    /// `0x423ad0` (new game): `want` truckers hired out (`forhire` of `GAME.CNF`): each time one
    /// of the truckers not standing anywhere yet is drawn (`0x45e0a0`) and sent to a slot of a
    /// parking place for sale (`goHireIt`, `0x419030`: the route to it, the slot taken), and at
    /// the start of a game put right at its end (`0x43e610`).
    pub fn hire_out(&mut self, want: usize) {
        let mut have = 0;
        for _ in 0..100 {
            if want <= have {
                return;
            }
            let cands: Vec<usize> = (0..self.players.len())
                .filter(|&k| {
                    let p = &self.players[k];
                    p.kind == 2 && p.state != 0x100 && p.state != 0x101 && p.vehicle.is_some()
                })
                .collect();
            if cands.is_empty() {
                return;
            }
            let k = cands[(self.rng.next() % cands.len() as u32) as usize];
            // 0x4786a0: the route given up
            self.players[k].route.clear();
            have += 1;
            self.go_hire_it(k);
            let mut p = self.players[k].clone();
            // 0x43e610: put at the end of the route
            if let (Some(last), Some(place)) = (p.route.last(), p.place) {
                let q = self.net.nodes[last.node].pos;
                p.place = Some(self.net.locate(&p.route, q, &place));
                // standing at the end: its waypoint is reached
                p.waypoint = false;
            }
            self.players[k] = p;
        }
    }

    /// `goHireIt` (`0x419030`): player `k` stands no longer (`0x477d60`), is hired out (state
    /// 0x101, one more of them, `0x68d14c`) and goes to a slot of a parking place for sale
    /// (`0x418c10`, `0x419490`), drawn again until there is a route; the slot is taken.
    fn go_hire_it(&mut self, k: usize) {
        self.players[k].route.clear();
        self.unstand(k);
        self.players[k].state = 0x101;
        self.hired_count += 1;
        let mut p = self.players[k].clone();
        let mut taken = None;
        while p.route.is_empty() {
            let Some((lot, slot)) = self.parking_place(0) else { break };
            let Some(n) = self.slot_node(lot, slot) else { break };
            self.set_route(&mut p, &Spot::Node(n));
            taken = Some((lot, slot));
        }
        if let Some((lot, slot)) = taken {
            self.slots[lot][slot] = Some(k);
            self.lot_used[lot] += 1;
            p.parked = Some((lot, slot));
        }
        self.players[k] = p;
    }

    /// `sendForSomething` (`0x4194e0`, in the normal game): a trucker who carries nothing goes to
    /// be hired out (`goHireIt`) while fewer are than are wanted (`0x418f70`: it has a vehicle and
    /// no company); else a number is drawn: at least a million times the driver's `+0x28` it
    /// goes anywhere (`0x417820`), else for the closest order (`0x4185d0`).
    fn send_for_something(&mut self, k: usize) {
        let p = &self.players[k];
        if self.hired_count < self.hired_want && p.kind == 2 && p.state == 0 && p.vehicle.is_some() && p.loads.is_empty() {
            self.go_hire_it(k);
            return;
        }
        let wander = self.players[k].name.and_then(|n| self.names.get(n)).map_or(1.0, |n| n.wander);
        let r = (self.rng.next() % 1_000_000) as f64;
        if wander * 1_000_000.0 <= r {
            let mut p = self.players[k].clone();
            self.trace_to_random(&mut p);
            self.players[k] = p;
        } else {
            self.send_for_closest_order(k);
        }
    }

    /// The move of the players in a tick while their rooms are not loaded (`processMove`,
    /// `0x41ae60`; `prescribeAdvance`, `0x41a9d0`; `advancePlayer`, `0x435a90`): 20 m/s (police
    /// 1.3 times that) times the time since the last tick along the route. The human player, the
    /// vehicles standing for sale and those hired out do not move.
    pub fn tick_move(&mut self, now: f64) {
        let dt = now - self.last_tick;
        for k in 0..self.players.len() {
            self.end_standing(k, now);
            self.move_player(k, dt);
            // then, for a racer, the race (0x416b80)
            if self.players[k].state == 5 && self.players[k].kind != 0 {
                self.race_step(k);
            }
            self.semi_arrival(k);
            // 0x4165a0: not the human player, not standing and nowhere to go
            let p = &self.players[k];
            if p.kind != 0 && p.kind != 1 && p.flags & 4 == 0 && !p.waypoint {
                self.act_on_arrival(k, now);
            }
        }
    }

    /// The start of `processMove` (`0x41ae60`): a vehicle hired out whose wait is over goes on
    /// when all hired out wait (`0x419760`); any other that stands stops standing when its time
    /// is over (`0x477d60`; at once while `0x66c68c` is set, which it is not in a game), and so
    /// does one with flag 0x8000000. A vehicle
    /// for sale whose time is over would go when its dealer has enough (`+0x3cc`; not ported).
    fn end_standing(&mut self, k: usize, now: f64) {
        let p = &self.players[k];
        if p.state == 0x101 && p.flags & 4 != 0 && p.until <= now && self.hired_want <= self.hired_waiting {
            self.cancel_hiring(k, now);
        }
        let p = &self.players[k];
        if p.flags & 4 != 0 && p.state != 0x101 && p.state != 0x100 && (self.stand_free || p.until <= now) {
            self.unstand(k);
        }
        let p = &self.players[k];
        if p.flags & 0x800_0000 != 0 && p.until <= now {
            self.unstand(k);
        }
    }

    /// `0x477cd0`: it stands no longer (a vehicle hired out no longer counts as waiting).
    fn unstand(&mut self, k: usize) {
        let p = &mut self.players[k];
        if p.flags & 4 != 0 && p.state == 0x101 {
            self.hired_waiting = self.hired_waiting.saturating_sub(1);
        }
        p.flags &= 0xf7ff_fffb;
        p.until = 0.0;
    }

    /// `0x477bd0`: it stands for `t` s.
    fn stand(&mut self, k: usize, now: f64, t: f64) {
        let p = &mut self.players[k];
        p.flags |= 4;
        p.until = now + t;
    }

    /// `0x4540a0` with the towns: a town drawn among those that are not filling stations or
    /// deal in liquids (`+0x60 & 3`).
    fn random_town(&mut self) -> Option<usize> {
        let picks: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].flags & 3 == 0).collect();
        if picks.is_empty() {
            return None;
        }
        Some(picks[self.rng.next() as usize % picks.len()])
    }

    /// `0x4050f0`: the town nearest to `q` among those of [`World::random_town`].
    fn nearest_plain_town(&self, q: [f64; 3]) -> Option<usize> {
        let mut best = f64::MAX;
        let mut found = None;
        for (k, t) in self.towns.iter().enumerate() {
            if t.flags & 3 != 0 {
                continue;
            }
            let Some(n) = self.net.town_node(t) else { continue };
            let c = self.net.nodes[n].pos;
            let d = [q[0] - c[0], q[1] - c[1], q[2] - c[2]];
            let l = ((d[0] * d[0] + d[2] * d[2]) + d[1] * d[1]).sqrt();
            if l < best {
                best = l;
                found = Some(k);
            }
        }
        found
    }

    /// `0x418400`: up to 100 times a node number of town `t` is drawn (`next() % 9` of the
    /// second generator); the first whose order slot for the player's vehicle (trucks slot 0,
    /// else slot 1; `getOrderInfo`, `0x441120`) holds an order, else -1.
    fn pick_order(&mut self, k: usize, t: usize) -> i32 {
        let truck = self.players[k].vehicle.is_some_and(|v| matches!(self.vehicles[v].kind, 2 | 3));
        for _ in 0..100 {
            let r = (self.rng2.next() % 9) as usize;
            let held = self.town_slots.get(t).and_then(|n| n.get(r)).is_some_and(|s| s[if truck { 0 } else { 1 }].is_some());
            if held {
                return r as i32;
            }
        }
        -1
    }

    /// `traceToOrderNumber` (`0x4180f0`): a route to node `r` of town `t`.
    fn trace_to_order(&mut self, k: usize, t: usize, r: usize) {
        let Some((name, _)) = self.towns[t].nodes.get(r) else { return };
        let Some(n) = self.net.nodes.iter().position(|x| &x.name == name) else { return };
        let mut p = self.players[k].clone();
        p.route.clear();
        self.set_route(&mut p, &Spot::Node(n));
        self.players[k] = p;
    }

    /// `sendForClosestOrder` (`0x4185d0`): a trucker (carrying nothing) goes for an order in
    /// the nearest town (`0x4050f0`), else to a random destination (`0x417820`); others go to a
    /// random destination.
    fn send_for_closest_order(&mut self, k: usize) {
        if self.players[k].kind != 2 {
            let mut p = self.players[k].clone();
            self.trace_to_random(&mut p);
            self.players[k] = p;
            return;
        }
        let q = self.players[k].place.map_or([0.0; 3], |s| self.net.spot_coordinates(&s));
        let Some(t) = self.nearest_plain_town(q) else { return };
        self.players[k].town = self.towns[t].number;
        let r = self.pick_order(k, t);
        self.players[k].order = r;
        if r == -1 {
            let mut p = self.players[k].clone();
            self.trace_to_random(&mut p);
            self.players[k] = p;
            return;
        }
        self.trace_to_order(k, t, r as usize);
        self.players[k].state = 3;
    }

    /// `actOnSemiArrival` (`0x417300`): at the end node of its route a player has reached its
    /// waypoint.
    fn semi_arrival(&mut self, k: usize) {
        let p = &self.players[k];
        if !p.waypoint {
            return;
        }
        let Some(last) = p.route.last().map(|s| s.node) else { return };
        let at = match p.place {
            Some(Spot::Node(n)) => Some(n),
            Some(Spot::Road(q)) => q.node,
            None => None,
        };
        if at == Some(last) {
            self.players[k].waypoint = false;
        }
    }

    /// `actOnArrival` (`0x419cb0`), for a player with nowhere to go. A ghost is sent to a random
    /// destination (`0x417820`); the game is told (events 8 and 10, which it lets pass); a racer
    /// goes on racing; a vehicle hired out waits 1200 + `next() % 1500` s at its slot (flags 0x400
    /// and 4, `0x477bd0`), and when more than all hired out but two wait, one whose wait is over
    /// (drawn) is no longer for hire (`0x419760`). Players other than truckers go to a random
    /// node other than the nearest one (`0x40a2a0`). The truckers' choices (orders, towns,
    /// parkings) are not ported.
    fn act_on_arrival(&mut self, k: usize, now: f64) {
        let kind = self.players[k].kind;
        if self.debug_arrivals {
            let p = &self.players[k];
            println!("arrival {} type {} state {:#x} flags {:#x}", k, kind, p.state, p.flags);
        }
        if kind == 8 {
            let mut p = self.players[k].clone();
            self.trace_to_random(&mut p);
            self.players[k] = p;
            return;
        }
        let state = self.players[k].state;
        if state == 5 {
            match self.players[k].laps {
                10 => self.finish_race(k, now),
                l if l < 10 => self.race_step(k),
                _ => {}
            }
            return;
        }
        if state == 0x101 {
            let wait = (self.rng.next() % 1500 + 1200) as f64;
            let p = &mut self.players[k];
            p.flags |= 0x400 | 4;
            p.until = now + wait;
            self.hired_waiting += 1;
            if (self.hired_want as i64) - 2 < self.hired_waiting as i64 {
                let picks: Vec<usize> = (0..self.players.len()).filter(|&j| self.players[j].state == 0x101 && self.players[j].flags & 4 != 0 && self.players[j].until <= now).collect();
                if !picks.is_empty() {
                    let j = picks[self.rng.next() as usize % picks.len()];
                    self.cancel_hiring(j, now);
                }
            }
            return;
        }
        if kind == 2 {
            if state == 4 {
                // at the town of its load: unloaded (0x443d50 at the town's first node), then it
                // stands for a second and goes on (0x4194e0); with nothing to unload it just goes on
                let Some(to) = self.players[k].loads.first().map(|l| l.to) else {
                    self.players[k].state = 0;
                    self.send_for_something(k);
                    return;
                };
                self.unload_order(k, to, 0, now);
                self.players[k].state = 0;
                self.stand(k, now, 1.0);
                self.send_for_something(k);
                return;
            }
            if state == 3 {
                // at the node of its order: loaded (0x443d50), it goes to the town the load is
                // for (0x417ed0, state 4), else for the closest order (0x4185d0); then it stands
                // for a second
                let town = self.towns.iter().position(|t| t.number == self.players[k].town);
                let order = self.players[k].order;
                if let (Some(t), true) = (town, 0 <= order) {
                    self.load_order(k, t, order as usize, now);
                }
                if self.players[k].loads.is_empty() {
                    self.send_for_closest_order(k);
                } else {
                    self.trace_to_delivery(k);
                    self.players[k].state = 4;
                }
                self.stand(k, now, 1.0);
                return;
            }
            // 0x404e30: a random town, a node with an order there (0x418400), and a route to
            // it (state 3), else to another town (0x40a2a0); then it stands for a second
            let Some(t) = self.random_town() else { return };
            self.players[k].town = self.towns[t].number;
            let r = self.pick_order(k, t);
            self.players[k].order = r;
            if r == -1 {
                let mut p = self.players[k].clone();
                self.trace_away(&mut p);
                self.players[k] = p;
            } else {
                self.trace_to_order(k, t, r as usize);
                self.players[k].state = 3;
            }
            self.stand(k, now, 1.0);
            return;
        }
        let mut p = self.players[k].clone();
        self.trace_away(&mut p);
        self.players[k] = p;
    }

    /// A racer at its finish (`actOnArrival`, `0x419dc0`, ten check points passed): a number is
    /// drawn (360 + `next() % 360`, the time of the marks a racer gets when competitors have
    /// arrived, not ported), the game is told (event 11), one more point is counted; a
    /// competitor has arrived (`0x417530`). It races no more (`0x417680`: out of the racers, flag
    /// 0x4000 and its route go, state 0), stands 20 s and goes anywhere (`0x417820`); when no
    /// racer is left and the human player does not race, the race waits again (`0x4194a0`).
    fn finish_race(&mut self, k: usize, now: f64) {
        let _ = self.rng.next() % 360 + 360;
        self.players[k].laps += 1;
        if self.players[k].tracked {
            self.competitors_arrived += 1;
            self.players[k].tracked = false;
        }
        self.racers.retain(|&r| r != k);
        let p = &mut self.players[k];
        p.flags &= !0x4000;
        p.state = 0;
        p.route.clear();
        self.stand(k, now, 20.0);
        let mut p = self.players[k].clone();
        self.trace_to_random(&mut p);
        self.players[k] = p;
        self.players[k].laps = 0;
        let human_races = self.players.iter().any(|p| p.kind == 1 && p.state == 5);
        if !human_races && self.racers.is_empty() {
            self.race_state = 3;
            self.race_wait = now;
        }
    }

    /// `cancelSellingOrHiring` (`0x419760`) for a vehicle hired out: it counts no longer, its
    /// slot is free, it stands no longer (`0x477d60`); a trucker goes for the closest order
    /// (`0x4185d0`), others to a random node other than the nearest one (`0x405a60`,
    /// `0x40a2a0`); then it stands for 20 s. (With cargo it would deliver it, `0x417ed0`.)
    fn cancel_hiring(&mut self, k: usize, now: f64) {
        if self.players[k].state == 0x101 {
            self.hired_count = self.hired_count.saturating_sub(1);
            if self.players[k].flags & 4 != 0 {
                self.hired_waiting = self.hired_waiting.saturating_sub(1);
            }
        }
        let p = &mut self.players[k];
        p.route.clear();
        p.state = 0;
        p.flags &= 0xffff_fbbf;
        self.unstand(k);
        if let Some((lot, slot)) = self.players[k].parked.take() {
            self.slots[lot][slot] = None;
            self.lot_used[lot] = self.lot_used[lot].saturating_sub(1);
        }
        if !self.players[k].loads.is_empty() {
            self.trace_to_delivery(k);
            self.players[k].state = 4;
        } else if self.players[k].kind == 2 {
            self.send_for_closest_order(k);
        } else {
            let mut p = self.players[k].clone();
            self.trace_away(&mut p);
            self.players[k] = p;
        }
        self.stand(k, now, 20.0);
    }

    /// `processMove` (`0x41ae60`) of one player: along its route at the AI speed.
    fn move_player(&mut self, k: usize, dt: f64) {
        {
            let in_scene = self.in_scene(&self.players[k]);
            let p = &mut self.players[k];
            // 0x41ae60: not the human player (flag 1) and not standing (flag 4)
            if p.kind == 0 || p.kind == 1 || p.flags & 4 != 0 || p.state == 0x100 || p.state == 0x101 || p.driven {
                return;
            }
            let Some(Spot::Road(mut place)) = p.place else { return };
            // 0x41a9d0: the speed of the human player, at least 50 km/h (the game's speed of
            // the vehicle when it drives one, 0x4d5c50, is not asked here)
            let mut v = 13.88888888888889;
            if v < self.travel_speed {
                v = self.travel_speed;
            }
            if p.kind == 5 {
                v *= 1.3;
            }
            if in_scene && 1e-6 < dt {
                // in the scene: through the filter, and not faster than it goes now less what
                // it loses slowing down
                let mut u = 1.0;
                if !p.full_speed {
                    let x = p.drag + 5.0;
                    if 0.0 < x && x < 35.0 {
                        u = p.drag * 0.02857142857142857;
                    }
                }
                let n = (dt * 100.0) as i64;
                for _ in 1..n {
                    p.ramp.step(u);
                }
                v *= p.ramp.step(u);
                if 0.0 < p.drag {
                    let w = p.velocity;
                    let s = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
                    let cap = s * s * -0.5 / (p.drag + 0.1) * dt + s;
                    if cap < v {
                        v = cap;
                    }
                }
            }
            let mut dist = dt * v;
            self.net.advance_route(&mut place, &mut dist, &p.route);
            p.place = Some(Spot::Road(place));
        }
    }

    /// `calculatePlayerSpeedsAndFares` (`0x423f40`, end of a tick): each player's speed from how
    /// far it moved since the last tick, through its filters, run once per hundredth of a
    /// second (moves over 100 m/s are left out); the human player's sets the travel speed.
    pub fn update_speeds(&mut self, now: f64) {
        let dt = now - self.last_tick;
        if dt < 1e-6 {
            return;
        }
        let steps = (dt * 100.0) as i32;
        for k in 0..self.players.len() {
            let fuel = match self.players[k].vehicle {
                Some(v) if self.players[k].kind == 2 => self.tech_of(v).map_or(0.0, |t| t.fuel_use as f64),
                _ => 0.0,
            };
            let p = &mut self.players[k];
            let Some(place) = p.place else { continue };
            let q = self.net.spot_coordinates(&place);
            let d = [q[0] - p.last_pos[0], q[1] - p.last_pos[1], q[2] - p.last_pos[2]];
            p.last_pos = q;
            let moved = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
            if p.kind == 2 {
                // the fuel a trucker of no company pays for (0.002 a unit of its record's use per metre)
                p.money -= moved * fuel * 0.002;
            }
            let raw = moved / dt;
            if raw < 100.0 {
                for _ in 0..steps - 1 {
                    p.filters[0].step(raw);
                    p.filters[1].step(raw);
                }
                p.speed = p.filters[0].step(raw);
                p.speed2 = p.filters[1].step(raw);
                let k = 1.0 / dt;
                p.velocity = [k * d[0], d[1] * k, d[2] * k];
            }
            if p.kind == 1 {
                self.travel_speed = p.speed;
            }
        }
    }

    /// `0x4244c0`: how much a vehicle at the offsets `now` and `soon` from the human player is
    /// wanted in the scene: the inverse distance (at least 1 m; beyond 400 m three times as far).
    fn closeness(now: [f64; 3], soon: [f64; 3]) -> f64 {
        let mut best = 0.0;
        for d in [now, soon] {
            let l = (d[0] * d[0] + d[2] * d[2] + d[1] * d[1]).sqrt();
            let mut c = 1.0;
            if 1.0 <= l {
                c = l;
                if 400.0 < l {
                    c = l * 3.0;
                }
            }
            if best < 1.0 / c {
                best = 1.0 / c;
            }
        }
        best
    }

    /// The id the game gives the next player's object (event 1 of `0x4d76b0`).
    fn next_object(&mut self) -> u32 {
        let id = 100_000 + self.object_counter;
        self.object_counter = (self.object_counter + 1) % 500;
        id
    }

    /// Whether the game has the player's object in its scene (`0x4d8a40`).
    pub fn in_scene(&self, p: &AiPlayer) -> bool {
        p.object.is_some_and(|id| self.loaded.contains(&id))
    }

    /// `0x40b0d0`: where a player is: for the human player where the game has its vehicle
    /// (`+0x268`), else its place on the roads (`+0x130`).
    pub fn position(&self, p: &AiPlayer) -> Option<[f64; 3]> {
        if p.flags & 1 != 0 {
            if let Some(h) = self.human_pos {
                return Some(h);
            }
        }
        p.place.map(|q| self.net.spot_coordinates(&q))
    }

    /// `xLoadEntities` (`0x424550`, every ninth tick): which players' vehicles the game should
    /// have in its scene. Each player but the human one gets a want (`0x4244c0`) of where it is
    /// now relative to the human player and of where it will be in a quarter of a second
    /// relative to where he will be: the human and standing players (flags 1, 4) go on at
    /// their velocity; moving players run their route ahead at the speed of the second filter,
    /// and for them the original takes only a quarter of his velocity off, not his place, so
    /// their second point is far from him unless he is near the world's origin. ×1.2 when in
    /// the scene, ×1.25 when tracked, ×0.7 for ghosts (type 8), ×1.05 for flag 0x80000. In that
    /// order, highest first (the STL sort), the objects of players not wanted at all (1e-8) are
    /// unloaded, then those not in the scene and farther than the load distance (0 the first
    /// time, then 150 m) are loaded as long as `fits` says there is memory for one more, else
    /// the least wanted ones in the scene beyond that distance are unloaded first. `fits`
    /// stands in for the game's memory budget (`0x4d8a90`); the hidden objects (`0x68cf6c`) and
    /// the game's own (`0x67b2b8`) are not weighed yet.
    pub fn load_entities(&mut self, fits: &mut dyn FnMut(&std::collections::BTreeSet<u32>, Option<u32>) -> bool) -> Vec<SceneChange> {
        self.load_counter += 1;
        if 8 < self.load_counter {
            self.load_counter = 0;
        } else if self.load_counter != 0 {
            return Vec::new();
        }
        let Some(human) = self.players.iter().position(|p| p.kind == 1) else { return Vec::new() };
        let Some(hpos) = self.position(&self.players[human]) else { return Vec::new() };
        let hv = self.players[human].velocity;
        let hv4 = [hv[0] * 0.25, hv[1] * 0.25, hv[2] * 0.25];
        // (player, want, distance)
        let mut list: Vec<(usize, f64, f64)> = Vec::new();
        let mut wants = self.debug_wants.as_ref().map(|_| Vec::new());
        for (k, p) in self.players.iter().enumerate() {
            // 0x417660: not the human player (unless flag 0x100)
            if p.kind == 0 || (p.kind == 1 && p.flags & 0x100 == 0) {
                continue;
            }
            let (Some(place), Some(pos)) = (p.place, self.position(p)) else { continue };
            let (now, soon) = if p.flags & 5 == 0 {
                let c = self.net.spot_coordinates(&place);
                let ahead = match place {
                    Spot::Road(mut q) => {
                        let mut d = p.speed2 * 0.25;
                        self.net.advance_route(&mut q, &mut d, &p.route);
                        self.net.coordinates(&q)
                    }
                    Spot::Node(_) => c,
                };
                ([c[0] - hpos[0], c[1] - hpos[1], c[2] - hpos[2]], [ahead[0] - hv4[0], ahead[1] - hv4[1], ahead[2] - hv4[2]])
            } else {
                let v = p.velocity;
                let ahead = [pos[0] + v[0] * 0.25, pos[1] + v[1] * 0.25, pos[2] + v[2] * 0.25];
                let hsoon = [hpos[0] + hv4[0], hpos[1] + hv4[1], hpos[2] + hv4[2]];
                ([pos[0] - hpos[0], pos[1] - hpos[1], pos[2] - hpos[2]], [ahead[0] - hsoon[0], ahead[1] - hsoon[1], ahead[2] - hsoon[2]])
            };
            if let Some(w) = &mut wants {
                w.push((k, now, soon));
            }
            let mut want = Self::closeness(now, soon);
            let d = [pos[0] - hpos[0], pos[1] - hpos[1], pos[2] - hpos[2]];
            let dist = (d[1] * d[1] + d[2] * d[2] + d[0] * d[0]).sqrt();
            if self.in_scene(p) {
                want *= 1.2;
            }
            if p.tracked {
                want *= 1.25;
            }
            if p.kind == 8 {
                want *= 0.7;
            }
            if p.flags & 0x80000 != 0 {
                want *= 1.05;
            }
            list.push((k, want, dist));
        }
        if wants.is_some() {
            self.debug_wants = wants;
        }
        crate::stl::sort(&mut list, |a, b| a.1 > b.1);
        let mut changes = Vec::new();
        // the players not wanted at all: out
        while let Some(&(k, want, _)) = list.last() {
            if 1e-8 < want {
                break;
            }
            if self.unload(k) {
                changes.push(SceneChange::Unload(k));
            }
            list.pop();
        }
        let mut back = list.len();
        let mut i = 0;
        'next: while i < list.len() {
            let (k, _, dist) = list[i];
            if !self.in_scene(&self.players[k]) && self.load_distance < dist {
                while !fits(&self.loaded, self.players[k].object) {
                    // make room: out with the least wanted player in the scene beyond the load
                    // distance; when there is none behind this one, stop
                    loop {
                        if back <= i + 1 {
                            break 'next;
                        }
                        back -= 1;
                        let (j, _, d) = list[back];
                        if self.load_distance < d && self.unload(j) {
                            changes.push(SceneChange::Unload(j));
                            break;
                        }
                    }
                }
                if let Some(id) = self.players[k].object {
                    self.loaded.insert(id);
                    changes.push(SceneChange::Load(k));
                }
            }
            i += 1;
        }
        self.load_distance = 150.0;
        self.loads_done = true;
        changes
    }

    /// Take the player's object out of the scene; false when it was not in it.
    fn unload(&mut self, k: usize) -> bool {
        self.players[k].object.is_some_and(|id| self.loaded.remove(&id))
    }

    /// `0x4842f0`: how sharply a route turns where its roads meet: the smallest cosine between
    /// the way one road is left and the next is entered (1 for a route of one road).
    fn route_turn(&self, route: &[RouteStep]) -> f64 {
        let net = &*self.net;
        let dir = |road: usize, at_end: bool, flip: bool| {
            let r = &net.roads[road];
            let p = if at_end { net.road_end(road) } else { net.road_start(road) };
            let mut t = net.tangent(&crate::road::Place { reversed: false, ..p });
            let _ = r;
            if flip {
                t = [-t[0], -t[1], -t[2]];
            }
            t
        };
        let mut best = 1.0;
        for w in route.windows(2) {
            let (Some(r0), Some(r1)) = (w[0].road, w[1].road) else { break };
            let along0 = net.roads[r0].start == w[0].node;
            let along1 = net.roads[r1].start == w[1].node;
            let a = if along0 { dir(r0, true, false) } else { dir(r0, false, true) };
            let b = if along1 { dir(r1, false, false) } else { dir(r1, true, true) };
            let ka = 1.0 / (a[0] * a[0] + a[2] * a[2] + a[1] * a[1]).sqrt();
            let kb = 1.0 / (b[0] * b[0] + b[2] * b[2] + b[1] * b[1]).sqrt();
            let c = b[0] * kb * ka * a[0] + a[1] * ka * b[1] * kb + b[2] * kb * a[2] * ka;
            if c < best {
                best = c;
            }
        }
        best
    }

    /// `0x417b70`: like `traceToArbitraryDestination`, but a route that turns back sharply
    /// (cosine below −0.8) is drawn again, up to three times.
    fn trace_without_turns(&mut self, p: &mut AiPlayer) {
        let place = p.place.expect("placed");
        let which = match place {
            Spot::Node(n) => self.net.nodes[n].flags,
            Spot::Road(q) => q.node.map_or(0, |n| self.net.nodes[n].flags),
        };
        p.route.clear();
        let mut tries = 0;
        while p.route.is_empty() {
            let Some(dst) = self.net.random_destination(&mut self.rng, which, p.kind) else { return };
            self.set_route(p, &Spot::Node(dst));
            tries += 1;
            if !p.route.is_empty() && self.route_turn(&p.route) < -0.8 && tries < 3 {
                p.route.clear();
            }
        }
    }

    /// `updateGhosts` (`0x420ed0`, a task of the game clock, after `xLoadEntities` ran once): the
    /// ghosts (type 8) are the traffic around the human player. A ghost goes when it is out of
    /// the scene and within 100 m of him (it could not appear unseen), or when it is not in the
    /// scene within 330 m and is older than 72 s or farther than 700 m. While there are fewer
    /// than wanted (and the human player is not in state 5) one may come: when the scene has no
    /// memory to spare only half the time; up to eight times a direction is drawn and the place
    /// nearest to the point 320 m that way on the roads of his room (`0x4365d0`); if that is on a
    /// segment without flags 0x10 and 0x40 and at least 160 m away, a ghost is put there with
    /// the chance √(road width / 14) (half that for `attr2` 2, a quarter for 3) and sent to a
    /// random destination (`0x420cb0`).
    pub fn update_ghosts(&mut self, now: f64, fits: &mut dyn FnMut(&std::collections::BTreeSet<u32>, Option<u32>) -> bool) -> Vec<SceneChange> {
        let mut changes = Vec::new();
        let Some(human) = self.players.iter().position(|p| p.kind == 1) else { return changes };
        let Some(h) = self.position(&self.players[human]) else { return changes };
        // the count of ghosts the spawning compares (0x68cca8 + 0x20) is the one before this
        // task deletes any
        let ghosts = self.players.iter().filter(|p| p.kind == 8).count();
        let mut gone = Vec::new();
        for (k, p) in self.players.iter().enumerate() {
            if p.kind != 8 {
                continue;
            }
            let Some(q) = self.position(p) else { continue };
            let d = [q[0] - h[0], q[1] - h[1], q[2] - h[2]];
            let in_scene = self.in_scene(p);
            if !in_scene && ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]).sqrt() < 100.0 {
                gone.push(k);
                continue;
            }
            if in_scene && ((d[1] * d[1] + d[2] * d[2]) + d[0] * d[0]).sqrt() <= 330.0 {
                continue;
            }
            if now - p.born <= 72.0 && ((d[1] * d[1] + d[2] * d[2]) + d[0] * d[0]).sqrt() <= 700.0 {
                continue;
            }
            gone.push(k);
        }
        for k in gone {
            // 0x4466a0: the player is deleted (and its object with it); before, selling or hiring
            // out is cancelled (0x419760), which sends it on to a random destination
            let mut p = self.players[k].clone();
            self.trace_away(&mut p);
            if self.unload(k) {
                changes.push(SceneChange::Unload(k));
            }
            let p = &mut self.players[k];
            p.kind = 0;
            p.place = None;
            p.route.clear();
        }
        if self.ghost_want <= ghosts || self.players[human].state == 5 {
            return changes;
        }
        if !fits(&self.loaded, None) && self.rng.next() & 1 == 0 {
            return changes;
        }
        // 0x438d70: the room of the human player's place
        let room = match self.players[human].place {
            Some(Spot::Road(q)) => Some(self.net.room_of(&q).to_string()),
            _ => None,
        };
        for _ in 0..8 {
            let a = (self.rng.next() % 0x9965e) as f64 * 1e-5;
            let x = a.cos() * 160.0;
            let y = a.sin() * 160.0;
            let q = [x + x + h[0], y + y + h[1], h[2] + 0.0];
            // 0x4365d0 (without a room 0x436900, here the stand-in `nearest`)
            let found = match &room {
                Some(r) => self.net.nearest_in_room(r, q),
                None => self.net.nearest(q),
            };
            let Some(place) = found else { continue };
            let seg = &self.net.segments[self.net.roads[place.road].segments[place.segment]];
            if self.net.flags_of(self.net.roads[place.road].segments[place.segment]) & 0x50 != 0 {
                continue;
            }
            let c = self.net.coordinates(&place);
            let d = [c[0] - h[0], c[1] - h[1], c[2] - h[2]];
            if ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]).sqrt() < 160.0 {
                continue;
            }
            let mut chance = (seg.width_at(place.t, place.piece) * 0.07142857142857142).sqrt();
            if seg.attr2 == 2 {
                chance *= 0.5;
            } else if seg.attr2 == 3 {
                chance *= 0.25;
            }
            if chance * 1000.0 <= (self.rng.next() % 1000) as f64 {
                return changes;
            }
            // 0x420cb0
            let mut p = self.new_player(8);
            p.place = Some(Spot::Road(place));
            p.born = now;
            // +0x118 stays 0: the first speed of the ghost is over 100 m/s and left out
            self.trace_without_turns(&mut p);
            self.players.push(p);
            return changes;
        }
        changes
    }

    /// `updateDealers` (`0x422370`, every 27.5 s): each vehicle for sale gets a lifetime of
    /// 900 + `next() % 120` s (drawn every time); one older than that and more than 2000 m from
    /// the human player is sold off (`0x446c80`, deleted by `killDelayedPlayers`, its slot
    /// freed). Those within 700 m in the scene stay as they are, the others are put back on
    /// their slots (`0x422210`, where they are anyway here). Then the kinds are filled up again
    /// (`0x4221d0` with 700 m, which the slot choice does not use).
    pub fn update_dealers(&mut self, now: f64) {
        let h = self.players.iter().find(|p| p.kind == 1).and_then(|p| self.position(p)).unwrap_or([0.0; 3]);
        let mut gone = Vec::new();
        for k in 0..self.players.len() {
            if self.players[k].kind != 9 {
                continue;
            }
            let life = (self.rng.next() % 120 + 900) as f64;
            let p = &self.players[k];
            if life < now - p.born {
                let q = self.position(p).unwrap_or([0.0; 3]);
                let d = [h[0] - q[0], h[1] - q[1], h[2] - q[2]];
                if 2000.0 < ((d[2] * d[2] + d[1] * d[1]) + d[0] * d[0]).sqrt() {
                    gone.push(k);
                }
            }
        }
        for k in gone {
            if let Some((lot, slot)) = self.players[k].parked {
                self.slots[lot][slot] = None;
                self.lot_used[lot] -= 1;
            }
            if let Some(kind) = self.players[k].dealer {
                self.dealer_have[kind] -= 1;
            }
            self.unload(k);
            let p = &mut self.players[k];
            p.kind = 0;
            p.place = None;
            p.route.clear();
        }
        self.refill_dealers();
    }

    /// `provideCompetitionRegular` (`0x4262a0`, every 30 s) as far as the human player has no
    /// order (`0x425790`): the tracked competitors are let go (`0x4256f0`) and drawn again how
    /// many there will be, 3 + `next() % (⌊3.1 · traffic + 4⌋ − 2)`, and how they compete,
    /// `next() & 3` (`0x425610`). The branches for a human player with an order are not ported.
    pub fn provide_competition(&mut self) {
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        if self.players[h].state == 5 {
            return;
        }
        for p in &mut self.players {
            p.tracked = false;
        }
        let n = (self.car_share * 3.1 + 4.0) as u32;
        self.competitors = self.rng.next() % (n - 2) + 3;
        self.competition = self.rng.next() & 3;
    }

    /// `markAmbushes` (`0x42c0b0`, every 300 s): police ambushes are set with the chance 0.3
    /// (`next() % 1000000 < 300000`); mafia ones only while the human player is wanted
    /// (`+0xe0`, `+0xc9`) and his `+0x300` is over 0.2, which never holds at the start (not
    /// ported further).
    pub fn mark_ambushes(&mut self) {
        for k in 0..self.ambushes.len() {
            self.ambushes[k].active = false;
            if self.ambushes[k].kind == 2 {
                self.ambushes[k].active = ((self.rng.next() % 1_000_000) as f64) < 300000.0;
            }
        }
    }

    /// `damageCargo` (`0x420b90`, every 5.1 s): for each trucker a number is drawn; when it is
    /// at least a million times the driver's care (never while that is 1), its cargo would be
    /// damaged (`0x446d20`; the truckers carry nothing yet).
    pub fn damage_cargo(&mut self) {
        for k in 0..self.players.len() {
            if self.players[k].kind != 2 {
                continue;
            }
            let care = self.players[k].name.and_then(|n| self.names.get(n)).map_or(1.0, |n| n.care);
            let draw = (self.rng.next() % 1_000_000) as f64;
            if care * 1_000_000.0 <= draw {
                // 0x446d20: the cargo is damaged (no cargo is carried yet)
            }
        }
    }

    /// `ObstaclesTask` (`0x42b3a0`): when the obstacle that is set (if any) is more than 500 m
    /// from the human player, up to five times one is drawn; the first that is more than 500 m
    /// from him and from the one set is set instead (`0x42b170`).
    pub fn obstacles_task(&mut self) {
        if self.obstacles.is_empty() {
            return;
        }
        let h = self.players.iter().find(|p| p.kind == 1).and_then(|p| self.position(p));
        let far = |a: [f64; 3], b: [f64; 3]| {
            let (dx, dy, dz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
            500.0 < (dx * dx + dz * dz + dy * dy).sqrt()
        };
        let away = |w: &World, o: usize| h.is_none_or(|h| far(w.obstacles[o].pos, h));
        let cur = self.obstacles.iter().position(|o| o.active);
        if cur.is_some_and(|c| !away(self, c)) {
            return;
        }
        for _ in 0..5 {
            let o = self.rng.next() as usize % self.obstacles.len();
            if away(self, o) && cur.is_none_or(|c| far(self.obstacles[o].pos, self.obstacles[c].pos)) {
                self.set_obstacle(o);
                return;
            }
        }
    }

    /// `0x42b170`: obstacle `o` is set and the others not (`0x42b060`): a set one has road works
    /// at its works nodes (`0x41cff0`) and flag 0x40 on the segments of its road (cleared again,
    /// but for segments with 0x80, when it goes). With a road, the route searches start afresh
    /// (`0x484280`) and the players whose route takes that road find a new one to the same end
    /// (`0x40a1d0`).
    pub fn set_obstacle(&mut self, o: usize) {
        for k in 0..self.obstacles.len() {
            if k != o && self.obstacles[k].active {
                self.obstacles[k].active = false;
                for &n in &self.obstacles[k].works.clone() {
                    self.remove_works(n);
                }
                if let Some(r) = self.obstacles[k].road {
                    let mut f = self.net.seg_flags.borrow_mut();
                    for &sg in &self.net.roads[r].segments {
                        if f[sg] & 0x80 == 0 {
                            f[sg] &= !0x40;
                        }
                    }
                }
            }
        }
        if !self.obstacles[o].active {
            for &n in &self.obstacles[o].works.clone() {
                let name = self.net.nodes[n].name.clone();
                self.road_works(&name);
            }
        }
        self.obstacles[o].active = true;
        let Some(r) = self.obstacles[o].road else { return };
        {
            let mut f = self.net.seg_flags.borrow_mut();
            for &sg in &self.net.roads[r].segments {
                f[sg] |= 0x40;
            }
        }
        self.net.clear_steps(0);
        self.net.clear_steps(1);
        for k in 0..self.players.len() {
            if self.players[k].kind == 0 || !self.players[k].route.iter().any(|s| s.road == Some(r)) {
                continue;
            }
            let Some(dest) = self.players[k].route.last().map(|s| s.node) else { continue };
            let at = match self.players[k].place {
                Some(Spot::Node(n)) => Some(n),
                Some(Spot::Road(q)) => q.node,
                None => None,
            };
            if at != Some(dest) {
                let mut p = self.players[k].clone();
                self.set_route(&mut p, &Spot::Node(dest));
                self.players[k] = p;
            }
        }
    }

    /// `updateRating` (`0x41fcf0`, every 8 s): each player's earnings go into its smoothed
    /// earning (`0x4880e0`) and start again; then each player's share is the load weight its
    /// vehicle can carry (`0x41fca0`: the human player and truckers only; with those of its
    /// company's players, none here) over that of all. A share of at least `0x66c710` (0.51)
    /// would make it the winner (`+0x308`, event to the game; not ported, no one gets there).
    fn update_rating(&mut self) {
        for p in &mut self.players {
            p.earning = p.earned * 0.00390625 + 0.99609375 * p.earning;
            p.earned = 0.0;
        }
        let fleet = |w: &World, k: usize| if matches!(w.players[k].kind, 1 | 2) { w.capacity(k).0 } else { 0.0 };
        let mut total = 0.0;
        for k in 0..self.players.len() {
            total = fleet(self, k) + total;
        }
        if total == 0.0 {
            return;
        }
        for k in 0..self.players.len() {
            let f = fleet(self, k);
            self.players[k].fleet = f;
            self.players[k].share = f / total;
        }
    }

    /// The human player's share of the load capacity (`+0x300` of its record, `updateRating`).
    pub fn human_share(&self) -> f64 {
        self.players.iter().find(|p| p.kind == 1).map_or(0.0, |p| p.share)
    }

    /// `expireParkedCars` (`0x42a2c0`, every 19 s): parked cars (not road works) put more than
    /// 900 s ago and more than 2000 m from the human player go: their present counts one less
    /// and the object is destroyed (`0x41ef10`: the game is told, the node is free, one parked
    /// car less).
    fn expire_parked_cars(&mut self, now: f64) {
        let Some(h) = self.players.iter().find(|p| p.kind == 1).and_then(|p| self.position(p)) else { return };
        let mut i = 0;
        while i < self.objects.len() {
            let o = &self.objects[i];
            let (dx, dy, dz) = (h[0] - o.pos[0], h[1] - o.pos[1], h[2] - o.pos[2]);
            if o.flags & 0x400 == 0 && o.flags & 0xf0 == 0x80 && 900.0 < now - o.made && 2000.0 < (dx * dx + dz * dz + dy * dy).sqrt() {
                if let Some(pr) = o.present {
                    self.presents[pr].count = self.presents[pr].count.saturating_sub(1);
                }
                let n = o.node;
                self.objects.remove(i);
                self.taken[n] = false;
                self.object_made.parkings -= 1;
                continue;
            }
            i += 1;
        }
    }

    /// `0x41f970`: the road works at node `n` go (`0x41ef10`).
    fn remove_works(&mut self, n: usize) {
        if let Some(i) = self.objects.iter().position(|o| o.node == n && o.flags & 0x400 != 0) {
            self.objects.remove(i);
            self.taken[n] = false;
            self.object_made.parkings -= 1;
        }
    }

    /// `provideRacesRegular` (`0x4279d0`, every 30 s). Waiting (3), the race is set up 300 s
    /// after the wait began (track 1, `0x4279a0`). Gathering (0), when the human player is at
    /// least 700 m from `start<track>_3`, racers are drawn until there are four (`0x45eba0`,
    /// else without the `+0xa8` condition `0x45ed90`): truckers in a tractor (for a human player
    /// in a truck), not tracked, not standing, for sale, hired out or racing, not in the scene.
    /// Each drops its loads (`0x4778a0`) and is put at `start<track>_<n>` (n from 1) with a
    /// route to `check_point1` and state 5, standing there (`0x477c50`) until the race is over;
    /// with the fourth the race runs (2) for the human player's route cost to the start / 10,
    /// at most 3660, in whole minutes. Running past that, it is over (1, `0x4278b0`).
    pub fn provide_races(&mut self, now: f64) {
        if self.race_state == 2 && self.race_duration < now - self.race_start {
            // 0x4278b0: the racers stand no more (0x477cd0)
            self.race_state = 1;
            for k in self.racers.clone() {
                self.unstand(k);
            }
            return;
        }
        if self.race_state == 3 && 300.0 < now - self.race_wait {
            self.race_state = 0;
            self.race_track = 1;
        }
        if self.race_state != 0 {
            return;
        }
        let Some(human) = self.players.iter().position(|p| p.kind == 1) else { return };
        let node = |w: &World, name: &str| w.net.nodes.iter().position(|n| n.name == name);
        let Some(s3) = node(self, &format!("start{}_3", self.race_track)) else { return };
        let h = self.position(&self.players[human]).unwrap_or([0.0; 3]);
        let q = self.net.nodes[s3].pos;
        let d = [h[0] - q[0], h[1] - q[1], h[2] - q[2]];
        if (d[0] * d[0] + d[2] * d[2] + d[1] * d[1]).sqrt() < 700.0 {
            return;
        }
        let human_truck = self.players[human].vehicle.is_some_and(|v| matches!(self.vehicles[v].kind, 2 | 3));
        while self.racers.len() < self.racers_want {
            // 0x425d40, 0x425b40 and the scene: who may race
            let picks: Vec<usize> = (0..self.players.len())
                .filter(|&k| {
                    let p = &self.players[k];
                    let Some(v) = p.vehicle else { return false };
                    let vk = self.vehicles[v].kind;
                    let fits = if human_truck { matches!(vk, 2 | 3) } else { true };
                    fits && !p.tracked && p.kind == 2 && !matches!(p.state, 0x100 | 0x101 | 5) && vk != 2 && p.flags & 4 == 0 && p.flags & 1 == 0 && !self.in_scene(p)
                })
                .collect();
            if picks.is_empty() {
                return;
            }
            let k = picks[self.rng.next() as usize % picks.len()];
            // 0x4778a0: its loads go; 0x417770: one more racer
            self.racers.push(k);
            let n = self.racers.len();
            let start = node(self, &format!("start{}_{}", self.race_track, n));
            let cp1 = node(self, "check_point1");
            self.unload(k);
            let mut p = self.players[k].clone();
            p.state = 5;
            p.flags |= 0x4000;
            p.race_index = n - 1;
            p.laps = 0;
            p.route.clear();
            if let Some(st) = start {
                p.place = Some(Spot::Node(st));
            }
            p.loads.clear();
            if let Some(cp) = cp1 {
                self.set_route(&mut p, &Spot::Node(cp));
            }
            // 0x477c50: it stands at the start until the race is over
            p.flags |= 4;
            p.until = f64::MAX;
            self.players[k] = p;
            if self.racers_want <= self.racers.len() {
                self.race_state = 2;
                self.race_start = now;
                let from = self.players[human].place;
                let to = node(self, &format!("start{}_{}", self.race_track, n));
                let mut c = match (from, to) {
                    (Some(f), Some(t)) => self.spot_route_cost(&f, t) * 0.1,
                    _ => 0.0,
                };
                if !(c <= 3660.0) {
                    c = 3660.0;
                }
                self.race_duration = ((c * 0.016666666666666666) as i64) as f64 * 60.0;
            }
        }
    }

    /// `0x487500` from a spot: the route cost to node `to` (mode 0, or mode 1 when that costs
    /// less than a tenth).
    pub fn spot_route_cost(&self, from: &Spot, to: usize) -> f64 {
        let to = Spot::Node(to);
        let mut d = self.net.route_from(from, &to, 0).map_or(0.0, |(_, c)| c);
        if let Some((_, c)) = self.net.route_from(from, &to, 1) {
            if c * 10.0 < d {
                d = c;
            }
        }
        d
    }

    /// `0x416b80`, each tick for a racer: near (100 m) a check point it goes on to the next
    /// (`check_point1` → 2 → 3 → 1; the other way round on track 2), after nine of them to
    /// `finish<track>_<n>`; a new route when the target changed (and one more passed). Its
    /// standing is minus the route cost left (less a per-lap table that is not ported).
    pub fn race_step(&mut self, k: usize) {
        let node = |w: &World, name: &str| w.net.nodes.iter().position(|n| n.name == name);
        let p = &self.players[k];
        let Some(place) = p.place else { return };
        let dest = |p: &AiPlayer| p.route.last().map(|s| s.node);
        let cost_to_dest = |w: &World, p: &AiPlayer| match dest(p) {
            Some(d) => w.net.route_from(&place, &Spot::Node(d), 0).map_or(0.0, |(_, c)| c),
            None => 0.0,
        };
        if 9 < p.laps {
            let c = cost_to_dest(self, p);
            self.players[k].race_progress = -c;
            return;
        }
        let (Some(cp1), Some(cp2), Some(cp3)) = (node(self, "check_point1"), node(self, "check_point2"), node(self, "check_point3")) else { return };
        let at = self.net.spot_coordinates(&place);
        let near = |w: &World, n: usize| {
            let q = w.net.nodes[n].pos;
            let d = [at[0] - q[0], at[1] - q[1], at[2] - q[2]];
            (d[0] * d[0] + d[2] * d[2] + d[1] * d[1]).sqrt() < 100.0
        };
        let forward = match self.race_track {
            1 => p.kind != 7,
            2 => p.kind == 7,
            _ => true,
        };
        let order = if forward { [(cp1, cp2), (cp2, cp3), (cp3, cp1)] } else { [(cp1, cp3), (cp2, cp1), (cp3, cp2)] };
        let Some(mut target) = order.iter().find(|(c, _)| near(self, *c)).map(|(_, t)| *t) else {
            let c = cost_to_dest(self, p);
            self.players[k].race_progress = -c;
            return;
        };
        if p.laps == 9 {
            if let Some(f) = node(self, &format!("finish{}_{}", self.race_track, p.race_index + 1)) {
                target = f;
            }
        }
        if dest(p) != Some(target) {
            let mut p = self.players[k].clone();
            p.route.clear();
            self.set_route(&mut p, &Spot::Node(target));
            let c = self.net.route_from(&p.place.unwrap_or(place), &Spot::Node(target), 0).map_or(0.0, |(_, c)| c);
            p.laps += 1;
            p.race_progress = -c;
            self.players[k] = p;
        } else {
            let c = cost_to_dest(self, p);
            self.players[k].race_progress = -c;
        }
    }

    /// The tasks of the game clock a new game starts (`0x434780`), at the time 0 (the period of
    /// `ObstaclesTask` is drawn: 600 + `next() % 300`). Ported of them:
    /// `updateGhosts` (every 15^(1 − 0.7·traffic) × 2.667 s); the others only take their turns.
    pub fn create_tasks(&mut self, car_share: f64) {
        let ghosts = 15f64.powf(1.0 - car_share * 0.7) * 2.66666666666;
        let obstacles = (600 + self.rng.next() % 300) as f64;
        self.tasks = [
            ("formOrders", 0.0, 10.0),
            ("updateRating", 0.0, 8.0),
            ("updatePresents", 0.0, 7.0),
            ("updateGhosts", 0.0, ghosts),
            ("updateDealers", 0.0, 27.5),
            // its period drawn with the AI generator (0x434f1f), its time from 0x68d108
            ("ObstaclesTask", 600.0, obstacles),
            ("updateAmbushes", 0.0, 1.0),
            ("provideRacesRegular", 0.0, 30.0),
            ("provideCompetitionRegular", 0.0, 30.0),
            ("updateCompanies", 0.0, 30.0),
            ("expirePlayerOrders", 0.0, 1.0),
            ("expirePresentOrders", 0.0, 1.0),
            ("destroyStraySemitrailers", 0.0, 10.0),
            ("expireParkedCars", 0.0, 19.0),
            ("updateChasedCars", 0.0, 3.0),
            ("markAmbushes", 0.0, 300.0),
            ("detectJams", 0.0, 9.2),
            ("recoverServices", 0.0, 27.1),
            ("changeLanes", 0.0, 7.3),
            ("damageCargo", 0.0, 5.1),
        ]
        .into_iter()
        .map(|(name, time, period)| Task { name, time, period })
        .collect();
    }

    /// A tick of the AI world as far as ported: the moves, the speeds, the time (`0x42d6e0`).
    pub fn tick(&mut self, now: f64, fits: &mut dyn FnMut(&std::collections::BTreeSet<u32>, Option<u32>) -> bool) -> Vec<SceneChange> {
        let mut changes = Vec::new();
        if self.first_load {
            self.first_load = false;
            self.load_counter = 200;
            changes.extend(self.load_entities(fits));
        }
        // the game clock (0x42d960): the first task in the list that is due, then it goes to the
        // end of the list
        if let Some(i) = self.tasks.iter().position(|t| t.time < now) {
            let mut t = self.tasks.remove(i);
            if t.name == "updateGhosts" && self.loads_done {
                changes.extend(self.update_ghosts(now, fits));
            }
            if t.name == "formOrders" {
                self.form_orders(now);
            }
            if t.name == "updateDealers" {
                self.update_dealers(now);
            }
            if t.name == "provideCompetitionRegular" {
                self.provide_competition();
            }
            if t.name == "markAmbushes" {
                self.mark_ambushes();
            }
            if t.name == "damageCargo" {
                self.damage_cargo();
            }
            if t.name == "provideRacesRegular" {
                self.provide_races(now);
            }
            if t.name == "updateRating" {
                self.update_rating();
            }
            if t.name == "expireParkedCars" {
                self.expire_parked_cars(now);
            }
            if t.name == "updatePresents" {
                // 0x41ed50: the objects that went missing, at least 700 m from the human player
                self.create_objects(700.0);
            }
            if t.name == "ObstaclesTask" {
                self.obstacles_task();
                // its task object (0x435690) draws its next period: 600 + next() % 300
                t.period = (600 + self.rng.next() % 300) as f64;
            }
            t.time += t.period;
            if t.time + t.period * 8.0 < now {
                t.time = now + t.period;
            }
            self.tasks.push(t);
        }
        if let Some(r) = self.debug_rng_at_moves.take() {
            self.rng = r;
        }
        self.tick_move(now);
        self.update_speeds(now);
        changes.extend(self.load_entities(fits));
        self.last_tick = now;
        changes
    }

    /// `createPlayersRegular` (`0x422f20`) for the human player and the types placed at random on
    /// the roads: `counts[type]` players of types 2..7 (`playertypes` of `GAME.CNF`). Each gets a
    /// random place (`0x422d60`; the first one drawn is not used) from which there is a route to
    /// `node_store_0000` (`0x478260`); a trucker looks for an order in its town (`0x4185d0`: up
    /// to 100 draws of the second generator, `0x418400`; at the start of a game no town has orders
    /// yet, so all are drawn) and then, like the others, drives to a random destination.
    pub fn create_players_regular(&mut self, counts: &[usize; 12], live_vehicle: Option<usize>) {
        let mut human = self.new_player(1);
        human.vehicle = live_vehicle;
        self.players.push(human);
        let store = self.net.nodes.iter().position(|n| n.name == "node_store_0000");
        for kind in [2u32, 3, 4, 5, 6, 7] {
            for _ in 0..counts[kind as usize] {
                let mut p = self.new_player(kind);
                let _ = self.net.random_start(&mut self.rng);
                loop {
                    let Some(place) = self.net.random_start(&mut self.rng) else { break };
                    p.place = Some(Spot::Road(place));
                    let Some(store) = store else { break };
                    self.set_route(&mut p, &Spot::Node(store));
                    if !p.route.is_empty() {
                        break;
                    }
                }
                if kind == 2 {
                    for _ in 0..100 {
                        self.rng2.next();
                    }
                }
                self.trace_to_random(&mut p);
                self.players.push(p);
            }
        }
    }
}

/// The numbers `GAME.CNF` of a map gives for a new game.
#[derive(Debug, Clone, Default)]
pub struct GameCnf {
    /// `playertypes`: how many players of each type (live 1, aitruck 2, police 5, mafia 6,
    /// ghost 8, dumb 9) and the human player's vehicle model and colour.
    pub counts: [usize; 12],
    pub live: Option<(String, i32)>,
    pub start: Option<[f64; 3]>,
    /// `forsale`, `forsale_stolen`, `forsale_used`, `forhire`.
    pub dealers: [usize; 3],
    pub forhire: usize,
    /// `fuel`, `semitrailers`, `missions`, `containers`, `parkings`: the objects (`0x408080`).
    pub objects: crate::objects::ObjectCounts,
}

impl GameCnf {
    pub fn read(map_dir: &std::path::Path) -> GameCnf {
        let mut c = GameCnf::default();
        let Ok(data) = std::fs::read(map_dir.join("GAME.CNF")) else { return c };
        let Ok(plain) = ht2_formats::sdte::decrypt(&data) else { return c };
        let Ok(forms) = ht2_formats::sexpr::parse(&ht2_formats::sdte::cp1251_to_string(&plain)) else { return c };
        let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
        let num = |s: Option<&Sexp>| s.and_then(|s| s.as_text()).and_then(|t| t.parse::<f64>().ok());
        for f in tops {
            let Some(l) = f.as_list() else { continue };
            let Some(head) = l.first().and_then(|s| s.as_text()) else { continue };
            match head {
                "playertypes" => {
                    for t in &l[1..] {
                        let Some(e) = t.as_list() else { continue };
                        let n = num(e.get(1)).unwrap_or(0.0) as usize;
                        let k = match e.first().and_then(|s| s.as_text()) {
                            Some("live") => {
                                c.live = Some((e.get(2).and_then(|s| s.as_text()).unwrap_or_default().to_string(), num(e.get(3)).unwrap_or(0.0) as i32));
                                1
                            }
                            Some("aitruck") => 2,
                            Some("police") => 5,
                            Some("mafia") => 6,
                            Some("ghost") => 8,
                            Some("dumb") => 9,
                            _ => continue,
                        };
                        c.counts[k] = n;
                    }
                }
                "startposition" => c.start = Some([num(l.get(1)).unwrap_or(0.0), num(l.get(2)).unwrap_or(0.0), num(l.get(3)).unwrap_or(0.0)]),
                "forsale" => c.dealers[0] = num(l.get(1)).unwrap_or(0.0) as usize,
                "forsale_stolen" => c.dealers[1] = num(l.get(1)).unwrap_or(0.0) as usize,
                "forsale_used" => c.dealers[2] = num(l.get(1)).unwrap_or(0.0) as usize,
                "forhire" => c.forhire = num(l.get(1)).unwrap_or(0.0) as usize,
                "fuel" => c.objects.fuel = num(l.get(1)).unwrap_or(0.0) as usize,
                "semitrailers" => c.objects.semitrailers = num(l.get(1)).unwrap_or(0.0) as usize,
                "missions" => c.objects.missions = num(l.get(1)).unwrap_or(0.0) as usize,
                "containers" => c.objects.containers = num(l.get(1)).unwrap_or(0.0) as usize,
                "parkings" => c.objects.parkings = num(l.get(1)).unwrap_or(0.0) as usize,
                _ => {}
            }
        }
        c
    }
}

impl World {
    /// The start of a new game on a map as far as ported (`0x434780`): the markets, the vehicle
    /// lists, the regular players, the vehicles for sale and those hired out, the objects
    /// (`0x41d340`). `car_share` is the traffic setting (1 for 100 %). The human player is put
    /// at `human` (the game's place of its vehicle). After the objects come the road works
    /// (`0x41cff0`) and a timer drawn with the second generator; `0x42cb20` (the players'
    /// rooms in the game's scene) and the counts of the parking places draw nothing.
    pub fn new_game(game_dir: &std::path::Path, map_dir: &std::path::Path, net: std::rc::Rc<RoadNet>, car_share: f64, human: Option<Spot>) -> World {
        let scripts = crate::road::read_map_scripts(map_dir);
        let lots = Parking::from_scripts(&scripts);
        let vehicles = Vehicle::read(game_dir);
        let names = DriverName::read(game_dir);
        let cnf = GameCnf::read(map_dir);
        // 0x414670: the cost of the way between every two towns (its route searches fill the
        // route tables the later searches follow)
        let towns = crate::road::Town::from_scripts(&scripts);
        let town_dist = net.town_distances(&towns);
        let mut rng = AiRng::default();
        let _offers = market_offers(&lots, &vehicles, &mut rng);
        let lists = sort_vehicles(&vehicles, &mut rng, car_share);
        let live = cnf.live.as_ref().and_then(|(m, c)| vehicles.iter().position(|v| &v.model == m && v.colour == *c));
        let mut w = World::new(net, lots, vehicles, lists, names, rng, 0);
        w.tech = std::fs::read(game_dir.join("vehicle.tech")).ok().and_then(|d| ht2_formats::tech::parse(&d).ok()).unwrap_or_default();
        let mut counts = cnf.counts;
        counts[1] = 0;
        counts[8] = 0;
        counts[9] = 0;
        w.create_players_regular(&counts, live);
        w.players[0].place = human;
        w.create_dealers(cnf.dealers);
        // 0x423c00
        w.stand_free = false;
        w.hired_want = cnf.forhire;
        w.hire_out(cnf.forhire);
        // 0x41d340: the objects
        w.towns = towns;
        w.town_dist = town_dist;
        w.merchandise = crate::objects::Merchandise::read(game_dir);
        let liquid: Vec<String> = w.merchandise.iter().filter(|m| m.flags & 4 != 0).map(|m| m.name.clone()).collect();
        w.mark_liquid_towns(&liquid);
        w.presents = crate::objects::Present::read(game_dir);
        w.object_want = cnf.objects.clone();
        if let Some(st) = cnf.start {
            w.human_pos = Some(st);
        }
        w.create_objects(0.0);
        // in the normal game (0x68d188 0 or 1): the road works of the map
        w.road_works("pos_work_dr_33");
        w.road_works("pos_work_dr_37");
        // 0x66c6e8: 600 to 1199 s, drawn with the second generator
        w.obstacle_time = (600 + w.rng2.next() % 600) as f64;
        w.ghost_want = cnf.counts[8];
        w.car_share = car_share;
        w.ambushes = Ambush::from_scripts(&scripts);
        w.obstacles = Obstacle::from_scripts(&scripts, &w.net);
        w.create_tasks(car_share);
        w
    }
}
