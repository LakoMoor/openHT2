//! Vehicle condition score used by police activity (`0x557090`).
use crate::mem::Mem;

/// Original component damage ratios, stopped state and +0x224/+0x5080 gates.
/// Explicit stores reproduce x87 f32 rounding between component products.
pub fn fn_00557090(m:&Mem,vehicle:u32)->f32 {
    let mut score=m.i32(vehicle+0x5080) as f32;
    let state=m.f32(vehicle+0x224);
    // x87 comparison branches also take the unordered path.
    if !(state>=0.5) {score=if !(state>=0.) {0.}else{state+state};}
    let upper=if m.u32(m.u32(vehicle+0x5454)+12)!=0 {0.}else{1.};
    if !(upper>=score) {score=upper;}
    let parts=m.u32(vehicle+0x2540);
    let mut product=1f32;
    for (i,weight) in [0.1,0.5,0.5,0.5,0.6,0.6,0.3,0.3,0.3].into_iter().enumerate() {
        let base=parts+i as u32*20;
        let candidate=1.-(m.i32(base+4) as f64/m.i32(base+8) as f64).abs()*weight;
        let stored=candidate as f32;
        if !(candidate>=score as f64) {score=stored;}
        product=(((stored as f64+1.)*product as f64)*0.5) as f32;
    }
    if !(product>=score) {score=product;}
    if !(score>=0.) {score=0.;}
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_police_activity_and_radar_phase() {
        for (case,line) in include_str!("../../../tools/emu/golden_police_activity.tsv").lines().enumerate() {
            let p:Vec<_>=line.split('\t').collect();let inputs:Vec<u32>=p[0].split(',').map(|v|v.parse().unwrap()).collect();
            let values:Vec<u32>=p[1].split(',').map(|v|v.parse().unwrap()).collect();
            let mut m=Mem::new();let vehicle=0x2a000000;let police=vehicle+0x10;
            let player=vehicle+0x10000;let parts=vehicle+0x20000;let state=vehicle+0x30000;let driver=vehicle+0x40000;
            m.set_u32(vehicle+0x5080,inputs[0]);m.set_u32(vehicle+0x224,inputs[1]);
            m.set_u32(vehicle+0x5454,state);m.set_u32(state+12,inputs[2]);m.set_u32(vehicle+0x2540,parts);
            for i in 0..9 {m.set_u32(parts+i*20+4,inputs[3+i as usize*2]);m.set_u32(parts+i*20+8,inputs[4+i as usize*2]);}
            m.set_u32(police+0x5450,driver);m.set_u32(police+0x5568,values[0]);
            let addresses=[police+0x5594,driver+0x4074,driver+0x4068,player+0x51d8];
            for (address,value) in addresses.into_iter().zip(&values[2..]){m.set_u32(address,*value);}
            crate::instruments::police_radar_phase(&mut m,police,player,values[1],p[2].parse().unwrap());
            assert_eq!(addresses.map(|a|m.u32(a)).to_vec(),p[3].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case}");
        }
    }
    #[test]
    fn original_vehicle_condition_bits() {
        for (case,line) in include_str!("../../../tools/emu/golden_vehicle_condition.tsv").lines().enumerate() {
            let p:Vec<_>=line.split('\t').collect();let inputs:Vec<u32>=p[0].split(',').map(|v|v.parse().unwrap()).collect();
            let mut m=Mem::new();let vehicle=0x2a000000;let parts=vehicle+0x10000;let state=parts+0x10000;
            m.set_u32(vehicle+0x5080,inputs[0]);m.set_u32(vehicle+0x224,inputs[1]);
            m.set_u32(vehicle+0x5454,state);m.set_u32(state+12,inputs[2]);m.set_u32(vehicle+0x2540,parts);
            for i in 0..9 {m.set_u32(parts+i*20+4,inputs[3+i as usize*2]);m.set_u32(parts+i*20+8,inputs[4+i as usize*2]);}
            let actual=fn_00557090(&m,vehicle);
            assert_eq!(actual.to_bits(),p[1].parse::<u32>().unwrap(),"case {case}");
            assert_eq!((actual as f64).to_bits(),p[2].parse::<u64>().unwrap(),"case {case} return precision");
        }
    }
}
