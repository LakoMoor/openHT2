//! The competitors of the human player (`Ai_main.cpp`): truckers that take the same order as
//! he does and race him to its town. `provideCompetitionRegular` (`0x4262a0`, every 30 s and
//! after he loads), `provideCompetitionSpecial` (`0x425dd0`), how far each one has to go
//! (`0x41a7d0`, every tick `0x4287b0`) and the human player's place among them (`0x4285c0`).

use crate::orders::Load;
use crate::road::Spot;
use crate::world::World;

impl World {
    /// `0x417660`: the human player (type 1 without flag 0x100).
    pub fn is_human(&self, k: usize) -> bool {
        let p = &self.players[k];
        p.kind == 1 && p.flags & 0x100 == 0
    }

    /// How many players are in the list of competitors (`0x68a980`).
    pub fn competitor_count(&self) -> u32 {
        self.competitor_list.len() as u32
    }

    /// The kind of the vehicle of player `k` (`+0x1fc`, `0x68cfd8[..] + 0x20`): 1 car, 2 lorry,
    /// 3 tractor; `None` without one.
    fn vehicle_kind(&self, k: usize) -> Option<u32> {
        self.players[k].vehicle.map(|v| self.vehicles[v].kind)
    }

    /// `0x4175c0`: player `k` (not the human one) joins the competitors.
    pub(crate) fn track(&mut self, k: usize) {
        if !self.is_human(k) {
            if !self.competitor_list.contains(&k) {
                self.competitor_list.push(k);
            }
            self.players[k].tracked = true;
        }
        // 0x477210: flag 0x80 while a competitor
        self.mirror_tracked(k);
    }

    /// The part of `0x477210` that is kept: flag 0x80 while the player is a competitor.
    pub(crate) fn mirror_tracked(&mut self, k: usize) {
        let p = &mut self.players[k];
        if p.tracked {
            p.flags |= 0x80;
        } else {
            p.flags &= !0x80;
        }
    }

    /// `0x417530`: player `k` is a competitor no more.
    pub(crate) fn untrack(&mut self, k: usize) {
        if let Some(index) = self.competitor_list.iter().position(|&c| c == k) {
            self.competitor_list.remove(index);
        }
        let p = &mut self.players[k];
        p.tracked = false;
        p.comp_count = 0;
        p.comp_place = 0;
        p.comp_told = -1;
        self.mirror_tracked(k);
    }

    /// `0x4256f0`: all competitors are let go, none has arrived, and how many there will be and
    /// how they compete are drawn again (`0x425610`).
    pub(crate) fn release_competitors(&mut self) {
        self.competitors_arrived = 0;
        self.competitor_list.clear();
        for k in 0..self.players.len() {
            self.players[k].tracked = false;
            self.mirror_tracked(k);
        }
        self.draw_competition();
    }

    /// `0x425610`: how many competitors there will be, 3 + `next() % (⌊3.1 · traffic + 4⌋ − 2)`
    /// (`0x68d15c`), and how they compete, `next() & 3` (`0x68d160`).
    pub(crate) fn draw_competition(&mut self) {
        let n = (self.car_share * 3.1 + 4.0) as u32;
        self.competitors = self.rng.next() % (n - 2) + 3;
        self.competition = self.rng.next() & 3;
    }

    /// `0x425790`: in a game (not a race of the human player), a human player without an order
    /// of a town lets the competitors go (`0x4256f0`) and his place among them is forgotten
    /// (`+0x20c`, `+0x210`, `+0x214`); another player that competes and carries nothing is a
    /// competitor no more (`0x417530`).
    pub(crate) fn drop_competition(&mut self, k: usize) {
        if self.players[k].state == 5 {
            return;
        }
        if self.players[k].kind == 1 {
            if self.players[k].loads.first().is_none_or(|l| l.kind != 1) {
                self.release_competitors();
                let p = &mut self.players[k];
                p.comp_count = 0;
                p.comp_place = 0;
                p.comp_told = -1;
            }
        } else if self.players[k].tracked && self.players[k].loads.is_empty() {
            self.untrack(k);
        }
    }

