//! Cabin needle transforms from the cabin branch of `0x53d1f0`.
use crate::{mem::Mem,matrix};

/// 0x4d13eb..0x4d1492: police position transformed into the human vehicle's
/// local frame, then its original vector length. Both matrices are supplied
/// by the owning scene and must use the original 12-float layout.
pub fn police_relative_distance(m:&mut Mem,human:[f32;12],police:[f32;12])->f64 {
    const BASE:u32=0x2ffd_4700;let inverse=BASE+48;let point=BASE+96;
    matrix::put12(m,BASE,&human);crate::math::mat_inverse(m,inverse,BASE);
    let x=police[9] as f64;let y=police[10] as f64;let z=police[11] as f64;
    for axis in 0..3u32 {
        let value=((m.f32(inverse+24+axis*4) as f64*z+m.f32(inverse+12+axis*4) as f64*y)+m.f32(inverse+axis*4) as f64*x)+m.f32(inverse+36+axis*4) as f64;
        m.set_f32(point+axis*4,value as f32);
    }
    crate::math::vec_len(m,point)
}

/// Radar-emission branch `0x4d14b9..0x4d1526` of police update `0x4d1360`.
/// Caller supplies the already computed distance and flags from 0x43f620,
/// after the original health/activity gate. This assigns, rather than maximizes,
/// the player's signal; equipment ownership is checked later by the viewer.
pub fn police_radar_signal(m:&mut Mem,police:u32,player:u32,flags:u32,distance:f64) {
    if m.i32(police+0x5568)==5 && flags&0x80000!=0 && !(distance>=100.) {
        m.set_f32(player+0x51d8,(distance*0.01) as f32);
    }
}

/// Activity and radar phase of `0x4d1360`, starting at 0x4d1492. `police`
/// is the subobject at vehicle+0x10. Geometry/distance gathering is caller-owned.
pub fn police_radar_phase(m:&mut Mem,police:u32,player:u32,flags:u32,distance:f64) {
    let driver=m.u32(police+0x5450);
    m.set_u32(driver+0x4068,m.u32(driver+0x4068)&!2);
    if !(crate::condition::fn_00557090(m,police-0x10) as f64>0.3) {
        m.set_u32(police+0x5594,0);m.set_u32(driver+0x4074,0);
        m.set_u32(driver+0x4068,m.u32(driver+0x4068)|1);
    }else{police_radar_signal(m,police,player,flags,distance);}
}

/// Scene-owner bridge for 0x4d13eb..0x4d1526. The caller supplies both world
/// matrices, the police subobject and the player's scene vehicle; those records
/// must already contain the original health, driver and police-type fields.
pub fn police_radar_from_scene(m:&mut Mem,police:u32,player:u32,flags:u32,human_pose:[f32;12],police_pose:[f32;12]) {
    let distance=police_relative_distance(m,human_pose,police_pose);
    police_radar_phase(m,police,player,flags,distance);
}

/// Radar prefix of viewer update `0x538370`, through signal reset at 0x538529.
/// Sound is returned as gain; geometry commands retain their original node IDs.
pub fn radar_panel(m:&mut Mem,viewer:u32)->(Vec<KeyCommand>,Option<f32>) {
    let vehicle=m.u32(viewer+0x268);
    if vehicle==0 {return (vec![],None);}
    let mut commands=Vec::new();let mut sound=None;
    let mut key=|offset,value|commands.push(KeyCommand{node:m.u32(viewer+offset),value,digit:false});
    if m.u32(vehicle+0x5400)==0 {key(0x1b8,0);}else{
        key(0x1b8,1);
        let signal=m.f32(vehicle+0x51d8);
        if !(signal>0.) {key(0x1bc,0);}else{
            let strength=signal.min(1.) as f64;
            let level=(1-(-3.*strength).trunc() as i32).clamp(1,3);
            let clock=m.u32(0x696ca0);let ticks=m.i32(clock+0xbb0);
            let mut start=m.i32(viewer+0x1c0);let mut period=m.i32(viewer+0x1c4);
            if ticks>=start.wrapping_add(period) {
                start=ticks;m.set_i32(viewer+0x1c0,start);
                let interval=(1./(strength*5.+2.)) as f32 as f64;
                let quotient=interval/crate::globals::dt_hi(m);
                // MSVC _ftol returns i64; callers here retain only EAX.
                period=if quotient.is_finite() && quotient>=i64::MIN as f64 && quotient<9223372036854775808. {
                    quotient.trunc() as i64 as i32
                }else{0};
                m.set_i32(viewer+0x1c4,period);
            }
            let middle=start.wrapping_add(period/2);
            let on=if period&1==0 {ticks<middle}else{ticks<=middle};
            if on {sound=Some(if m.u32(viewer+0x578)!=0 {0.5}else{1.});}
            commands.push(KeyCommand{node:m.u32(viewer+0x1bc),value:if on {level}else{0},digit:false});
        }
    }
    m.set_u32(vehicle+0x51d8,0);
    (commands,sound)
}
/// Layout table writes performed by the four profile reads in `0x4dfb67`.
pub fn install_view_tables(m:&mut Mem,tables:&ht2_formats::view_config::ViewTables) {
    for (base,values) in [0x696bf0,0x696c04,0x696c18,0x696c2c].into_iter().zip(tables.0){
        for (i,value) in values.into_iter().enumerate(){m.set_i32(base+i as u32*4,value);}
    }
}
/// Rectangle passed to the scene by `0x532330`. Original table indices are
/// signed view modes; arithmetic preserves the original 32-bit wrapping.
pub fn view_rectangle(m:&Mem,scenario:u32)->[i32;4] {
    let surface=m.u32(scenario+0x24);let width=m.i32(surface+0x100);let height=m.i32(surface+0x104);
    let mode=m.i32(scenario+0x574);let shift=(mode as u32).wrapping_mul(4);
    let (x_offset,y_offset)=if mode==0 {(0x560,0x564)}else{(0x558,0x55c)};
    let mut x=m.i32(0x696bf8u32.wrapping_add(shift)).wrapping_add(m.i32(scenario+x_offset));
    let mut top=((height.wrapping_sub(480) as f64*0.2061855670103093
        +m.i32(0x696c0cu32.wrapping_add(shift)) as f64)+m.i32(scenario+y_offset) as f64).trunc() as i64 as i32;
    let mut bottom=m.i32(0x696c20u32.wrapping_add(shift)).wrapping_add(top);
    if width>=780 && height>=580 {top=top.wrapping_add(50);bottom=bottom.wrapping_add(50);}
    if x<0 {x=0;}
    if top<m.i32(0x672d7c){top=m.i32(0x672d7c);}
    if bottom<=top {bottom=top.wrapping_add(1);}
    if bottom>height {bottom=height;}
    if x<0 {x=0;}
    let right=if x<width || x.wrapping_add(1)<width {width}else{x.wrapping_add(1)};
    [x,top,right,bottom]
}
/// `0x4de990`: view transform before cabin suspension. Scenario matrices
/// +0x1d8 and +0x5f0 are supplied by their original owners.
pub fn cabin_view(m:&mut Mem,dst:u32,ignore_tilt:bool) {
    const S:u32=0x2ffd_4900;
    let (result,translation,rotation,tilt,temporary)=(S,S+48,S+96,S+144,S+192);
    matrix::put12(m,result,&matrix::IDENTITY);
    let scenario=m.u32(0x6d2078);
    if scenario==0 || m.u32(scenario+0x268)==0 {
        let bytes=m.read(result,48);m.write(dst,&bytes);return;
    }
    let vehicle=m.u32(scenario+0x268);
    let height=if m.u32(vehicle+0x51a8)!=0 {m.f32(vehicle+0x4f48)}else{0.};
    let up:[f32;3]=core::array::from_fn(|i|m.f32(0x702630+i as u32*4));
    matrix::mat_translation(m,translation,height*up[0],height*up[1],height*up[2]);
    if m.u32(0x6cec68)!=0 {
        let angle=-m.f32(0x696d10);
        matrix::put12(m,tilt,&matrix::IDENTITY);
        if !ignore_tilt {
            let factor=if (angle as f64).abs()<=core::f64::consts::FRAC_PI_2 {(angle as f64).cos()}else{0.};
            matrix::mat_rot_x(m,tilt,(m.f32(scenario+200) as f64*factor) as f32);
        }
        matrix::mat_rot_z(m,rotation,angle);
        matrix::mat_mul(m,temporary,tilt,rotation);
        matrix::mat_mul(m,result,temporary,translation);
    }else{
        let angle=match m.i32(scenario+0x574){1=>Some(0x3f060a92),2=>Some(0x3fc90fdb),-2=>Some(0xbfc90fdb),-1=>Some(0xbf3ba866),_=>None};
        if let Some(bits)=angle {
            matrix::mat_rot_z(m,rotation,f32::from_bits(bits));
            matrix::mat_mul(m,result,rotation,translation);
        }else {let bytes=m.read(translation,48);m.write(result,&bytes);}
        matrix::mat_rot_x(m,tilt,(height as f64 * -0.25) as f32);
        matrix::mat_mul(m,temporary,scenario+0x1d8,tilt);
        matrix::mat_mul(m,result,temporary,result);
    }
    matrix::mat_mul(m,dst,scenario+0x5f0,result);
}
#[derive(Debug,PartialEq,Eq)]
pub struct KeyCommand {pub node:u32,pub value:i32,pub digit:bool}
/// `0x5e1a10`, also the guarded behavior of `0x4e48a2`.
/// `case_count` is the signed result of the node's virtual +0x3c query.
pub fn set_panel_switch(m:&mut Mem,node:u32,key:i32,case_count:i32) {
    if node!=0 && m.u32(node+0x10)==0x12 && key>=0 && key<case_count {
        m.set_i32(node+0x38,key);
    }
}
/// `0x5e1a40`: wrong node types and missing nodes report -1.
pub fn panel_switch(m:&Mem,node:u32)->i32 {
    if node==0 || m.u32(node+0x10)!=0x12 {-1} else {m.i32(node+0x38)}
}

