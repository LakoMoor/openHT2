//! Numeric king.exe 0x41fcf0 observations from tools/emu/gen_rating_golden.py.
use ht2_sim::{road::{AiRng, RoadNet}, world::{Categories, Vehicle, World}};
use std::{rc::Rc, str::{FromStr, SplitWhitespace}};
fn take<T: FromStr>(s: &mut SplitWhitespace<'_>) -> T { s.next().unwrap().parse().ok().unwrap() }
fn close(got: f64, want: f64) { assert!((got - want).abs() <= 1e-12 * want.abs().max(1.0), "{got} != {want}"); }

#[test]
fn rating_companies_and_victory_match_original() {
    for (case, line) in include_str!("data/rating_golden.txt").lines().enumerate() {
        let mut s = line.split_whitespace();
        let n: usize = take(&mut s);
        let threshold = take(&mut s);
        let mut w = World::new(Rc::new(RoadNet::default()), vec![], vec![], Categories::default(), vec![], AiRng::default(), 0);
        w.create_players_regular(&[0; 12], None);
        let prototype = w.players[0].clone();
        w.players = vec![prototype; n];
        w.victory_share = threshold;
        for k in 0..n {
            let kind = take(&mut s);
            let cap: f32 = take(&mut s);
            let boss: i32 = take(&mut s);
            let p = &mut w.players[k];
            p.kind = kind;
            p.vehicle = Some(k);
            p.boss = (boss >= 0).then_some(boss as usize);
            p.earned = take(&mut s);
            p.earning = take(&mut s);
            p.share = take(&mut s);
            p.fleet = take(&mut s);
            p.won = take::<u8>(&mut s) != 0;
            let key = format!("V{k}");
            w.vehicles.push(Vehicle { id: key.clone(), model: key.clone(), colour: 0, kind: 2, roles: vec![], price: 0.0, power: 0.0, trailers: vec![], liquid: false });
            // Synthetic vehicle.tech record: only the model and capacity matter here.
            let mut bytes = vec![0; 57 + 2416];
            bytes[0..4].copy_from_slice(&4u32.to_le_bytes());
            bytes[12..16].copy_from_slice(&2416u32.to_le_bytes());
            bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
            bytes[57 + 2272..57 + 2272 + key.len()].copy_from_slice(key.as_bytes());
            bytes[57 + 272..57 + 276].copy_from_slice(&cap.to_le_bytes());
            bytes[57 + 2342..57 + 2342 + key.len()].copy_from_slice(key.as_bytes());
            w.tech.extend(ht2_formats::tech::parse(&bytes).unwrap());
        }
        for k in 0..n {
            if let Some(b) = w.players[k].boss { w.players[b].members.push(k); }
        }
        for repeat in 0..3 {
            w.update_rating();
            assert_eq!(w.victory_share, take::<f64>(&mut s), "case {case}/{repeat}");
            assert_eq!(w.expanded_dealer_vehicles, take::<u8>(&mut s) != 0, "global case {case}/{repeat}");
            let events: usize = take(&mut s);
            let expected: Vec<usize> = (0..events).map(|_| take(&mut s)).collect();
            assert_eq!(w.victory_events, expected, "events case {case}/{repeat}");
            for p in &w.players {
                close(p.earning, take(&mut s));
                assert_eq!(p.earned, take::<f64>(&mut s));
                close(p.share, take(&mut s));
                close(p.fleet, take(&mut s));
                assert_eq!(p.won, take::<u8>(&mut s) != 0, "won case {case}/{repeat}");
            }
        }
        assert!(s.next().is_none());
    }
}
