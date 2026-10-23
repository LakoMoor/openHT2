//! The objects of the AI world a new game puts on the map (`0x41d340`, list `0x68cf6c`): fuel
//! barrels at hidden places, semitrailers at bases, semitrailers with orders on the roads,
//! containers at hidden places and parked cars. Each is put at a free node whose name starts
//! with the phase's prefix, and the node is marked (flag 0x10).

use crate::road::Place;
use crate::world::World;
use ht2_formats::sexpr::Sexp;

/// What can be put at a hidden place or a parking (`etrashboxes.sch`, list `0x68d0b8`):
/// `( vehicle ID "model" colour kind )`, kind `parked` 1, `fuel` 2, `container` 3,
/// `roadwork` 4.
#[derive(Debug, Clone)]
pub struct Present {
    pub id: String,
    pub model: String,
    pub colour: i32,
    /// `+0x1c`.
    pub kind: u32,
    /// `+0x28`: how many have been put.
    pub count: u32,
}

impl Present {
    pub fn read(game_dir: &std::path::Path) -> Vec<Present> {
        let Ok(data) = std::fs::read(game_dir.join("etrashboxes.sch")) else { return Vec::new() };
        let Ok(plain) = ht2_formats::sdte::decrypt(&data) else { return Vec::new() };
        let Ok(forms) = ht2_formats::sexpr::parse(&ht2_formats::sdte::cp1251_to_string(&plain)) else { return Vec::new() };
        let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
        tops.into_iter()
            .filter_map(|f| f.form("vehicle"))
            .map(|r| {
                let text = |k: usize| r.get(k).and_then(|s| s.as_text()).unwrap_or_default();
                Present {
                    id: text(0).to_string(),
                    model: text(1).to_string(),
                    colour: text(2).parse().unwrap_or(0),
                    kind: match text(3) {
                        "parked" => 1,
                        "fuel" => 2,
                        "container" => 3,
                        "roadwork" => 4,
                        _ => 0,
                    },
                    count: 0,
                }
            })
            .collect()
    }
}

/// A merchandise of `merchandise.sch` (`0x68ccb8`, 0x68 bytes; its number is its place in the
/// file): `( NAME pack n n ( price price share ( flags ) ) ... )`.
#[derive(Debug, Clone)]
pub struct Merchandise {
    pub name: String,
    /// `+0x30` (`0x40b4a0`): `illegal` 1, `formafia` 2, `fuel` 4 (liquid), `cars` 8,
    /// `trucks` 0x10.
    pub flags: u32,
    /// `+0x10`, `+0x18`: the weight and the load area of a unit (the two numbers after the pack).
    pub unit_weight: f64,
    pub unit_area: f64,
    /// `+0x20`, `+0x28`: the share and the value of the price list (its third and second number).
    pub share: f64,
    pub value: f64,
}

impl Merchandise {
    pub fn read(game_dir: &std::path::Path) -> Vec<Merchandise> {
        let Ok(data) = std::fs::read(game_dir.join("merchandise.sch")) else { return Vec::new() };
        let Ok(plain) = ht2_formats::sdte::decrypt(&data) else { return Vec::new() };
        let Ok(forms) = ht2_formats::sexpr::parse(&ht2_formats::sdte::cp1251_to_string(&plain)) else { return Vec::new() };
        let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
        let mut out = Vec::new();
        for t in tops {
            let Some(l) = t.as_list() else { continue };
            let Some(name) = l.first().and_then(|s| s.as_text()) else { continue };
            let words = l.iter().filter_map(|s| s.as_list()).flat_map(|s| s.iter().filter_map(|x| x.as_list())).flat_map(|x| x.iter().filter_map(|w| w.as_text()));
            let flags = words.fold(0, |f, w| {
                f | match w {
                    "illegal" => 1,
                    "formafia" => 2,
                    "fuel" => 4,
                    "cars" => 8,
                    "trucks" => 0x10,
                    _ => 0,
                }
            });
            let num = |s: Option<&Sexp>| s.and_then(|s| s.as_text()).and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0);
            let prices = l.get(4).and_then(|s| s.as_list()).unwrap_or(&[]);
            out.push(Merchandise { name: name.to_string(), flags, unit_weight: num(l.get(2)), unit_area: num(l.get(3)), share: num(prices.get(2)), value: num(prices.get(1)) });
        }
        out
    }
}