#[cfg(test)]
mod panel_switch_tests {
    use super::*;
    #[test]
    fn original_police_relative_distance() {
        for (case,line) in include_str!("../../../tools/emu/golden_police_relative_distance.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let decode=|s:&str|-> [f32;12] {let bytes=(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect::<Vec<_>>();std::array::from_fn(|i|f32::from_le_bytes(bytes[i*4..i*4+4].try_into().unwrap()))};
            let mut m=Mem::new();let actual=police_relative_distance(&mut m,decode(p[0]),decode(p[1]));let expected=f64::from_bits(p[2].parse().unwrap());
            assert!(actual.to_bits().abs_diff(expected.to_bits())<=1,"case {case} distance");
            assert_eq!(actual<100.,expected<100.,"case {case} threshold");assert_eq!((actual*0.01) as f32,(expected*0.01) as f32,"case {case} signal");
        }
    }
    #[test]
    fn original_police_radar_emission() {
        for (case,line) in include_str!("../../../tools/emu/golden_police_radar.tsv").lines().enumerate() {
            let p:Vec<u64>=line.split('\t').map(|v|v.parse::<i128>().unwrap() as u64).collect();
            let mut m=Mem::new();let police=0x2a000000;let player=police+0x10000;
            m.set_u32(police+0x5568,p[0] as u32);m.set_u32(player+0x51d8,p[3] as u32);
            police_radar_signal(&mut m,police,player,p[1] as u32,f64::from_bits(p[2]));
            assert_eq!(m.u32(player+0x51d8),p[4] as u32,"case {case}");
        }
    }
    #[test]
    fn police_signal_reaches_panel_then_is_consumed() {
        let mut m=Mem::new();let police=0x2a000000;let player=police+0x10000;
        let viewer=police+0x20000;let clock=police+0x30000;let body=police+0x40000;let lamp=body+0x100;
        m.set_u32(police+0x5568,5);m.set_u32(player+0x5400,1);
        m.set_u32(viewer+0x268,player);m.set_u32(viewer+0x1b8,body);m.set_u32(viewer+0x1bc,lamp);
        m.set_u32(viewer+0x578,1);m.set_u32(viewer+0x1c4,2);
        m.set_u32(body+0x10,0x12);m.set_u32(lamp+0x10,0x12);
        m.set_u32(0x696ca0,clock);crate::globals::install_clock(&mut m,clock,0.,0.1);
        police_radar_signal(&mut m,police,player,0x80000,50.);
        assert_eq!(m.f32(player+0x51d8),0.5);
        let (commands,sound)=radar_panel(&mut m,viewer);
        assert_eq!(sound,Some(0.5));
        for command in commands {set_panel_switch(&mut m,command.node,command.value,if command.node==body {2}else{4});}
        assert_eq!((panel_switch(&m,body),panel_switch(&m,lamp)),(1,2));
        assert_eq!(m.u32(player+0x51d8),0);
        let (commands,sound)=radar_panel(&mut m,viewer);
        assert_eq!(sound,None);
        for command in commands {set_panel_switch(&mut m,command.node,command.value,if command.node==body {2}else{4});}
        assert_eq!((panel_switch(&m,body),panel_switch(&m,lamp)),(1,0));
    }
    #[test]
    fn police_scene_pose_reaches_radar_panel() {
        let mut m=Mem::new();let vehicle=0x2a000000;let police=vehicle+0x10;let player=vehicle+0x10000;let parts=vehicle+0x20000;let state=vehicle+0x30000;let driver=vehicle+0x40000;let viewer=vehicle+0x50000;let clock=vehicle+0x60000;
        m.set_u32(vehicle+0x5080,1);m.set_f32(vehicle+0x224,1.);m.set_u32(vehicle+0x5454,state);m.set_u32(vehicle+0x2540,parts);
        for i in 0..9 {m.set_i32(parts+i*20+8,100);}
        m.set_u32(police+0x5450,driver);m.set_u32(police+0x5568,5);m.set_u32(player+0x5400,1);
        m.set_u32(viewer+0x268,player);m.set_u32(viewer+0x1b8,1);m.set_u32(viewer+0x1bc,2);m.set_u32(viewer+0x1c4,2);
        m.set_u32(0x696ca0,clock);crate::globals::install_clock(&mut m,clock,0.,0.1);
        let human=matrix::IDENTITY;let mut police_pose=matrix::IDENTITY;police_pose[9]=50.;
        police_radar_from_scene(&mut m,police,player,0x80000,human,police_pose);
        assert_eq!(m.f32(player+0x51d8),0.5);
        let (commands,_)=radar_panel(&mut m,viewer);assert!(commands.iter().any(|c|c.node==2 && c.value==2));assert_eq!(m.f32(player+0x51d8),0.);
    }
    #[test]
    fn original_radar_panel_updates() {
        for (case,line) in include_str!("../../../tools/emu/golden_radar_panel.tsv").lines().enumerate() {
            let p:Vec<_>=line.split('\t').collect();let input:Vec<i64>=p[0].split(',').map(|v|v.parse().unwrap()).collect();
            let mut m=Mem::new();let viewer=0x2a000000;let vehicle=viewer+0x10000;let clock=vehicle+0x10000;
            m.set_u32(viewer+0x268,if input[0]!=0 {vehicle}else{0});m.set_u32(vehicle+0x5400,input[1] as u32);
            m.set_u32(viewer+0x578,input[2] as u32);m.set_u32(vehicle+0x51d8,input[3] as u32);
            m.set_u32(clock+0xbb0,input[4] as u32);m.set_u32(viewer+0x1c0,input[5] as u32);m.set_u32(viewer+0x1c4,input[6] as u32);
            m.set_u32(0x696ca0,clock);crate::globals::install_clock(&mut m,clock,0.,p[1].parse().unwrap());
            m.set_u32(viewer+0x1b8,11);m.set_u32(viewer+0x1bc,12);
            let (commands,sound)=radar_panel(&mut m,viewer);
            let output=[m.u32(viewer+0x1c0),m.u32(viewer+0x1c4),m.u32(vehicle+0x51d8)];
            assert_eq!(output.to_vec(),p[2].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case} state");
            assert_eq!(commands.iter().map(|c|format!("{},{}",c.node,c.value)).collect::<Vec<_>>().join(";"),p[3],"case {case} keys");
            assert_eq!(sound.map(|v|v.to_bits().to_string()).unwrap_or_default(),p[4],"case {case} sound");
        }
    }
    #[test]
    fn original_switch_setters_and_getter() {
        for (case,line) in include_str!("../../../tools/emu/golden_panel_switch.tsv").lines().enumerate() {
            let p:Vec<i64>=line.split('\t').map(|v|v.parse().unwrap()).collect();
            let mut m=Mem::new();let storage=0x2a000000;
            let node=if p[0]!=0 {storage} else {0};
            for i in [5,7] {
                m.set_u32(storage+0x10,p[1] as u32);m.set_u32(storage+0x38,p[4] as u32);
                set_panel_switch(&mut m,node,p[3] as i32,p[2] as i32);
                assert_eq!(m.u32(storage+0x38),p[i] as u32,"case {case}");
                assert_eq!(panel_switch(&m,node) as u32,p[i+1] as u32,"case {case}");
            }
        }
    }
}
/// Key commands emitted by the cabin branch of `0x53d1f0`. Missing nodes
/// are skipped; odometer nodes are visited from +0x1338 down to +0x1328.
pub fn cabin_keys(m:&Mem,panel:u32,gear:i32,automatic:i32,park:i32,low_fuel:i32,digits:[i32;5])->Vec<KeyCommand> {
    let mut commands=Vec::new();
    for (offset,value) in [(0x1318,gear),(0x131c,i32::from(automatic==0)),(0x1320,park),(0x1324,low_fuel)] {
        let node=m.u32(panel+offset);if node!=0 {commands.push(KeyCommand{node,value,digit:false});}
    }
    for (i,value) in digits.into_iter().enumerate(){
        let node=m.u32(panel+0x1338-i as u32*4);if node!=0 {commands.push(KeyCommand{node,value,digit:true});}
    }
    commands
}
/// Steering-wheel matrix portion of `0x4decc0`, before cabin suspension motion.
pub fn cabin_steering(m:&mut Mem,base:u32,dst:u32,steering:f32) {
    let angle=(steering as f64*core::f64::consts::FRAC_PI_2) as f32;
    matrix::mat_rot_z(m,0x2ffd_4840,angle);
    matrix::mat_mul(m,dst,0x2ffd_4840,base);
}
/// Source selection in 0x53d601..0x53d69a, with the optional 0x4ff5b0
/// override supplied by the owner. Scene and physical records are distinct.
pub fn dashboard_speed(m:&mut Mem,scene:u32,mut override_speed:impl FnMut(&mut Mem,u32)->f32)->f32 {
    let car=m.u32(scene+0x5460);
    if car==0 {return m.f32(scene+0x51e4);}
    let aux=m.u32(scene+0x5464);
    if aux!=0 {return crate::math::vec_len(m,aux+0x64) as f32;}
    if m.u32(car+0x3344)!=0 {return crate::math::vec_len(m,scene+0xcf4) as f32;}
    let speed=m.f32(car+if m.u32(car+0x3464)!=0 {0x3328}else{0x3330});
    if m.u32(0x6d20ec)!=0 {override_speed(m,m.u32(scene+0x5204))}else{speed}
}
/// `0x4ff5b0`: speed override using the physical vehicle's tech record.
pub fn dashboard_speed_override(m:&Mem,car:u32,gear:u32)->f32 {
    let rec=crate::gearbox::tech_record(m,m.i32(car+0x8cc));
    crate::curves::fn_00507550(crate::TechRec::new(&rec),gear as i32,m.f32(car+0x2640)).0
}
/// Complete source selection when the owning scene record is available.
pub fn dashboard_scene_speed(m:&mut Mem,scene:u32)->f32 {
    let car=m.u32(scene+0x5460);
    dashboard_speed(m,scene,|m,gear|dashboard_speed_override(m,car,gear))
}
/// Physical-car branch usable before the owning scene record is wired up.
/// None means the original requires a scene velocity or speed override.
pub fn dashboard_physical_speed(m:&Mem,car:u32)->Option<f32> {
    if m.u32(car+0x3344)!=0 || m.u32(0x6d20ec)!=0 {return None;}
    Some(m.f32(car+if m.u32(car+0x3464)!=0 {0x3328}else{0x3330}))
}
pub fn cabin_needles(m:&mut Mem,bases:[u32;3],destinations:[u32;3],speed:f32,rev:f32,fuel_fraction:f32) {
    let angles=[rev*f32::from_bits(0x3dc104fb),speed.abs()*f32::from_bits(0x3d20d97c),fuel_fraction*f32::from_bits(0x4016cbe4)];
    for i in 0..3 {
        matrix::mat_rot_y(m,0x2ffd_4800,angles[i]);
        matrix::mat_mul(m,destinations[i],0x2ffd_4800,bases[i]);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_dashboard_speed_selection() {
        for (case,line) in include_str!("../../../tools/emu/golden_dashboard_speed_source.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|x|x.parse().unwrap()).collect();let viewer=0x2a000000;let scene=viewer+0x10000;let car=viewer+0x20000;let aux=viewer+0x30000;let mut m=Mem::new();
            m.set_u32(scene+0x5460,if v[0]==0 {0}else{car});m.set_u32(scene+0x5464,if v[0]==3 {aux}else{0});m.set_u32(car+0x3344,u32::from(v[0]==2));m.set_u32(car+0x3464,v[1]);
            for (address,value) in [(scene+0x51e4,v[2]),(car+0x3328,v[3]),(car+0x3330,v[4])] {m.set_u32(address,value);}
            for i in 0..3 {m.set_u32(scene+0xcf4+i*4,v[5+i as usize]);m.set_u32(aux+0x64+i*4,v[8+i as usize]);}
            let actual=dashboard_speed(&mut m,scene,|_,_|panic!("unexpected override"));assert_eq!(actual.to_bits(),p[1].parse::<u32>().unwrap(),"case {case}");
            if v[0]==1 {assert_eq!(dashboard_physical_speed(&m,car),Some(actual),"case {case} physical");}
        }
    }
    #[test]
    fn original_layout_table_writes(){
        for line in include_str!("../../../tools/emu/golden_view_tables.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let values=core::array::from_fn(|i|if p[i]=="<missing>"{None}else{Some(p[i])});
            let mut m=Mem::new();install_view_tables(&mut m,&ht2_formats::view_config::ViewTables::from_values(values));
            assert_eq!((0..20).map(|i|m.i32(0x696bf0+i*4)).collect::<Vec<_>>(),p[4].split(',').map(|s|s.parse::<i32>().unwrap()).collect::<Vec<_>>());
        }
    }
    #[test]
    fn original_view_rectangles(){
        for line in include_str!("../../../tools/emu/golden_view_rectangle.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<i32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let s=0x2a000000;let surface=s+0x10000;
            m.set_u32(s+0x24,surface);m.set_i32(surface+0x100,v[0]);m.set_i32(surface+0x104,v[1]);m.set_i32(s+0x574,v[2]);
            for (o,value) in [0x558,0x55c,0x560,0x564].into_iter().zip(&v[3..7]){m.set_i32(s+o,*value);}
            for (address,value) in [0x696bf8u32,0x696c0c,0x696c20].into_iter().zip(&v[7..10]){m.set_i32(address.wrapping_add((v[2] as u32).wrapping_mul(4)),*value);}
            m.set_i32(0x672d7c,v[10]);
            assert_eq!(view_rectangle(&m,s).to_vec(),p[1].split(',').map(|s|s.parse::<i32>().unwrap()).collect::<Vec<_>>());
        }
    }
    #[test]
    fn original_cabin_view_transforms(){
        for (case,line) in include_str!("../../../tools/emu/golden_cabin_view.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let f:Vec<i64>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();crate::globals::install_constants(&mut m);
            let s=0x2a000000;let v=s+0x10000;let out=v+0x10000;
            m.set_u32(0x6d2078,if f[0]!=0{s}else{0});m.set_u32(s+0x268,if f[1]!=0{v}else{0});m.set_u32(0x6cec68,f[2] as u32);
            m.set_i32(s+0x574,f[4] as i32);m.set_u32(v+0x51a8,f[5] as u32);m.set_u32(0x696d10,f[6] as u32);m.set_u32(v+0x4f48,f[7] as u32);m.set_u32(s+200,f[8] as u32);
            for (o,data) in [(0x1d8,p[1]),(0x5f0,p[2])]{for (i,value) in data.split(',').enumerate(){m.set_u32(s+o+i as u32*4,value.parse().unwrap());}}
            cabin_view(&mut m,out,f[3]!=0);
            assert_eq!((0..12).map(|i|m.u32(out+i*4)).collect::<Vec<_>>(),p[3].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case}");
        }
    }
    #[test]
    fn original_cabin_key_commands(){
        for line in include_str!("../../../tools/emu/golden_cabin_keys.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let panel=0x2a010000;
            for (i,v) in p[0].split(',').enumerate(){m.set_u32(panel+0x1318+i as u32*4,v.parse().unwrap());}
            let v:Vec<i32>=p[1].split(',').map(|v|v.parse().unwrap()).collect();
            let actual=cabin_keys(&m,panel,v[0],v[1],v[2],v[3],v[4..].try_into().unwrap());
            let expected=p[2].split(';').filter(|s|!s.is_empty()).map(|s|{let v:Vec<_>=s.split(',').collect();KeyCommand{node:v[0].parse().unwrap(),value:v[1].parse().unwrap(),digit:v[2]=="1"}}).collect::<Vec<_>>();
            assert_eq!(actual,expected);
        }
    }
    #[test]
    fn original_cabin_steering(){
        for line in include_str!("../../../tools/emu/golden_cabin_steering.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let base=0x2a000000;let out=base+0x100;
            for (i,v) in p[1].split(',').enumerate(){m.set_u32(base+i as u32*4,v.parse().unwrap());}
            cabin_steering(&mut m,base,out,f32::from_bits(p[0].parse().unwrap()));
            assert_eq!((0..12).map(|i|m.u32(out+i*4)).collect::<Vec<_>>(),p[2].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>());
        }
    }
    #[test]
    fn original_cabin_needle_transforms(){
        for (case,line) in include_str!("../../../tools/emu/golden_cabin_needles.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let bases=[0x2a000000,0x2a000040,0x2a000080];let out=[0x2a000100,0x2a000140,0x2a000180];
            for i in 0..3 {for (j,v) in p[i+1].split(',').enumerate(){m.set_u32(bases[i]+j as u32*4,v.parse().unwrap());}}
            let v:Vec<f32>=p[0].split(',').map(|v|f32::from_bits(v.parse().unwrap())).collect();
            cabin_needles(&mut m,bases,out,v[0],v[1],v[2]);
            for i in 0..3 {assert_eq!((0..12).map(|j|m.u32(out[i]+j*4)).collect::<Vec<_>>(),p[i+4].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case} needle {i}");}
        }
    }
}

/// Snapshot field publication in `0x54d6c3..0x54d80c`.
/// Floating payloads follow Unicorn; hardware signaling-NaN exceptions are unverified.
/// This is only the copy block; the surrounding scene restore has other effects.
pub fn restore_dashboard_snapshot(m:&mut Mem,scene:u32) {
    for (src,dst) in [(0x281c,0x51f8),(0x2894,0x5210),(0x289c,0x5220),(0x28a4,0x521c),(0x2814,0x51e4),(0x288c,0x51fc),(0x2818,0x51e8),(0x2818,0x51ec),(0x2890,0x520c),(0x2810,0x51f4),(0x28b0,0x5200),(0x2898,0x5214),(0x28a0,0x5218),(0x28ac,0x522c),(0x28a8,0x5228)] {
        let value=m.u32(scene+src);m.set_u32(scene+dst,value);
    }
    m.set_u32(scene+0x5204,m.u32(scene+0x2820)&255);
    let steering=(m.u32(scene+0x27c4) as u8 as i8) as f64;
    m.set_f32(scene+0x51e0,(steering/f32::from_bits(0x3f490fdb) as f64*(1.0/127.0)) as f32);
    let flags=m.u32(scene+0x27b4);
    for (dst,value) in [(0x5208,flags&2),(0x51b4,(flags>>3)&1),(0x51b8,((flags>>2)&1)+((flags>>3)&2)),(0x5178,(flags>>5)&1),(0x517c,(flags>>6)&1),(0x5224,(flags>>17)&1),(0x51c0,(flags>>18)&1)] {m.set_u32(scene+dst,value);}
}
#[cfg(test)]
mod snapshot_tests {
    use super::*;
    #[test]
    fn native_snapshot_publication() {
        let src=[0x281c,0x2894,0x289c,0x28a4,0x2814,0x288c,0x2818,0x2818,0x2890,0x2810,0x28b0,0x2898,0x28a0,0x28ac,0x28a8,0x2820,0x27b4,0x27c4];
        let dst=[0x51f8,0x5210,0x5220,0x521c,0x51e4,0x51fc,0x51e8,0x51ec,0x520c,0x51f4,0x5200,0x5214,0x5218,0x522c,0x5228,0x5204,0x5208,0x51b4,0x51b8,0x5178,0x517c,0x5224,0x51c0,0x51e0];
        for line in include_str!("../../../tools/emu/golden_scene_dashboard_snapshot.tsv").lines() {
            let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let scene=0x2a000000;
            for (offset,value) in src.into_iter().zip(input.split(',')){m.set_u32(scene+offset,value.parse().unwrap());}
            restore_dashboard_snapshot(&mut m,scene);
            for (offset,value) in dst.into_iter().zip(output.split(',')){assert_eq!(m.u32(scene+offset),value.parse::<u32>().unwrap());}
        }
    }
}

/// Header decoding `0x541adf..0x541b6a` of the packed vehicle snapshot.
/// Remaining wheel, pose and extended-state payload is not decoded here.
pub fn decode_snapshot_header(m:&mut Mem,snapshot:u32) {
    let flags=m.u32(snapshot+0x18);m.set_u32(snapshot+0x50,flags);
    let mut bytes=m.read(snapshot+0x54,1);bytes[0]=((flags>>7)&7) as u8;m.write(snapshot+0x54,&bytes);
    let packed=m.read(snapshot+0x1c,1)[0];
    m.set_f32(snapshot+0x58,((packed>>3) as u32*4) as f32);
    if packed&7<=4 {m.set_u32(snapshot+0x5c,(packed&7) as u32);}
}
#[cfg(test)]
mod snapshot_header_tests {
    use super::*;
    #[test]
    fn native_header_decode() {
        for line in include_str!("../../../tools/emu/golden_snapshot_header.tsv").lines() {
            let (input,output)=line.split_once('\t').unwrap();let v:Vec<u32>=input.split(',').map(|s|s.parse().unwrap()).collect();let want:Vec<u32>=output.split(',').map(|s|s.parse().unwrap()).collect();
            let mut m=Mem::new();let p=0x2a000000;m.set_u32(p+0x18,v[0]);m.set_u32(p+0x1c,v[1]);m.set_u32(p+0x5c,v[2]);m.set_u32(p+0x54,0xaabbccdd);
            decode_snapshot_header(&mut m,p);
            assert_eq!([m.u32(p+0x50),m.read(p+0x54,1)[0] as u32,m.u32(p+0x58),m.u32(p+0x5c)],want.as_slice());assert_eq!(m.u32(p+0x54)&0xffffff00,0xaabbcc00);
        }
    }
}

/// Decode header and wheels through `0x541bfe`; returns payload cursor.
/// Original caller supplies the valid wheel count (normally 0..8).
pub fn decode_snapshot_wheels(m:&mut Mem,p:u32)->u32 {
    decode_snapshot_header(m,p);
    let marker=m.read(p+0x1d,1);m.write(p+0x60,&marker);
    let mut cursor=0x1e;
    for i in 0..m.i32(p+0x10).max(0) as u32 {
        let b=m.read(p+cursor,3);m.write(p+0x61+i,&b[..1]);
        let packed=u16::from_le_bytes([b[1],b[2]]) as u32;
        m.set_f32(p+0x6c+i*4,((packed&1023) as f64*f64::from_bits(0x3f59251ff843357e)) as f32);
        m.set_f32(p+0x8c+i*4,(((packed>>10)&7) as f64*f64::from_bits(0x3fc2492492492492)) as f32);
        cursor+=3;
    }
    let flags=m.read(p+cursor,1)[0];
    for i in 0..4 {m.write(p+0xbe+i,&[(flags>>i)&1]);}
    cursor
}
#[cfg(test)]
mod snapshot_wheel_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_header_and_wheels() {
        for line in include_str!("../../../tools/emu/golden_snapshot_wheels.tsv").lines(){
            let (input,output)=line.split_once('\t').unwrap();let input=bytes(input);let mut m=Mem::new();let p=0x2a000000;m.write(p,&input);
            let cursor=decode_snapshot_wheels(&mut m,p);assert_eq!(cursor,0x1e+3*m.u32(p+0x10));assert_eq!(m.read(p+0x50,0x72),bytes(output));
        }
    }
}

/// Decode the four pose records and packed nibbles through `0x541cac`.
/// Returns the next payload byte offset; version checks and extension follow.
pub fn decode_snapshot_pose(m:&mut Mem,p:u32)->u32 {
    let mut cursor=decode_snapshot_wheels(m,p)+1;
    for i in 0..4 {
        let value=m.read(p+cursor,1)[0];cursor+=1;
        m.set_f32(p+0xd0+i*12,(m.f32(p+0xc4) as f64-value as f64*f64::from_bits(0x3f893b352540c735)) as f32);
    }
    for i in 0..4 {
        let b=m.read(p+cursor,2);cursor+=2;
        let x=(b[0] as i8 as f64/127.0) as f32;let y=(b[1] as i8 as f64/127.0) as f32;
        let dst=p+0xf8+i*12;m.set_f32(dst,x);m.set_f32(dst+4,y);
        m.set_f32(dst+8,(1.0-((x as f64*x as f64)+(y as f64*y as f64)).sqrt()) as f32);
    }
    let flags=m.read(p+cursor,1)[0];m.write(p+0xbc,&[flags>>4,flags&15]);cursor+1
}
#[cfg(test)]
mod snapshot_pose_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_pose_decode() {
        for line in include_str!("../../../tools/emu/golden_snapshot_pose.tsv").lines(){
            let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let p=0x2a000000;m.write(p,&bytes(input));
            let cursor=decode_snapshot_pose(&mut m,p);assert_eq!(cursor,0x2c+3*m.u32(p+0x10));assert_eq!(m.read(p+0x50,0xd8),bytes(output));
        }
    }
}

