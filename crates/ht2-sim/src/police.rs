//! The police and the mafia after the human player (`Ai_main.cpp`): what makes him wanted
//! (offsets of his list node: `+0xd0` by the police, `+0xe0` by the mafia, `+0xf0`, `+0xf8`,
//! `+0x100`, and the flags `+0xc8` something to hide, `+0xc9` of interest to the mafia, `+0x108`
//! the alarm), and what the AI does about it when he passes a node (`0x40b100`): the two
//! nearest police cars (`copsOnHim`, `0x40a690`) or mafia cars (`mafiaOnHim`, `0x40aa00`) go for
//! his place. Crossing a state border (a node event `STATEBORDER`, flag 0x10, `0x40ae80`) clears
//! what the police have on him.

use crate::world::World;

impl World {
    /// `accountForGuilt` (`0x442850`), after cargo changes. The flags accumulate across loads
    /// until a random draw clears them; this deliberately does not test each load separately.
    /// Merchandise flag 2 matters to the mafia only when the human player's share exceeds
    /// 0.15 (`0x4297f0`), even when this function is called for an AI player.
    pub fn account_for_guilt(&mut self, k: usize) {
        let interested = self.human_share() > 0.15;
        let mut hidden = false;
        let mut mafia = false;
        let mut decision = -1;
        for load in &mut self.players[k].loads {
            if let Some(m) = load.illegal {
                hidden = true;
                if self.merchandise[m].flags & 2 != 0 && interested {
                    mafia = true;
                }
            }
            if self.merchandise[load.merchandise].flags & 2 != 0 && interested {
                mafia = true;
            }
            if hidden {
                if self.rng.next() % 1_000_000 < 200_000 {
                    hidden = false;
                    mafia = false;
                } else if mafia {
                    if load.mafia_decision == -1 {
                        self.rng.next();
                        load.mafia_decision = 0;
                    }
                    decision = load.mafia_decision;
                }
            } else if mafia {
                let threshold = if 10000.0 <= load.price { 200_000 } else { 500_000 };
                if self.rng.next() % 1_000_000 < threshold {
                    mafia = false;
                }
            }
        }
        if hidden && mafia {
            if decision == 1 {
                hidden = false;
            } else {
                mafia = false;
            }
        }
        let p = &mut self.players[k];
        p.hide = hidden;
        p.for_mafia = mafia;
        p.alarmed = p.vehicle_flags & 2 != 0 && self.rng.next() % 1_000_000 < 500_000;
    }

    /// `0x56f120` (the game): the human player's vehicle did something the police count (its
    /// object in a room whose `+0x1a0` is 0, both in state 2): `+0xd0` grows by `value` (6200 in
    /// the game) and flag 0x20 of `+0xd8` is set.
    pub fn offence(&mut self, value: f64) {
        let Some(h) = self.players.iter().position(|p| p.kind == 1) else { return };
        let p = &mut self.players[h];
        p.police_score += value;
        p.offence_flags |= 0x20;
    }

    /// `0x40b100` for the human player passing node `n` in a game: a state border there
    /// (`STATEBORDER`) clears what the police have on him (`0x40ae80`); when they have something, the
    /// police go for him (`0x40a690`); when the mafia have, the mafia (`0x40aa00`). (The message
    /// of a node event the way he goes, and the race finish, are not ported.)
    pub(crate) fn human_passes(&mut self, h: usize, n: usize) {
        if self.border.get(n).copied().unwrap_or(false) {
            self.wanted_reset(h);
        }
        let p = &self.players[h];
        if 0.0 < p.police_score || p.hide || 0.0 < p.scores[0] || 0.0 < p.scores[1] || 0.0 < p.scores[2] || p.alarmed {
            self.cops_on_him(h);
        }
        let p = &self.players[h];
        if 0.0 < p.mafia_score || p.for_mafia {
            self.mafia_on_him(h);
        }
    }