    /// `0x425c10`: a competitor drives something other than a lorry or a tractor.
    fn competitor_in_car(&self) -> bool {
        self.competitor_list.iter().any(|&c| !matches!(self.vehicle_kind(c), Some(2 | 3)))
    }

    /// `0x425c60`: player `c` may compete with the human player `h` by its vehicle: with a lorry
    /// or a tractor only one of those; else a lorry or tractor only once a competitor drives
    /// something else.
    fn vehicle_fits(&self, h: usize, c: usize) -> bool {
        if matches!(self.vehicle_kind(h), Some(2 | 3)) {
            matches!(self.vehicle_kind(c), Some(2 | 3))
        } else {
            !(matches!(self.vehicle_kind(c), Some(2 | 3)) && !self.competitor_in_car())
        }
    }

    /// `0x425bb0`: a trucker with a vehicle that is no tractor, not a competitor yet, not for
    /// sale, hired out or racing.
    fn may_compete(&self, c: usize) -> bool {
        let p = &self.players[c];
        p.vehicle.is_some() && !p.tracked && p.kind == 2 && !matches!(p.state, 0x100 | 0x101 | 5) && self.vehicle_kind(c) != Some(3)
    }

    /// `0x4571a0` (`0x45e520`, `with_boss`) and `0x457210` (`0x45e710`): of the players that may
    /// compete with the human player `h`, also not in the game's scene, not the human player
    /// and (`with_boss`) not working for him, one drawn (`next() % n`). The original also leaves
    /// out a player whose `+0x60` is the human player (never set in the cases checked so far;
    /// not ported).
    fn draw_competitor(&mut self, h: usize, with_boss: bool) -> Option<usize> {
        let cands: Vec<usize> = (0..self.players.len())
            .filter(|&c| {
                let p = &self.players[c];
                p.kind != 0 && self.vehicle_fits(h, c) && self.may_compete(c) && !(with_boss && p.boss == Some(h)) && !self.in_scene(p) && p.flags & 1 == 0
            })
            .collect();
        if cands.is_empty() {
            return None;
        }
        Some(cands[(self.rng.next() % cands.len() as u32) as usize])
    }