/// Decode a vehicle snapshot (`0x541ad0`). Diagnostic calls are returned as errors.
/// The original may continue after diagnostics; callers must handle errors explicitly.
pub fn decode_vehicle_snapshot(m:&mut Mem,p:u32)->Result<(),&'static str> {
    let cursor=decode_snapshot_pose(m,p);let base=cursor-0x14;
    let size=u16::from_le_bytes(m.read(p+0x14,2).try_into().unwrap()) as u32;
    if m.u32(p+0x50)&1==0 {
        return if base==size && base==m.u32(p+4) {Ok(())}else{Err("snapshot base version mismatch")};
    }
    let b=m.read(p+cursor,10);let percent=f64::from_bits(0x3fd9191919191919);let fraction=f64::from_bits(0x3f70101010101010);
    for (dst,value,scale) in [(0xb0,b[0],percent),(0xb4,b[3],percent),(0xb8,b[4],fraction),(0x128,b[5],fraction),(0x144,b[8],fraction),(0x148,b[9],fraction)] {m.set_f32(p+dst,(value as f64*scale) as f32);}
    m.set_f32(p+0xac,(u16::from_le_bytes([b[1],b[2]]) as u32*100) as f32);
    let flags=b[6] as u32;
    for (dst,value) in [(0x12c,flags&1),(0x130,(flags>>1)&1),(0x134,(flags>>2)&7),(0x138,(flags>>5)&1),(0x13c,flags>>6),(0x140,b[7] as u32&1)] {m.set_u32(p+dst,value);}
    if m.u32(p+8)>size {return Err("snapshot extended version mismatch");}
    if base+10>0x3a || base+10!=m.u32(p+8) {return Err("snapshot extended size mismatch");}
    Ok(())
}
#[cfg(test)]
mod snapshot_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn complete_native_snapshot_decode() {
        for line in include_str!("../../../tools/emu/golden_snapshot_full.tsv").lines(){
            let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let p=0x2a000000;m.write(p,&bytes(input));
            decode_vehicle_snapshot(&mut m,p).unwrap();assert_eq!(m.read(p+0x50,0xfc),bytes(output));
        }
    }
}

