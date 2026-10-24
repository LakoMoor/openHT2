//! The truckers' orders (`Ai_ordr.cpp` and `loadOrder`, `0x443d50`): what a vehicle can carry,
//! loading an order at a town's store node, its price, and unloading at the town it goes to.

use crate::road::Spot;
use crate::world::World;

/// A load of a vehicle (`0x4715f0`, 0x90 bytes in the list at `+0x1c` of the semitrailer, else
/// of the vehicle): an order taken at a store node.
#[derive(Debug, Clone)]
pub struct Load {
    /// `+0`, `+4`, `+8`, `+0xc`: as in the order (the merchandise, the illegal and the legal one
    /// when it was swapped, the town it goes to).
    pub merchandise: usize,
    pub illegal: Option<usize>,
    pub legal: Option<usize>,
    pub to: usize,
    /// `+0x10`: the town it was loaded in; `+0x18`: the node (a position at it).
    pub from: usize,
    pub at: usize,
    /// `+0x38`: how many units.
    pub amount: f64,
    /// `+0x40`: what it pays (`0x471200`).
    pub price: f64,
    /// `+0x48`: by when it is to be there (now + the order's time, ×1.3 with a semitrailer).
    pub deadline: f64,
    /// `+0x50`: when it was loaded.
    pub loaded: f64,
    /// `+0x58`: the order's time (`+0x18`), ×1.3 with a semitrailer.
    pub time: f64,
    /// `+0x60`: its condition (1 when loaded; the damage is 1 less it).
    pub condition: f64,
    /// `+0x68`: 1 for an order of a town.
    pub kind: u32,
    /// `+0x70`: the route cost from where it was loaded to the unload node of its town.
    pub cost: f64,
}

impl World {
    /// The record of `vehicle.tech` of catalogue vehicle `v` (`+0x40`, found by the model at
    /// offset 2342 of the records, `0x42e9f0`).
    pub fn tech_of(&self, v: usize) -> Option<&ht2_formats::tech::Tech> {
        let model = &self.vehicles.get(v)?.model;
        self.tech.iter().find(|t| &t.key == model)
    }

    /// `0x41b910`: what player `k` can carry: the weight (`0x41b830`: the vehicle's capacity,
    /// unless it is a semitrailer) and the load area (of the vehicle unless it is a tractor, and
    /// of its semitrailer).
    pub fn capacity(&self, k: usize) -> (f64, f64) {
        let (mut weight, mut area) = (0.0, 0.0);
        let p = &self.players[k];
        for (v, count_weight) in [(p.vehicle, true), (p.trailer, false)] {
            let Some(v) = v else { continue };
            let Some(t) = self.tech_of(v) else { continue };
            let kind = self.vehicles[v].kind;
            if kind != 4 && count_weight {
                weight = t.capacity as f64 + weight;
            }
            if kind != 3 {
                area = t.load_space[1] as f64 * t.load_space[0] as f64 + area;
            }
        }
        (weight, area)
    }

    /// `0x41ba50`: how much of merchandise `m` fits in an empty vehicle of player `k`: the weight
    /// and the area it takes (each at most the vehicle's, and 85 % of what the other allows) and
    /// the units.
    fn fits(&self, k: usize, m: usize) -> (f64, f64, f64) {
        let (mut w, mut a) = self.capacity(k);
        let m = &self.merchandise[m];
        let d = a * m.unit_weight / m.unit_area;
        if d < w {
            w = d;
        }
        let d = m.unit_area * w / m.unit_weight;
        if d < a {
            a = d;
        }
        w *= 0.85;
        a *= 0.85;
        (w, a, w / m.unit_weight)
    }

    /// The unload node of town `t` (`+0x40`).
    pub fn unload_node(&self, t: usize) -> Option<usize> {
        self.net.town_node(&self.towns[t])
    }

    /// `0x471200` for load `i` of player `k` (not a swapped illegal one, `trailer` when it pulls
    /// a semitrailer): ((1 − share) + 0.2) · √(value · units) and the route cost ·
    /// 0.001 · 115 (40 when its vehicle is not a lorry or tractor), times 3; ×1.5 with a
    /// semitrailer.
    fn price_load(&mut self, k: usize, i: usize, trailer: bool) {
        let l = &self.players[k].loads[i];
        let m = &self.merchandise[l.merchandise];
        let base = ((1.0 - m.share) + 0.2) * (m.value * l.amount).sqrt();
        if l.cost == 0.0 {
            let (from, to) = (l.from, l.to);
            let c = self.town_dist.get(from).and_then(|r| r.get(to)).copied().unwrap_or(0.0);
            self.players[k].loads[i].cost = c;
        }
        let lorry = self.players[k].vehicle.is_some_and(|v| matches!(self.vehicles[v].kind, 2 | 3));
        let d = if lorry { 115.0 } else { 40.0 };
        let l = &mut self.players[k].loads[i];
        let mut price = (d * l.cost * 0.001 + base) * 3.0;
        if l.amount == 0.0 {
            price = 0.0;
        }
        if trailer {
            price *= 1.5;
        }
        if l.kind == 3 {
            price *= 4.0;
        }
        l.price = price;
    }