    /// `0x457280` (`0x45e8f0`): of the players that may compete, a trucker on its way to an order
    /// (state 4) loaded at the same store node for the same town as `load`, within 400 m of the
    /// human player, one drawn.
    fn draw_rival(&mut self, h: usize, load: &Load) -> Option<usize> {
        let hp = self.position(&self.players[h]).unwrap_or([0.0; 3]);
        let cands: Vec<usize> = (0..self.players.len())
            .filter(|&c| {
                let p = &self.players[c];
                if p.kind == 0 || !self.vehicle_fits(h, c) || !self.may_compete(c) || p.state != 4 {
                    return false;
                }
                let Some(l) = p.loads.first() else { return false };
                if l.at != load.at || l.to != load.to {
                    return false;
                }
                let q = self.position(p).unwrap_or([0.0; 3]);
                let d = [hp[0] - q[0], hp[1] - q[1], hp[2] - q[2]];
                (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt() < 400.0
            })
            .collect();
        if cands.is_empty() {
            return None;
        }
        Some(cands[(self.rng.next() % cands.len() as u32) as usize])
    }

    /// `0x425820`: player `c` takes the human player's load `load`: as many units as it can
    /// carry by weight or by area, whichever is fewer (`0x41b910`, without the 85 %), at the
    /// price for that many; then `accountForChangeInLoadState` (`0x442c60`).
    fn give_load(&mut self, c: usize, load: &Load) {
        let (w, a) = self.capacity(c);
        let m = &self.merchandise[load.merchandise];
        let by_weight = w / m.unit_weight;
        let by_area = a / m.unit_area;
        let units = if by_area <= by_weight { by_area } else { by_weight };
        let mut l = load.clone();
        l.amount = units;
        l.price = load.price * units / load.amount;
        self.players[c].loads.push(l);
        self.account_for_load_change(c);
    }

    /// `updateAdvancementRegular` (`0x41a7d0`): how far player `k` (at its place) has still to
    /// go to the unload node of the town of its first load, as a negative number (`+0x1f8`): the
    /// straight distance within 100 m, else the route cost (mode 0, `0x486800`); 0 without a
    /// load.
    pub(crate) fn update_advancement(&mut self, k: usize) {
        let mut left = 0.0;
        if let Some(to) = self.players[k].loads.first().map(|l| l.to) {
            if let Some(n) = self.unload_node(to) {
                let q = self.position(&self.players[k]).unwrap_or([0.0; 3]);
                let c = self.net.nodes[n].pos;
                let (x, y, z) = (c[0] - q[0], c[1] - q[1], c[2] - q[2]);
                left = (y * y + x * x + z * z).sqrt();
                if 100.0 < left {
                    left = match self.players[k].place {
                        Some(from) => self.net.route_from(&from, &Spot::Node(n), 0).map_or(0.0, |(_, c)| c),
                        None => 0.0,
                    };
                }
            }
        }
        self.players[k].advancement = -left;
    }

    /// `0x409f70`: whether competitor `k` is behind the human player (his `+0x1f8` greater).
    pub(crate) fn competitor_behind(&self, k: usize) -> bool {
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return false };
        0.0 < self.players[h].advancement - self.players[k].advancement
    }

    /// `retraceCompetitorRegular` (`0x409fc0`): a competitor (not racing) behind the human
    /// player that is one of every three (`+0x350`) follows him (a waypoint of kind 2 at his
    /// place, `0x478260`); else such a waypoint goes and the route is traced again (`0x478630`).
    pub(crate) fn retrace_competitor(&mut self, k: usize) {
        let p = &self.players[k];
        if !p.tracked || p.state == 5 {
            return;
        }
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        let mut p = self.players[k].clone();
        if self.competitor_behind(k) && p.shadow {
            if let Some(to) = self.players[h].place {
                self.trace_to(&mut p, &to, 2);
            }
        } else {
            let before = p.waypoints.len();
            p.waypoints.retain(|w| w.1 != 2);
            if p.waypoints.len() != before {
                self.retrace(&mut p);
            }
        }
        self.players[k] = p;
    }