/// External owners needed by the scene snapshot restore (`0x54d690`).
pub trait SnapshotRestoreHooks {
    fn body_flag(&mut self,m:&mut Mem,body:u32,enabled:u32);
    fn update_vehicle(&mut self,m:&mut Mem,scene:u32);
    fn update_geometry(&mut self,m:&mut Mem,body:u32);
}
/// Restore orchestration with actual decoding; geometry remains owner-supplied.
pub fn restore_scene_snapshot(m:&mut Mem,scene:u32,h:&mut impl SnapshotRestoreHooks)->Result<(),&'static str> {
    m.set_u32(scene+0xdf4,2);
    decode_vehicle_snapshot(m,scene+0x2764)?;
    h.body_flag(m,scene+0x10,m.u32(scene+0x27b4)&1);
    restore_dashboard_snapshot(m,scene);
    h.update_vehicle(m,scene);h.update_geometry(m,scene+0x10);
    m.set_u32(scene+0xdf4,0);Ok(())
}
#[cfg(test)]
mod scene_restore_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    struct Hooks{scene:u32,trace:Vec<u32>}
    impl SnapshotRestoreHooks for Hooks {
        fn body_flag(&mut self,m:&mut Mem,body:u32,enabled:u32){assert_eq!(body,self.scene+0x10);assert_eq!(m.u32(self.scene+0xdf4),2);assert_eq!(enabled,m.u32(self.scene+0x27b4)&1);self.trace.push(1);}
        fn update_vehicle(&mut self,m:&mut Mem,scene:u32){assert_eq!(scene,self.scene);assert_eq!(m.u32(scene+0xdf4),2);self.trace.push(2);}
        fn update_geometry(&mut self,m:&mut Mem,body:u32){assert_eq!(body,self.scene+0x10);assert_eq!(m.u32(self.scene+0xdf4),2);self.trace.push(3);}
    }
    #[test]
    fn native_scene_restore_with_real_decoder() {
        for line in include_str!("../../../tools/emu/golden_scene_restore.tsv").lines(){
            let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let scene=0x2a000000;m.write(scene+0x2764,&bytes(input));let mut h=Hooks{scene,trace:vec![]};
            restore_scene_snapshot(&mut m,scene,&mut h).unwrap();assert_eq!(m.read(scene+0x51b4,0x7c),bytes(output));assert_eq!(h.trace,[1,2,3]);assert_eq!(m.u32(scene+0xdf4),0);
        }
    }
}

/// Geometry-update entry gate `0x55197a..0x5519f5` (finite clock delta).
pub fn scene_update_due(flags:u32,mode:i32,delta:f64,frame:i32)->bool {
    if flags&0x100==0 {return false;}
    if mode<=1 {return true;}
    if delta==0.0 {return false;}
    let reciprocal=1.0/delta.abs();
    let truncated=reciprocal.trunc();
    let interval=if !truncated.is_finite() || truncated>i32::MAX as f64 {1}else{(truncated as i32).max(1)};
    frame%interval==0
}
#[cfg(test)]
mod scene_gate_tests {
    use super::*;
    #[test]
    fn native_scene_update_gate(){
        for line in include_str!("../../../tools/emu/golden_scene_update_gate.tsv").lines(){
            let (input,want)=line.split_once('\t').unwrap();let v:Vec<_>=input.split(',').collect();
            assert_eq!(scene_update_due(v[0].parse().unwrap(),v[1].parse().unwrap(),v[3].parse().unwrap(),v[4].parse().unwrap()),want=="1");
        }
    }
}