    /// `0x40ae80` in a game: the police scores of player `k` go (`+0xd0`, `+0xd8`, `+0xf0`,
    /// `+0xf8`, `+0x100`; `0x477210`); when nothing is left against him (`+0xc8`, `+0x108` too),
    /// the police cars after him give up (`+0x3c0`), those within 300 m of the human player
    /// pausing 120 s (`0x477cb0`). The game is told (event 0x12, not ported).
    fn wanted_reset(&mut self, k: usize) {
        let p = &mut self.players[k];
        p.police_score = 0.0;
        p.offence_flags = 0;
        p.scores = [0.0; 3];
        let p = &self.players[k];
        if p.police_score <= 0.0 && !p.hide && p.scores.iter().all(|&s| s <= 0.0) && !p.alarmed {
            let h = self.human_point().unwrap_or([0.0; 3]);
            let now = self.now;
            for c in 0..self.players.len() {
                if self.players[c].kind != 5 || self.players[c].target != Some(k) {
                    continue;
                }
                self.players[c].target = None;
                let q = self.position(&self.players[c]).unwrap_or([0.0; 3]);
                let y = q[1] - h[1];
                if ((q[2] - h[2]) * (q[2] - h[2]) + y * y + (q[0] - h[0]) * (q[0] - h[0])).sqrt() < 300.0 {
                    // 0x477c60
                    let p = &mut self.players[c];
                    p.flags |= 0x800_0000;
                    p.until = now + 120.0;
                }
            }
        }
    }

    /// The players of `kind` that do not stand and are not with an ambush (0x80000), nearest
    /// to the place of player `h` first (`0x45bab0`, `0x45bbc0`).
    fn nearest_of_kind(&self, h: usize, kind: u32) -> Vec<usize> {
        let q = self.players[h].place.map_or([0.0; 3], |s| self.net.spot_coordinates(&s));
        let mut c: Vec<usize> = (0..self.players.len())
            .filter(|&k| {
                let p = &self.players[k];
                p.kind == kind && p.flags & 4 == 0 && p.flags & 0x80000 == 0
            })
            .collect();
        let d: Vec<f64> = (0..self.players.len())
            .map(|k| {
                let r = self.players[k].place.map_or([0.0; 3], |s| self.net.spot_coordinates(&s));
                ((q[0] - r[0]) * (q[0] - r[0]) + (q[2] - r[2]) * (q[2] - r[2]) + (q[1] - r[1]) * (q[1] - r[1])).sqrt()
            })
            .collect();
        crate::stl::sort(&mut c, |a, b| d[*a] < d[*b]);
        c
    }

    /// `0x40a650`: no node of the route is a state border.
    fn route_clear_of_events(&self, k: usize) -> bool {
        !self.players[k].route.iter().any(|s| self.border.get(s.node).copied().unwrap_or(false))
    }

    /// `copsOnHim` (`0x40a690`, not in a race of his): the two police cars nearest to the human
    /// player `h` go for his place (a waypoint of kind 2); one whose route crosses a
    /// state border gives up and goes anywhere else (`0x40a2a0`), the others are
    /// after him (`+0x3c0`).
    fn cops_on_him(&mut self, h: usize) {
        if self.players[h].state == 5 {
            return;
        }
        let Some(to) = self.players[h].place else { return };
        let cands = self.nearest_of_kind(h, 5);
        let mut after = 0;
        for &c in cands.iter().take(2) {
            if after < 2 {
                let mut p = self.players[c].clone();
                self.trace_to(&mut p, &to, 2);
                self.players[c] = p;
                if self.debug_arrivals {
                    println!("arrival cops {c} clear {} route {:?} wp {:?}", self.route_clear_of_events(c), self.players[c].route.iter().map(|s| s.node).collect::<Vec<_>>(), self.players[c].waypoints);
                }
                if self.route_clear_of_events(c) {
                    after += 1;
                    self.players[c].target = Some(h);
                } else {
                    let mut p = self.players[c].clone();
                    p.route.clear();
                    self.trace_away(&mut p);
                    self.players[c] = p;
                }
            } else if self.players[c].target == Some(h) {
                self.players[c].target = None;
            }
        }
    }

    /// `mafiaOnHim` (`0x40aa00`, not in a race of his): the two mafia cars nearest to the human
    /// player go for his place (a waypoint of kind 2) and are after him (`+0x3c0`).
    fn mafia_on_him(&mut self, h: usize) {
        if self.players[h].state == 5 {
            return;
        }
        let Some(to) = self.players[h].place else { return };
        let cands = self.nearest_of_kind(h, 6);
        for &c in cands.iter().take(2) {
            let mut p = self.players[c].clone();
            self.trace_to(&mut p, &to, 2);
            p.target = Some(h);
            self.players[c] = p;
        }
    }
}
