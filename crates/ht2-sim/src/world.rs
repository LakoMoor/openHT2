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

/// A driver name of `names.sch`: `( "NAME" gender role number ... )`.
#[derive(Debug, Clone)]
pub struct DriverName {
    pub name: String,
    /// `+0x18`: 1 trucker, 2 mafia, 3 police.
    pub role: u32,
}

impl DriverName {
    pub fn parse(forms: &[Sexp]) -> Vec<DriverName> {
        let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
        tops.into_iter()
            .filter_map(|f| f.as_list())
            .filter(|l| matches!(l.first(), Some(Sexp::Str(_))))
            .map(|l| DriverName {
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

    pub fn step(&mut self, x: f64) -> f64 {
        let y2 = self.y2;
        self.y2 = self.y1;
        self.y1 = x * self.a + y2 * self.b + self.y1 * self.c;
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
}

/// What the AI world asks the game to do with its scene (`xLoadEntities`, `0x424550`): the
/// scene is loaded room by room (`+0x4c` of a player is the room it is in); the vehicles of the
/// players in a loaded room are driven in the scene, the others only on the road network.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneChange {
    /// Load the room (`0x4d8b20`).
    Load(String),
    /// Unload it (`0x4d8ae0`).
    Unload(String),
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
    /// The rooms the game has loaded (`0x4d8a40`).
    pub loaded_rooms: std::collections::BTreeSet<String>,
    /// `updateGhosts` (`0x420ed0`): how many ghosts (type 8) to keep around the human player,
    /// how often the task runs (15^(1 − 0.7·traffic) × 2.667 s, `0x434e0e`) and when next; it
    /// runs only after `xLoadEntities` ran once (`0x68d130`).
    pub ghost_want: usize,
    pub ghost_period: f64,
    pub ghost_next: f64,
    pub loads_done: bool,
}

impl World {
    pub fn new(net: std::rc::Rc<RoadNet>, lots: Vec<Parking>, vehicles: Vec<Vehicle>, lists: Categories, names: Vec<DriverName>, rng: AiRng, time: u32) -> Self {
        let slots: Vec<Vec<Option<usize>>> = lots.iter().map(|l| vec![None; l.nodes.len()]).collect();
        let lot_used = vec![0; lots.len()];
        let name_use = vec![[0, 0]; names.len()];
        World { net, lots, slots, lot_used, vehicles, lists, names, name_use, cursor: [0; 12], rng, rng2: AiRng::new(time ^ 161_803_398), players: Vec::new(), last_tick: time as f64, travel_speed: 20.0, load_counter: 0, load_distance: 0.0, loaded_rooms: Default::default(), ghost_want: 0, ghost_period: 6.0, ghost_next: 0.0, loads_done: false }
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
        AiPlayer { kind, vehicle, trailer, name, place: None, route: Vec::new(), parked: None, state: 0, last_pos: [0.0; 3], speed: 20.0, speed2: 0.0, velocity: [0.0; 3], filters: [SpeedFilter::new(1e-4), SpeedFilter::new(1e-3)], driven: false, born: 0.0 }
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

    /// `0x478260` / `0x4872f0`: the route (mode 0) from the player's place to `to`; when there is
    /// one, the player's place is set again from its coordinates on it (`0x43dc60`).
    fn set_route(&self, p: &mut AiPlayer, to: &Spot) {
        let Some(from) = p.place else { return };
        if let Some((r, _)) = self.net.route_from(&from, to, 0) {
            p.route = r;
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
        for (kind, &want) in wants.iter().enumerate() {
            for _ in 0..want.min(100) {
                let Some((lot, slot)) = self.parking_place(kind as u32) else { break };
                let mut p = self.new_player(9);
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
            let mut p = self.players[k].clone();
            // 0x4786a0: the route given up; goHireIt
            p.route.clear();
            p.state = 0x101;
            have += 1;
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
            // 0x43e610: put at the end of the route
            if let (Some(last), Some(place)) = (p.route.last(), p.place) {
                let q = self.net.nodes[last.node].pos;
                p.place = Some(self.net.locate(&p.route, q, &place));
            }
            self.players[k] = p;
        }
    }

    /// The move of the players in a tick while their rooms are not loaded (`processMove`,
    /// `0x41ae60`; `prescribeAdvance`, `0x41a9d0`; `advancePlayer`, `0x435a90`): 20 m/s (police
    /// 1.3 times that) times the time since the last tick along the route. The human player, the
    /// vehicles standing for sale and those hired out do not move.
    pub fn tick_move(&mut self, now: f64) {
        let dt = now - self.last_tick;
        for p in &mut self.players {
            if p.kind == 1 || p.state == 0x100 || p.state == 0x101 || p.driven {
                continue;
            }
            let Some(Spot::Road(mut place)) = p.place else { continue };
            // 0x41a9d0: the speed of the human player, at least 50 km/h
            let mut v = 13.88888888888889;
            if v < self.travel_speed {
                v = self.travel_speed;
            }
            if p.kind == 5 {
                v *= 1.3;
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
        for p in &mut self.players {
            let Some(place) = p.place else { continue };
            let q = self.net.spot_coordinates(&place);
            let d = [q[0] - p.last_pos[0], q[1] - p.last_pos[1], q[2] - p.last_pos[2]];
            p.last_pos = q;
            let raw = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt() / dt;
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
            let l = (d[1] * d[1] + d[2] * d[2] + d[0] * d[0]).sqrt();
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

    /// The room a player is in (`+0x4c`): that of the road segment under it.
    pub fn room_of(&self, p: &AiPlayer) -> Option<String> {
        match p.place? {
            Spot::Road(q) => Some(self.net.room_of(&q).to_string()),
            Spot::Node(n) => {
                let node = &self.net.nodes[n];
                let road = node.out.first().or(node.inc.first())?;
                Some(self.net.segment_rooms[self.net.roads[*road].segments[0]].clone())
            }
        }
    }

    /// `xLoadEntities` (`0x424550`, every ninth tick): which rooms the game should have loaded.
    /// Each player gets the closeness of where it is and will be in a quarter of a second
    /// (moving players: their route ahead at the speed of the second filter) to the human
    /// player and where he will be (×1.2 when its room is loaded, ×0.7 for type 8); in that
    /// order, highest first (the STL sort), rooms not loaded are loaded when the player is
    /// farther than the load distance, as long as `fits` says there is room for one more, else
    /// rooms of the least wanted players beyond that distance are unloaded first; rooms of
    /// players not wanted at all (1e-8) are unloaded. `fits` stands in for the game's memory
    /// budget (`0x4d8a90`).
    pub fn load_entities(&mut self, fits: &mut dyn FnMut(&std::collections::BTreeSet<String>) -> bool) -> Vec<SceneChange> {
        self.load_counter += 1;
        if self.load_counter < 9 {
            if self.load_counter != 0 {
                return Vec::new();
            }
        } else {
            self.load_counter = 0;
        }
        let Some(human) = self.players.iter().position(|p| p.kind == 1) else { return Vec::new() };
        let Some(hplace) = self.players[human].place else { return Vec::new() };
        let hpos = self.net.spot_coordinates(&hplace);
        let hv = self.players[human].velocity;
        let hsoon = [hpos[0] + hv[0] * 0.25, hpos[1] + hv[1] * 0.25, hpos[2] + hv[2] * 0.25];
        // (room, closeness, distance)
        let mut list: Vec<(String, f64, f64)> = Vec::new();
        for p in &self.players {
            let (Some(place), Some(room)) = (p.place, self.room_of(p)) else { continue };
            let pos = self.net.spot_coordinates(&place);
            let now = [pos[0] - hpos[0], pos[1] - hpos[1], pos[2] - hpos[2]];
            let soon = match place {
                Spot::Road(mut q) if p.kind != 1 && p.state != 0x100 && p.state != 0x101 => {
                    let mut d = p.speed2 * 0.25;
                    self.net.advance_route(&mut q, &mut d, &p.route);
                    let c = self.net.coordinates(&q);
                    [c[0] - hsoon[0], c[1] - hsoon[1], c[2] - hsoon[2]]
                }
                _ => {
                    let v = p.velocity;
                    let c = [pos[0] + v[0] * 0.25, pos[1] + v[1] * 0.25, pos[2] + v[2] * 0.25];
                    [c[0] - hsoon[0], c[1] - hsoon[1], c[2] - hsoon[2]]
                }
            };
            let mut want = Self::closeness(now, soon);
            let dist = (now[2] * now[2] + now[1] * now[1] + now[0] * now[0]).sqrt();
            if self.loaded_rooms.contains(&room) {
                want *= 1.2;
            }
            if p.kind == 8 {
                want *= 0.7;
            }
            list.push((room, want, dist));
        }
        crate::stl::sort(&mut list, |a, b| a.1 > b.1);
        let mut changes = Vec::new();
        // the rooms of players not wanted at all: out
        while let Some((room, want, _)) = list.last() {
            if 1e-8 < *want {
                break;
            }
            if self.loaded_rooms.remove(room) {
                changes.push(SceneChange::Unload(room.clone()));
            }
            list.pop();
        }
        let mut back = list.len();
        let mut i = 0;
        'next: while i < list.len() && i < back {
            let (room, _, dist) = list[i].clone();
            if !self.loaded_rooms.contains(&room) && self.load_distance < dist {
                while !fits(&self.loaded_rooms) {
                    // make room: unload the room of the least wanted player beyond the load
                    // distance; when there is none behind this one, stop
                    loop {
                        if back <= i + 1 {
                            break 'next;
                        }
                        back -= 1;
                        let (r, _, d) = &list[back];
                        if self.load_distance < *d && self.loaded_rooms.remove(r) {
                            changes.push(SceneChange::Unload(r.clone()));
                            break;
                        }
                    }
                }
                self.loaded_rooms.insert(room.clone());
                changes.push(SceneChange::Load(room));
            }
            i += 1;
        }
        self.load_distance = 150.0;
        self.loads_done = true;
        changes
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

    /// `updateGhosts` (`0x420ed0`, a task of the game clock): the ghosts (type 8) are the traffic
    /// around the human player. Those out of the scene, or more than 330 m away in it, go when
    /// older than 72 s or farther than 700 m. While there are fewer than wanted, one may come:
    /// (half the time while the scene has memory to spare) up to eight times a direction is
    /// drawn, the road place nearest to the point 320 m that way (stand-in for `0x4365d0`), and
    /// if that is at least 160 m away, on a road without flags 0x10 and 0x40, a ghost is put
    /// there with the chance √(road width / 14) (half that for `attr2` 2, a quarter for 3), and
    /// sent to a random destination.
    pub fn update_ghosts(&mut self, now: f64) {
        let Some(human) = self.players.iter().position(|p| p.kind == 1) else { return };
        let Some(hplace) = self.players[human].place else { return };
        let h = self.net.spot_coordinates(&hplace);
        let dist = |q: [f64; 3]| ((q[0] - h[0]) * (q[0] - h[0]) + (q[1] - h[1]) * (q[1] - h[1]) + (q[2] - h[2]) * (q[2] - h[2])).sqrt();
        for k in 0..self.players.len() {
            let p = &self.players[k];
            if p.kind != 8 {
                continue;
            }
            let Some(place) = p.place else { continue };
            let d = dist(self.net.spot_coordinates(&place));
            let loaded = self.room_of(p).is_some_and(|r| self.loaded_rooms.contains(&r));
            let keep = (loaded && d <= 330.0) || (now - p.born <= 72.0 && d <= 700.0);
            if !keep {
                // 0x4466a0: the player is deleted
                let p = &mut self.players[k];
                p.kind = 0;
                p.place = None;
                p.route.clear();
            }
        }
        let ghosts = self.players.iter().filter(|p| p.kind == 8).count();
        if self.ghost_want <= ghosts {
            return;
        }
        // 0x4d8a90 with no room: the scene's memory is not used up (stand-in: always)
        if self.rng.next() & 1 == 0 {
            return;
        }
        for _ in 0..8 {
            let a = (self.rng.next() % 0x9965e) as f64 * 1e-5;
            let x = a.cos() * 160.0;
            let y = a.sin() * 160.0;
            let q = [x + x + h[0], y + y + h[1], h[2] + 0.0];
            let Some(place) = self.net.nearest(q) else { continue };
            let seg = &self.net.segments[self.net.roads[place.road].segments[place.segment]];
            if seg.flags & 0x50 != 0 {
                continue;
            }
            if dist(self.net.coordinates(&place)) < 160.0 {
                continue;
            }
            let mut chance = (seg.width_at(place.t, place.piece) * 0.07142857142857142).sqrt();
            if seg.attr2 == 2 {
                chance *= 0.5;
            } else if seg.attr2 == 3 {
                chance *= 0.25;
            }
            if chance * 1000.0 <= (self.rng.next() % 1000) as f64 {
                return;
            }
            // 0x420cb0
            let mut p = self.new_player(8);
            p.place = Some(Spot::Road(place));
            p.born = now;
            p.last_pos = self.net.coordinates(&place);
            self.trace_without_turns(&mut p);
            self.players.push(p);
            return;
        }
    }

    /// A tick of the AI world as far as ported: the moves, the speeds, the time (`0x42d6e0`).
    pub fn tick(&mut self, now: f64) {
        // the game clock's tasks (0x42d960)
        if self.loads_done && self.ghost_next < now {
            self.ghost_next = now + self.ghost_period;
            self.update_ghosts(now);
        }
        self.tick_move(now);
        self.update_speeds(now);
        self.last_tick = now;
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
                _ => {}
            }
        }
        c
    }
}

impl World {
    /// The start of a new game on a map as far as ported (`0x434780`): the markets, the vehicle
    /// lists, the regular players, the vehicles for sale and those hired out. `car_share` is the
    /// traffic setting (1 for 100 %). The human player is put at `human` (the game's place of
    /// its vehicle). Not yet ported: the hidden objects and trailers with orders (`0x41d340`),
    /// so the random numbers run apart from the original's after this.
    pub fn new_game(game_dir: &std::path::Path, map_dir: &std::path::Path, net: std::rc::Rc<RoadNet>, car_share: f64, human: Option<Spot>) -> World {
        let scripts = crate::road::read_map_scripts(map_dir);
        let lots = Parking::from_scripts(&scripts);
        let vehicles = Vehicle::read(game_dir);
        let names = DriverName::read(game_dir);
        let cnf = GameCnf::read(map_dir);
        let mut rng = AiRng::default();
        let _offers = market_offers(&lots, &vehicles, &mut rng);
        let lists = sort_vehicles(&vehicles, &mut rng, car_share);
        let live = cnf.live.as_ref().and_then(|(m, c)| vehicles.iter().position(|v| &v.model == m && v.colour == *c));
        let mut w = World::new(net, lots, vehicles, lists, names, rng, 0);
        let mut counts = cnf.counts;
        counts[1] = 0;
        counts[8] = 0;
        counts[9] = 0;
        w.create_players_regular(&counts, live);
        w.players[0].place = human;
        w.create_dealers(cnf.dealers);
        w.hire_out(cnf.forhire);
        w.ghost_want = cnf.counts[8];
        w.ghost_period = 15f64.powf(1.0 - car_share * 0.7) * 2.66666666666;
        w
    }
}