/// Initial geometry basis in `0x5519f5..0x551a0f`, after the scheduling gate.
/// Room lookup and remaining vehicle effects must follow in the owner.
pub fn scene_geometry_basis(m:&mut Mem,scene:u32,dst:u32) {
    crate::matrix::mat_mul(m,dst,scene+0x2c4,scene+0x204);
}
#[cfg(test)]
mod scene_basis_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_scene_basis(){
        for line in include_str!("../../../tools/emu/golden_scene_basis.tsv").lines(){
            let (input,output)=line.split_once('\t').unwrap();let b=bytes(input);let mut m=Mem::new();let scene=0x2a000000;let dst=scene+0x10000;
            m.write(scene+0x2c4,&b[..48]);m.write(scene+0x204,&b[48..]);scene_geometry_basis(&mut m,scene,dst);assert_eq!(m.read(dst,48),bytes(output));
        }
    }
}

/// Preparation through `0x551a27`: basis followed by the room query.
/// The original uses the basis translation as the segment endpoint.
/// Remaining effects are deliberately left to the owner.
pub fn prepare_scene_geometry(m:&mut Mem,world:&mut dyn crate::contact::World,scene:u32,basis:u32,rec:u32)->u32 {
    scene_geometry_basis(m,scene,basis);
    crate::step::query_segment_room(m,world,m.u32(scene+0x330),scene+0x228,basis+36,rec)
}
#[cfg(test)]
mod geometry_prepare_tests {
    use super::*;
    struct Query{start:[u32;3],end:[u32;3],calls:u32}
    impl crate::contact::World for Query {
        fn collide(&mut self,m:&mut Mem,obj:u32,mode:u32,rec:u32)->u32 {
            assert_eq!(obj,77);assert_eq!(mode,0);self.start=std::array::from_fn(|i|m.u32(rec+0x4c+i as u32*4));self.end=std::array::from_fn(|i|m.u32(rec+0x58+i as u32*4));self.calls+=1;m.set_u32(rec+0x68,91);2
        }
    }
    #[test]
    fn geometry_query_uses_composed_translation(){
        let mut m=Mem::new();let scene=0x2a000000;let basis=scene+0x10000;let rec=basis+0x1000;
        crate::matrix::put12(&mut m,scene+0x2c4,&crate::matrix::IDENTITY);crate::matrix::put12(&mut m,scene+0x204,&crate::matrix::IDENTITY);
        for (i,v) in [3.,5.,7.].into_iter().enumerate(){m.set_f32(scene+0x204+36+i as u32*4,v);}
        m.set_u32(scene+0x330,77);let start=m.read(scene+0x228,12);
        let mut w=Query{start:[0;3],end:[0;3],calls:0};assert_eq!(prepare_scene_geometry(&mut m,&mut w,scene,basis,rec),91);
        assert_eq!(w.end,[3f32.to_bits(),5f32.to_bits(),7f32.to_bits()]);assert_eq!(w.calls,1);assert_eq!(m.read(scene+0x228,12),start);
    }
}

#[cfg(test)]
mod native_prepare_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    struct Query{expected:Vec<u8>,flags:u32,room:u32,calls:u32}
    impl crate::contact::World for Query {
        fn collide(&mut self,m:&mut Mem,_:u32,mode:u32,rec:u32)->u32{assert_eq!(mode,0);assert_eq!(m.read(rec+0x4c,24),self.expected);m.set_u32(rec+0x68,self.room);self.calls+=1;self.flags}
    }
    #[test]
    fn complete_native_geometry_preparation(){
        for line in include_str!("../../../tools/emu/golden_geometry_prepare.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let input=bytes(v[0]);let a:Vec<u32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let scene=0x2a000000;let obj=scene+0x20000;let basis=scene+0x30000;let rec=basis+0x1000;
            m.write(scene+0x2c4,&input[..48]);m.write(scene+0x204,&input[48..]);m.set_u32(scene+0x330,obj);
            let mut w=Query{expected:bytes(v[1]),flags:a[0],room:a[1],calls:0};let room=prepare_scene_geometry(&mut m,&mut w,scene,basis,rec);
            assert_eq!(m.read(basis,48),bytes(v[2]));assert_eq!(w.calls,1);assert_eq!(room,if a[0]&2!=0 {a[1]}else{obj});
        }
    }
}

/// First effect point/direction in `0x551a75..0x551b95`.
/// Caller checks scene flags and the tech Y coordinate threshold before emission.
pub fn scene_effect_transform(m:&mut Mem,basis:u32,tech_point:u32,dst:u32) {
    for i in 0..3 {
        for (offset,source) in [(0,tech_point),(12,tech_point+12)] {
            let a=m.f32(basis+i*4) as f64*m.f32(source) as f64;
            let b=m.f32(basis+24+i*4) as f64*m.f32(source+8) as f64;
            let c=m.f32(basis+12+i*4) as f64*m.f32(source+4) as f64;
            let value=if offset==0 {(a+b)+c+m.f32(basis+36+i*4) as f64}else{(a+b)+c};
            m.set_f32(dst+offset+i*4,value as f32);
        }
    }
}
#[cfg(test)]
mod effect_transform_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_effect_transform(){
        for line in include_str!("../../../tools/emu/golden_scene_effect.tsv").lines(){let v:Vec<_>=line.split('\t').collect();let b=bytes(v[0]);let mut m=Mem::new();let p=0x2a000000;m.write(p+0x2c4,&b[..48]);m.write(p+0x204,&b[48..]);m.write(p+0x10000,&bytes(v[1]));scene_geometry_basis(&mut m,p,p+0x20000);scene_effect_transform(&mut m,p+0x20000,p+0x10000,p+0x30000);assert_eq!(m.read(p+0x30000,24),bytes(v[2]));}
    }
}

/// First effect branch of `0x551970`; caller supplies the prepared basis.
/// Returns point and direction for the original sink's arguments `(0,1)`.
pub fn first_scene_effect(m:&mut Mem,scene:u32,basis:u32,scratch:u32)->Option<([f32;3],[f32;3])> {
    if m.u32(scene+0x51a4)&2==0 || m.u32(scene+0x27b4)&0x400==0 {return None;}
    let tech=0x697fa8u32.wrapping_add(m.u32(scene+0x28b4).wrapping_mul(0x970));
    if !(m.f32(tech+8).abs()<10.0) {return None;}
    scene_effect_transform(m,basis,tech,scratch);
    Some((std::array::from_fn(|i|m.f32(scratch+i as u32*4)),std::array::from_fn(|i|m.f32(scratch+12+i as u32*4))))
}
#[cfg(test)]
mod effect_dispatch_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_first_effect_dispatch(){
        for line in include_str!("../../../tools/emu/golden_scene_effect_dispatch.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let b=bytes(v[0]);let f:Vec<u32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let p=0x2a000000;
            m.write(p+0x2c4,&b[..48]);m.write(p+0x204,&b[48..]);m.write(0x697fa8+f[2]*0x970,&bytes(v[1]));m.set_u32(p+0x51a4,f[0]);m.set_u32(p+0x27b4,f[1]);m.set_u32(p+0x28b4,f[2]);scene_geometry_basis(&mut m,p,p+0x10000);
            let effect=first_scene_effect(&mut m,p,p+0x10000,p+0x20000);assert_eq!(effect.is_some(),v[2]!="-");if effect.is_some(){assert_eq!(m.read(p+0x20000,24),bytes(v[2]));}
        }
    }
}

/// Both ordered effect requests in `0x551a27..0x551cf3`.
/// Actual effect allocation and rendering are not performed here.
pub fn scene_effect_requests(m:&mut Mem,scene:u32,basis:u32,scratch:u32)->Vec<([f32;3],[f32;3])> {
    let mut effects=Vec::new();
    if m.u32(scene+0x51a4)&2==0 || m.u32(scene+0x27b4)&0x400==0 {return effects;}
    let tech=0x697fa8u32.wrapping_add(m.u32(scene+0x28b4).wrapping_mul(0x970));
    for point in [tech,tech+24] {
        if m.f32(point+8).abs()<10.0 {
            scene_effect_transform(m,basis,point,scratch);
            effects.push((std::array::from_fn(|i|m.f32(scratch+i as u32*4)),std::array::from_fn(|i|m.f32(scratch+12+i as u32*4))));
        }
    }
    effects
}
#[cfg(test)]
mod effect_pair_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_effect_pair(){
        for line in include_str!("../../../tools/emu/golden_scene_effect_pair.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let b=bytes(v[0]);let f:Vec<u32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let p=0x2a000000;
            m.write(p+0x2c4,&b[..48]);m.write(p+0x204,&b[48..]);m.write(0x697fa8+f[2]*0x970,&bytes(v[1]));m.set_u32(p+0x51a4,f[0]);m.set_u32(p+0x27b4,f[1]);m.set_u32(p+0x28b4,f[2]);scene_geometry_basis(&mut m,p,p+0x10000);
            let effects=scene_effect_requests(&mut m,p,p+0x10000,p+0x20000);let actual:Vec<u8>=effects.into_iter().flat_map(|(a,b)|a.into_iter().chain(b)).flat_map(f32::to_le_bytes).collect();assert_eq!(actual,if v[2]=="-" {vec![]}else{bytes(v[2])});
        }
    }
}