    /// `provideCompetitionRegular` (`0x4262a0`) for the human player (not racing). Without an
    /// order of a town or with a liquid one, the competitors go (`0x425790`) and `0x41ef90`
    /// runs. Else: the competitors are drawn if none are (`0x425610`); when none has come yet
    /// and he is more than 1000 (route cost, `0x487500`) from where he loaded,
    /// `provideCompetitionSpecial`. Then while there are fewer than wanted, truckers on their
    /// way from the same store to the same town near him join (`0x457280`). One more comes
    /// (`0x425a90`) while fewer than `0x68d160` have come, or fewer than wanted and he is more
    /// than 400 m from the store: a trucker drawn (`0x4571a0`, else `0x457210`) takes his order
    /// at the store and is put on its way some way behind him. Then every third competitor
    /// follows him when behind (`+0x350`).
    pub fn provide_competition(&mut self) {
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        if self.players[h].state == 5 {
            return;
        }
        let load = match self.players[h].loads.first() {
            Some(l) if l.kind == 1 && self.merchandise[l.merchandise].flags & 4 == 0 => l.clone(),
            _ => {
                self.drop_competition(h);
                self.mark_town_aa(h);
                return;
            }
        };
        if self.competitors == 0 {
            self.draw_competition();
        }
        if self.competitor_count() + self.competitors_arrived == 0 {
            let cost = match self.players[h].place {
                Some(to) => self.spot_cost(&Spot::Node(load.at), &to),
                None => 0.0,
            };
            if 1000.0 < cost {
                self.provide_competition_special();
                return;
            }
        }
        while self.competitor_count() + self.competitors_arrived < self.competitors {
            let Some(c) = self.draw_rival(h, &load) else { break };
            self.track(c);
            self.update_advancement(c);
        }
        let at = self.net.nodes[load.at].pos;
        if self.more_competition(at) {
            if let Some(c) = self.draw_competitor(h, true).or_else(|| self.draw_competitor(h, false)) {
                self.players[c].loads.clear();
                self.give_load(c, &load);
                self.teleport_to(c, Spot::Node(load.at));
                self.route_to_town(c, load.to);
                let hp = self.human_point().unwrap_or([0.0; 3]);
                let d = |w: &World| {
                    let q = w.players[c].place.map_or([0.0; 3], |s| w.net.spot_coordinates(&s));
                    let z = hp[2] - q[2];
                    (z * z + (hp[1] - q[1]) * (hp[1] - q[1]) + (hp[0] - q[0]) * (hp[0] - q[0])).sqrt()
                };
                if d(self) < 400.0 {
                    let n = (self.competition as i32 * 400 - 400) as f64;
                    let mut spread = n;
                    if load.cost * 0.33 < n {
                        spread = load.cost * 0.33;
                    }
                    let mut gap = 400.0;
                    if 400.0 <= spread {
                        gap = spread;
                        if 3000.0 < spread {
                            gap = 3000.0;
                        }
                    }
                    if self.competition < 2 {
                        gap = 20.0;
                    } else {
                        gap /= (self.competition - 1) as f64;
                    }
                    let mut i = 0;
                    while (i as f64) < 400.0 {
                        if 400.0 < d(self) {
                            break;
                        }
                        self.advance_player(c, 4.0);
                        i += 1;
                    }
                    let r = (self.rng.next() % ((gap * 0.3333333333333333) as i64 as u32)) as f64;
                    let off = self.competitor_count() as f64 * gap + r;
                    self.advance_player(c, off);
                }
                self.track(c);
                self.players[c].state = 4;
                self.unstand(c);
                self.update_advancement(c);
            }
        }
        for (i, &c) in self.competitor_list.clone().iter().enumerate() {
            self.players[c].shadow = i % 3 == 0;
        }
    }

    /// `0x425a90`: one more competitor is wanted while fewer than `0x68d160` have come, or
    /// fewer than wanted and the human player is more than 400 m from `at`.
    fn more_competition(&self, at: [f64; 3]) -> bool {
        let n = self.competitor_count() + self.competitors_arrived;
        if n < self.competition {
            return true;
        }
        if n < self.competitors {
            let h = self.human_point().unwrap_or([0.0; 3]);
            let d = ((h[0] - at[0]) * (h[0] - at[0]) + (h[2] - at[2]) * (h[2] - at[2]) + (h[1] - at[1]) * (h[1] - at[1])).sqrt();
            return 400.0 < d;
        }
        false
    }

