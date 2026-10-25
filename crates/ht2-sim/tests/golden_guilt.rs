//! Numeric observations of king.exe 0x442850/0x42c0b0 from tools/emu/gen_guilt_golden.py.
//! Covers share predicates, cargo on a vehicle/trailer, sticky decisions, ambushes and RNG.
use ht2_sim::{objects::Merchandise, orders::Load, road::{AiRng, RoadNet}, world::{Ambush, Categories, World}};
use std::{rc::Rc, str::{FromStr, SplitWhitespace}};

fn take<T: FromStr>(words: &mut SplitWhitespace<'_>) -> T {
    words.next().expect("fixture field").parse().ok().expect("fixture number")
}

fn rng(words: &mut SplitWhitespace<'_>) -> AiRng {
    let i = take(words);
    let j = take(words);
    let table = std::array::from_fn(|_| take::<u32>(words));
    AiRng::from_state(table, i, j)
}

#[test]
fn account_for_guilt_matches_original() {
    check(false);
}

#[test]
fn load_change_uses_guilt_and_marks_ambushes() {
    check(true);
}

fn check(through_load_change: bool) {
    for (case, line) in include_str!("data/guilt_golden.txt").lines().enumerate() {
        let mut words = line.split_whitespace();
        let share: f64 = take(&mut words);
        let flags: u32 = take(&mut words);
        let tow: u32 = take(&mut words);
        let own: u32 = take(&mut words);
        let score: f64 = take(&mut words);
        let count: usize = take(&mut words);
        let table = std::array::from_fn(|_| take::<u32>(&mut words));
        let mut w = World::new(Rc::new(RoadNet::default()), vec![], vec![], Categories::default(), vec![], AiRng::from_state(table, 0, 31), 0);
        w.create_players_regular(&[0; 12], None);
        w.players[0].share = share;
        w.players[0].mafia_score = score;
        let k = if own != 0 { 0 } else {
            let mut ai = w.players[0].clone();
            ai.kind = 2;
            // The predicate must use the human share, even for another player's cargo.
            ai.share = if share > 0.15 { 0.0 } else { 0.51 };
            w.players.push(ai);
            1
        };
        // Loads are stored on the player in this port; the generator checks both carrier
        // choices (including an invalid trailer falling back to the vehicle) in the original.
        w.players[k].trailer = if tow == 1 { Some(0) } else { None };
        w.players[k].vehicle_flags = flags;
        w.players[k].hide = true;
        w.players[k].for_mafia = true;
        w.players[k].alarmed = true;
        // Isolate the load-change call from the separately ported competitor logic.
        w.players[k].state = 5;
        w.ambushes = [1, 2, 0, 1, 2, 2, 1].into_iter().map(|kind| Ambush { id: String::new(), kind, active: true, node: String::new(), facing: String::new(), objects: vec![], links: vec![], pos: [0.0; 3], dir: [0.0; 3], place: None, facing_place: None, set: false, player: None }).collect();
        w.merchandise = (0..4).map(|flags| Merchandise { name: String::new(), flags, unit_weight: 1.0, unit_area: 1.0, share: 0.0, value: 0.0 }).collect();
        for _ in 0..count {
            let merchandise = take(&mut words);
            let illegal: i32 = take(&mut words);
            let price = take(&mut words);
            let mafia_decision = take(&mut words);
            w.players[k].loads.push(Load { merchandise, illegal: (illegal >= 0).then_some(illegal as usize), legal: None, to: 0, from: 0, at: 0, amount: 1.0, price, deadline: 0.0, loaded: 0.0, time: 0.0, condition: 1.0, kind: 1, cost: 0.0, mafia_decision });
        }
        for repeat in 0..3 {
            if through_load_change {
                w.account_for_load_change(k);
            } else {
                w.account_for_guilt(k);
            }
            let expected = std::array::from_fn::<_, 3, _>(|_| take::<u8>(&mut words) != 0);
            let p = &w.players[k];
            assert_eq!([p.hide, p.for_mafia, p.alarmed], expected, "case {case}, call {repeat}");
            for load in &p.loads {
                assert_eq!(load.mafia_decision, take::<i32>(&mut words), "case {case}, call {repeat}");
            }
            let guilt_rng = rng(&mut words);
            if !through_load_change {
                assert_eq!(format!("{:?}", w.rng), format!("{guilt_rng:?}"), "case {case}, call {repeat}: guilt generator");
                w.mark_ambushes();
            }
            for a in &w.ambushes {
                assert_eq!(a.active, take::<u8>(&mut words) != 0, "case {case}, call {repeat}: ambush type {}", a.kind);
            }
            let ambush_rng = rng(&mut words);
            assert_eq!(format!("{:?}", w.rng), format!("{ambush_rng:?}"), "case {case}, call {repeat}: ambush generator");
        }
        assert!(words.next().is_none(), "extra fixture fields in case {case}");
    }
}