/// The merchandise that is liquid (flag `fuel`).
pub fn liquid_goods(game_dir: &std::path::Path) -> Vec<String> {
    Merchandise::read(game_dir).into_iter().filter(|m| m.flags & 4 != 0).map(|m| m.name).collect()
}

/// An order (`0x470d80`, 0x90 bytes): carry `merchandise` from `from` to the unloading node of
/// town `to` by `deadline`.
#[derive(Debug, Clone)]
pub struct Order {
    /// `+0`, `+0xc`.
    pub merchandise: usize,
    pub to: usize,
    /// `+0x18`: where the load is.
    pub from: Place,
    /// `+0x38`: how much, `+0x40` the pay; `+0x48` the deadline (900 s after `+0x50`, when it
    /// was made); `+0x58` the fee for the distance (0.13 per unit of route cost); `+0x68` its
    /// kind (2 here); `+0x70` the route cost to the destination (`0x487500`).
    pub amount: f64,
    pub pay: f64,
    pub deadline: f64,
    pub made: f64,
    pub fee: f64,
    pub kind: u32,
    pub distance: f64,
}

/// An object of the AI world (`0x70` bytes).
#[derive(Debug, Clone)]
pub struct WorldObject {
    /// `+8`: 0x10 at a hidden place (fuel), 0x20 a semitrailer at a base, 0x60 on a road, 0x40
    /// a container at a hidden place.
    pub flags: u32,
    /// `+0x18`: the node it stands at.
    pub node: usize,
    /// `+0x20`: where, and `+0x38` the place on the roads there (`0x436900`).
    pub pos: [f64; 3],
    pub place: Option<Place>,
    /// `+0x14` → `+4`: the semitrailer (catalogue number).
    pub vehicle: Option<usize>,
    /// `+0x6c`: what it is (`etrashboxes.sch`).
    pub present: Option<usize>,
    /// `+0x14` → `+0x1c`: the orders of a semitrailer on the road.
    pub orders: Vec<Order>,
    /// `+0x58`: what a container holds (`0x20` bytes): the merchandise, the town that takes it,
    /// the amount (`+0x10`) and the fee for the distance there (`+0x18`, 0.1 per unit of route
    /// cost).
    pub cargo: Option<Cargo>,
    /// `+0x60`: when a parked car was put.
    pub made: f64,
}

#[derive(Debug, Clone)]
pub struct Cargo {
    pub merchandise: usize,
    pub to: Option<usize>,
    pub amount: f64,
    pub fee: f64,
}