    /// `provideCompetitionSpecial` (`0x425dd0`): while there are fewer competitors than wanted,
    /// a trucker drawn (`0x4571a0`, else `0x457210`) drops its loads (`0x4778a0`), takes the
    /// human player's order (`0x425820`) at its store (`0x4786a0`), goes for its town
    /// (`0x478260`), is put on that route at the human player's point (`0x43dc60`) and moved
    /// on it ((`0x68d160` − competitors) · 150 + `next() % 20` − 10 m, less one 150 when that
    /// is not above 0, and at least 150 either way, `0x435a90`); it competes in state 4.
    fn provide_competition_special(&mut self) {
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        let Some(load) = self.players[h].loads.first().cloned() else { return };
        while self.competitor_count() + self.competitors_arrived < self.competitors {
            let Some(c) = self.draw_competitor(h, true).or_else(|| self.draw_competitor(h, false)) else { break };
            self.players[c].loads.clear();
            self.give_load(c, &load);
            self.teleport_to(c, Spot::Node(load.at));
            self.route_to_town(c, load.to);
            if let Some(hp) = self.players[h].place.map(|s| self.net.spot_coordinates(&s)) {
                let mut p = self.players[c].clone();
                if let Some(from) = p.place {
                    p.place = Some(self.net.keep_node(Some(&from), self.net.locate(&p.route, hp, &from)));
                }
                self.players[c] = p;
            }
            let mut k = self.competition as i32 - self.competitor_count() as i32 - self.competitors_arrived as i32;
            if k < 1 {
                k -= 1;
            }
            let r = (self.rng.next() % 20) as f64;
            let mut off = (k as f64 * 150.0 + r) - 10.0;
            if -150.0 < off && off < 0.0 {
                off = -150.0;
            } else if off < 150.0 && 0.0 < off {
                off = 150.0;
            }
            self.advance_player(c, off);
            self.track(c);
            self.players[c].state = 4;
            self.unstand(c);
            self.update_advancement(c);
        }
    }

    /// `0x4786a0` to a spot: route and waypoints go, the place is set (`0x474290`).
    fn teleport_to(&mut self, k: usize, to: Spot) {
        let place = self.net.keep_node(self.players[k].place.as_ref(), to);
        let p = &mut self.players[k];
        p.route.clear();
        p.waypoints.clear();
        p.place = Some(place);
    }

    /// `0x478260` (kind 1) to the unload node of town `t` (`0x426270`, `0x404e80`).
    fn route_to_town(&mut self, k: usize, t: usize) {
        let Some(n) = self.unload_node(t) else { return };
        let mut p = self.players[k].clone();
        self.trace_to(&mut p, &Spot::Node(n), 1);
        self.players[k] = p;
    }

    /// `advancePlayer` (`0x435a90`): `dist` metres along its route (back when negative).
    fn advance_player(&mut self, k: usize, mut dist: f64) {
        let p = &mut self.players[k];
        let Some(Spot::Road(mut place)) = p.place else { return };
        self.net.advance_route(&mut place, &mut dist, &p.route);
        p.place = Some(Spot::Road(place));
    }

    /// `updateAdvancementsRegular` (`0x4287b0`, each tick in a game): while there are
    /// competitors, how far the human player (not racing) and each competitor have still to go.
    /// When a competitor's changes by more than 1, its `+0x1f9` flips (not ported).
    pub fn update_advancements(&mut self) {
        if self.competitor_list.is_empty() {
            return;
        }
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        if self.players[h].state == 5 {
            return;
        }
        self.update_advancement(h);
        for k in 0..self.players.len() {
            if self.players[k].kind != 0 && self.players[k].tracked {
                self.update_advancement(k);
            }
        }
    }

    /// `calculateNewPlace` (`0x427f40`): how many competitors are ahead of the human player
    /// (`0x409f70` false) and how near the nearest is.
    fn competitors_ahead(&self) -> (u32, f64) {
        let mut ahead = 0;
        let mut near = f64::MAX;
        for k in 0..self.players.len() {
            if self.players[k].kind == 0 || !self.players[k].tracked {
                continue;
            }
            let Some(h) = self.players.iter().position(|p| p.kind == 1) else { break };
            let d = self.players[h].advancement - self.players[k].advancement;
            if d <= 0.0 {
                ahead += 1;
            }
            if d.abs() < near {
                near = d.abs();
            }
        }
        (ahead, near)
    }

