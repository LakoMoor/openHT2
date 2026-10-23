//! The truckers' companies (`updateCompanies`, `0x429930`; `ai_play.cpp`): a rich trucker with a
//! licence hires a free one, who then works for him.

use crate::world::World;

impl World {
    /// `0x418f70`: player `k` may be hired: a trucker with a vehicle, in no company (neither
    /// working for one nor having one) and not hired out.
    pub fn can_hire(&self, k: usize) -> bool {
        let p = &self.players[k];
        p.vehicle.is_some() && p.boss.is_none() && p.members.is_empty() && p.kind == 2 && p.state != 0x101
    }

    /// `0x477350`: the pay of player `k` per unit: (the human player's share · 4 + 1) · the price
    /// of its vehicle / 1024.
    pub fn salary(&self, k: usize) -> f64 {
        let f = self.human_share() * 4.0 + 1.0;
        match self.players[k].vehicle {
            Some(v) => f * self.vehicles[v].price * 0.0009765625,
            None => 0.0,
        }
    }

    /// `0x473600`: what the vehicle of player `k` is worth: its price (×0.65 and ×0.45 with the
    /// flags 1 and 2 of the vehicle, `+0x28`) and its parts (`0x476980`, none on the AI's).
    pub fn vehicle_value(&self, k: usize) -> f64 {
        let p = &self.players[k];
        let Some(v) = p.vehicle else { return 0.0 };
        let mut x = self.vehicles[v].price;
        if p.vehicle_flags & 1 != 0 {
            x *= 0.65;
        }
        if p.vehicle_flags & 2 != 0 {
            x *= 0.45;
        }
        x
    }

    /// `updateCompanies` (`0x429930`, every 30 s). Truckers in a company (having one or working
    /// for one, `0x429380`) should be (truckers wanted − hired out) · min(1, t / 42000), whole
    /// (`0x429340`). While fewer are, the richest trucker with a licence and no company founds
    /// one (while there are fewer than `0x66c700` = 7 companies, not counting the human
    /// player's; else the richest that has one hires again), and a free trucker is drawn
    /// (`0x45ef70`) to work for him (`0x476bc0`). When at most one trucker is free, the
    /// employees of the company worth least (`0x429690`) would quit (not ported: it does not
    /// happen with the truckers of a map).
    pub fn update_companies(&mut self, now: f64) {
        let have = self.players.iter().filter(|p| p.kind == 2 && (!p.members.is_empty() || p.boss.is_some())).count() as i64;
        let mut f = (self.truckers_want as i64 - self.hired_want as i64) as f64;
        if now < 42000.0 {
            f *= now / 42000.0;
        }
        let want = f as i64;
        if have < want {
            let mut best = None;
            let mut best_money = 0.0;
            let human_company = self.players.iter().any(|p| p.kind == 1 && !p.members.is_empty());
            let companies = self.companies as i64 - i64::from(human_company);
            let founding = companies < self.company_cap as i64;
            for (k, p) in self.players.iter().enumerate() {
                let fits = if founding { p.members.is_empty() && p.boss.is_none() } else { !p.members.is_empty() };
                if !p.licences.is_empty() && best_money < p.money && p.kind == 2 && p.state != 0x101 && fits {
                    best_money = p.money;
                    best = Some(k);
                }
            }
            if let Some(b) = best {
                let picks: Vec<usize> = (0..self.players.len()).filter(|&k| self.players[k].state != 0x100 && self.players[k].state != 0x101 && self.can_hire(k)).collect();
                if !picks.is_empty() {
                    let e = picks[self.rng.next() as usize % picks.len()];
                    self.hire(b, e, now);
                }
            }
        }
        // 0x419000: the free truckers; with at most one the least worth company would break up
    }

    /// `0x476bc0`: trucker `b` hires trucker `e` (neither the other, neither working for anyone,
    /// `e` not his already, `b` with a licence). `e`'s pay (`0x477350`) is kept (`+0x358`); when
    /// ten times it and what `e`'s vehicle is worth (`0x473670`) is less than `b`'s money, `b`
    /// pays it: `e` gets it, ten times the pay as an advance too (`+0xb8`); `e` works for `b`
    /// (`+0xa8`, in `b`'s list `+0x98`), `b` gives up a licence (`0x4773e0`: the soonest to run
    /// out), `e` is no longer hired out (`0x419760`). With his first employee `b`'s company is
    /// founded (`0x476a30`: one more, and a logo of the eight drawn, `next() & 7`, until a free
    /// one, at most 100 times, else the first free one).
    pub fn hire(&mut self, b: usize, e: usize, now: f64) -> bool {
        if b == e || self.players[e].boss.is_some() || self.players[b].boss.is_some() {
            return false;
        }
        if self.players[b].members.contains(&e) || self.players[e].members.contains(&b) {
            return false;
        }
        if self.players[b].licences.is_empty() {
            return false;
        }
        let s = self.salary(e);
        self.players[e].salary = s;
        let v = self.vehicle_value(e);
        let cost = self.players[e].salary * 10.0 + v;
        if !(cost < self.players[b].money) {
            return false;
        }
        self.players[b].money -= cost;
        let pe = &mut self.players[e];
        pe.money = pe.salary * 10.0 + pe.money + v;
        pe.advance = pe.salary * 10.0 + pe.advance;
        self.players[b].members.push(e);
        // 0x4773e0: sorted latest first, the last goes
        let lic = &mut self.players[b].licences;
        lic.sort_by(|x, y| y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal));
        lic.pop();
        self.players[e].boss = Some(b);
        if self.players[e].state == 0x101 {
            self.cancel_hiring(e, now);
        }
        self.players[e].company_balance = 0.0;
        // 0x477210: 0x800 works for a company, 0x1000 has employees
        self.players[e].flags |= 0x800;
        self.players[b].flags |= 0x1000;
        if self.players[b].members.len() == 1 {
            self.companies += 1;
            if self.players[b].logo.is_none() {
                for _ in 0..100 {
                    let l = (self.rng.next() & 7) as usize;
                    if !self.logos[l] {
                        self.logos[l] = true;
                        self.players[b].logo = Some(l);
                        break;
                    }
                }
                if self.players[b].logo.is_none() && !self.logos[0] {
                    self.logos[0] = true;
                    self.players[b].logo = Some(0);
                }
            }
        }
        true
    }

    /// `0x477420` (each tick, after `processMove`): the licences of player `k` that have run out
    /// go.
    pub fn expire_licences(&mut self, k: usize, now: f64) {
        self.players[k].licences.retain(|&t| now <= t);
    }
}