impl World {
    /// `0x45dbb0`: a free node (no flag 0x10) whose name starts with `prefix`, at least `min`
    /// from the human player, drawn with the AI generator; `None` (nothing drawn) when there is
    /// none.
    fn pick_node(&mut self, prefix: &str, min: f64) -> Option<usize> {
        let h = self.players.iter().find(|p| p.kind == 1).and_then(|p| self.position(p)).unwrap_or([0.0; 3]);
        let picks: Vec<usize> = (0..self.net.nodes.len())
            .filter(|&k| {
                let n = &self.net.nodes[k];
                if n.flags & 0x10 != 0 || self.taken[k] || !n.name.starts_with(prefix) {
                    return false;
                }
                let d = [n.pos[0] - h[0], n.pos[1] - h[1], n.pos[2] - h[2]];
                min <= (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
            })
            .collect();
        if picks.is_empty() {
            return None;
        }
        Some(picks[self.rng.next() as usize % picks.len()])
    }

    /// The weighted choice of `0x41d340`: among the items of positive weight, one; with more than
    /// one, `next() % 1000000` · 10⁻⁶ of the total is drawn and the item whose share holds it is
    /// taken (none when it falls outside, which the original reports as an error).
    fn weighted(&mut self, weights: impl Iterator<Item = (usize, f64)>) -> Option<usize> {
        let mut items = Vec::new();
        let mut total = 0.0;
        for (i, w) in weights {
            if 0.0 < w {
                total = w + total;
                items.push((i, w));
            }
        }
        match items.len() {
            0 => None,
            1 => Some(items[0].0),
            _ => {
                let r = (self.rng.next() % 1_000_000) as f64 * total * 1e-6;
                let mut acc = 0.0;
                for (i, w) in items {
                    if acc <= r && r <= acc + w {
                        return Some(i);
                    }
                    acc += w;
                }
                None
            }
        }
    }

    /// A new object at node `node`, which is marked (flag 0x10, kept in `taken`).
    fn new_object(&mut self, flags: u32, node: usize) -> WorldObject {
        let pos = self.net.nodes[node].pos;
        self.taken[node] = true;
        WorldObject { flags, node, pos, place: self.net.nearest(pos), vehicle: None, present: None, orders: Vec::new(), cargo: None, made: 0.0 }
    }

    /// `0x405030`: the town nearest to `q` that is not a filling station (flag 2), by its node.
    fn nearest_town(&self, q: [f64; 3]) -> Option<usize> {
        let mut best = f64::MAX;
        let mut found = None;
        for (k, t) in self.towns.iter().enumerate() {
            if t.flags & 2 != 0 {
                continue;
            }
            let Some(n) = self.net.town_node(t) else { continue };
            let p = self.net.nodes[n].pos;
            let d = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
            let l = ((d[1] * d[1] + d[2] * d[2]) + d[0] * d[0]).sqrt();
            if l < best {
                best = l;
                found = Some(k);
            }
        }
        found
    }

    /// `0x41eb80`: how much semitrailer `v` is wanted at node `node`: its weight (`+0x70`, ×12
    /// for `STrailerStorm`), twice that when the human player's vehicle can pull it, and ×32
    /// when it carries liquids exactly where the nearest town deals in them (else 0).
    fn semitrailer_weight(&self, v: usize, node: Option<usize>) -> f64 {
        let veh = &self.vehicles[v];
        if veh.kind != 4 {
            return 0.0;
        }
        let mut w = if veh.model == "STrailerStorm" { 12.0 } else { 1.0 } * self.vehicle_weight[v];
        if let Some(h) = self.players.iter().find(|p| p.kind == 1).and_then(|p| p.vehicle) {
            if self.vehicles[h].trailers.iter().any(|t| *t == veh.id) {
                w = w + w;
            }
        }
        let Some(node) = node else { return w };
        let town = self.nearest_town(self.net.nodes[node].pos);
        let liquid_town = town.is_some_and(|t| self.towns[t].flags & 1 != 0);
        if liquid_town == veh.liquid {
            w * 32.0
        } else {
            0.0
        }
    }

    /// `0x404f80`: the town nearest to `q`; `0x4051b0` with `but`: the one nearest to that town
    /// other than it.
    fn nearest_town_any(&self, q: [f64; 3], but: Option<usize>) -> Option<usize> {
        let mut best = f64::MAX;
        let mut found = None;
        for (k, t) in self.towns.iter().enumerate() {
            if Some(k) == but {
                continue;
            }
            let Some(n) = self.net.town_node(t) else { continue };
            let p = self.net.nodes[n].pos;
            let d = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
            let l = ((d[1] * d[1] + d[2] * d[2]) + d[0] * d[0]).sqrt();
            if l < best {
                best = l;
                found = Some(k);
            }
        }
        found
    }

    /// `0x456a10`: a merchandise of the town (its list, with repeats) that is legal and whose
    /// flags under `mask` are `value`, weighted 1, ×0.25 for each order slot of the town's nodes
    /// that holds it.
    fn town_merchandise(&mut self, town: usize, value: u32, mask: u32) -> Option<usize> {
        // names are found whatever their case (`c_TABLEWARE` in TOWN_AD)
        let goods: Vec<usize> = self.towns[town].goods.iter().filter_map(|g| self.merchandise.iter().position(|m| m.name.eq_ignore_ascii_case(g))).collect();
        let weights: Vec<(usize, f64)> = goods
            .into_iter()
            .map(|m| {
                let f = self.merchandise[m].flags;
                let mut w = 0.0;
                if f & 1 == 0 && f & mask == value {
                    w = 1.0;
                    for slots in self.town_slots.get(town).into_iter().flatten() {
                        for o in slots.iter().flatten() {
                            if o.merchandise == m {
                                w *= 0.25;
                            }
                        }
                    }
                }
                (m, w)
            })
            .collect();
        self.weighted(weights.into_iter())
    }

    /// `0x4566e0`: any legal merchandise whose liquid flag is `liquid`, drawn evenly.
    fn any_merchandise(&mut self, liquid: bool) -> Option<usize> {
        let picks: Vec<usize> = (0..self.merchandise.len()).filter(|&m| self.merchandise[m].flags & 1 == 0 && (self.merchandise[m].flags & 4 != 0) == liquid).collect();
        if picks.is_empty() {
            return None;
        }
        Some(picks[self.rng.next() as usize % picks.len()])
    }

    /// `0x404cf0`: whether town `t` takes merchandise `m`: at its first unloading node, liquids
    /// when that is a fuel node, else the others.
    fn takes(&self, t: usize, m: usize) -> bool {
        let Some(&(_, f)) = self.towns[t].nodes.iter().find(|(_, f)| f & 1 != 0) else { return false };
        (self.merchandise[m].flags & 4 != 0) == (f & 4 != 0)
    }

    /// `0x487500`: the route cost from `from` to node `to` (mode 0, or mode 1 when that costs
    /// less than a tenth).
    fn route_cost(&self, from: &Place, to: usize) -> f64 {
        let from = crate::road::Spot::Road(*from);
        let to = crate::road::Spot::Node(to);
        let mut d = self.net.route_from(&from, &to, 0).map_or(0.0, |(_, c)| c);
        if let Some((_, c)) = self.net.route_from(&from, &to, 1) {
            if c * 10.0 < d {
                d = c;
            }
        }
        d
    }

    /// `0x434070` (new game): towns that deal in a liquid merchandise get flag 1.
    pub fn mark_liquid_towns(&mut self, liquid: &[String]) {
        for t in &mut self.towns {
            if t.goods.iter().any(|g| liquid.iter().any(|l| l.eq_ignore_ascii_case(g))) {
                t.flags |= 1;
            }
        }
    }

    /// `0x41d340`: the objects of a new game (`min` 0) or the ones missing later
    /// (`updatePresents`, `min` 700): fuel barrels at hidden places (`fuel` of `GAME.CNF`),
    /// semitrailers at bases (`semitrailers`), semitrailers with an order on the roads
    /// (`missions`), containers at hidden places (`containers`) and parked cars (`parkings`),
    /// each at least `min` from the human player.
    pub fn create_objects(&mut self, min: f64) {
        // fuel: at "pos_hidden" nodes, the present of kind 2 that was put least (0.75^count)
        for _ in 0..1000 {
            if self.object_want.fuel <= self.object_made.fuel {
                break;
            }
            let Some(node) = self.pick_node("pos_hidden", min) else { break };
            let mut o = self.new_object(0x10, node);
            self.object_made.fuel += 1;
            let weights: Vec<(usize, f64)> = self.presents.iter().enumerate().map(|(i, p)| (i, if p.kind == 2 { 0.75f64.powf(p.count as f64) } else { 0.0 })).collect();
            if let Some(i) = self.weighted(weights.into_iter()) {
                o.present = Some(i);
                self.presents[i].count += 1;
            }
            self.objects.push(o);
        }
        // semitrailers at "pos_base" nodes
        for _ in 0..1000 {
            if self.object_want.semitrailers <= self.object_made.semitrailers {
                break;
            }
            let Some(node) = self.pick_node("pos_base", min) else { break };
            let weights: Vec<(usize, f64)> = (0..self.vehicles.len()).map(|v| (v, self.semitrailer_weight(v, Some(node)))).collect();
            let Some(v) = self.weighted(weights.into_iter()) else { break };
            self.vehicle_weight[v] *= 0.75;
            let mut o = self.new_object(0x20, node);
            self.object_made.semitrailers += 1;
            o.vehicle = Some(v);
            self.objects.push(o);
        }
        // semitrailers with an order at "pos_road" nodes (missions): a merchandise of the nearest
        // town (else of the town nearest to it, else any; liquids for tanks) for a town that
        // takes it
        for _ in 0..1000 {
            if self.object_want.missions <= self.object_made.missions {
                break;
            }
            let Some(node) = self.pick_node("pos_road", min) else { break };
            // weighed without a node (the town does not count here)
            let weights: Vec<(usize, f64)> = (0..self.vehicles.len()).map(|v| (v, self.semitrailer_weight(v, None))).collect();
            let Some(v) = self.weighted(weights.into_iter()) else { break };
            self.vehicle_weight[v] *= 0.75;
            let mut o = self.new_object(0x60, node);
            o.vehicle = Some(v);
            self.object_made.missions += 1;
            let Some(mut town) = self.nearest_town_any(o.pos, None) else {
                self.objects.push(o);
                continue;
            };
            let merchandise = if !self.vehicles[v].liquid {
                match self.town_merchandise(town, 0, 4) {
                    Some(m) => Some(m),
                    None => {
                        let at = self.net.town_node(&self.towns[town]).map_or([0.0; 3], |n| self.net.nodes[n].pos);
                        town = self.nearest_town_any(at, Some(town)).unwrap_or(town);
                        match self.town_merchandise(town, 0, 4) {
                            Some(m) => Some(m),
                            None => self.any_merchandise(false),
                        }
                    }
                }
            } else {
                self.any_merchandise(true)
            };
            // 0x404d90: a town other than the source that takes it, drawn evenly
            let dest = merchandise.and_then(|m| {
                let picks: Vec<usize> = (0..self.towns.len()).filter(|&t| t != town && self.takes(t, m)).collect();
                (!picks.is_empty()).then(|| picks[self.rng.next() as usize % picks.len()])
            });
            if let (Some(m), Some(to), Some(from)) = (merchandise, dest, o.place) {
                let unload = self.net.town_node(&self.towns[to]).unwrap_or(0);
                // 0x471780 and 0x471200 each find the route cost
                let fee = 0.1 * self.route_cost(&from, unload) * 1.3;
                let distance = self.route_cost(&from, unload);
                o.orders.push(Order { merchandise: m, to, from, amount: 0.0, pay: 0.0, deadline: self.last_tick + 900.0, made: self.last_tick, fee, kind: 2, distance });
            }
            self.objects.push(o);
        }
        // containers at "pos_hidden" nodes: a legal merchandise that is not liquid, drawn evenly
        // (0x41b120), for a town that takes it (0x404d40), and the present of kind 3
        for _ in 0..1000 {
            if self.object_want.containers <= self.object_made.containers {
                break;
            }
            let Some(node) = self.pick_node("pos_hidden", min) else { break };
            let mut o = self.new_object(0x40, node);
            self.object_made.containers += 1;
            if let Some(m) = self.any_merchandise(false) {
                let picks: Vec<usize> = (0..self.towns.len()).filter(|&t| self.takes(t, m)).collect();
                let to = (!picks.is_empty()).then(|| picks[self.rng.next() as usize % picks.len()]);
                let fee = match (to, o.place) {
                    (Some(t), Some(from)) => 0.1 * self.route_cost(&from, self.net.town_node(&self.towns[t]).unwrap_or(0)),
                    _ => 0.0,
                };
                o.cargo = Some(Cargo { merchandise: m, to, amount: 0.0, fee });
            }
            let weights: Vec<(usize, f64)> = self.presents.iter().enumerate().map(|(i, p)| (i, if p.kind == 3 { 0.75f64.powf(p.count as f64) } else { 0.0 })).collect();
            if let Some(i) = self.weighted(weights.into_iter()) {
                o.present = Some(i);
                self.presents[i].count += 1;
            }
            self.objects.push(o);
        }
        // parked cars at "pos_parking" nodes: the present of kind 1 put least
        for _ in 0..1000 {
            if self.object_want.parkings <= self.object_made.parkings {
                break;
            }
            let Some(node) = self.pick_node("pos_parking", min) else { break };
            let mut o = self.new_object(0x80, node);
            self.object_made.parkings += 1;
            let weights: Vec<(usize, f64)> = self.presents.iter().enumerate().map(|(i, p)| (i, if p.kind == 1 { 0.75f64.powf(p.count as f64) } else { 0.0 })).collect();
            if let Some(i) = self.weighted(weights.into_iter()) {
                o.present = Some(i);
                self.presents[i].count += 1;
            }
            o.made = self.last_tick;
            self.objects.push(o);
        }
    }
}

impl World {
    /// `0x41cff0`: road works (the present of kind 4, `Katok`) at the node named `name`; they
    /// count with the parked cars.
    pub fn road_works(&mut self, name: &str) {
        let Some(node) = self.net.nodes.iter().position(|n| n.name == name) else { return };
        if self.net.nodes[node].flags & 0x10 != 0 || self.taken[node] {
            return;
        }
        let mut o = self.new_object(0x480, node);
        self.object_made.parkings += 1;
        let weights: Vec<(usize, f64)> = self.presents.iter().enumerate().map(|(i, p)| (i, if p.kind == 4 { 0.75f64.powf(p.count as f64) } else { 0.0 })).collect();
        let Some(i) = self.weighted(weights.into_iter()) else { return };
        o.present = Some(i);
        self.presents[i].count += 1;
        o.made = self.last_tick;
        self.objects.push(o);
    }
}

/// An order of a town's store node (`fillTownOrder`, `0x41b3a0`, 0x20 bytes): carry `merchandise`
/// to town `to` by `deadline`; `fee` is a tenth of the cost of the way between the towns
/// (`0x41b300`). When an illegal merchandise was slipped in, `legal` is the one asked for.
#[derive(Debug, Clone)]
pub struct TownOrder {
    pub merchandise: usize,
    pub illegal: Option<usize>,
    pub legal: Option<usize>,
    pub to: Option<usize>,
    pub deadline: f64,
    pub fee: f64,
}

impl World {
    /// `formOrders` (`0x41b720`, a task of the game clock every 10 s, in the normal game): in
    /// every town, the orders of the store nodes (not unloading or dummy ones, two slots each)
    /// that are due go; then, in towns that deal in anything, empty slots get a new order.
    pub fn form_orders(&mut self, now: f64) {
        if (now - self.orders_time).abs() < 20.0 {
            return;
        }
        if self.town_slots.len() != self.towns.len() {
            self.town_slots = self.towns.iter().map(|t| vec![[None, None]; t.nodes.len()]).collect();
        }
        for t in 0..self.towns.len() {
            for k in 0..self.towns[t].nodes.len() {
                if self.towns[t].nodes[k].1 & 9 != 0 {
                    continue;
                }
                for s in 0..2 {
                    if self.town_slots[t][k][s].as_ref().is_some_and(|o| o.deadline <= now) {
                        self.town_slots[t][k][s] = None;
                    }
                }
            }
            if self.towns[t].goods.is_empty() {
                continue;
            }
            for k in 0..self.towns[t].nodes.len() {
                if self.towns[t].nodes[k].1 & 9 != 0 {
                    continue;
                }
                for s in 0..2 {
                    if self.town_slots[t][k][s].is_none() {
                        self.town_slots[t][k][s] = self.fill_town_order(t, k, s, now);
                    }
                }
            }
        }
    }

