//! king.exe 0x41b3a0 observations; regenerate with tools/emu/gen_illegal_order_golden.py.
use ht2_sim::{objects::{Merchandise, TownOrder}, road::{AiRng, RoadNet}, world::{Categories, World}};
use std::{rc::Rc, str::{FromStr, SplitWhitespace}};

fn take<T: FromStr>(s: &mut SplitWhitespace<'_>) -> T {
    s.next().unwrap().parse().ok().unwrap()
}

#[test]
fn illegal_order_matches_original() {
    let mut substitutions = 0;
    for (case, line) in include_str!("data/illegal_order_golden.txt").lines().enumerate() {
        let mut s = line.split_whitespace();
        let share = take(&mut s);
        let slot = take(&mut s);
        let n: usize = take(&mut s);
        let flags: Vec<u32> = (0..n).map(|_| take(&mut s)).collect();
        let table = std::array::from_fn(|_| take(&mut s));
        let mut w = World::new(Rc::new(RoadNet::default()), vec![], vec![], Categories::default(), vec![], AiRng::from_state(table, 0, 31), 0);
        w.create_players_regular(&[0; 12], None);
        w.players[0].share = share;
        w.merchandise = flags.into_iter().map(|flags| Merchandise { name: String::new(), flags, unit_weight: 1.0, unit_area: 1.0, share: 0.0, value: 0.0 }).collect();
        // The original selects the deadline immediately before the substitution branch.
        let deadline = (w.rng.next() % 300 + 200) as f64 + 1234.5;
        let mut order = TownOrder { merchandise: 0, illegal: None, legal: None, to: Some(1), deadline, fee: 54321.0 * 0.1 };
        w.slip_illegal_merchandise(&mut order, slot);
        let got = [order.merchandise as i32, order.illegal.map_or(-1, |m| m as i32), order.legal.map_or(-1, |m| m as i32), order.to.map_or(-1, |t| t as i32)];
        let expected = std::array::from_fn::<_, 4, _>(|_| take::<i32>(&mut s));
        assert_eq!(got, expected, "case {case}");
        substitutions += usize::from(order.illegal.is_some());
        assert_eq!(order.deadline, take::<f64>(&mut s), "deadline case {case}");
        assert_eq!(order.fee, take::<f64>(&mut s), "fee case {case}");
        let i = take(&mut s);
        let j = take(&mut s);
        let expected_table = std::array::from_fn(|_| take(&mut s));
        assert_eq!(format!("{:?}", w.rng), format!("{:?}", AiRng::from_state(expected_table, i, j)), "RNG case {case}");
        assert!(s.next().is_none());
    }
    assert_eq!(substitutions, 18);
}