    /// `loadOrder` (`0x443d50`) at node `r` of town `t`, as much as fits (`1e6`): at a store node
    /// with an order in the slot for the vehicle (trucks 0, cars 1), when what it carries leaves
    /// room for 85 % of the vehicle (else -8, -3, -4), a load of it (`0x4715f0`; added to one of
    /// the same merchandise for the same town); its price; with a semitrailer its time ×1.3.
    /// Then `accountForChangeInLoadState` (`0x442c60`). Whether it loaded.
    pub fn load_order(&mut self, k: usize, t: usize, r: usize, now: f64) -> bool {
        self.load_order_at(k, t, r, 1e6, now).0 == 0
    }

    /// `loadOrder` (`0x443d50`) as the game calls it for the human player (message 0x11):
    /// `amount` above 0 loads at a store node, below 0 unloads at the unload node. What it
    /// returns: 0, or -1 (nothing to do there), -8, -3, -4 (no room), -10 (a liquid needs a tank
    /// semitrailer), -11 (a tank takes only liquids); the units loaded (else `amount`); the
    /// index of the load (else -1). For the human player `0x68d100` is set to now, and at the
    /// end `provideCompetitionRegular` runs while fewer competitors than `0x68d160` are in the
    /// list (`0x4262a0`, until it adds none).
    pub fn load_order_at(&mut self, k: usize, t: usize, r: usize, amount: f64, now: f64) -> (i32, f64, i32) {
        if self.is_human(k) {
            self.orders_time = now;
        }
        let Some(&(_, attr)) = self.towns.get(t).and_then(|x| x.nodes.get(r)) else { return (-1, amount, -1) };
        if attr & 1 != 0 {
            if amount < 0.0 {
                self.unload_order(k, t, r, now);
                self.after_order(k);
                return (0, amount, -1);
            }
            return (-1, amount, -1);
        }
        if !(0.0 < amount) {
            return (-1, amount, -1);
        }
        match self.load_at_store(k, t, r, now) {
            Ok((units, index)) => {
                self.after_order(k);
                (0, units, index)
            }
            Err(code) => (code, amount, -1),
        }
    }

    /// The end of `loadOrder` (`0x444e05`): for the human player the competition is provided
    /// while fewer than `0x68d160` are in the list; then `accountForChangeInLoadState`.
    fn after_order(&mut self, k: usize) {
        if self.is_human(k) {
            let mut n = self.competitor_count();
            if n + self.competitors_arrived < self.competition {
                loop {
                    self.provide_competition();
                    let m = self.competitor_count();
                    if n == m {
                        break;
                    }
                    n = m;
                    if self.competition <= m + self.competitors_arrived {
                        break;
                    }
                }
            }
        }
        self.account_for_load_change(k);
    }

    /// `0x41b9a0`: whether the vehicle of player `k` (its semitrailer when it has one) is a tank.
    fn tank(&self, k: usize) -> bool {
        let p = &self.players[k];
        let mut tank = p.vehicle.is_some_and(|v| self.vehicles[v].liquid);
        if let Some(t) = p.trailer {
            tank = self.vehicles[t].liquid;
        }
        tank
    }