/// Allocator and particle-constructor boundary of `0x4bac90`.
pub trait SceneEffectSink {
    fn allocate(&mut self,m:&mut Mem,size:u32,pool:u32)->u32;
    fn construct(&mut self,m:&mut Mem,object:u32,args:[u32;6]);
}
/// Original sink wrapper; actual allocator and particle simulation are owner-supplied.
pub fn emit_scene_effect(m:&mut Mem,point:u32,direction:u32,sink:&mut impl SceneEffectSink) {
    let object=sink.allocate(m,0xb0,m.u32(0x6f6a8c));
    if object!=0 {sink.construct(m,object,[m.u32(0x696224),point,direction,1f32.to_bits(),0x695bf0,m.u32(0x696208)]);}
}
#[cfg(test)]
mod sink_tests {
    use super::*;
    struct Sink{object:u32,allocation:Vec<u32>,ctor:Vec<u32>}
    impl SceneEffectSink for Sink {
        fn allocate(&mut self,_:&mut Mem,size:u32,pool:u32)->u32{self.allocation=vec![size,pool];self.object}
        fn construct(&mut self,_:&mut Mem,object:u32,args:[u32;6]){self.ctor=vec![object];self.ctor.extend(args);}
    }
    #[test]
    fn native_effect_sink_wrapper(){
        for line in include_str!("../../../tools/emu/golden_effect_sink.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let parse=|s:&str|s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>();let v=parse(p[0]);let mut m=Mem::new();m.set_u32(0x6f6a8c,v[0]);m.set_u32(0x696224,v[1]);m.set_u32(0x696208,v[2]);let mut sink=Sink{object:v[7],allocation:vec![],ctor:vec![]};emit_scene_effect(&mut m,v[3],v[4],&mut sink);assert_eq!(sink.allocation,parse(p[1]));assert_eq!(sink.ctor,parse(p[2]));}
    }
}

/// Derived state of particle constructor `0x59c420`; base creation is explicit.
pub fn construct_scene_particle(m:&mut Mem,object:u32,args:[u32;6],mut base:impl FnMut(&mut Mem,u32,[u32;9])) {
    let [texture,point,direction,scale,config,count]=args;let scale=f32::from_bits(scale);
    base(m,object,[texture,point,(scale*0.5).to_bits(),(scale*4.0).to_bits(),config,count,((0.75f64/count as f64) as f32).to_bits(),0.75f32.to_bits(),100]);
    for offset in (0x94..0xac).step_by(4){m.set_u32(object+offset,0);}
    m.set_u32(object,0x650dd4);m.set_u32(object+0x14,0x650d88);m.set_u32(object+0x80,m.u32(object+0x80)|1);
    let p=m.read(point,12);m.write(object+0x94,&p);let d=m.read(direction,12);m.write(object+0xa0,&d);m.set_f32(object+0xac,3.0);
}
#[cfg(test)]
mod particle_ctor_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_constructor(){
        for line in include_str!("../../../tools/emu/golden_particle_ctor.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let parse=|s:&str|s.split(',').map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>();let v=parse(p[0]);let mut m=Mem::new();let object=0x2a000000;let point=object+0x1000;let direction=point+12;m.write(point,&bytes(p[1]));construct_scene_particle(&mut m,object,[v[0],point,direction,v[1],v[2],v[3]],|m,obj,args|{assert_eq!(args.as_slice(),parse(p[2]));m.set_u32(obj+0x80,v[4]);});assert_eq!(m.read(object,0xb0),bytes(p[3]));}
    }
}

/// `0x59ab10`: common particle size state, with root constructor supplied.
pub fn construct_particle_base(m:&mut Mem,object:u32,args:[u32;9],mut root:impl FnMut(&mut Mem,u32,[u32;11])) {
    let point=args[1];
    root(m,object,[args[0],m.u32(0x6f6a68),m.u32(point),m.u32(point+4),m.u32(point+8),args[2],args[2],args[4],args[5],args[6],args[7]]);
    m.set_u32(object,0x650c24);m.set_u32(object+0x14,0x650bd8);
    for offset in [0x84,0x8c] {m.set_u32(object+offset,args[2]);}
    for offset in [0x88,0x90] {m.set_u32(object+offset,args[3]);}
}
#[cfg(test)]
mod particle_base_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_base(){
        for line in include_str!("../../../tools/emu/golden_particle_base.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let parse=|s:&str|s.split(',').map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>();let v=parse(p[0]);let mut m=Mem::new();let object=0x2a000000;m.write(v[1],&bytes(p[1]));m.set_u32(0x6f6a68,v[9]);construct_particle_base(&mut m,object,v[..9].try_into().unwrap(),|_,obj,args|{assert_eq!(obj,object);assert_eq!(args.as_slice(),parse(p[2]));});assert_eq!(m.read(object,0x94),bytes(p[3]));}
    }
}

/// Root particle fields `0x59a7ba..0x59a80d`, following base identities.
/// Geometry allocations and registration are still required by the owner.
pub fn initialize_particle_root_fields(m:&mut Mem,object:u32,args:[u32;11]) {
    m.set_u32(object,0x650bd0);m.set_u32(object+0x14,0x650b84);m.set_u32(object+0x80,0);
    for (offset,value) in [(0x58,args[0]),(0x5c,args[1]),(0x4c,args[2]),(0x50,args[3]),(0x54,args[4]),(0x78,args[7]),(0x60,args[9]),(0x64,args[10])] {m.set_u32(object+offset,value);}
}
#[cfg(test)]
mod root_fields_tests {
    use super::*;
    #[test]
    fn native_root_fields(){
        for line in include_str!("../../../tools/emu/golden_particle_root_fields.tsv").lines(){let (input,output)=line.split_once('\t').unwrap();let args:Vec<u32>=input.split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let obj=0x2a000000;initialize_particle_root_fields(&mut m,obj,args.try_into().unwrap());let want:Vec<u8>=(0..output.len()).step_by(2).map(|i|u8::from_str_radix(&output[i..i+2],16).unwrap()).collect();assert_eq!(m.read(obj,0x84),want);}
    }
}

/// Geometry content in `0x59a843..0x59a91d`, before texture attachment.
/// Owner supplies initialized 40-byte descriptor and four 16-byte arrays.
pub fn initialize_particle_geometry(m:&mut Mem,geometry:u32,arrays:[u32;4],width:f32,height:f32,color:u32) {
    m.write(geometry,&[2,1]);m.write(geometry+4,&4u16.to_le_bytes());
    for (i,p) in arrays.into_iter().enumerate(){m.set_u32(geometry+8+i as u32*4,p);}
    let values=[[0.,1.,1.,0.],[1.,1.,0.,0.],[-width*0.5,width*0.5,width*0.5,-width*0.5],[-height*0.5,-height*0.5,height*0.5,height*0.5]];
    for (p,v) in arrays.into_iter().zip(values){for (i,x) in v.into_iter().enumerate(){m.set_f32(p+i as u32*4,x);}}
    m.set_u32(geometry+0x18,0);m.write(geometry+0x22,&0xfffu16.to_le_bytes());m.set_u32(geometry+0x24,color);
}
#[cfg(test)]
mod particle_geometry_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_geometry(){for line in include_str!("../../../tools/emu/golden_particle_geometry.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let base=0x2a020000;let geometry=base+4;let arrays=std::array::from_fn(|i|base+(i as u32+1)*0x100);initialize_particle_geometry(&mut m,geometry,arrays,f32::from_bits(v[0]),f32::from_bits(v[1]),v[2]);assert_eq!(m.read(geometry,40),bytes(p[1]));let actual:Vec<u8>=arrays.into_iter().flat_map(|a|m.read(a,16)).collect();assert_eq!(actual,bytes(p[2]));}}
}

/// Registration and initial counters `0x59a929..0x59a95a`, after attachment.
pub fn finish_particle_root(m:&mut Mem,object:u32,height_bits:u32,mut register:impl FnMut(&mut Mem,u32,u32,u32)) {
    register(m,m.u32(object+0x5c),0,object+0x14);
    for offset in [0x68,0x74,0x6c,0x7c] {m.set_u32(object+offset,0);}
    let flags=m.read(object+0x3c,1)[0];m.write(object+0x3c,&[flags&254]);m.set_u32(object+0x70,height_bits);
}
#[cfg(test)]
mod particle_finish_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_finish(){for line in include_str!("../../../tools/emu/golden_particle_finish.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let object=0x2a000000;m.write(object,&bytes(p[0]));m.set_u32(object+0x5c,object+0x1000);let expected:Vec<u32>=p[2].split(',').map(|s|s.parse().unwrap()).collect();finish_particle_root(&mut m,object,p[1].parse().unwrap(),|_,manager,mode,node|assert_eq!([manager,mode,node].as_slice(),expected));assert_eq!(m.read(object,0x84),bytes(p[3]));}}
}

/// Attach geometry (`0x5fa4f0`) then update its bounds (`0x5fa5f0`).
/// The owner implements bound construction/merge for each descriptor.
pub fn attach_particle_geometry(m:&mut Mem,node:u32,geometry:u32,count:i32,mut bounds:impl FnMut(&mut Mem,u32,u32,bool)) {
    m.set_u32(node+0x30,count as u32);m.set_u32(node+0x34,geometry);
    if count>0 {
        let origin=m.read(node+0x38,12);m.write(node+0x14,&origin);
        bounds(m,geometry,node+0x14,false);
        for i in 1..count {bounds(m,geometry+i as u32*0x28,node+0x14,true);}
    }
}
#[cfg(test)]
mod particle_attach_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_geometry_attachment(){for line in include_str!("../../../tools/emu/golden_particle_attach.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let node=0x2a000000;m.write(node,&bytes(p[0]));let count=p[1].parse::<i32>().unwrap();let mut calls=0;attach_particle_geometry(&mut m,node,node+0x1000,count,|_,geometry,dst,merge|{assert_eq!(geometry,node+0x1000+(if merge {(calls+1)/2}else{0}) as u32*0x28);assert_eq!(dst,node+0x14);calls+=if merge {2}else{1};});assert_eq!(calls,p[3].parse::<i32>().unwrap());assert_eq!(m.read(node,0x44),bytes(p[2]));}}
}

/// `0x5e6800`: geometry sphere is centred at zero with maximum vertex length.
/// Input centre is added to each X/Z vertex before measuring the radius.
pub fn particle_geometry_bounds(m:&mut Mem,geometry:u32,bounds:u32) {
    let count=u16::from_le_bytes(m.read(geometry+4,2).try_into().unwrap()) as u32;
    let x=m.u32(geometry+0x10);let z=m.u32(geometry+0x14);let origin=[m.f32(bounds),m.f32(bounds+4),m.f32(bounds+8)];let mut center=[0f32;3];
    for i in 0..count {let v=[m.f32(x+i*4)+origin[0],origin[1],m.f32(z+i*4)+origin[2]];for j in 0..3 {center[j]+=v[j];}}
    for v in &mut center {*v/=count as f32;}
    let mut radius=0f32;
    for i in 0..count {
        let vx=(center[0] as f64-(m.f32(x+i*4) as f64+origin[0] as f64)) as f32;
        let vy=center[1]-origin[1];let vz=center[2]-(m.f32(z+i*4)+origin[2]);
        let length=((vx as f64*vx as f64+vy as f64*vy as f64)+vz as f64*vz as f64).sqrt();if (radius as f64)<length {radius=length as f32;}
    }
    for i in 0..3 {m.set_u32(bounds+i as u32*4,if count==0 {0xffc00000}else{center[i].to_bits()});}m.set_f32(bounds+12,radius);
}
#[cfg(test)]
mod particle_bounds_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_geometry_bounds(){for line in include_str!("../../../tools/emu/golden_particle_bounds.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let n=p[0].parse::<usize>().unwrap();let raw=bytes(p[1]);let mut m=Mem::new();let geometry=0x2a000000;let x=geometry+0x1000;let z=x+0x1000;let bound=z+0x1000;m.write(geometry+4,&(n as u16).to_le_bytes());m.set_u32(geometry+0x10,x);m.set_u32(geometry+0x14,z);m.write(bound,&raw[..12]);m.write(x,&raw[12..12+4*n]);m.write(z,&raw[12+4*n..]);particle_geometry_bounds(&mut m,geometry,bound);assert_eq!(m.read(bound,16),bytes(p[2]));}}
}

/// Original sphere merge `0x5e5580`; scratch is a caller-owned 12-byte vector.
pub fn merge_particle_bounds(m:&mut Mem,dst:u32,other:u32,scratch:u32) {
    let a:[f32;4]=std::array::from_fn(|i|m.f32(dst+i as u32*4));let b:[f32;4]=std::array::from_fn(|i|m.f32(other+i as u32*4));
    for i in 0..3 {m.set_f32(scratch+i as u32*4,a[i]-b[i]);}
    crate::matrix::vec_unit(m,scratch,scratch);
    for i in 0..3 {
        let d=m.f32(scratch+i as u32*4) as f64;
        let mut ap=d*a[3] as f64;let mut bp=d*b[3] as f64;
        if i<2 {ap=(ap as f32) as f64;bp=(bp as f32) as f64;}
        let mut hi=ap+a[i] as f64;let mut lo=b[i] as f64-bp;
        if i<2 {hi=(hi as f32) as f64;lo=(lo as f32) as f64;}
        let mut sum=hi+lo;if i>0 {sum=(sum as f32) as f64;}
        m.set_f32(dst+i as u32*4,(sum*0.5) as f32);
        m.set_f32(scratch+i as u32*4,(hi-lo) as f32);
    }
    let length=crate::math::vec_len(m,scratch);m.set_f32(dst+12,(length*0.5) as f32);
}
#[cfg(test)]
mod bounds_merge_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_sphere_merge(){for line in include_str!("../../../tools/emu/golden_bounds_merge.tsv").lines(){let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let p=0x2a000000;m.write(p,&bytes(input));merge_particle_bounds(&mut m,p,p+16,p+0x1000);assert_eq!(m.read(p,16),bytes(output));}}
}

/// Attach descriptors and compute actual sphere bounds, replacing the hook boundary.
/// Scratch requires 28 bytes disjoint from the node and descriptors.
pub fn attach_particle_geometry_with_bounds(m:&mut Mem,node:u32,geometry:u32,count:i32,scratch:u32) {
    attach_particle_geometry(m,node,geometry,count,|m,descriptor,dst,merge|{
        if merge {
            let origin=m.read(node+0x38,12);m.write(scratch,&origin);particle_geometry_bounds(m,descriptor,scratch);merge_particle_bounds(m,dst,scratch,scratch+16);
        }else{particle_geometry_bounds(m,descriptor,dst);}
    });
}

#[cfg(test)]
mod full_attachment_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_full_attachment(){for line in include_str!("../../../tools/emu/golden_particle_attach_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let count=p[0].parse::<i32>().unwrap();let vertices=bytes(p[2]);let mut m=Mem::new();let node=0x2a000000;let geometry=node+0x1000;m.write(node+0x38,&bytes(p[1]));for i in 0..count as u32 {let d=geometry+i*40;let x=node+0x2000+i*0x100;m.write(d+4,&4u16.to_le_bytes());m.set_u32(d+0x10,x);m.set_u32(d+0x14,x+16);m.write(x,&vertices[i as usize*32..i as usize*32+32]);}attach_particle_geometry_with_bounds(&mut m,node,geometry,count,node+0x10000);assert_eq!(m.read(node,0x44),bytes(p[3]));}}
}

/// External memory and scene identities required by the root particle ctor.
pub trait ParticleRootOwner {
    fn initialize_identities(&mut self,m:&mut Mem,object:u32);
    fn allocate_zeroed(&mut self,m:&mut Mem,size:u32)->u32;
    fn initialize_descriptor(&mut self,m:&mut Mem,descriptor:u32);
    fn register(&mut self,m:&mut Mem,manager:u32,mode:u32,node:u32);
}
/// Root ctor assembled from verified blocks; successful allocations only.
/// Owner supplies the original base identities and descriptor construction.
pub fn construct_particle_root(m:&mut Mem,object:u32,args:[u32;11],scratch:u32,owner:&mut impl ParticleRootOwner)->Result<(),&'static str> {
    owner.initialize_identities(m,object);initialize_particle_root_fields(m,object,args);
    let allocation=owner.allocate_zeroed(m,0x2c);if allocation==0 {return Err("particle descriptor allocation failed");}
    m.set_u32(allocation,1);let geometry=allocation+4;owner.initialize_descriptor(m,geometry);
    let mut arrays=[0;4];for a in &mut arrays {*a=owner.allocate_zeroed(m,16);if *a==0 {return Err("particle array allocation failed");}}
    initialize_particle_geometry(m,geometry,arrays,f32::from_bits(args[5]),f32::from_bits(args[6]),m.u32(args[7]));
    attach_particle_geometry_with_bounds(m,object+0x14,geometry,1,scratch);
    finish_particle_root(m,object,args[8],|m,manager,mode,node|owner.register(m,manager,mode,node));Ok(())
}
#[cfg(test)]
mod root_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    struct Owner{next:u32}
    impl ParticleRootOwner for Owner {
        fn initialize_identities(&mut self,_:&mut Mem,_:u32){}
        fn allocate_zeroed(&mut self,m:&mut Mem,size:u32)->u32{let p=self.next;self.next+=0x100;m.write(p,&vec![0;size as usize]);p}
        fn initialize_descriptor(&mut self,_:&mut Mem,_:u32){}
        fn register(&mut self,_:&mut Mem,manager:u32,mode:u32,node:u32){assert_eq!([manager,mode,node],[0x2a010000,0,0x2a000014]);}
    }
    #[test]
    fn native_full_root_constructor(){for line in include_str!("../../../tools/emu/golden_particle_root_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let obj=0x2a000000;m.set_u32(v[7],v[11]);let mut owner=Owner{next:obj+0x20000};construct_particle_root(&mut m,obj,v[..11].try_into().unwrap(),obj+0x30000,&mut owner).unwrap();assert_eq!(m.read(obj,0x84),bytes(p[1]));assert_eq!(m.read(obj+0x20004,40),bytes(p[2]));}}
}

/// Assemble `0x59c420 -> 0x59ab10 -> 0x59a780` with actual geometry/bounds.
/// Allocation errors leave a partially initialized object for owner cleanup.
pub fn construct_complete_scene_particle(m:&mut Mem,object:u32,args:[u32;6],scratch:u32,owner:&mut impl ParticleRootOwner)->Result<(),&'static str> {
    let mut result=Ok(());
    construct_scene_particle(m,object,args,|m,obj,base_args|{
        construct_particle_base(m,obj,base_args,|m,obj,root_args|{result=construct_particle_root(m,obj,root_args,scratch,owner);});
    });result
}
#[cfg(test)]
mod particle_chain_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    struct Owner{next:u32}
    impl ParticleRootOwner for Owner {
        fn initialize_identities(&mut self,_:&mut Mem,_:u32){}
        fn allocate_zeroed(&mut self,m:&mut Mem,size:u32)->u32{let p=self.next;self.next+=0x100;m.write(p,&vec![0;size as usize]);p}
        fn initialize_descriptor(&mut self,_:&mut Mem,_:u32){}
        fn register(&mut self,_:&mut Mem,manager:u32,mode:u32,node:u32){assert_eq!([manager,mode,node],[0x2a010000,0,0x2a000014]);}
    }
    #[test]
    fn native_complete_particle_chain(){for line in include_str!("../../../tools/emu/golden_particle_chain.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let obj=0x2a000000;m.set_u32(0x6f6a68,obj+0x10000);m.set_u32(v[4],v[6]);m.write(v[1],&bytes(p[1]));let mut owner=Owner{next:obj+0x20000};construct_complete_scene_particle(&mut m,obj,v[..6].try_into().unwrap(),obj+0x30000,&mut owner).unwrap();assert_eq!(m.read(obj,0xb0),bytes(p[2]));assert_eq!(m.read(obj+0x20004,40),bytes(p[3]));}}
}

/// Drag phase `0x59c510..0x59c61c`; position/lifetime update must follow.
/// Scratch is a caller-owned 12-byte vector.
pub fn particle_drag(m:&mut Mem,object:u32,scratch:u32) {
    let drag=m.f32(object+0xac);if drag==0.0 || drag.is_nan(){return;}
    crate::matrix::vec_unit(m,scratch,object+0xa0);
    let dt=m.f64(m.u32(object+0x58)+0xb90) as f32;
    let old:[f32;3]=std::array::from_fn(|i|m.f32(object+0xa0+i as u32*4));let mut next=[0f32;3];let mut change=[0f32;3];
    for i in 0..3 {
        let force=-(m.f32(scratch+i as u32*4) as f64)*drag as f64;
        let force=if i==0 {force}else{(force as f32) as f64};
        let delta=(force*dt as f64) as f32;change[i]=delta;next[i]=old[i]+delta;m.set_f32(object+0xa0+i as u32*4,next[i]);
    }
    let dot=(next[2] as f64*change[2] as f64+next[1] as f64*change[1] as f64)+next[0] as f64*change[0] as f64;
    if dot>0.0 {m.set_f32(object+0xac,0.0);for i in 0..3 {m.set_u32(object+0xa0+i*4,m.u32(0x702650+i*4));}}
}
#[cfg(test)]
mod drag_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_drag(){for line in include_str!("../../../tools/emu/golden_particle_drag.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let obj=0x2a000000;m.set_u32(obj+0x58,obj+0x1000);m.write(obj+0xa0,&bytes(p[0]));m.write(obj+0x1000+0xb90,&bytes(p[1]));particle_drag(&mut m,obj,obj+0x2000);assert_eq!(m.read(obj+0xa0,16),bytes(p[2]));}}
}

/// `0x59c510`: drag and position; common lifetime/size update is owner-supplied.
pub fn update_scene_particle_motion(m:&mut Mem,object:u32,scratch:u32,mut common:impl FnMut(&mut Mem,u32)) {
    particle_drag(m,object,scratch);
    let x=m.f32(object+0xa0);let y=m.f32(object+0xa4);
    // x87 unordered comparisons follow the equality path here.
    if (x==0.0 || x.is_nan()) && (y==0.0 || y.is_nan()){m.set_f32(object+0xa8,0.5);}
    let dt=m.f64(m.u32(object+0x58)+0xb90);
    for i in 0..3 {
        let delta=dt*m.f32(object+0xa0+i*4) as f64;
        let delta=if i<2 {(delta as f32) as f64}else{delta};
        let value=(delta+m.f32(object+0x4c+i*4) as f64) as f32;
        m.set_f32(object+0x4c+i*4,value);m.set_f32(object+0x28+i*4,value);
    }
    common(m,object);
}
#[cfg(test)]
mod particle_motion_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_complete_particle_motion(){for line in include_str!("../../../tools/emu/golden_particle_motion.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let raw=bytes(p[0]);let mut m=Mem::new();let obj=0x2a000000;m.set_u32(obj+0x58,obj+0x1000);m.write(obj+0x4c,&raw[..12]);m.write(obj+0xa0,&raw[12..]);m.write(obj+0x1000+0xb90,&bytes(p[1]));let mut calls=0;update_scene_particle_motion(&mut m,obj,obj+0x2000,|_,o|{assert_eq!(o,obj);calls+=1;});assert_eq!(calls,1);assert_eq!(m.read(obj,0xb0),bytes(p[2]));}}
}

/// `0x59ac90`: interpolate quad sizes, then invoke lifecycle update.
pub fn update_particle_size(m:&mut Mem,object:u32,mut lifecycle:impl FnMut(&mut Mem,u32)) {
    let duration=m.f32(object+0x64) as f64;
    let mut fraction=(duration-m.f32(object+0x68) as f64)/duration;
    if fraction.is_nan() || fraction<0.0 {fraction=0.0;}else if fraction>1.0 {fraction=1.0;}
    let complement=1.0-fraction;
    let width=m.f32(object+0x88) as f64*complement+m.f32(object+0x84) as f64*fraction;
    let height=m.f32(object+0x90) as f64*complement+m.f32(object+0x8c) as f64*fraction;
    let geometry=m.u32(object+0x48);let x=m.u32(geometry+0x10);let z=m.u32(geometry+0x14);
    for (i,v) in [-width*0.5,width*0.5,width*0.5,-width*0.5].into_iter().enumerate(){m.set_f32(x+i as u32*4,v as f32);}
    for (i,v) in [-height*0.5,-height*0.5,height*0.5,height*0.5].into_iter().enumerate(){m.set_f32(z+i as u32*4,v as f32);}
    lifecycle(m,object);
}
#[cfg(test)]
mod particle_size_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_sizes(){for line in include_str!("../../../tools/emu/golden_particle_size.tsv").lines(){let (input,output)=line.split_once('\t').unwrap();let raw=bytes(input);let mut m=Mem::new();let obj=0x2a000000;let g=obj+0x1000;let x=g+0x1000;m.set_u32(obj+0x48,g);m.set_u32(g+0x10,x);m.set_u32(g+0x14,x+16);m.write(obj+0x64,&raw[..8]);m.write(obj+0x84,&raw[8..]);let mut calls=0;update_particle_size(&mut m,obj,|_,o|{assert_eq!(o,obj);calls+=1;});assert_eq!(calls,1);assert_eq!(m.read(x,32),bytes(output));}}
}

/// `0x59aa10`: animation frame, colour, expiry and elapsed time.
/// Valid input requires positive frame count and representable frame conversion.
pub fn update_particle_lifecycle(m:&mut Mem,object:u32,mut remove:impl FnMut(&mut Mem,u32)) {
    let frame=(m.f32(object+0x68) as f64/m.f32(object+0x60) as f64+m.i32(object+0x74) as f64).trunc() as i32;
    let count=m.i32(object+0x70);let frame=if m.u32(object+0x80)&1!=0 {if frame<count {frame}else{count.wrapping_sub(1)}}else{frame%count};
    m.set_u32(object+0x6c,frame as u32);m.set_u32(object+0x7c,m.u32(object+0x7c).wrapping_add(1));
    let color=m.u32(m.u32(object+0x78).wrapping_add((frame as u32).wrapping_mul(4)));m.set_u32(m.u32(object+0x48)+0x24,color);
    if m.f32(object+0x68)>m.f32(object+0x64) {remove(m,object);}else{let dt=m.f64(m.u32(object+0x58)+0xb90);m.set_f32(object+0x68,(dt+m.f32(object+0x68) as f64) as f32);}
}
#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_lifecycle(){for line in include_str!("../../../tools/emu/golden_particle_lifecycle.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let raw=bytes(p[0]);let mut m=Mem::new();let obj=0x2a000000;let clock=obj+0x1000;let colors=clock+0x2000;let g=colors+0x2000;m.set_u32(obj+0x58,clock);m.set_u32(obj+0x48,g);m.set_u32(obj+0x78,colors);for (i,offset) in [0x68,0x60,0x64,0x70,0x74,0x80,0x7c].into_iter().enumerate(){m.write(obj+offset,&raw[i*4..i*4+4]);}for i in 0..1000u32{m.set_u32(colors+i*4,i.wrapping_mul(1234567));}m.write(clock+0xb90,&bytes(p[1]));let mut removed=0;update_particle_lifecycle(&mut m,obj,|_,_|removed+=1);assert_eq!(m.read(obj+0x68,24),bytes(p[2]));assert_eq!(m.read(g+0x24,4),bytes(p[3]));assert_eq!(removed,p[4].parse::<u32>().unwrap());}}
}

/// Complete derived update chain; owner performs the original deletion action.
pub fn update_scene_particle(m:&mut Mem,object:u32,scratch:u32,mut remove:impl FnMut(&mut Mem,u32)) {
    update_scene_particle_motion(m,object,scratch,|m,object|update_particle_size(m,object,|m,object|update_particle_lifecycle(m,object,&mut remove)));
}
#[cfg(test)]
mod particle_full_update_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_full_particle_update(){for line in include_str!("../../../tools/emu/golden_particle_update_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let obj=0x2a000000;let clock=obj+0x1000;let g=clock+0x1000;let x=g+0x1000;let colors=x+0x1000;m.write(obj,&bytes(p[0]));m.write(clock+0xb90,&bytes(p[1]));m.set_u32(g+0x10,x);m.set_u32(g+0x14,x+16);for i in 0..1000u32{m.set_u32(colors+i*4,i.wrapping_mul(1234567));}let mut removed=0;update_scene_particle(&mut m,obj,obj+0x5000,|_,o|{assert_eq!(o,obj);removed+=1;});assert_eq!(m.read(obj,0xb0),bytes(p[2]));assert_eq!(m.read(x,32),bytes(p[3]));assert_eq!(m.read(g+0x24,4),bytes(p[4]));assert_eq!(removed,p[5].parse::<u32>().unwrap());}}
}

/// `0x579370` marks for manager cleanup; it does not free the object immediately.
pub fn mark_particle_for_removal(m:&mut Mem,object:u32){m.set_u32(object+4,1);}
/// Complete particle update including the actual original removal mark.
pub fn tick_scene_particle(m:&mut Mem,object:u32,scratch:u32) {
    update_scene_particle(m,object,scratch,mark_particle_for_removal);
}
#[cfg(test)]
mod marked_particle_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_update_with_real_removal_mark(){for line in include_str!("../../../tools/emu/golden_particle_update_marked.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let obj=0x2a000000;let clock=obj+0x1000;let g=clock+0x1000;let x=g+0x1000;let colors=x+0x1000;m.write(obj,&bytes(p[0]));m.write(clock+0xb90,&bytes(p[1]));m.set_u32(g+0x10,x);m.set_u32(g+0x14,x+16);for i in 0..1000u32{m.set_u32(colors+i*4,i.wrapping_mul(1234567));}tick_scene_particle(&mut m,obj,obj+0x5000);assert_eq!(m.read(obj,0xb0),bytes(p[2]));assert_eq!(m.read(x,32),bytes(p[3]));assert_eq!(m.read(g+0x24,4),bytes(p[4]));}}
}

/// `0x5794f0`: unlink a valid live pool object and prepend it to the free list.
/// Destruction of owned resources must precede this recycle operation.
pub fn recycle_effect_object(m:&mut Mem,pool:u32,object:u32) {
    let next=m.u32(object+8);let prev=m.u32(object+12);
    if m.u32(pool+8)==object {m.set_u32(pool+8,next);}
    if next!=0 {m.set_u32(next+12,prev);}if prev!=0 {m.set_u32(prev+8,next);}
    let free=m.u32(pool+4);
    if free==0 {m.set_u32(pool+4,object);m.set_u32(object+8,0);m.set_u32(object+12,0);}else{
        m.set_u32(object+12,m.u32(free+12));m.set_u32(object+8,free);m.set_u32(free+12,object);m.set_u32(pool+4,object);
    }
}
#[cfg(test)]
mod effect_pool_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_pool_recycle(){for line in include_str!("../../../tools/emu/golden_effect_pool_release.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let pool=0x2a000000;m.write(pool,&bytes(p[0]));recycle_effect_object(&mut m,pool,p[1].parse().unwrap());assert_eq!(m.read(pool,0x200),bytes(p[2]));}}
}
