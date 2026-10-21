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

/// The merchandise (`merchandise.sch`) that is liquid (flag `fuel`, bit 4 of `+0x30`).
pub fn liquid_goods(game_dir: &std::path::Path) -> Vec<String> {
    let Ok(data) = std::fs::read(game_dir.join("merchandise.sch")) else { return Vec::new() };
    let Ok(plain) = ht2_formats::sdte::decrypt(&data) else { return Vec::new() };
    let Ok(forms) = ht2_formats::sexpr::parse(&ht2_formats::sdte::cp1251_to_string(&plain)) else { return Vec::new() };
    let tops: Vec<&Sexp> = forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).collect();
    let mut out = Vec::new();
    for t in tops {
        let Some(l) = t.as_list() else { continue };
        let Some(name) = l.first().and_then(|s| s.as_text()) else { continue };
        // ( NAME pack n n ( price price share ( flags ) ) ... )
        let flags = l.iter().filter_map(|s| s.as_list()).flat_map(|s| s.iter().filter_map(|x| x.as_list())).flat_map(|x| x.iter().filter_map(|w| w.as_text()));
        if flags.into_iter().any(|w| w == "fuel") {
            out.push(name.to_string());
        }
    }
    out
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
        WorldObject { flags, node, pos, place: self.net.nearest(pos), vehicle: None, present: None }
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

    /// `0x434070` (new game): towns that deal in a liquid merchandise get flag 1.
    pub fn mark_liquid_towns(&mut self, liquid: &[String]) {
        for t in &mut self.towns {
            if t.goods.iter().any(|g| liquid.contains(g)) {
                t.flags |= 1;
            }
        }
    }

    /// `0x41d340` as far as ported: the fuel barrels at hidden places (`fuel` of `GAME.CNF`) and
    /// the semitrailers at bases (`semitrailers`), at least `min` from the human player.
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