    /// The loading part of `loadOrder`: the units loaded and the index of the load, or the code.
    fn load_at_store(&mut self, k: usize, t: usize, r: usize, now: f64) -> Result<(f64, i32), i32> {
        let Some(&(ref name, _)) = self.towns.get(t).and_then(|x| x.nodes.get(r)) else { return Err(-1) };
        let Some(node) = self.net.nodes.iter().position(|n| &n.name == name) else { return Err(-1) };
        let truck = self.players[k].vehicle.is_some_and(|v| matches!(self.vehicles[v].kind, 2 | 3));
        let Some(order) = self.town_slots.get(t).and_then(|n| n.get(r)).and_then(|s| s[if truck { 0 } else { 1 }].clone()) else { return Err(-1) };
        let Some(to) = order.to else { return Err(-1) };
        if self.is_human(k) {
            let liquid = self.merchandise[order.merchandise].flags & 4 != 0;
            if liquid && !self.tank(k) {
                // NEED_A_PETROL_SEMITRAILER
                return Err(-10);
            }
            if !liquid && self.tank(k) {
                // LOAD_AT_AN_OIL_TANK
                return Err(-11);
            }
        }
        let (mut used_w, mut used_a) = (0.0, 0.0);
        for l in &self.players[k].loads {
            let m = &self.merchandise[l.merchandise];
            used_w = m.unit_weight * l.amount + used_w;
            used_a = m.unit_area * l.amount + used_a;
        }
        let (cap_w, cap_a) = self.capacity(k);
        let (w, a, units) = self.fits(k, order.merchandise);
        if cap_w < w + used_w || cap_a < a + used_a {
            // NOT_ENOUGH_SPACE for the human player
            return Err(-8);
        }
        if 0.0 <= used_w - cap_w {
            return Err(-3);
        }
        if 0.0 <= used_a - cap_a {
            return Err(-4);
        }
        let cost = match self.unload_node(to) {
            Some(u) => self.spot_route_cost(&Spot::Node(node), u),
            None => 0.0,
        };
        let load = Load { merchandise: order.merchandise, illegal: order.illegal, legal: order.legal, to, from: t, at: node, amount: units, price: 0.0, deadline: now + order.fee, loaded: now, time: order.fee, condition: 1.0, kind: 1, cost };
        let loads = &mut self.players[k].loads;
        let i = match loads.iter().position(|l| l.merchandise == load.merchandise && l.to == load.to) {
            Some(i) => {
                loads[i].amount += units;
                i
            }
            None => {
                loads.push(load);
                loads.len() - 1
            }
        };
        let mut index = i as i32;
        if self.players[k].loads[i].amount != 0.0 {
            let trailer = self.players[k].trailer.is_some();
            self.price_load(k, i, trailer);
            if trailer {
                let l = &mut self.players[k].loads[i];
                l.time *= 1.3;
                l.deadline = (1.3 - 1.0) * order.fee + l.deadline;
            }
        } else {
            self.players[k].loads.remove(i);
            index = -1;
        }
        Ok((units, index))
    }

    /// `loadOrder` (`0x443d50`) unloading (`-1e6`) at node `r` of town `t`: at an unload node,
    /// every load for the town goes; what they pay (`0x471450`: the price less the damage) is
    /// earned (`+0x80`, `+0x2f8`; a player of a company would pay it to the company). Then a
    /// number is drawn (360 + `next() % 360`, used for competitors), and for a player who is not
    /// a competitor one in 24 (`0x429850`) licences it for 300 s (`0x4774e0`). The human player,
    /// when he brought an order of a town before any competitor arrived (and there are some),
    /// counts one more such arrival (`+0x218`) and gets ⌊√(that + 1)⌋ licences (at most 3) for
    /// the time drawn; else the count is 0. A competitor counts as arrived (`0x68d164`), and
    /// the first to arrive gets a licence for that time. The game is told (event 11).
    pub fn unload_order(&mut self, k: usize, t: usize, r: usize, now: f64) {
        self.unload_at(k, t, r, now);
        if !self.is_human(k) {
            self.account_for_load_change(k);
        }
    }

    fn unload_at(&mut self, k: usize, t: usize, r: usize, now: f64) {
        let Some(&(_, attr)) = self.towns.get(t).and_then(|x| x.nodes.get(r)) else { return };
        if attr & 1 == 0 {
            return;
        }
        let mut earned = 0.0;
        let competitor = self.players[k].tracked;
        let human = self.is_human(k);
        let (mut delivered, mut of_town) = (false, false);
        let mut i = 0;
        while i < self.players[k].loads.len() {
            if self.players[k].loads[i].to != t {
                i += 1;
                continue;
            }
            let l = self.players[k].loads.remove(i);
            delivered = true;
            if l.kind == 1 {
                of_town = true;
            }
            let m = &self.merchandise[l.merchandise];
            // 0x4713f0: the damage, (1 − condition) · √(value · units) · 10
            let damage = (1.0 - l.condition) * (m.value * l.amount).sqrt() * 10.0;
            let mut pay = l.price;
            if (competitor || human) && self.competitors_arrived == 0 && 0.0 < pay && l.kind == 1 {
                pay *= 1.5;
            }
            earned = (pay - damage) + earned;
        }
        match self.players[k].boss {
            None => self.players[k].money += earned,
            Some(b) => {
                self.players[k].company_balance += earned;
                self.players[b].money += earned;
            }
        }
        self.players[k].earned += earned;
        let time = (self.rng.next() % 360 + 360) as f64;
        if !competitor && self.players[k].kind == 1 {
            if self.competitors_arrived == 0 && delivered && of_town && !self.competitor_list.is_empty() {
                let p = &mut self.players[k];
                p.wins += 1;
                let n = (((p.wins + 1) as f64).sqrt() as i64).min(3);
                for _ in 0..n {
                    p.licences.push(now + time);
                }
                p.licences.sort_by(|a, b| a.total_cmp(b));
            } else {
                self.players[k].wins = 0;
            }
        } else if !competitor {
            // 0x429850: one in 24, 12 or 6 as the human player's share is over 0.1 or 0.2
            let h = self.human_share();
            let mut d = 1;
            if 0.1 < h {
                d = 2;
            }
            if 0.2 < h {
                d = 4;
            }
            if self.rng.next() % (24 / d) == 0 {
                self.players[k].licences.push(now + 300.0);
            }
        } else {
            if self.competitors_arrived == 0 && delivered && !self.competitor_list.is_empty() {
                let p = &mut self.players[k];
                p.licences.push(now + time);
                p.licences.sort_by(|a, b| a.total_cmp(b));
            }
            // COMPETITOR_ARRIVED
            self.competitors_arrived += 1;
        }
    }

