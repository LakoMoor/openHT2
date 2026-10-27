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
    /// (`0x45ef70`) to work for him (`0x476bc0`). When at most one trucker is free (`0x419000`),
    /// the company worth least (`0x429690`) breaks up: its employees leave (`0x476f70`, the last
    /// first), its owner keeps the money he had, and there may be one company less (`0x66c700`,
    /// not below 1); an owner who works for someone himself leaves him instead.
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
        let free = (0..self.players.len()).filter(|&k| self.players[k].state != 0x100 && self.players[k].state != 0x101 && self.can_hire(k)).count();
        if 1 < free {
            return;
        }
        // 0x429690: of the owners other than the human player, the one worth least
        let mut worst = None;
        let mut least = f64::MAX;
        for k in 0..self.players.len() {
            if self.players[k].kind != 1 && !self.players[k].members.is_empty() {
                let c = self.capital(k);
                if c < least {
                    least = c;
                    worst = Some(k);
                }
            }
        }
        let Some(w) = worst else { return };
        let money = self.players[w].money;
        match self.players[w].boss {
            None => {
                while let Some(&e) = self.players[w].members.last() {
                    if !self.leave(w, e) {
                        break;
                    }
                }
                self.players[w].money = money;
                if 1 < self.company_cap {
                    self.company_cap -= 1;
                }
            }
            Some(b) => {
                self.leave(b, w);
            }
        }
    }

    /// `getPlayerCapital` (`0x429530`): what an owner and his employees' vehicles are worth
    /// (`0x473680`: the vehicle less its damage in the scene · 5000; no damage here).
    pub fn capital(&self, k: usize) -> f64 {
        let mut c = self.vehicle_value(k);
        for &e in &self.players[k].members {
            c = self.vehicle_value(e) + c;
        }
        c
    }

    /// `0x476f70`: employee `e` leaves `b`: he works for no one, his account starts again, and
    /// `b` buys his vehicle back (`0x473670`; what `e` cannot pay, `b` does not get). With no
    /// employee left `b`'s company is gone (`0x476b50`: one less, its logo free).
    pub fn leave(&mut self, b: usize, e: usize) -> bool {
        if b == e || self.players[b].members.is_empty() || self.players[e].boss.is_none() {
            return false;
        }
        let Some(i) = self.players[b].members.iter().position(|&m| m == e) else { return false };
        self.players[b].members.remove(i);
        self.players[e].boss = None;
        self.players[e].company_balance = 0.0;
        let v = self.vehicle_value(e);
        self.players[e].money -= v;
        let before = self.players[b].money;
        self.players[b].money = v + before;
        if self.players[e].money < 0.0 {
            self.players[b].money = v + before + self.players[e].money;
            self.players[e].money = 0.0;
        }
        self.refresh_represented_flags(b);
        self.refresh_represented_flags(e);
        if self.players[b].members.is_empty() {
            self.companies = self.companies.saturating_sub(1);
            if self.players[b].kind != 1 {
                if let Some(l) = self.players[b].logo.take() {
                    self.logos[l] = false;
                }
            }
        }
        true
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
        // 0x477210 refreshes both players after the membership changes.
        self.refresh_represented_flags(b);
        self.refresh_represented_flags(e);
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
