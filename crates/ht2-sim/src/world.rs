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
    pub filters: [SpeedFilter; 2],
}

/// What the start of a new game keeps between its steps.
pub struct World<'a> {
    pub net: &'a RoadNet,
    pub lots: &'a [Parking],
    /// Per parking place and slot, the player standing there (`slot + 8`), and how many do
    /// (`+0x28`).
    pub slots: Vec<Vec<Option<usize>>>,
    pub lot_used: Vec<usize>,
    pub vehicles: &'a [Vehicle],
    pub lists: &'a Categories,
    pub names: &'a [DriverName],
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
}

impl<'a> World<'a> {
    pub fn new(net: &'a RoadNet, lots: &'a [Parking], vehicles: &'a [Vehicle], lists: &'a Categories, names: &'a [DriverName], rng: AiRng, time: u32) -> Self {
        let slots = lots.iter().map(|l| vec![None; l.nodes.len()]).collect();
        World { net, lots, slots, lot_used: vec![0; lots.len()], vehicles, lists, names, name_use: vec![[0, 0]; names.len()], cursor: [0; 12], rng, rng2: AiRng::new(time ^ 161_803_398), players: Vec::new(), last_tick: time as f64, travel_speed: 20.0 }
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
        AiPlayer { kind, vehicle, trailer, name, place: None, route: Vec::new(), parked: None, state: 0, last_pos: [0.0; 3], speed: 20.0, filters: [SpeedFilter::new(1e-4), SpeedFilter::new(1e-3)] }
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
            if p.kind == 1 || p.state == 0x100 || p.state == 0x101 {
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
                p.filters[1].step(raw);
            }
            if p.kind == 1 {
                self.travel_speed = p.speed;
            }
        }
    }

    /// A tick of the AI world as far as ported: the moves, the speeds, the time (`0x42d6e0`).
    pub fn tick(&mut self, now: f64) {
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