    /// `accountForChangeInLoadState` (`0x442c60`) for a player other than the human:
    /// - `accountForGuilt` (`0x442850`): its loads are not illegal and the human player is not
    ///   wanted, so it carries nothing to hide (`+0xc8`, `+0xc9`); when its vehicle has flag 2
    ///   (`+0x28`) a number is drawn, under 500000 it is alarmed (`+0x108`);
    /// - a competitor that carries nothing is one no more (`0x425790`, `0x417530`);
    /// - flag 0x8000 when a load goes to `TOWN_AA` (`0x41ef90`);
    /// - its rating (`+0x3c0`, 10000 at the start): 0.8 of it and 0.2 of what the first load is
    ///   worth (`0x4715a0`: the price ×1.5, see below, and √(value · units) · 10 less the damage);
    ///   flags 0x100000 and 0x1000000 go when it carries nothing;
    /// - `markAmbushes` (`0x42c0b0`).
    /// The flags of `0x477210` are not kept.
    pub fn account_for_load_change(&mut self, k: usize) {
        if self.players[k].vehicle_flags & 2 != 0 {
            let r = (self.rng.next() % 1_000_000) as f64;
            self.players[k].alarmed = r < 500000.0;
        } else {
            self.players[k].alarmed = false;
        }
        self.drop_competition(k);
        self.mark_town_aa(k);
        match self.players[k].loads.first() {
            None => self.players[k].flags &= 0xfeef_ffff,
            Some(l) => {
                let m = &self.merchandise[l.merchandise];
                let a = (m.value * l.amount).sqrt() * 10.0;
                let b = (1.0 - l.condition) * (m.value * l.amount).sqrt() * 10.0;
                // 0x4715a0 calls 0x471560 one argument short: its flag is what lies on the stack,
                // which is not 0, so the price counts ×1.5 (for an order of a town that pays)
                let mut price = l.price;
                if 0.0 < price && l.kind == 1 {
                    price *= 1.5;
                }
                let worth = price + (a - b);
                let p = &mut self.players[k];
                p.rating = p.rating * 0.8 + worth * 0.2;
            }
        }
        self.mark_ambushes();
    }

    /// `expirePlayerOrders` (`0x429fb0`, every second): the loads of the human player whose time
    /// is over go (`0x429cd0`, the first such one each time) and their penalties (`0x471420`:
    /// √(value · units) · 10) come off his money (`+0x88` of his node). When one went, the
    /// competition is dropped if no order of a town is left (`0x425790`), `0x41ef90` runs and the
    /// game is told (event 13, `ORDER_EXPIRED`; not ported). `0x426f20` tells the game when his
    /// money is below 0 (not ported).
    pub fn expire_player_orders(&mut self) {
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        let now = self.now;
        let mut penalty = 0.0;
        let mut any = false;
        while let Some(i) = self.players[h].loads.iter().position(|l| l.deadline < now) {
            let l = self.players[h].loads.remove(i);
            penalty = (self.merchandise[l.merchandise].value * l.amount).sqrt() * 10.0 + penalty;
            any = true;
        }
        self.players[h].money -= penalty;
        if any {
            self.drop_competition(h);
            self.mark_town_aa(h);
        }
    }

    /// `0x41ef90`: flag 0x8000 while a load goes to `TOWN_AA`.
    pub(crate) fn mark_town_aa(&mut self, k: usize) {
        let aa = self.players[k].loads.iter().any(|l| self.towns.get(l.to).is_some_and(|t| t.id == "TOWN_AA"));
        let p = &mut self.players[k];
        p.flags &= !0x8000;
        if aa {
            p.flags |= 0x8000;
        }
    }

    /// `0x417ed0`: a route to the unload node of the town its first load goes to.
    pub fn trace_to_delivery(&mut self, k: usize) {
        let Some(to) = self.players[k].loads.first().map(|l| l.to) else { return };
        let Some(n) = self.unload_node(to) else { return };
        let mut p = self.players[k].clone();
        p.route.clear();
        self.set_route(&mut p, &Spot::Node(n));
        self.players[k] = p;
    }
}