    /// `reportCompetitionState` (`0x4285c0`, each tick): while there are competitors, the human
    /// player's place among them (`recalculateCompetitionPlace`, `0x428230`); when it changed
    /// (`+0x214`), the game is told (`0x427ee0`, not ported). When no competitor is within 12
    /// of him, his `+0x20c` is how many there are and his place `+0x210` the ones arrived and
    /// ahead; in a game, for the four points 12 m to his sides and front and back (`0x68cc28`,
    /// turned by his matrix `+0x228`) the place on the roads of his room (`0x4365d0`) is tried:
    /// at the first that puts fewer ahead his place is that, and his `+0x1f8` stays as there.
    pub fn report_competition(&mut self) {
        if self.competitor_list.is_empty() {
            return;
        }
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        let (n, near) = self.competitors_ahead();
        if 12.0 < near {
            self.players[h].comp_count = (self.competitor_count() + self.competitors_arrived) as i32;
            let place = (self.competitors_arrived + n) as i32;
            if self.players[h].state != 5 {
                let m = self.human_matrix;
                for off in [[-12.0, 0.0, 0.0], [12.0, 0.0, 0.0], [0.0, -12.0, 0.0], [0.0, 12.0, 0.0]] {
                    // 0x487e50: the offset turned by the matrix (rows at +0, +0x18, +0x30) and
                    // moved by its translation (+0x48)
                    let q: [f64; 3] = core::array::from_fn(|c| (off[1] * m[3 + c] + m[c] * off[0] + m[6 + c] * off[2]) + m[9 + c]);
                    let found = match &self.human_room {
                        Some(r) => self.net.nearest_in_room(r, q),
                        None => self.net.nearest(q),
                    };
                    let Some(p) = found else { continue };
                    let saved = self.players[h].place;
                    self.players[h].place = Some(Spot::Road(p));
                    self.update_advancement(h);
                    self.players[h].place = saved;
                    let (n2, _) = self.competitors_ahead();
                    if (self.competitors_arrived + n2) < place as u32 {
                        self.players[h].comp_place = (self.competitors_arrived + n2) as i32;
                        self.report_place(h);
                        return;
                    }
                }
                self.update_advancement(h);
            }
            self.players[h].comp_place = place;
        }
        self.report_place(h);
    }

    /// The end of `reportCompetitionState`: a new place is remembered (`+0x214`) and told.
    fn report_place(&mut self, h: usize) {
        let p = &mut self.players[h];
        if p.comp_place != p.comp_told {
            p.comp_told = p.comp_place;
        }
    }

    /// `setPlayerPosition` (`0x43e610`) for the human player as the game reports where his
    /// vehicle is: his place becomes `place`; when its road is not the one before, he has passed
    /// the node the two roads share (`0x47fc80`) and `0x40b100` runs for it: in a game every
    /// competitor's route is looked at again (`0x40a180` → `0x409fc0`). (Its branches for a
    /// wanted human player, the node events and the race finish are not ported; they change
    /// nothing in the AI while he is not wanted.)
    pub fn place_human(&mut self, place: Spot) {
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        let road = |s: &Option<Spot>| match s {
            Some(Spot::Road(q)) => Some(q.road),
            _ => None,
        };
        let old = road(&self.players[h].place);
        self.players[h].place = Some(place);
        let new = road(&self.players[h].place);
        if old == new {
            return;
        }
        let (Some(a), Some(b)) = (old, new) else { return };
        let (ra, rb) = (&self.net.roads[a], &self.net.roads[b]);
        let Some(node) = [ra.start, ra.end].into_iter().find(|n| *n == rb.start || *n == rb.end) else { return };
        self.human_passes(h, node);
        for k in 0..self.players.len() {
            if self.players[k].kind != 0 {
                self.retrace_competitor(k);
            }
        }
    }

    /// `0x487500` between two spots: the route cost (mode 0, or mode 1 when that costs less
    /// than a tenth).
    pub(crate) fn spot_cost(&self, from: &Spot, to: &Spot) -> f64 {
        let mut d = self.net.route_from(from, to, 0).map_or(0.0, |(_, c)| c);
        if let Some((_, c)) = self.net.route_from(from, to, 1) {
            if c * 10.0 < d {
                d = c;
            }
        }
        d
    }
}