    /// `fillTownOrder` (`0x41b3a0`): for slot `s` of node `k` of town `t`: at a fuel node any
    /// liquid merchandise, else one of the town's (slot 0 for trucks, slot 1 for cars); a town
    /// that takes it (`0x454230`); the deadline 200 + `next() % 300` s from now. With the human
    /// player's `+0x300` over 0.25 (stand-in: never, it is 0 at the start), one time in five an
    /// illegal merchandise would be slipped in.
    fn fill_town_order(&mut self, t: usize, k: usize, s: usize, now: f64) -> Option<TownOrder> {
        let m = if self.towns[t].nodes[k].1 & 4 == 0 {
            self.town_merchandise(t, if s == 1 { 8 } else { 0x10 }, 0x1c)
        } else {
            self.any_merchandise(true)
        }?;
        let to = self.order_town(t, m);
        let deadline = (self.rng.next() % 300 + 200) as f64 + now;
        let fee = to.map_or(0.0, |to| self.town_dist.get(t).and_then(|d| d.get(to)).copied().unwrap_or(0.0) * 0.1);
        Some(TownOrder { merchandise: m, illegal: None, legal: None, to, deadline, fee })
    }

    /// `0x454230`: a town other than `from` that takes `m`, weighted 1, ×0.001 for each order of
    /// `from`'s nodes that already goes there.
    fn order_town(&mut self, from: usize, m: usize) -> Option<usize> {
        let weights: Vec<(usize, f64)> = (0..self.towns.len())
            .map(|u| {
                let mut w = 0.0;
                if u != from && self.takes(u, m) {
                    w = 1.0;
                    for slots in &self.town_slots[from] {
                        for o in slots.iter().flatten() {
                            if o.to == Some(u) {
                                w *= 0.001;
                            }
                        }
                    }
                }
                (u, w)
            })
            .collect();
        self.weighted(weights.into_iter())
    }
}

/// How many objects of each kind a new game puts (`GAME.CNF`).
#[derive(Debug, Clone, Default)]
pub struct ObjectCounts {
    /// `fuel` (`0x68a98c`), `semitrailers` (`0x68cf54`), `missions` (`0x68d0a0`), `containers`
    /// (`0x68a964`), `parkings` (`0x68bb54`).
    pub fuel: usize,
    pub semitrailers: usize,
    pub missions: usize,
    pub containers: usize,
    pub parkings: usize,
}
