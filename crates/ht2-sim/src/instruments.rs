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

/// `0x579480`: take the free-list head and prepend to the active list.
pub fn take_effect_object(m:&mut Mem,pool:u32)->u32 {
    let object=m.u32(pool+4);if object==0 {return 0;}
    let previous=m.u32(object+12);let next=m.u32(object+8);let active=m.u32(pool+8);
    if active==0 {m.set_u32(pool+8,object);m.set_u32(object+8,0);m.set_u32(object+12,0);}else{
        m.set_u32(object+12,m.u32(active+12));m.set_u32(object+8,active);m.set_u32(active+12,object);m.set_u32(pool+8,object);
    }
    m.set_u32(pool+4,next);if next!=0 {m.set_u32(next+12,previous);}object
}
#[cfg(test)]
mod effect_pool_take_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_pool_take(){for line in include_str!("../../../tools/emu/golden_effect_pool_take.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let pool=0x2a000000;m.write(pool,&bytes(p[0]));assert_eq!(take_effect_object(&mut m,pool),p[1].parse::<u32>().unwrap());assert_eq!(m.read(pool,0x200),bytes(p[2]));}}
}

/// `0x579380`, valid non-null fixed-size pool; size is unused by the original.
pub fn allocate_effect_object(m:&mut Mem,pool:u32)->u32 {
    let object=take_effect_object(m,pool);if object==0 {return 0;}
    m.set_u32(0x6f4bc8,0);m.set_u32(0x6f4bcc,m.u32(object+8));m.set_u32(0x6f4bd0,m.u32(object+12));m.set_u32(0x6f4bd4,m.u32(object+16));
    m.set_u32(object+16,pool);m.set_u32(object+4,0);object
}
/// `0x579310`: restore the allocation handshake into the base object.
pub fn initialize_effect_identity(m:&mut Mem,object:u32) {
    m.set_u32(object,0x650334);
    for i in 0..4 {m.set_u32(object+4+i*4,m.u32(0x6f4bc8+i*4));}
}
#[cfg(test)]
mod allocation_chain_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_allocation_identity_chain(){for line in include_str!("../../../tools/emu/golden_effect_allocation_chain.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let pool=0x2a000000;m.write(pool,&bytes(p[0]));let object=allocate_effect_object(&mut m,pool);assert_eq!(object,p[1].parse::<u32>().unwrap());if object!=0 {initialize_effect_identity(&mut m,object);}assert_eq!(m.read(pool,0x200),bytes(p[2]));}}
}

/// Manager cleanup phase `0x57975d..0x5797be` (after update passes).
/// Saves next before destructor, whose owner must recycle/free the object.
pub fn cleanup_effect_pools(m:&mut Mem,manager:u32,mut destroy:impl FnMut(&mut Mem,u32)) {
    let mut i=0;
    while i<m.u32(manager+0x1b0) {
        let pool=m.u32(manager+0x20+i*4);let mut object=m.u32(pool+8);
        while object!=0 {let next=m.u32(object+8);if m.u32(object+4)!=0 {destroy(m,object);}object=next;}
        i+=1;
    }
}
#[cfg(test)]
mod cleanup_pool_tests {
    use super::*;
    #[test]
    fn native_cleanup_traversal(){for line in include_str!("../../../tools/emu/golden_effect_cleanup.tsv").lines(){let (marks,want)=line.split_once('\t').unwrap();let parse=|s:&str|s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>();let marks=parse(marks);let mut m=Mem::new();let manager=0x2a000000;let pool=manager+0x1000;m.set_u32(manager+0x1b0,1);m.set_u32(manager+0x20,pool);m.set_u32(pool+8,if marks.is_empty(){0}else{pool+0x100});for (i,mark) in marks.iter().enumerate(){let obj=pool+0x100+i as u32*32;m.set_u32(obj+4,*mark);m.set_u32(obj+8,if i+1<marks.len(){obj+32}else{0});}let mut trace=vec![];cleanup_effect_pools(&mut m,manager,|m,obj|{trace.push(obj);m.set_u32(obj+8,0);});assert_eq!(trace,parse(want));}}
}

/// Dependencies called by the particle destructor in original order.
pub trait ParticleDestroyOwner {
    fn detach(&mut self,m:&mut Mem,manager:u32);
    fn release_node(&mut self,m:&mut Mem,node:u32);
    fn destroy_texture(&mut self,m:&mut Mem,node:u32);
}
/// `0x5a2a30 -> 0x59aa90`; actual geometry release remains owner-supplied.
pub fn destroy_scene_particle(m:&mut Mem,object:u32,owner:&mut impl ParticleDestroyOwner) {
    m.set_u32(object,0x650c24);m.set_u32(object+0x14,0x650bd8);
    m.set_u32(object,0x650bd0);m.set_u32(object+0x14,0x650b84);
    owner.detach(m,m.u32(object+0x5c));owner.release_node(m,object+0x14);owner.destroy_texture(m,object+0x14);
    m.set_u32(object,0x650334);m.set_u32(object+4,0);
}
#[cfg(test)]
mod particle_destroy_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    struct Owner(Vec<u32>);
    impl ParticleDestroyOwner for Owner {
        fn detach(&mut self,_:&mut Mem,p:u32){self.0.extend([0x5e48f0,p]);}
        fn release_node(&mut self,_:&mut Mem,p:u32){self.0.extend([0x5ef780,p]);}
        fn destroy_texture(&mut self,_:&mut Mem,p:u32){self.0.extend([0x5fa720,p]);}
    }
    #[test]
    fn native_particle_destructor(){for line in include_str!("../../../tools/emu/golden_particle_destroy.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let obj=0x2a000000;m.write(obj,&bytes(p[0]));let mut owner=Owner(vec![]);destroy_scene_particle(&mut m,obj,&mut owner);assert_eq!(m.read(obj,0xb0),bytes(p[1]));assert_eq!(owner.0,p[2].split(',').map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>());}}
}

/// `0x4c47f0`: release five descriptor arrays, including null pointers.
pub fn destroy_particle_descriptor(m:&mut Mem,descriptor:u32,free:&mut impl FnMut(&mut Mem,u32)) {
    for offset in [8,12,16,20,24] {free(m,m.u32(descriptor+offset));}
}
/// `0x5fa720`: reverse-order descriptor destruction and allocation release.
/// Base node destruction (`0x5ef660`) is still owner supplied.
pub fn destroy_particle_texture(m:&mut Mem,node:u32,mut free:impl FnMut(&mut Mem,u32),mut base:impl FnMut(&mut Mem,u32)) {
    m.set_u32(node,0x652d10);let geometry=m.u32(node+0x34);
    if geometry!=0 {let count=m.u32(geometry-4);for i in (0..count).rev(){destroy_particle_descriptor(m,geometry+i*40,&mut free);}free(m,geometry-4);}
    base(m,node);
}
#[cfg(test)]
mod texture_destroy_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_texture_destruction(){for line in include_str!("../../../tools/emu/golden_texture_destroy.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let node=0x2a000000;m.write(node,&bytes(p[0]));let payload=bytes(p[1]);m.set_u32(node+0x1000,(payload.len()/40) as u32);m.write(node+0x1004,&payload);let mut trace=vec![];let mut base_calls=0;destroy_particle_texture(&mut m,node,|_,ptr|trace.push(ptr),|_,obj|{assert_eq!(obj,node);base_calls+=1;});assert_eq!(base_calls,1);assert_eq!(m.read(node,0x44),bytes(p[2]));assert_eq!(trace,p[3].split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>());}}
}

/// `0x5ef660`: free node name storage and restore the base vtable.
pub fn destroy_effect_node_base(m:&mut Mem,node:u32,free:&mut impl FnMut(&mut Mem,u32)) {
    m.set_u32(node,0x652840);free(m,m.u32(node+12));m.set_u32(node,0x64d81c);
}
/// Full texture geometry/name destruction, with only the memory free external.
pub fn destroy_particle_texture_complete(m:&mut Mem,node:u32,mut free:impl FnMut(&mut Mem,u32)) {
    m.set_u32(node,0x652d10);let geometry=m.u32(node+0x34);
    if geometry!=0 {let count=m.u32(geometry-4);for i in (0..count).rev(){destroy_particle_descriptor(m,geometry+i*40,&mut free);}free(m,geometry-4);}
    destroy_effect_node_base(m,node,&mut free);
}
#[cfg(test)]
mod texture_full_destroy_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_complete_texture_destruction(){for line in include_str!("../../../tools/emu/golden_texture_destroy_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let node=0x2a000000;m.write(node,&bytes(p[0]));let payload=bytes(p[1]);m.set_u32(node+0x1000,(payload.len()/40) as u32);m.write(node+0x1004,&payload);let mut trace=vec![];destroy_particle_texture_complete(&mut m,node,|_,ptr|trace.push(ptr));assert_eq!(m.read(node,0x44),bytes(p[2]));assert_eq!(trace,p[3].split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>());}}
}

/// `0x5ef780`: unlink a scene node; linked-list helper is explicit.
pub fn unlink_effect_node(m:&mut Mem,node:u32,mut linked:impl FnMut(&mut Mem,u32))->bool {
    if m.u32(node+4)!=0 {linked(m,node);m.set_u32(node+0x2c,0);m.set_u32(node+0x24,0);return true;}
    let parent=m.u32(node+0x2c);if parent==0 {return false;}
    let count=m.i32(parent+0x34);let slots=m.u32(parent+0x30);
    let mut i=0;while i<count && m.u32(slots+i as u32*4)!=node {i+=1;}
    if i==count {return false;}
    let next=m.u32(node+8);m.set_u32(slots+i as u32*4,next);if next!=0 {m.set_u32(next+4,0);}
    m.set_u32(node+4,0);m.set_u32(node+8,0);m.set_u32(node+0x2c,0);true
}
#[cfg(test)]
mod node_unlink_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_node_unlink(){for line in include_str!("../../../tools/emu/golden_effect_node_unlink.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let base=0x2a000000;m.write(base,&bytes(p[0]));let result=unlink_effect_node(&mut m,base+0x100,|_,node|assert_eq!(node,base+0x100));assert_eq!(result,p[2]=="1");assert_eq!(m.read(base,0x400),bytes(p[1]));}}
}

/// Complete `0x5ef780`, using the shared original linked-list unlink.
pub fn unlink_effect_node_complete(m:&mut Mem,node:u32)->bool {
    unlink_effect_node(m,node,crate::controller::unlink)
}
#[cfg(test)]
mod node_unlink_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_complete_node_unlink(){for line in include_str!("../../../tools/emu/golden_effect_node_unlink_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let base=0x2a000000;m.write(base,&bytes(p[0]));assert_eq!(unlink_effect_node_complete(&mut m,base+0x100),p[2]=="1");assert_eq!(m.read(base,0x400),bytes(p[1]));}}
}

/// Complete particle destructor chain, with only memory free owner-supplied.
/// `0x5e48f0` is an empty method in the supported executable.
pub fn destroy_scene_particle_complete(m:&mut Mem,object:u32,mut free:impl FnMut(&mut Mem,u32)) {
    m.set_u32(object,0x650c24);m.set_u32(object+0x14,0x650bd8);
    m.set_u32(object,0x650bd0);m.set_u32(object+0x14,0x650b84);
    unlink_effect_node_complete(m,object+0x14);
    destroy_particle_texture_complete(m,object+0x14,&mut free);
    m.set_u32(object,0x650334);m.set_u32(object+4,0);
}
#[cfg(test)]
mod particle_complete_destroy_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_complete_particle_destruction(){for line in include_str!("../../../tools/emu/golden_particle_destroy_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let obj=0x2a000000;m.write(obj,&bytes(p[0]));let payload=bytes(p[1]);m.set_u32(obj+0x1000,(payload.len()/40) as u32);m.write(obj+0x1004,&payload);let mut trace=vec![];destroy_scene_particle_complete(&mut m,obj,|_,ptr|trace.push(ptr));assert_eq!(m.read(obj,0xb0),bytes(p[2]));assert_eq!(trace,p[3].split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>());}}
}

/// Scalar deleting branch of `0x5a2060` (manager passes flags=1).
/// Array deletion is a different ABI and is not handled by this entry point.
pub fn delete_scene_particle(m:&mut Mem,object:u32,free:impl FnMut(&mut Mem,u32)) {
    destroy_scene_particle_complete(m,object,free);
    recycle_effect_object(m,m.u32(object+16),object);
}
#[cfg(test)]
mod particle_delete_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_delete_and_recycle(){for line in include_str!("../../../tools/emu/golden_particle_delete.tsv").lines(){let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let obj=0x2a000000;m.write(obj,&bytes(input));delete_scene_particle(&mut m,obj,|_,_|{});assert_eq!(m.read(obj,0x400),bytes(output));}}
}

/// Cleanup for pools known to contain scene particles of the supported class.
/// Generic managers must dispatch other classes to their own destructors.
pub fn cleanup_scene_particle_pools(m:&mut Mem,manager:u32,mut free:impl FnMut(&mut Mem,u32)) {
    cleanup_effect_pools(m,manager,|m,object|delete_scene_particle(m,object,&mut free));
}
#[cfg(test)]
mod particle_cleanup_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_full_particle_cleanup(){for line in include_str!("../../../tools/emu/golden_particle_cleanup_full.tsv").lines(){let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let manager=0x2a000000;m.write(manager,&bytes(input));cleanup_scene_particle_pools(&mut m,manager,|_,_|{});assert_eq!(m.read(manager,0x2000),bytes(output));}}
}

/// Update phase of `0x5796e0`: repeat prefixes when callbacks prepend objects.
/// Next is cached before each virtual update, as in the original.
pub fn update_effect_pools(m:&mut Mem,manager:u32,mut update:impl FnMut(&mut Mem,u32)) {
    let mut i=0;
    while i<m.u32(manager+0x1b0) {
        let pool=m.u32(manager+0x20+i*4);let mut boundary=0;
        loop {
            let first=m.u32(pool+8);let mut object=first;
            while object!=boundary {let next=m.u32(object+8);update(m,object);object=next;}
            boundary=first;if m.u32(pool+8)==first {break;}
        }
        i+=1;
    }
}
/// Complete manager phases, with virtual class-specific operations external.
pub fn tick_effect_pools(m:&mut Mem,manager:u32,update:impl FnMut(&mut Mem,u32),destroy:impl FnMut(&mut Mem,u32)) {
    update_effect_pools(m,manager,update);cleanup_effect_pools(m,manager,destroy);
}
#[cfg(test)]
mod manager_tick_tests {
    use super::*;
    #[test]
    fn native_manager_update_order(){for line in include_str!("../../../tools/emu/golden_effect_manager_tick.tsv").lines(){let (input,output)=line.split_once('\t').unwrap();let v:Vec<u32>=input.split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let manager=0x2a000000;let pool=manager+0x1000;m.set_u32(manager+0x1b0,1);m.set_u32(manager+0x20,pool);m.set_u32(pool+8,if v[0]==0 {0}else{pool+0x100});for i in 0..v[0]{let obj=pool+0x100+i*32;m.set_u32(obj+8,if i+1<v[0]{obj+32}else{0});}let mut insert=v[1]!=0;let mut trace=vec![];tick_effect_pools(&mut m,manager,|m,obj|{trace.push(obj);if insert {insert=false;let new=pool+0x800;m.set_u32(new+8,m.u32(pool+8));m.set_u32(pool+8,new);}},|_,_|panic!("unexpected deletion"));assert_eq!(trace,output.split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>());}}
}

/// Manager tick for homogeneous pools of this particle class.
/// Scratch and free services belong to the runtime owner.
pub fn tick_scene_particle_pools(m:&mut Mem,manager:u32,scratch:u32,mut free:impl FnMut(&mut Mem,u32)) {
    tick_effect_pools(m,manager,|m,obj|tick_scene_particle(m,obj,scratch),|m,obj|delete_scene_particle(m,obj,&mut free));
}
#[cfg(test)]
mod particle_manager_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_multitick_particle_manager(){for line in include_str!("../../../tools/emu/golden_particle_manager_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let manager=0x2a000000;m.write(manager,&bytes(p[1]));for _ in 0..p[0].parse::<u32>().unwrap(){tick_scene_particle_pools(&mut m,manager,manager+0x10000,|_,_|{});}assert_eq!(m.read(manager,0x5000),bytes(p[2]));}}
}

/// `0x5ef6d0 -> 0x5fa6f0`: texture-node identity and owned NUL-terminated name.
/// Caller provides name bytes without terminator; empty name does not allocate.
pub fn initialize_particle_texture_node(m:&mut Mem,node:u32,name:&[u8],mut allocate:impl FnMut(&mut Mem,u32)->u32)->Result<(),&'static str> {
    m.set_u32(node,0x652840);
    for offset in [4,8,0x14,0x18,0x1c,0x20] {m.set_u32(node+offset,0);}
    if name.is_empty() {m.set_u32(node+12,0);}else{
        let size=name.len() as u32+1;let p=allocate(m,size);if p==0 {return Err("particle node name allocation failed");}
        m.set_u32(node+12,p);m.write(p,name);m.write(p+name.len() as u32,&[0]);
    }
    m.write(node+0x28,&[0]);m.set_u32(node+0x10,11);m.set_u32(node+0x2c,0);m.set_u32(node+0x24,0);
    for offset in [0x38,0x3c,0x40,0x30,0x34] {m.set_u32(node+offset,0);}m.set_u32(node,0x652d10);Ok(())
}
#[cfg(test)]
mod texture_node_ctor_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_texture_node_constructor(){for line in include_str!("../../../tools/emu/golden_texture_node_ctor.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let node=0x2a000000;m.write(node,&bytes(p[0]));let value=if p[2]=="0" {vec![]}else{bytes(p[1])};let mut seen=vec![];initialize_particle_texture_node(&mut m,node,&value,|_,size|{seen.push(size);node+0x2000}).unwrap();assert_eq!(m.read(node,0x44),bytes(p[3]));assert_eq!(seen,p[4].split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>());if !value.is_empty(){let mut want=value;want.push(0);assert_eq!(m.read(node+0x2000,want.len()),want);}}}
}

/// Real base-identity chain called at the start of root particle construction.
/// The owner retains allocation responsibility for the copied node name.
pub fn initialize_particle_identities(m:&mut Mem,object:u32,name:&[u8],allocate:impl FnMut(&mut Mem,u32)->u32)->Result<(),&'static str> {
    initialize_effect_identity(m,object);
    initialize_particle_texture_node(m,object+0x14,name,allocate)
}
#[cfg(test)]
mod particle_identity_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_identity_chain(){for line in include_str!("../../../tools/emu/golden_particle_identity_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let obj=0x2a000000;m.write(obj,&bytes(p[0]));for (i,v) in p[1].split(',').enumerate(){m.set_u32(0x6f4bc8+i as u32*4,v.parse().unwrap());}let name=bytes(p[2]);initialize_particle_identities(&mut m,obj,&name,|_,_|obj+0x2000).unwrap();assert_eq!(m.read(obj,0xb0),bytes(p[3]));}}
}

/// Actual particle manager class uses `0x593270`, always slot zero.
/// `mode` is ignored by this class; other scene classes may differ.
pub fn register_particle_sort_node(m:&mut Mem,manager:u32,node:u32)->bool {
    if m.i32(manager+0x34)<=0 {return false;}
    m.set_u32(node+0x2c,manager);m.set_u32(node+0x24,m.u32(manager+0x24));
    let slots=m.u32(manager+0x30);let old=m.u32(slots);
    m.set_u32(node+8,old);m.set_u32(node+4,0);if old!=0 {m.set_u32(old+4,node);}m.set_u32(slots,node);true
}
#[cfg(test)]
mod particle_register_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_particle_registration(){for line in include_str!("../../../tools/emu/golden_particle_register.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let manager=0x2a000000;m.write(manager,&bytes(p[0]));assert_eq!(register_particle_sort_node(&mut m,manager,manager+0x100),p[2]=="1");assert_eq!(m.read(manager,0x400),bytes(p[1]));}}
}

/// Use the actual sort-node registration while retaining owner's memory services.
pub struct RegisteredParticleOwner<'a,T:ParticleRootOwner>(pub &'a mut T);
impl<T:ParticleRootOwner> ParticleRootOwner for RegisteredParticleOwner<'_,T> {
    fn initialize_identities(&mut self,m:&mut Mem,object:u32){self.0.initialize_identities(m,object);}
    fn allocate_zeroed(&mut self,m:&mut Mem,size:u32)->u32{self.0.allocate_zeroed(m,size)}
    fn initialize_descriptor(&mut self,m:&mut Mem,descriptor:u32){self.0.initialize_descriptor(m,descriptor);}
    fn register(&mut self,m:&mut Mem,manager:u32,_mode:u32,node:u32){register_particle_sort_node(m,manager,node);}
}
#[cfg(test)]
mod registration_roundtrip_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_registration_unlink_sequences(){for line in include_str!("../../../tools/emu/golden_particle_registration_roundtrip.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let manager=0x2a000000;m.write(manager,&bytes(p[0]));for i in 0..p[1].parse::<u32>().unwrap(){assert!(register_particle_sort_node(&mut m,manager,manager+0x200+i*0x80));}for node in p[2].split(',').filter(|s|!s.is_empty()){assert!(unlink_effect_node_complete(&mut m,node.parse().unwrap()));}assert_eq!(m.read(manager,0x1000),bytes(p[3]));}}
}

/// `0x4c5000`: descriptor constructor clears owned pointers and colour only.
/// All other bytes intentionally retain their pre-construction contents.
pub fn initialize_particle_descriptor(m:&mut Mem,descriptor:u32) {
    for offset in [0x18,0x14,0x10,0xc,8,0x24] {m.set_u32(descriptor+offset,0);}
}
/// Owner adapter using actual descriptor initialization and sort registration.
/// Memory and base identity initialization remain delegated to the runtime.
pub struct GeometryParticleOwner<'a,T:ParticleRootOwner>(pub &'a mut T);
impl<T:ParticleRootOwner> ParticleRootOwner for GeometryParticleOwner<'_,T> {
    fn initialize_identities(&mut self,m:&mut Mem,obj:u32){self.0.initialize_identities(m,obj);}
    fn allocate_zeroed(&mut self,m:&mut Mem,size:u32)->u32{self.0.allocate_zeroed(m,size)}
    fn initialize_descriptor(&mut self,m:&mut Mem,descriptor:u32){initialize_particle_descriptor(m,descriptor);}
    fn register(&mut self,m:&mut Mem,manager:u32,_mode:u32,node:u32){register_particle_sort_node(m,manager,node);}
}
#[cfg(test)]
mod descriptor_ctor_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_descriptor_constructor(){for line in include_str!("../../../tools/emu/golden_particle_descriptor_ctor.tsv").lines(){let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let descriptor=0x2a000000;m.write(descriptor,&bytes(input));initialize_particle_descriptor(&mut m,descriptor);assert_eq!(m.read(descriptor,40),bytes(output));}}
}

/// Runtime constructor owner with actual identity, descriptor and registration.
/// Allocator returns zeroed memory; any failure is retained in `error`.
pub struct AllocatedParticleOwner<F:FnMut(&mut Mem,u32)->u32> {
    pub allocate:F,
    pub error:Option<&'static str>,
}
impl<F:FnMut(&mut Mem,u32)->u32> ParticleRootOwner for AllocatedParticleOwner<F> {
    fn initialize_identities(&mut self,m:&mut Mem,obj:u32){if let Err(e)=initialize_particle_identities(m,obj,b"motionTexture",&mut self.allocate){self.error=Some(e);}}
    fn allocate_zeroed(&mut self,m:&mut Mem,size:u32)->u32{if self.error.is_some(){0}else{(self.allocate)(m,size)}}
    fn initialize_descriptor(&mut self,m:&mut Mem,descriptor:u32){initialize_particle_descriptor(m,descriptor);}
    fn register(&mut self,m:&mut Mem,manager:u32,_mode:u32,node:u32){register_particle_sort_node(m,manager,node);}
}
#[cfg(test)]
mod owned_particle_chain_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_owned_particle_chain(){for line in include_str!("../../../tools/emu/golden_particle_owned_chain.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let obj=0x2a000000;let manager=obj+0x10000;m.set_u32(0x6f6a68,manager);m.set_u32(manager+0x30,manager+0x2000);m.set_u32(manager+0x34,1);m.set_u32(v[4],v[6]);m.write(v[1],&bytes(p[1]));let mut next=obj+0x20000;let mut owner=AllocatedParticleOwner{allocate:|m:&mut Mem,size:u32|{let p=next;next+=0x100;m.write(p,&vec![0;size as usize]);p},error:None};construct_complete_scene_particle(&mut m,obj,v[..6].try_into().unwrap(),obj+0x30000,&mut owner).unwrap();assert_eq!(owner.error,None);assert_eq!(m.read(obj,0xb0),bytes(p[2]));assert_eq!(m.read(obj+0x20104,40),bytes(p[3]));assert_eq!(m.u32(manager+0x2000),obj+0x14);}}
}

/// Complete successful `0x4bac90` emission through pool and constructor chain.
/// The allocator owns geometry/name memory; failures leave partial state for cleanup.
pub fn emit_complete_scene_particle(m:&mut Mem,point:u32,direction:u32,scratch:u32,allocate:impl FnMut(&mut Mem,u32)->u32)->Result<Option<u32>,&'static str> {
    let object=allocate_effect_object(m,m.u32(0x6f6a8c));if object==0 {return Ok(None);}
    let args=[m.u32(0x696224),point,direction,1f32.to_bits(),0x695bf0,m.u32(0x696208)];
    let mut owner=AllocatedParticleOwner{allocate,error:None};
    construct_complete_scene_particle(m,object,args,scratch,&mut owner)?;
    if let Some(e)=owner.error {return Err(e);}Ok(Some(object))
}
#[cfg(test)]
mod particle_emit_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_complete_emission(){for line in include_str!("../../../tools/emu/golden_particle_emit_full.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let obj=0x2a000000;let manager=obj+0x10000;let pool=obj+0x14000;m.set_u32(0x6f6a68,manager);m.set_u32(manager+0x30,manager+0x2000);m.set_u32(manager+0x34,1);m.set_u32(pool+4,obj);m.set_u32(0x6f6a8c,pool);m.set_u32(0x696208,v[5]);m.set_u32(0x695bf0,v[6]);m.write(v[1],&bytes(p[1]));let mut next=obj+0x20000;let result=emit_complete_scene_particle(&mut m,v[1],v[2],obj+0x30000,|m,size|{let p=next;next+=0x100;m.write(p,&vec![0;size as usize]);p}).unwrap();assert_eq!(result,Some(obj));assert_eq!(m.read(obj,0xb0),bytes(p[2]));assert_eq!(m.read(obj+0x20104,40),bytes(p[3]));assert_eq!(m.u32(pool+8),obj);assert_eq!(m.u32(manager+0x2000),obj+0x14);}}
}

#[cfg(test)]
mod particle_life_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_creation_update_deletion_cycle(){for line in include_str!("../../../tools/emu/golden_particle_life_full.tsv").lines(){
        let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let obj=0x2a000000;let manager=obj+0x10000;let pool=obj+0x14000;let tick_manager=obj+0x15000;
        m.set_u32(0x6f6a68,manager);m.set_u32(manager+0x30,manager+0x2000);m.set_u32(manager+0x34,1);m.set_u32(pool+4,obj);m.set_u32(0x6f6a8c,pool);m.set_u32(0x696224,obj+0x4000);m.set_u32(obj+16,pool);m.set_u32(0x696208,1);m.set_u32(0x695bf0,v[6]);m.write(v[1],&bytes(p[1]));let mut next=obj+0x20000;
        assert_eq!(emit_complete_scene_particle(&mut m,v[1],v[2],obj+0x30000,|m,size|{let p=next;next+=0x100;m.write(p,&vec![0;size as usize]);p}).unwrap(),Some(obj));
        m.set_u32(tick_manager+0x1b0,1);m.set_u32(tick_manager+0x20,pool);m.write(obj+0x4000+0xb90,&0.1f64.to_le_bytes());let mut freed=vec![];
        for _ in 0..p[4].parse::<u32>().unwrap(){tick_scene_particle_pools(&mut m,tick_manager,obj+0x30000,|_,ptr|freed.push(ptr));}
        assert_eq!(m.read(obj,0xb0),bytes(p[2]));assert_eq!(m.read(obj+0x20104,40),bytes(p[3]));assert_eq!(m.read(pool+4,8),bytes(p[5]));assert_eq!(m.read(manager+0x2000,4),bytes(p[6]));assert_eq!(freed,p[7].split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>());
    }}
}

/// `0x4b8c40`: enumerate smoke materials until the first missing resource.
/// Lookup returns material pointers; the table does not contain RGBA colours.
pub fn load_smoke_materials(m:&mut Mem,mut lookup:impl FnMut(&mut Mem,&str)->u32)->Result<u32,&'static str> {
    m.set_u32(0x696208,0);let mut count=0;
    loop {
        let name=format!("msmoke{count:02}");let material=lookup(m,&name);m.set_u32(0x695bf0+count*4,material);
        if material==0 {break;}
        count+=1;m.set_u32(0x696208,count);m.set_f32(material+0x58,1.0);m.set_u32(material+0x20,m.u32(material+0x20)|0x40220);
    }
    if count==0 {Err("missing smoke materials")}else{Ok(count)}
}
#[cfg(test)]
mod smoke_material_tests {
    use super::*;
    #[test]
    fn native_smoke_material_enumeration(){for line in include_str!("../../../tools/emu/golden_smoke_materials.tsv").lines(){let p:Vec<_>=line.split('\t').collect();let count=p[0].parse::<u32>().unwrap();let mut m=Mem::new();let mut names=vec![];let actual=load_smoke_materials(&mut m,|_,name|{names.push(name.to_owned());let i=names.len() as u32-1;if i<count {0x2a000000+i*0x100}else{0}}).unwrap();assert_eq!(actual,count);assert_eq!(names.join(","),p[1]);let want:Vec<u32>=p[2].split(',').map(|s|s.parse().unwrap()).collect();for i in 0..count {let material=0x2a000000+i*0x100;assert_eq!(m.u32(material+0x20),want[i as usize]);assert_eq!(m.f32(material+0x58),1.0);assert_eq!(m.u32(0x695bf0+i*4),material);}}}
}

/// Position argument of a wheel effect request (`0x4bb050`, first parameter).
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum WheelEffectPoint {
    /// Computed in the caller's frame: own wheels (`+0x5460 == 0`).
    World([f32;3]),
    /// Address inside the linked object (`+0x5460 + 0x2ba4 + 0xf8*i`).
    Linked(u32),
}
/// One wheel request: point, direction, intensity (raw dword of `+0x27f0+4k`).
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct WheelEffect {pub point:WheelEffectPoint,pub direction:[f32;3],pub intensity:u32}

/// Wheel requests of `0x551970` in `0x551cf3..0x551ff4`; the sink `0x4bb050` is not performed.
/// `basis` is the 12-float result of `scene_geometry_basis`.
pub fn scene_wheel_requests(m:&mut Mem,scene:u32,basis:u32)->Vec<WheelEffect> {
    let mut out=Vec::new();
    let tech=0x697a10u32.wrapping_add(m.u32(scene+0x28b4).wrapping_mul(0x970));
    let b:Vec<f64>=(0..12).map(|i|m.f32(basis+i*4) as f64).collect();
    let direction=|m:&Mem,wheel:u32|{
        let s=if m.f32(tech+wheel*12)>0.0 {1.0f64}else{-1.0};
        let d0=(s*m.f32(0x702640) as f64) as f32 as f64;let d1=(s*m.f32(0x702644) as f64) as f32 as f64;let d2=s*m.f32(0x702648) as f64;
        [((d2*b[6]+d1*b[3])+d0*b[0]) as f32,((d2*b[7]+d1*b[4])+b[1]*d0) as f32,((d2*b[8]+d1*b[5])+b[2]*d0) as f32]
    };
    let owner=m.u32(scene+0x5460);
    if owner!=0 {
        for i in 0..m.u32(scene+0x28bc) as i32 {
            let i=i as u32;let direction=direction(m,i);let value=m.f32(scene+0x27f0+i*4);
            if value as f64>0.1 {out.push(WheelEffect{point:WheelEffectPoint::Linked(owner.wrapping_add(0x2ba4+0xf8*i)),direction,intensity:m.u32(scene+0x27f0+i*4)});}
        }
    }else{
        for k in 0..4u32 {
            let idx=m.u32(scene+0x4fb0+k*4);let direction=direction(m,idx);
            let value=m.f32(scene+0x27f0+idx.wrapping_mul(4));
            if m.rd8(scene+0x2821)&(1<<k)!=0 && value as f64>0.1 {
                let p:Vec<f64>=(0..3).map(|j|m.f32(scene+0x282c+k*12+j*4) as f64).collect();
                let point=[
                    (((b[3]*p[1]+b[6]*p[2])+b[0]*p[0])+b[9]) as f32,
                    (((b[4]*p[1]+b[7]*p[2])+b[1]*p[0])+b[10]) as f32,
                    (((b[5]*p[1]+b[8]*p[2])+b[2]*p[0])+b[11]) as f32];
                out.push(WheelEffect{point:WheelEffectPoint::World(point),direction,intensity:m.u32(scene+0x27f0+k*4)});
            }
        }
    }
    out
}
#[cfg(test)]
mod wheel_effect_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_wheel_effects(){
        for line in include_str!("../../../tools/emu/golden_scene_wheel_effect.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let b=bytes(v[0]);let f:Vec<u32>=v[6].split(',').map(|s|s.parse().unwrap()).collect();
            let mut m=Mem::new();let p=0x2a000000;
            m.write(p+0x2c4,&b[..48]);m.write(p+0x204,&b[48..]);m.write(0x697a10+f[0]*0x970,&bytes(v[1]));m.write(0x702640,&bytes(v[2]));
            m.write(p+0x4fb0,&bytes(v[3]));m.write(p+0x27f0,&bytes(v[4]));m.write(p+0x282c,&bytes(v[5]));m.write(p+0x2821,&[f[3] as u8]);
            m.set_u32(p+0x28b4,f[0]);m.set_u32(p+0x28bc,f[2]);m.set_u32(p+0x5460,if f[1]!=0 {0x2a020000}else{0});
            scene_geometry_basis(&mut m,p,p+0x10000);
            let actual:Vec<u8>=scene_wheel_requests(&mut m,p,p+0x10000).into_iter().flat_map(|e|{
                let mut o=match e.point{WheelEffectPoint::World(w)=>w.iter().flat_map(|x|x.to_le_bytes()).collect::<Vec<u8>>(),WheelEffectPoint::Linked(a)=>a.to_le_bytes().to_vec()};
                o.extend(e.direction.iter().flat_map(|x|x.to_le_bytes()));o.extend(e.intensity.to_le_bytes());o}).collect();
            assert_eq!(actual,if v[7]=="-" {vec![]}else{bytes(v[7])},"{line}");
        }
    }
}

/// Arguments of the dust particle constructor `0x59d790` as passed by `0x4bb050`.
#[derive(Debug,Clone,Copy,PartialEq)]
/// `material_table` holds `[0x696218]`, the dust frame count (the name is historical).
pub struct DustParticleRequest {pub object:u32,pub position:u32,pub velocity:[f32;3],pub size_a:f32,pub size_b:f32,pub material_table:u32,pub tail:u32}

/// `0x4bb050`: probabilistic dust emission for one wheel.
/// `rand` is the C library generator, `gate` is `0x4bb010`, `alloc` is `0x579380(200, [0x6f6a9c])`
/// and `construct` is `0x59d790`; all four are external here. Returns whether a particle was built.
pub fn emit_wheel_dust(m:&mut Mem,position:u32,direction:u32,intensity:f32,
    mut rand:impl FnMut(&mut Mem)->i32,gate:impl FnOnce(&mut Mem)->bool,alloc:impl FnOnce(&mut Mem,u32,u32)->u32,
    construct:impl FnOnce(&mut Mem,DustParticleRequest))->bool {
    const K:f64=f32::from_bits(0x38000100) as f64;
    if !gate(m) {return false;}
    if m.f32(0x69603c)<0.5 || m.f32(0x69603c).is_nan() {return false;}
    if (rand(m) as f64*K)>0.800000011920929 {return false;}
    let intensity=if intensity>1.0 {1.0} else {intensity};
    let clock=m.u32(0x696224);
    let mut inverse=1.0/m.f64(clock+0xb90);
    if inverse>20.0 {inverse=20.0;}
    let limit=((m.u32(0x671700) as f64*inverse)*(0.05f32 as f64)) as i64 as u32;
    if m.u32(0x6f6a6c)>limit {return false;}
    let a=(intensity as f64*0.5+1.0) as f32 as f64;
    let size_a=(((rand(m) as f64*0.5*K)+1.0)*a*2.0) as f32;
    let size_b=(((rand(m) as f64*0.5*K)+1.0)*a*3.5) as f32;
    let object=alloc(m,200,m.u32(0x6f6a9c));
    if object!=0 {
        let s=(rand(m) as f64*K)*(0.1f32 as f64)+(0.8f32 as f64);let s32=s as f32 as f64;
        let velocity=[(s32*m.f32(direction) as f64) as f32,(s32*m.f32(direction+4) as f64) as f32,(s*m.f32(direction+8) as f64) as f32];
        let request=DustParticleRequest{object,position,velocity,size_a,size_b,material_table:m.u32(0x696218),tail:m.u32(0x694830)};
        construct(m,request);
    }
    m.set_u32(0x6f6a70,m.u32(0x694830));
    object!=0
}
#[cfg(test)]
mod wheel_dust_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_wheel_dust(){
        for line in include_str!("../../../tools/emu/golden_wheel_dust_emit.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let (pos,dir,clock)=(0x2a000100u32,0x2a000120u32,0x2a030000u32);
            let intensity=f32::from_le_bytes(bytes(v[0]).try_into().unwrap());
            m.write(dir,&bytes(v[1]));m.write(pos,&bytes(v[2]));m.write(0x69603c,&bytes(v[3]));m.set_u32(0x696224,clock);m.write(clock+0xb90,&bytes(v[4]));
            m.set_u32(0x671700,200);
            let g:Vec<u32>=v[5].split(',').map(|s|s.parse().unwrap()).collect();
            m.set_u32(0x6f6a6c,g[0]);m.set_u32(0x694830,g[3]);m.set_u32(0x696218,g[4]);m.set_u32(0x6f6a9c,g[5]);m.set_u32(0x6f6a70,0);
            let rands:Vec<i32>=v[6].split(',').map(|s|s.parse().unwrap()).collect();let used=std::cell::Cell::new(0usize);let gate_called=std::cell::Cell::new(false);let alloc_called=std::cell::Cell::new(false);
            let mut got=None;
            emit_wheel_dust(&mut m,pos,dir,intensity,|_|{let r=rands[used.get()];used.set(used.get()+1);r},|_|{gate_called.set(true);g[1]!=0},
                |_,size,pool|{assert_eq!((size,pool),(200,g[5]));alloc_called.set(true);g[2]},|_,r|got=Some(r));
            let f:Vec<u32>=v[7].split(',').map(|s|s.parse().unwrap()).collect();
            assert_eq!((gate_called.get() as u32,used.get() as u32,alloc_called.get() as u32),(f[0],f[1],f[2]),"{line}");
            let actual:Vec<u8>=got.map(|r|{let mut o=Vec::new();for x in [r.object,r.size_a.to_bits(),r.size_b.to_bits(),r.material_table,r.tail]{o.extend(x.to_le_bytes());}for x in r.velocity{o.extend(x.to_le_bytes());}o}).unwrap_or_default();
            assert_eq!(actual,if v[8]=="-" {vec![]}else{bytes(v[8])},"{line}");
            assert_eq!(m.u32(0x6f6a70),u32::from_le_bytes(bytes(v[9]).try_into().unwrap()),"{line}");
        }
    }
}

/// Arguments of `0x59d790` (after `this`): clock, position vector, velocity vector, two sizes,
/// material table, frame count, kind (0..2 select the tag 0xfc..0xfe), lifetime.
#[derive(Debug,Clone,Copy)]
pub struct DustParticleArgs {pub clock:u32,pub position:u32,pub velocity:u32,pub size_a:f32,pub size_b:f32,pub table:u32,pub frames:u32,pub kind:u32,pub lifetime:f32}

/// `0x59d790` including the `0x59abb0` body. The root constructor `0x59a780` is supplied by the
/// owner with its 11 arguments `[clock, manager, x, y, z, 2*size_a, size_a, table, frames, lifetime/frames, lifetime]`.
/// `scratch` is a caller-owned 12-byte vector (the original's stack temporary).
pub fn construct_dust_particle(m:&mut Mem,object:u32,a:DustParticleArgs,scratch:u32,
    mut rand:impl FnMut(&mut Mem)->i32,root:impl FnOnce(&mut Mem,u32,[u32;11])) {
    const K:f64=f32::from_bits(0x38000100) as f64;
    let (xyz,sa2,sb2)=(std::array::from_fn::<u32,3,_>(|i|m.u32(a.position+i as u32*4)),(a.size_a as f64+a.size_a as f64) as f32,(a.size_b as f64+a.size_b as f64) as f32);
    let per_frame=(a.lifetime as f64/a.frames as f64) as f32;
    root(m,object,[a.clock,m.u32(0x6f6a68),xyz[0],xyz[1],xyz[2],sa2.to_bits(),a.size_a.to_bits(),a.table,a.frames,per_frame.to_bits(),a.lifetime.to_bits()]);
    for (o,v) in [(0x84,sa2),(0x88,sb2),(0x8c,a.size_a),(0x90,a.size_b)] {m.set_f32(object+o,v);}
    m.set_u32(object,0x650f34);m.set_u32(object+0x14,0x650ee8);
    for o in (0x94..0xb0).step_by(4) {m.set_u32(object+o,0);}
    m.set_u32(object+0x80,m.u32(object+0x80)|1);
    for i in 0..3 {let v=m.u32(a.velocity+i*4);m.set_u32(object+0xa0+i*4,v);}
    let (ax,ay,az)=(m.f32(0x702630) as f64,m.f32(0x702634) as f64,m.f32(0x702638) as f64);
    let (vx,vy,vz)=(m.f32(object+0xa0) as f64,m.f32(object+0xa4) as f64,m.f32(object+0xa8) as f64);
    m.set_f32(object+0xac,(az*vy-ay*vz) as f32);m.set_f32(object+0xb0,(ax*vz-az*vx) as f32);m.set_f32(object+0xb4,(ay*vx-ax*vy) as f32);
    let len=crate::math::vec_len(m,object+0xac);
    if len<0.01 {
        for i in 0..3 {let v=m.u32(0x702640+i*4);m.set_u32(object+0xac+i*4,v);}
    }else{
        crate::matrix::vec_div_in_place(m,object+0xac,scratch,len as f32);
    }
    crate::matrix::vec_scale_in_place(m,object+0xac,scratch,0.5);
    m.set_u32(object+0xc0,0);
    let first=(rand(m) as f64*K) as f32;
    let second=rand(m) as f64*K;
    m.set_f32(object+0xb8,((second-first as f64)*f32::from_bits(0x40490fda) as f64) as f32);
    let third=rand(m) as f64*f32::from_bits(0x35ccce67) as f64;m.set_f32(object+0xbc,third as f32);
    for i in 0..3 {let v=m.u32(a.position+i*4);m.set_u32(object+0x94+i*4,v);}
    m.set_u32(0x6f6a6c,m.u32(0x6f6a6c).wrapping_add(1));
    match a.kind {0=>m.set_u32(object+0xc4,0xfc),1=>m.set_u32(object+0xc4,0xfd),2=>m.set_u32(object+0xc4,0xfe),_=>{}}
}
#[cfg(test)]
mod dust_ctor_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_dust_constructor(){
        for line in include_str!("../../../tools/emu/golden_dust_ctor.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let axes=bytes(v[0]);let mut m=Mem::new();let (obj,pos,vel,scratch)=(0x2a040000u32,0x2a000100u32,0x2a000120u32,0x2a050000u32);
            let p:Vec<u32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();
            m.write(0x702630,&axes[..12]);m.write(0x702640,&axes[12..]);m.write(pos,&bytes(v[1]));m.write(vel,&bytes(v[2]));m.write(obj,&bytes(v[5]));
            m.set_u32(0x6f6a68,p[6]);m.set_u32(0x6f6a6c,p[7]);
            let rands:Vec<i32>=v[4].split(',').map(|s|s.parse().unwrap()).collect();let used=std::cell::Cell::new(0usize);let mut got=vec![];
            construct_dust_particle(&mut m,obj,DustParticleArgs{clock:p[8],position:pos,velocity:vel,size_a:f32::from_bits(p[1]),size_b:f32::from_bits(p[2]),table:p[3],frames:p[4],kind:p[0],lifetime:f32::from_bits(p[5])},scratch,
                |_|{let r=rands[used.get()];used.set(used.get()+1);r},|_,o,args|{assert_eq!(o,obj);got=args.to_vec();});
            let want:Vec<u32>=v[6].split(',').skip(1).map(|s|s.parse().unwrap()).collect();
            assert_eq!(got,want,"root args {line}");
            assert_eq!(m.read(obj,0xc8),bytes(v[7]),"{line}");
            assert_eq!(m.u32(0x6f6a6c),v[8].parse::<u32>().unwrap());assert_eq!(used.get().to_string(),v[9]);
        }
    }
}

/// `0x59dbc0` (vtable `0x650f34` slot 1): dust particle motion, swirl, size and geometry update.
/// `scratch` holds 0x100 bytes of caller-owned temporaries (the original's stack); the shared
/// lifecycle step `0x59aa10` is supplied by the caller.
pub fn update_dust_particle(m:&mut Mem,object:u32,scratch:u32,mut lifecycle:impl FnMut(&mut Mem,u32)) {
    m.set_u32(object+0x64,m.u32(0x6f6a70));
    let dt=m.f64(m.u32(object+0x58)+0xb90) as f32;let dt64=dt as f64;
    let roll=(dt64*m.f32(object+0xb8) as f64) as f32;
    let pos=[m.f32(0x702650),m.f32(0x702654),m.f32(0x702658)];
    let dir=[m.f32(object+0xa0),m.f32(object+0xa4),m.f32(object+0xa8)];
    let matrix=scratch;crate::matrix::mat_look(m,matrix,pos,dir,roll);
    crate::matrix::transform_in_place(m,object+0xac,matrix);
    let swirl=((dt64*m.f32(object+0xbc) as f64)+1.0) as f32;
    crate::matrix::vec_scale_in_place(m,object+0xac,scratch+0x40,swirl);
    crate::matrix::vec_add_in_place(m,object+0x4c,scratch+0x50,object+0xac);
    let (vx,vy,vz)=(m.f32(object+0xa0) as f64,m.f32(object+0xa4) as f64,m.f32(object+0xa8) as f64);
    let wind=m.u32(0x7025b8);
    let (px,py,pz)=(m.f32(object+0x4c) as f64,m.f32(object+0x50) as f64,m.f32(object+0x54) as f64);
    let (nx,ny,nz)=if wind==0 {
        (((dt64*vx)+px) as f32,(((dt64*vy) as f32 as f64)+py) as f32,(((dt64*vz) as f32 as f64)+pz) as f32)
    }else{
        let w=m.f32(wind+0x214) as f64;let k=0.2f32 as f64;
        let t0=(m.f32(0x702620) as f64*w) as f32 as f64;let t1=(m.f32(0x702624) as f64*w) as f32 as f64;let t2=w*m.f32(0x702628) as f64;
        let a0=(t0*k) as f32 as f64;let a1=(t1*k) as f32 as f64;let a2=t2*k;
        let x=(a0+vx) as f32 as f64;let y=(a1+vy) as f32 as f64;let z=a2+vz;
        let fx=(x*dt64) as f32 as f64;let fy=(y*dt64) as f32 as f64;let fz=z*dt64;
        ((fx+px) as f32,(fy+py) as f32,(fz+pz) as f32)
    };
    m.set_f32(object+0x4c,nx);m.set_f32(object+0x50,ny);m.set_f32(object+0x54,nz);
    let life=m.f32(object+0x64);let age=m.f32(object+0x68);
    let t=(life as f64-age as f64)/life as f64;
    let t=if t<0.0 {0.0} else if !(t<=1.0) && !t.is_nan() {1.0} else {t};
    let u=1.0-t;
    let a=((u*m.f32(object+0x88) as f64)+(t*m.f32(object+0x84) as f64)) as f32;
    let b=(u*m.f32(object+0x90) as f64)+(t*m.f32(object+0x8c) as f64);
    let base=m.f32(object+0x9c) as f64;
    let lower=b*(f32::from_bits(0x3eaaaaab) as f64)+base;
    let upper=((b*0.75)+base) as f32 as f64;
    let z0=m.f32(object+0x54) as f64;
    let mut z=if lower>z0 {lower} else {z0};
    if !(z<=upper) && !z.is_nan() {z=upper;}
    m.set_f32(object+0x54,z as f32);
    let g=m.u32(object+0x48);let (g1,g2)=(m.u32(g+0x10),m.u32(g+0x14));
    m.set_f32(g1,(a as f64*-0.5) as f32);m.set_f32(g1+12,(a as f64*-0.5) as f32);
    m.set_f32(g1+4,(a as f64*0.5) as f32);m.set_f32(g1+8,(a as f64*0.5) as f32);
    m.set_f32(g2,(b*-0.5) as f32);m.set_f32(g2+4,(b*-0.5) as f32);
    m.set_f32(g2+8,(b*0.5) as f32);m.set_f32(g2+12,(b*0.5) as f32);
    for i in 0..3 {let v=m.u32(object+0x4c+i*4);m.set_u32(object+0x28+i*4,v);}
    lifecycle(m,object);
}
#[cfg(test)]
mod dust_update_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_dust_update(){
        for line in include_str!("../../../tools/emu/golden_dust_update.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();
            let (obj,clock,geom,arr1,arr2,wind,scratch)=(0x2a040000u32,0x2a030000u32,0x2a050000u32,0x2a051000u32,0x2a052000u32,0x2a053000u32,0x2a060000u32);
            crate::globals::install_constants(&mut m);
            m.write(0x702620,&bytes(v[3]));m.write(0x6f6a70,&bytes(v[4]));
            m.write(obj,&bytes(v[0]));m.write(clock+0xb90,&bytes(v[1]));m.write(wind+0x214,&bytes(v[2]));
            m.set_u32(0x7025b8,if v[5]=="1" {wind}else{0});m.set_u32(geom+0x10,arr1);m.set_u32(geom+0x14,arr2);
            let mut called=0;let mut at=vec![];
            update_dust_particle(&mut m,obj,scratch,|m,o|{assert_eq!(o,obj);called=1;at=m.read(obj+0x28,12);});
            assert_eq!(m.read(obj,0xc8),bytes(v[6]),"{line}");
            assert_eq!(m.read(arr1,16),bytes(v[7]),"{line}");assert_eq!(m.read(arr2,16),bytes(v[8]),"{line}");
            assert_eq!(called.to_string(),v[9]);assert_eq!(at,bytes(v[10]));
        }
    }
}

/// `0x5a2cd0` with flags = 1 (scalar deleting destructor of the dust particle class):
/// `0x5a2ca0` decrements the live-dust counter `0x6f6a6c`, then runs the shared base destruction
/// `0x59aa90` and the pool recycle `0x5793f0`/`0x5794f0`. Memory release is supplied by the caller.
pub fn delete_dust_particle(m:&mut Mem,object:u32,free:impl FnMut(&mut Mem,u32)) {
    m.set_u32(0x6f6a6c,m.u32(0x6f6a6c).wrapping_sub(1));
    delete_scene_particle(m,object,free);
}
#[cfg(test)]
mod dust_delete_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_dust_delete(){
        for line in include_str!("../../../tools/emu/golden_dust_delete.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let obj=0x2a000000;
            m.write(obj,&bytes(v[0]));m.write(0x6f6a6c,&bytes(v[2]));
            delete_dust_particle(&mut m,obj,|_,_|{});
            assert_eq!(m.read(obj,0x400),bytes(v[1]));assert_eq!(m.read(0x6f6a6c,4),bytes(v[3]));
        }
    }
}

/// `0x5a9d40` (thiscall on the sky object): daylight brightness in `0..=1` at time offset `t`.
/// Uses x87 `fsin`; the port uses `f64::sin` (agreement checked to 1e-12 against the original).
pub fn sky_brightness(m:&Mem,sky:u32,t:f64)->f64 {
    let a=1.0-(m.f32(sky+0x220) as f64+m.f32(sky+0x220) as f64);
    let x=m.f64(sky+0x78).sin()*0.3+1.0;
    let y=1.0-a.abs();
    let s=(m.f64(sky+0x68)-1.5707963+t).sin();
    let v=((s*(x*y)+a)+1.0)*0.5;
    if v>1.0 {1.0} else if v<0.0 {0.0} else {v}
}
/// `0x4bb010`: dust is allowed while the sky is dim (`brightness <= 0.7`) and the sky's `+0x218`
/// value is at most 0.001. The sky object is `[0x6f33ac]`.
pub fn dust_gate(m:&Mem)->bool {
    let sky=m.u32(0x6f33ac);
    !(sky_brightness(m,sky,0.0)>0.7) && !(m.f32(sky+0x218)>f32::from_bits(0x3a83126f))
}
#[cfg(test)]
mod sky_gate_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_sky_gate(){
        for line in include_str!("../../../tools/emu/golden_sky_gate.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let sky=0x2a060000u32;
            m.write(sky,&bytes(v[0]));m.set_u32(0x6f33ac,sky);
            let t=f64::from_le_bytes(bytes(v[1]).try_into().unwrap());let want=f64::from_le_bytes(bytes(v[2]).try_into().unwrap());
            let got=sky_brightness(&m,sky,t);
            assert!((got-want).abs()<1e-12 || (got.is_nan()&&want.is_nan()),"{got} {want} {line}");
            assert_eq!(dust_gate(&m) as u32,v[3].parse::<u32>().unwrap(),"{line}");
        }
    }
}

/// Which particle constructor `0x4ba660` selected.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ExhaustKind {
    /// `0x59bed0` from pool `[0x6f6a7c]`, materials `0x695c58`.
    Fume,
    /// `0x59ad70` from pool `[0x6f6a80]`, materials `0x695bf0`.
    Smoke,
    /// `0x59b3f0` from pool `[0x6f6a88]`, materials `0x694998`.
    Soot,
}
/// Constructor call prepared by `0x4ba660` (the objects themselves are built by the owner).
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct ExhaustRequest {pub kind:ExhaustKind,pub object:u32,pub clock:u32,pub position:u32,pub direction:[f32;3],pub table:u32,pub extra:u32,pub tail:[u32;2]}

/// `0x4ba660`: random exhaust smoke for intensity `p >= 0.4`; above 0.7 an extra soot puff.
/// `rand` is the C `rand()`, `alloc(size, pool)` is `0x579380`.
pub fn emit_exhaust(m:&mut Mem,position:u32,direction:u32,p:f32,mut rand:impl FnMut(&mut Mem)->i32,
    mut alloc:impl FnMut(&mut Mem,u32,u32)->u32,mut construct:impl FnMut(&mut Mem,ExhaustRequest)) {
    const K:f64=f32::from_bits(0x38000100) as f64;
    let wait=(rand(m) as f64*K)+(0.2f32 as f64);
    let threshold=1.0-m.f32(0x696048) as f64;
    if !(threshold<wait) && !(threshold.is_nan()||wait.is_nan()) {return;}
    if !(p>=f32::from_bits(0x3ecccccd)) {return;}
    let chance=((p as f64-f32::from_bits(0x3ecccccd) as f64)*f32::from_bits(0x3fd55555) as f64) as f32;
    let dir:[f32;3]=std::array::from_fn(|i|m.f32(direction+i as u32*4));
    let clock=m.u32(0x696224);
    let fume=(rand(m) as f64*K)>chance as f64;
    let (kind,pool,table,extra)=if fume {(ExhaustKind::Fume,m.u32(0x6f6a7c),0x695c58,m.u32(0x69620c))}else{(ExhaustKind::Smoke,m.u32(0x6f6a80),0x695bf0,m.u32(0x696208))};
    let object=alloc(m,0xc0,pool);
    if object!=0 {construct(m,ExhaustRequest{kind,object,clock,position,direction:dir,table,extra,tail:[0x3fc00000,0]});}
    if p>f32::from_bits(0x3f333333) {
        let soot=[dir[0],dir[1],(dir[2] as f64*(0.8f32 as f64)) as f32];
        let pool=m.u32(0x6f6a88);let object=alloc(m,0xd4,pool);
        if object!=0 {construct(m,ExhaustRequest{kind:ExhaustKind::Soot,object,clock:m.u32(0x696224),position,direction:soot,table:0x694998,extra:m.u32(0x696210),tail:[0x3f000000,0x3f400000]});}
    }
}
#[cfg(test)]
mod exhaust_emit_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_exhaust_emit(){
        for line in include_str!("../../../tools/emu/golden_exhaust_emit.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let (pos,dirp)=(0x2a000100u32,0x2a000120u32);
            let p=f32::from_le_bytes(bytes(v[0]).try_into().unwrap());
            m.write(dirp,&bytes(v[1]));m.write(pos,&bytes(v[2]));m.write(0x696048,&bytes(v[3]));
            let g:Vec<u32>=v[4].split(',').map(|s|s.parse().unwrap()).collect();
            for (a,x) in [0x696224u32,0x696208,0x69620c,0x696210,0x6f6a7c,0x6f6a80,0x6f6a88].into_iter().zip(&g){m.set_u32(a,*x);}
            let rands:Vec<i32>=v[5].split(',').map(|s|s.parse().unwrap()).collect();let ret:u32=v[6].parse().unwrap();
            let used=std::cell::Cell::new(0usize);let mut allocs=vec![];let mut reqs=vec![];
            emit_exhaust(&mut m,pos,dirp,p,|_|{let r=rands[used.get()];used.set(used.get()+1);r},|_,s,pool|{allocs.push((s,pool));ret},|_,r|reqs.push(r));
            assert_eq!(used.get().to_string(),v[7],"{line}");
            let want_alloc:Vec<(u32,u32)>=if v[8]=="-" {vec![]}else{v[8].split(';').map(|s|{let (a,b)=s.split_once(':').unwrap();(a.parse().unwrap(),b.parse().unwrap())}).collect()};
            assert_eq!(allocs,want_alloc,"{line}");
            let want:Vec<&str>=if v[9]=="-" {vec![]}else{v[9].split(';').collect()};assert_eq!(reqs.len(),want.len(),"{line}");
            for (r,w) in reqs.iter().zip(want){
                let f:Vec<_>=w.split(':').collect();let a:Vec<u32>=f[2].split(',').map(|s|s.parse().unwrap()).collect();
                let name=match r.kind{ExhaustKind::Fume=>"A",ExhaustKind::Smoke=>"B",ExhaustKind::Soot=>"C"};
                assert_eq!(name,f[0]);assert_eq!(r.object.to_string(),f[1]);assert_eq!((r.clock,r.table,r.extra),(a[0],a[3],a[4]));
                assert_eq!(&r.tail[..if r.kind==ExhaustKind::Soot {2}else{1}],&a[5..],"{line}");
                assert_eq!(m.read(pos,12),bytes(f[3]));
                let dir:Vec<u8>=r.direction.iter().flat_map(|x|x.to_le_bytes()).collect();assert_eq!(dir,bytes(f[4]),"{line}");
            }
        }
    }
}

/// `0x59ad70` including the base `0x59ab10`: smoke particle (vtable `0x650c78`). Root `0x59a780`
/// is supplied by the owner. `position`/`direction` are 3-float vectors; `frames` must be non-zero.
pub fn construct_smoke_particle(m:&mut Mem,object:u32,clock:u32,position:u32,direction:u32,table:u32,frames:u32,lifetime:f32,
    rand:impl FnMut(&mut Mem)->i32,root:impl FnMut(&mut Mem,u32,[u32;11])) {
    construct_homing_particle(m,object,(0x650c78,0x650c2c),clock,position,direction,table,frames,lifetime,rand,root);
}
/// `0x59bed0`: same constructor with the fume class tables (`0x650d80`/`0x650d34`).
pub fn construct_fume_particle(m:&mut Mem,object:u32,clock:u32,position:u32,direction:u32,table:u32,frames:u32,lifetime:f32,
    rand:impl FnMut(&mut Mem)->i32,root:impl FnMut(&mut Mem,u32,[u32;11])) {
    construct_homing_particle(m,object,(0x650d80,0x650d34),clock,position,direction,table,frames,lifetime,rand,root);
}
/// Shared body of `0x59ad70` and `0x59bed0`; `tables` are the two method-table addresses written.
pub fn construct_homing_particle(m:&mut Mem,object:u32,tables:(u32,u32),clock:u32,position:u32,direction:u32,table:u32,frames:u32,lifetime:f32,
    mut rand:impl FnMut(&mut Mem)->i32,root:impl FnMut(&mut Mem,u32,[u32;11])) {
    const K:f64=f32::from_bits(0x38000100) as f64;
    let per_frame=(lifetime as f64/frames as f64) as f32;
    construct_particle_base(m,object,[clock,position,1f32.to_bits(),5f32.to_bits(),table,frames,per_frame.to_bits(),lifetime.to_bits(),100],root);
    for o in (0x94..0xac).step_by(4) {m.set_u32(object+o,0);}
    m.set_u32(object,tables.0);m.set_u32(object+0x14,tables.1);m.set_u32(object+0x80,m.u32(object+0x80)|1);
    for i in 0..3 {let v=m.u32(position+i*4);m.set_u32(object+0x94+i*4,v);let v=m.u32(direction+i*4);m.set_u32(object+0xa0+i*4,v);}
    m.set_f32(object+0xac,3.0);m.set_u32(object+0xb0,0);
    let first=(rand(m) as f64*K) as f32 as f64;let second=rand(m) as f64*K;
    m.set_u32(object+0xb8,0x3f3504f3);m.set_u32(object+0xbc,0x3f3504f3);
    let spin=(first-second)*f32::from_bits(0x40490fda) as f64;m.set_f32(object+0xb4,(spin+spin) as f32);
}
#[cfg(test)]
mod smoke_ctor_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_smoke_constructor(){
        for line in include_str!("../../../tools/emu/golden_smoke_ctor.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let (obj,pos,dir)=(0x2a040000u32,0x2a000100u32,0x2a000120u32);
            let p:Vec<u32>=v[2].split(',').map(|s|s.parse().unwrap()).collect();
            m.write(pos,&bytes(v[0]));m.write(dir,&bytes(v[1]));m.write(obj,&bytes(v[4]));m.set_u32(0x6f6a68,p[3]);
            let rands:Vec<i32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();let used=std::cell::Cell::new(0usize);let mut got=vec![];
            construct_smoke_particle(&mut m,obj,p[4],pos,dir,p[0],p[1],f32::from_bits(p[2]),|_|{let r=rands[used.get()];used.set(used.get()+1);r},|_,_,a|got=a.to_vec());
            let want:Vec<u32>=v[5].split(',').map(|s|s.parse().unwrap()).collect();assert_eq!(got,want,"{line}");
            assert_eq!(m.read(obj,0xc8),bytes(v[6]),"{line}");assert_eq!(used.get().to_string(),v[7]);
        }
    }
}

/// `0x59afd0` (vtable `0x650c78` slot 1): smoke particle homing/drag, drift, spin and texture corners.
/// `size_step` is the shared `0x59ac90` (size and lifecycle), supplied by the caller; `scratch`
/// is a caller-owned 0x40-byte area. Trigonometry uses `f64::sin/cos` in place of x87 `fsin/fcos`.
pub fn update_smoke_particle(m:&mut Mem,object:u32,scratch:u32,size_step:impl FnMut(&mut Mem,u32)) {
    update_homing_particle(m,object,scratch,f32::from_bits(0x3e570a3d),size_step);
}
/// `0x59c000` (vtable `0x650d80` slot 1): the fume update, identical to the smoke one except for the wind factor 0.2.
pub fn update_fume_particle(m:&mut Mem,object:u32,scratch:u32,size_step:impl FnMut(&mut Mem,u32)) {
    update_homing_particle(m,object,scratch,f32::from_bits(0x3e4ccccd),size_step);
}
/// Shared body of `0x59afd0` and `0x59c000`.
pub fn update_homing_particle(m:&mut Mem,object:u32,scratch:u32,wind_factor:f32,mut size_step:impl FnMut(&mut Mem,u32)) {
    homing_drag(m,object,scratch);
    let dt=m.f64(m.u32(object+0x58)+0xb90);
    advance_position_wide(m,object,dt,wind_factor as f64,false);
    for i in 0..3 {let v=m.u32(object+0x4c+i*4);m.set_u32(object+0x28+i*4,v);}
    size_step(m,object);
    spin_corners(m,object,dt);
}
/// Pull back towards the home point `+0x94` and brake along the velocity (shared by `0x59afd0`, `0x59c000`, `0x59b670`).
fn homing_drag(m:&mut Mem,object:u32,scratch:u32) {
    let f=|m:&Mem,o:u32|m.f32(object+o);
    let home_offset=[(f(m,0x94)as f64-f(m,0x4c)as f64)as f32,(f(m,0x98)as f64-f(m,0x50)as f64)as f32,(f(m,0x9c)as f64-f(m,0x54)as f64)as f32];
    for i in 0..3 {m.set_f32(scratch+i*4,home_offset[i as usize]);}
    if crate::math::vec_len(m,scratch)>15.0 {m.set_f32(object+0xac,20.0);}
    let s=m.f32(object+0xac);
    if s!=0.0 && !s.is_nan() {
        crate::matrix::vec_unit(m,scratch,object+0xa0);
        let dt=m.f64(m.u32(object+0x58)+0xb90) as f32 as f64;let s64=s as f64;
        let u:[f64;3]=std::array::from_fn(|i|-(m.f32(scratch+i as u32*4) as f64));
        let w=[(u[0]*s64*dt)as f32,(((u[1]*s64)as f32 as f64)*dt)as f32,(((u[2]*s64)as f32 as f64)*dt)as f32];
        for i in 0..3 {m.set_f32(scratch+0x10+i as u32*4,w[i]);}
        crate::matrix::vec_add_in_place(m,object+0xa0,scratch+0x20,scratch+0x10);
        let v:[f64;3]=std::array::from_fn(|i|m.f32(object+0xa0+i as u32*4) as f64);
        let dot=(v[2]*w[2] as f64+v[1]*w[1] as f64)+v[0]*w[0] as f64;
        if dot>0.0 {
            m.set_f32(object+0xac,0.0);
            for i in 0..3 {let z=m.u32(0x702650+i*4);m.set_u32(object+0xa0+i*4,z);}
        }
    }
}
/// Position step of the smoke/soot updates. Smoke passes the unrounded double clock, soot the
/// `f32`-narrowed one and rounds the `dt*vz` product (`round_z`) when there is no wind.
fn advance_position_wide(m:&mut Mem,object:u32,dt:f64,wind_factor:f64,round_z:bool) {
    let (px,py,pz)=(m.f32(object+0x4c) as f64,m.f32(object+0x50) as f64,m.f32(object+0x54) as f64);
    let (vx,vy,vz)=(m.f32(object+0xa0) as f64,m.f32(object+0xa4) as f64,m.f32(object+0xa8) as f64);
    let wind=m.u32(0x7025b8);
    let (nx,ny,nz)=if wind!=0 {
        let w=m.f32(wind+0x214) as f64;let k=wind_factor;
        let t0=(w*m.f32(0x702620) as f64) as f32 as f64;let t1=(w*m.f32(0x702624) as f64) as f32 as f64;let t2=w*m.f32(0x702628) as f64;
        let x=((t0*k)as f32 as f64+vx)as f32 as f64;let y=((t1*k)as f32 as f64+vy)as f32 as f64;let z=t2*k+vz;
        (((x*dt)as f32 as f64+px)as f32,((y*dt)as f32 as f64+py)as f32,(z*dt+pz)as f32)
    }else{
        ((((dt*vx)as f32 as f64)+px)as f32,(((dt*vy)as f32 as f64)+py)as f32,(if round_z {(dt*vz)as f32 as f64+pz}else{dt*vz+pz})as f32)
    };
    m.set_f32(object+0x4c,nx);m.set_f32(object+0x50,ny);m.set_f32(object+0x54,nz);
}
/// Spin angle `+0xb0` and the four texture corners (shared tail of `0x59afd0`, `0x59c000`, `0x59b670`).
fn spin_corners(m:&mut Mem,object:u32,dt:f64) {
    let f=|m:&Mem,o:u32|m.f32(object+o);
    let (life,age)=(f(m,0x64) as f64,f(m,0x68) as f64);
    let t=(life-age)/life;
    let t=if t<0.0 {0.0} else if t>1.0 {1.0} else {t};
    let angle=((f(m,0xb4) as f64*t)*t)*dt+f(m,0xb0) as f64;
    m.set_f32(object+0xb0,angle as f32);
    let stored=f(m,0xb0) as f64;
    let g=m.u32(object+0x48);let (a8,a12)=(m.u32(g+8),m.u32(g+0xc));
    let (cb,sb)=(f(m,0xb8) as f64,f(m,0xbc) as f64);
    for (k,addr) in [0x6f6a58u32,0x6f6a5c,0x6f6a60,0x6f6a54].into_iter().enumerate() {
        let base=m.f32(addr) as f64;let sum=if k==0 {base+angle}else{base+stored};
        m.set_f32(a8+k as u32*4,(sum.cos()*cb+0.5)as f32);
        m.set_f32(a12+k as u32*4,((base+stored).sin()*sb+0.5)as f32);
    }
}
/// `0x59b670` (vtable `0x650cd8` slot 1): soot particle. Without the `+0x7c` flag it only runs the
/// shared size step; otherwise homing drag, a swirl of `+0xc0` about the velocity, drift (wind factor 0.05),
/// the size step and the spin/corner tail. `scratch` is a caller-owned 0x80-byte area.
pub fn update_soot_particle(m:&mut Mem,object:u32,scratch:u32,mut size_step:impl FnMut(&mut Mem,u32)) {
    if m.u32(object+0x7c)==0 {size_step(m,object);return;}
    homing_drag(m,object,scratch);
    let dt32=m.f64(m.u32(object+0x58)+0xb90) as f32;let dt=dt32 as f64;
    let roll=(dt*m.f32(object+0xcc) as f64) as f32;
    let pos=[m.f32(0x702650),m.f32(0x702654),m.f32(0x702658)];
    let dir=[m.f32(object+0xa0),m.f32(object+0xa4),m.f32(object+0xa8)];
    let matrix=scratch+0x40;crate::matrix::mat_look(m,matrix,pos,dir,roll);
    crate::matrix::transform_in_place(m,object+0xc0,matrix);
    let swirl=(dt*m.f32(object+0xd0) as f64+1.0) as f32;
    crate::matrix::vec_scale_in_place(m,object+0xc0,scratch+0x10,swirl);
    crate::matrix::vec_add_in_place(m,object+0x4c,scratch+0x20,object+0xc0);
    advance_position_wide(m,object,dt,0.05f32 as f64,true);
    for i in 0..3 {let v=m.u32(object+0x4c+i*4);m.set_u32(object+0x28+i*4,v);}
    size_step(m,object);
    spin_corners(m,object,m.f64(m.u32(object+0x58)+0xb90));
}
#[cfg(test)]
mod smoke_update_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_smoke_update(){
        for line in include_str!("../../../tools/emu/golden_smoke_update.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();
            let (obj,clock,geom,a8,a12,wind,scratch)=(0x2a040000u32,0x2a030000u32,0x2a050000u32,0x2a051000u32,0x2a052000u32,0x2a053000u32,0x2a060000u32);
            crate::globals::install_constants(&mut m);
            m.write(0x702620,&bytes(v[3]));m.write(0x6f6a54,&bytes(v[4]));
            m.write(obj,&bytes(v[0]));m.write(clock+0xb90,&bytes(v[1]));m.write(wind+0x214,&bytes(v[2]));
            m.set_u32(0x7025b8,if v[5]=="1" {wind}else{0});m.set_u32(geom+8,a8);m.set_u32(geom+0xc,a12);
            let mut called=0;let mut at=vec![];
            update_smoke_particle(&mut m,obj,scratch,|m,o|{assert_eq!(o,obj);called+=1;at=m.read(obj+0x28,12);});
            assert_eq!(m.read(obj,0xc8),bytes(v[6]),"{line}");
            assert_eq!(m.read(a8,16),bytes(v[7]),"{line}");assert_eq!(m.read(a12,16),bytes(v[8]),"{line}");
            assert_eq!(called.to_string(),v[9]);assert_eq!(at,bytes(v[10]));
        }
    }
}

#[cfg(test)]
mod smoke_delete_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    /// `0x5a1c40` (flags = 1) is `0x5a2a30` + pool recycle, i.e. exactly `delete_scene_particle`.
    #[test]
    fn native_smoke_delete(){
        for line in include_str!("../../../tools/emu/golden_smoke_delete.tsv").lines(){
            let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let obj=0x2a000000;
            m.write(obj,&bytes(input));delete_scene_particle(&mut m,obj,|_,_|{});assert_eq!(m.read(obj,0x400),bytes(output));
        }
    }
    /// `0x5a3790` (flags = 1) is the same chain for the soot class.
    #[test]
    fn native_soot_delete(){
        for line in include_str!("../../../tools/emu/golden_soot_delete.tsv").lines(){
            let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let obj=0x2a000000;
            m.write(obj,&bytes(input));delete_scene_particle(&mut m,obj,|_,_|{});assert_eq!(m.read(obj,0x400),bytes(output));
        }
    }
}

#[cfg(test)]
mod fume_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_fume_constructor(){
        for line in include_str!("../../../tools/emu/golden_fume_ctor.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let (obj,pos,dir)=(0x2a040000u32,0x2a000100u32,0x2a000120u32);
            let p:Vec<u32>=v[2].split(',').map(|s|s.parse().unwrap()).collect();
            m.write(pos,&bytes(v[0]));m.write(dir,&bytes(v[1]));m.write(obj,&bytes(v[4]));m.set_u32(0x6f6a68,p[3]);
            let rands:Vec<i32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();let used=std::cell::Cell::new(0usize);let mut got=vec![];
            construct_fume_particle(&mut m,obj,p[4],pos,dir,p[0],p[1],f32::from_bits(p[2]),|_|{let r=rands[used.get()];used.set(used.get()+1);r},|_,_,a|got=a.to_vec());
            let want:Vec<u32>=v[5].split(',').map(|s|s.parse().unwrap()).collect();assert_eq!(got,want,"{line}");
            assert_eq!(m.read(obj,0xc8),bytes(v[6]),"{line}");assert_eq!(used.get().to_string(),v[7]);
        }
    }
    #[test]
    fn native_fume_update(){
        for line in include_str!("../../../tools/emu/golden_fume_update.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();
            let (obj,clock,geom,a8,a12,wind,scratch)=(0x2a040000u32,0x2a030000u32,0x2a050000u32,0x2a051000u32,0x2a052000u32,0x2a053000u32,0x2a060000u32);
            crate::globals::install_constants(&mut m);
            m.write(0x702620,&bytes(v[3]));m.write(0x6f6a54,&bytes(v[4]));
            m.write(obj,&bytes(v[0]));m.write(clock+0xb90,&bytes(v[1]));m.write(wind+0x214,&bytes(v[2]));
            m.set_u32(0x7025b8,if v[5]=="1" {wind}else{0});m.set_u32(geom+8,a8);m.set_u32(geom+0xc,a12);
            let mut called=0;let mut at=vec![];
            update_fume_particle(&mut m,obj,scratch,|m,o|{assert_eq!(o,obj);called+=1;at=m.read(obj+0x28,12);});
            assert_eq!(m.read(obj,0xc8),bytes(v[6]),"{line}");
            assert_eq!(m.read(a8,16),bytes(v[7]),"{line}");assert_eq!(m.read(a12,16),bytes(v[8]),"{line}");
            assert_eq!(called.to_string(),v[9]);assert_eq!(at,bytes(v[10]));
        }
    }
}

/// `0x59b3f0` including the base `0x59ab10`: soot particle (vtable `0x650cd8`, 0xd4 bytes).
/// Root `0x59a780` is supplied by the owner; `scratch` is a caller-owned 12-byte vector.
pub fn construct_soot_particle(m:&mut Mem,object:u32,clock:u32,position:u32,direction:u32,table:u32,frames:u32,size:f32,lifetime:f32,scratch:u32,
    mut rand:impl FnMut(&mut Mem)->i32,root:impl FnMut(&mut Mem,u32,[u32;11])) {
    const K:f64=f32::from_bits(0x38000100) as f64;
    let per_frame=(lifetime as f64/frames as f64) as f32;
    let tall=(size as f64*5.0) as f32;
    construct_particle_base(m,object,[clock,position,size.to_bits(),tall.to_bits(),table,frames,per_frame.to_bits(),lifetime.to_bits(),100],root);
    for o in (0x94..0xac).step_by(4) {m.set_u32(object+o,0);}
    for o in [0xc0,0xc4,0xc8] {m.set_u32(object+o,0);}
    m.set_u32(object,0x650cd8);m.set_u32(object+0x14,0x650c8c);m.set_u32(object+0x80,m.u32(object+0x80)|1);
    for i in 0..3 {let v=m.u32(position+i*4);m.set_u32(object+0x94+i*4,v);let v=m.u32(direction+i*4);m.set_u32(object+0xa0+i*4,v);}
    m.set_f32(object+0xac,3.0);m.set_u32(object+0xb0,0);
    let first=(rand(m) as f64*K) as f32 as f64;let second=rand(m) as f64*K;
    let spin=(first-second)*f32::from_bits(0x40490fda) as f64;m.set_f32(object+0xb4,(spin+spin) as f32);
    m.set_u32(object+0xb8,0x3f3504f3);m.set_u32(object+0xbc,0x3f3504f3);
    let (ax,ay,az)=(m.f32(0x702630) as f64,m.f32(0x702634) as f64,m.f32(0x702638) as f64);
    let (vx,vy,vz)=(m.f32(direction) as f64,m.f32(direction+4) as f64,m.f32(direction+8) as f64);
    m.set_f32(object+0xc0,(az*vy-ay*vz) as f32);m.set_f32(object+0xc4,(ax*vz-az*vx) as f32);m.set_f32(object+0xc8,(ay*vx-ax*vy) as f32);
    let len=crate::math::vec_len(m,object+0xc0);
    if len<0.01 {for i in 0..3 {let v=m.u32(0x702640+i*4);m.set_u32(object+0xc0+i*4,v);}}
    else{crate::matrix::vec_div_in_place(m,object+0xc0,scratch,len as f32);}
    crate::matrix::vec_scale_in_place(m,object+0xc0,scratch,f32::from_bits(0x3c23d70a));
    let third=(rand(m) as f64*K) as f32 as f64;let fourth=rand(m) as f64*K;
    m.set_f32(object+0xcc,((fourth-third)*f32::from_bits(0x412feddf) as f64) as f32);
    let fifth=rand(m) as f64;m.set_f32(object+0xd0,(fifth*f32::from_bits(0x3423d852) as f64) as f32);
}
#[cfg(test)]
mod soot_ctor_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_soot_constructor(){
        for line in include_str!("../../../tools/emu/golden_soot_ctor.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let axes=bytes(v[0]);let mut m=Mem::new();let (obj,pos,dir,scratch)=(0x2a040000u32,0x2a000100u32,0x2a000120u32,0x2a050000u32);
            let p:Vec<u32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();
            m.write(0x702630,&axes[..12]);m.write(0x702640,&axes[12..]);m.write(pos,&bytes(v[1]));m.write(dir,&bytes(v[2]));m.write(obj,&bytes(v[5]));m.set_u32(0x6f6a68,p[4]);
            let rands:Vec<i32>=v[4].split(',').map(|s|s.parse().unwrap()).collect();let used=std::cell::Cell::new(0usize);let mut got=vec![];
            construct_soot_particle(&mut m,obj,p[5],pos,dir,p[0],p[1],f32::from_bits(p[2]),f32::from_bits(p[3]),scratch,|_|{let r=rands[used.get()];used.set(used.get()+1);r},|_,_,a|got=a.to_vec());
            let want:Vec<u32>=v[6].split(',').map(|s|s.parse().unwrap()).collect();assert_eq!(got,want,"{line}");
            assert_eq!(m.read(obj,0xd4),bytes(v[7]),"{line}");assert_eq!(used.get().to_string(),v[8]);
        }
    }
}

#[cfg(test)]
mod soot_update_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_soot_update(){
        for line in include_str!("../../../tools/emu/golden_soot_update.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();
            let (obj,clock,geom,a8,a12,wind,scratch)=(0x2a040000u32,0x2a030000u32,0x2a050000u32,0x2a051000u32,0x2a052000u32,0x2a053000u32,0x2a060000u32);
            crate::globals::install_constants(&mut m);
            m.write(0x702620,&bytes(v[3]));m.write(0x6f6a54,&bytes(v[4]));
            m.write(obj,&bytes(v[0]));m.write(clock+0xb90,&bytes(v[1]));m.write(wind+0x214,&bytes(v[2]));
            m.set_u32(0x7025b8,if v[5]=="1" {wind}else{0});m.set_u32(geom+8,a8);m.set_u32(geom+0xc,a12);
            let mut called=0;let mut at=vec![];
            update_soot_particle(&mut m,obj,scratch,|m,o|{assert_eq!(o,obj);called+=1;at=m.read(obj+0x28,12);});
            {let (g,w)=(m.read(obj,0xd4),bytes(v[6]));let d:Vec<_>=(0..g.len()).filter(|&i|g[i]!=w[i]).collect();assert!(d.is_empty(),"diff offsets {d:x?} wind={} dt={}",v[5],v[1]);}
            assert_eq!(m.read(a8,16),bytes(v[7]),"{line}");assert_eq!(m.read(a12,16),bytes(v[8]),"{line}");
            assert_eq!(called.to_string(),v[9]);assert_eq!(at,bytes(v[10]));
        }
    }
}

/// Constructor call prepared by `0x4baf90` (the flash particle is built by the owner).
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct FlashRequest {pub object:u32,pub clock:u32,pub position:u32,pub size:f32,pub table:u32,pub frames:u32}

/// `0x4baf90`: flash particle for `[0x696054] > 0.5`; `alloc(0xb4, [0x6f6a98])` is `0x579380`.
pub fn emit_flash(m:&mut Mem,position:u32,size:f32,alloc:impl FnOnce(&mut Mem,u32,u32)->u32,construct:impl FnOnce(&mut Mem,FlashRequest)) {
    if !(m.f32(0x696054)>0.5) {return;}
    let pool=m.u32(0x6f6a98);let object=alloc(m,0xb4,pool);
    if object!=0 {construct(m,FlashRequest{object,clock:m.u32(0x696224),position,size,table:0x695c58,frames:m.u32(0x69620c)});}
}
/// `0x59d4a0` including the base `0x59ab10`: flash particle (vtable `0x650ed8`). Root `0x59a780` is the owner's.
pub fn construct_flash_particle(m:&mut Mem,object:u32,clock:u32,position:u32,size:f32,table:u32,frames:u32,root:impl FnMut(&mut Mem,u32,[u32;11])) {
    let per_frame=(f32::from_bits(0x3fa3d70a) as f64/frames as f64) as f32;
    let tall=(size as f64*4.0) as f32;
    construct_particle_base(m,object,[clock,position,0,tall.to_bits(),table,frames,per_frame.to_bits(),0x3fa3d70a,100],root);
    for o in (0x94..0xac).step_by(4) {m.set_u32(object+o,0);}
    m.set_u32(object,0x650ed8);m.set_u32(object+0x14,0x650e8c);m.set_u32(object+0x80,m.u32(object+0x80)|1);
    for i in 0..3 {let v=m.u32(position+i*4);m.set_u32(object+0x94+i*4,v);let z=m.u32(0x702650+i*4);m.set_u32(object+0xa0+i*4,z);}
    m.set_u32(object+0xac,0);m.set_u32(object+0xb0,0);
}
/// `0x59d590` (vtable `0x650ed8` slot 1): flash drift with a random sideways kick after frame 10 and a quadratic size fade.
/// `lifecycle` is the shared `0x59aa10`.
pub fn update_flash_particle(m:&mut Mem,object:u32,mut rand:impl FnMut(&mut Mem)->i32,mut lifecycle:impl FnMut(&mut Mem,u32)) {
    const K:f64=f32::from_bits(0x38000100) as f64;
    let frame=m.i32(object+0x6c);
    if frame>=14 {m.set_u32(object+0xa8,0x3f800000);}
    if frame>10 && m.u32(object+0xb0)==0 {
        let a=(rand(m) as f64*K) as f32 as f64;let b=rand(m) as f64*K;
        m.set_f32(object+0xa0,((b-a)*0.25) as f32);
        let c=(rand(m) as f64*K) as f32 as f64;let d=rand(m) as f64*K;
        m.set_u32(object+0xb0,1);
        m.set_f32(object+0xa4,((d-c)*0.25) as f32);
    }
    let dt=m.f64(m.u32(object+0x58)+0xb90);
    let (vx,vy,vz)=(m.f32(object+0xa0) as f64,m.f32(object+0xa4) as f64,m.f32(object+0xa8) as f64);
    let nx=(((dt*vx) as f32 as f64)+m.f32(object+0x4c) as f64) as f32;
    let ny=(((dt*vy) as f32 as f64)+m.f32(object+0x50) as f64) as f32;
    let nz=(dt*vz+m.f32(object+0x54) as f64) as f32;
    m.set_f32(object+0x28,nx);m.set_f32(object+0x4c,nx);m.set_f32(object+0x2c,ny);m.set_f32(object+0x50,ny);m.set_f32(object+0x30,nz);m.set_f32(object+0x54,nz);
    let (life,age)=(m.f32(object+0x64) as f64,m.f32(object+0x68) as f64);
    let t=(life-age)/life;let t=if t<0.0 {0.0} else if t>1.0 {1.0} else {t};
    let t2=t*t;
    let s=(1.0-t2)*m.f32(object+0x88) as f64+m.f32(object+0x84) as f64*t2;
    let big=(s*-0.5) as f32;let half=(s*0.5) as f32;
    let g=m.u32(object+0x48);let (a10,a14)=(m.u32(g+0x10),m.u32(g+0x14));
    for (i,v) in [big,half,half,big].into_iter().enumerate() {m.set_f32(a10+i as u32*4,v);}
    for (i,v) in [big,big,half,half].into_iter().enumerate() {m.set_f32(a14+i as u32*4,v);}
    lifecycle(m,object);
}
#[cfg(test)]
mod flash_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_flash_emit(){
        for line in include_str!("../../../tools/emu/golden_flash_emit.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let pos=0x2a000100u32;
            m.write(0x696054,&bytes(v[0]));m.write(pos,&bytes(v[1]));
            let size=f32::from_bits(v[2].parse().unwrap());let g:Vec<u32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();
            for (a,x) in [0x696224u32,0x69620c,0x6f6a98].into_iter().zip(&g){m.set_u32(a,*x);}
            let ret:u32=v[4].parse().unwrap();let mut allocs=vec![];let mut reqs=vec![];
            emit_flash(&mut m,pos,size,|_,s,p|{allocs.push(format!("{s}:{p}"));ret},|_,r|reqs.push(r));
            assert_eq!(if allocs.is_empty() {"-".to_string()}else{allocs.join(";")},v[5],"{line}");
            if v[6]=="-" {assert!(reqs.is_empty());}else{
                let f:Vec<&str>=v[6].split(':').collect();let a:Vec<u32>=f[1].split(',').map(|s|s.parse().unwrap()).collect();
                assert_eq!(reqs.len(),1);let r=reqs[0];
                assert_eq!((r.object.to_string(),r.clock,r.size.to_bits(),r.table,r.frames),(f[0].to_string(),a[0],a[2],a[3],a[4]),"{line}");
                assert_eq!(m.read(pos,12),bytes(f[2]));
            }
        }
    }
    #[test]
    fn native_flash_constructor(){
        for line in include_str!("../../../tools/emu/golden_flash_ctor.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let (obj,pos)=(0x2a040000u32,0x2a000100u32);
            let p:Vec<u32>=v[1].split(',').map(|s|s.parse().unwrap()).collect();
            m.write(0x702650,&[0;12]);m.write(pos,&bytes(v[0]));m.write(obj,&bytes(v[2]));m.set_u32(0x6f6a68,p[3]);
            let mut got=vec![];
            construct_flash_particle(&mut m,obj,p[4],pos,f32::from_bits(p[0]),p[1],p[2],|_,_,a|got=a.to_vec());
            let want:Vec<u32>=v[3].split(',').map(|s|s.parse().unwrap()).collect();assert_eq!(got,want,"{line}");
            assert_eq!(m.read(obj,0xb4),bytes(v[4]),"{line}");
        }
    }
    #[test]
    fn native_flash_update(){
        for line in include_str!("../../../tools/emu/golden_flash_update.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let (obj,clock,geom,a10,a14)=(0x2a040000u32,0x2a030000u32,0x2a050000u32,0x2a051000u32,0x2a052000u32);
            m.write(obj,&bytes(v[0]));m.write(clock+0xb90,&bytes(v[1]));m.set_u32(geom+0x10,a10);m.set_u32(geom+0x14,a14);
            let rands:Vec<i32>=v[2].split(',').map(|s|s.parse().unwrap()).collect();let used=std::cell::Cell::new(0usize);let mut called=0;let mut at=vec![];
            update_flash_particle(&mut m,obj,|_|{let r=rands[used.get()];used.set(used.get()+1);r},|m,o|{assert_eq!(o,obj);called+=1;at=m.read(obj+0x28,12);});
            assert_eq!(m.read(obj,0xb4),bytes(v[3]),"{line}");assert_eq!(m.read(a10,16),bytes(v[4]),"{line}");assert_eq!(m.read(a14,16),bytes(v[5]),"{line}");
            assert_eq!(used.get().to_string(),v[6]);assert_eq!(called.to_string(),v[7]);assert_eq!(at,bytes(v[8]));
        }
    }
    /// `0x5a2a50` (flags = 1) is again `0x5a2a30` + pool recycle.
    #[test]
    fn native_flash_delete(){
        for line in include_str!("../../../tools/emu/golden_flash_delete.tsv").lines(){
            let (input,output)=line.split_once('\t').unwrap();let mut m=Mem::new();let obj=0x2a000000;
            m.write(obj,&bytes(input));delete_scene_particle(&mut m,obj,|_,_|{});assert_eq!(m.read(obj,0x400),bytes(output));
        }
    }
}

/// The four material tables built by `aeffects.cpp` loaders.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum EffectMaterials {
    /// `0x4b8c40`: `msmokeNN` from 0 into `0x695bf0`, count `0x696208`.
    Smoke,
    /// `0x4b8fb0`: `mvaporNN` from 0 into `0x695c58`, count `0x69620c`.
    Fume,
    /// `0x4b8ad0`: `burnNN` from 1 into `0x694998`, count `0x696210`.
    Soot,
    /// `0x4b8f00`: `dustNN` from 0 into `0x694a00`, count `0x696218`.
    Dust,
}
impl EffectMaterials {
    /// (table address, count address, name prefix, first index, two-digit formatting)
    fn spec(self)->(u32,u32,&'static str,u32,bool) {
        match self {
            Self::Smoke=>(0x695bf0,0x696208,"msmoke",0,false),
            Self::Fume=>(0x695c58,0x69620c,"mvapor",0,false),
            Self::Soot=>(0x694998,0x696210,"burn",1,true),
            Self::Dust=>(0x694a00,0x696218,"dust",0,true),
        }
    }
    /// Loaders that report an error (`0x60aad0`) when not even the first material exists.
    pub fn requires_first(self)->bool {self!=Self::Soot}
    pub fn table(self)->u32 {self.spec().0}
    pub fn count_address(self)->u32 {self.spec().1}
}
/// Enumerate the materials of one table until the first missing one; each found material gets
/// opacity `+0x58 = 1.0` and flags `+0x20 |= 0x40220`. Returns the count.
pub fn load_effect_materials(m:&mut Mem,which:EffectMaterials,mut lookup:impl FnMut(&mut Mem,&str)->u32)->Result<u32,&'static str> {
    let (table,count_at,prefix,first,two_digits)=which.spec();
    m.set_u32(count_at,0);let mut count=0;let mut index=first;
    loop {
        let name=if two_digits || index>9 {format!("{prefix}{index:02}")}else{format!("{prefix}0{index}")};
        let material=lookup(m,&name);m.set_u32(table+(index-first)*4,material);
        if material==0 {break;}
        count+=1;m.set_u32(count_at,count);m.set_f32(material+0x58,1.0);m.set_u32(material+0x20,m.u32(material+0x20)|0x40220);
        index+=1;
    }
    if count==0 && which.requires_first() {Err("missing effect materials")}else{Ok(count)}
}
#[cfg(test)]
mod effect_material_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_effect_materials(){
        for line in include_str!("../../../tools/emu/golden_effect_materials.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();
            let which=match v[0]{"smoke"=>EffectMaterials::Smoke,"fume"=>EffectMaterials::Fume,"soot"=>EffectMaterials::Soot,_=>EffectMaterials::Dust};
            let avail:usize=v[1].parse().unwrap();let mats=0x2a070000u32;
            let pre:Vec<Vec<u8>>=if v[6].is_empty() {vec![]}else{v[6].split(',').map(bytes).collect()};
            for (i,p) in pre.iter().enumerate() {m.write(mats+0x100*i as u32,p);}
            let mut names=vec![];
            let result=load_effect_materials(&mut m,which,|_,name|{names.push(name.to_owned());if names.len()>avail {0}else{mats+0x100*(names.len() as u32-1)}});
            assert_eq!(names.join(","),v[4],"{line}");
            assert_eq!(m.u32(which.count_address()).to_string(),v[3]);
            assert_eq!(result.is_err(),v[5]=="1","{line}");
            let tbl=bytes(v[2]);for i in 0..names.len() {assert_eq!(m.read(which.table()+i as u32*4,4),tbl[i*4..i*4+4],"{line}");}
            let post:Vec<Vec<u8>>=if v[7].is_empty() {vec![]}else{v[7].split(',').map(bytes).collect()};
            for (i,p) in post.iter().enumerate() {assert_eq!(&m.read(mats+0x100*i as u32,0x60),p,"{line}");}
        }
    }
}

/// A flash request of `0x551970`: world point and the size dword passed to `0x4baf90`.
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct SceneFlash {pub point:[f32;3],pub size:u32}

/// Tail of `0x551970` (`0x55278b..0x552930`): two flash points from the vehicle record at
/// `0x6978cc + 0x970 * [scene+0x28b4]`; the sink `0x4baf90` is not performed. `basis` is the result
/// of `scene_geometry_basis`, `scratch` holds 0x30 bytes of temporaries (the original's stack).
pub fn scene_flash_requests(m:&mut Mem,scene:u32,basis:u32,scratch:u32)->Vec<SceneFlash> {
    let flags=m.u32(scene+0x27b4);
    if flags&0x18000==0 {return Vec::new();}
    let size:u32=if flags&0x10000!=0 {0x40000000}else{0x3f000000};
    let tech=0x6978ccu32.wrapping_add(m.u32(scene+0x28b4).wrapping_mul(0x970));
    let s=m.f32(tech);
    let (a,b,p1,d,e,p2)=(scratch,scratch+12,scratch+24,scratch+36,scratch+48,scratch+60);
    crate::matrix::vec_scale(m,a,0x702630,s);
    crate::matrix::vec_sub(m,b,tech+0x144,a);
    crate::matrix::transform(m,p1,b,basis);
    crate::matrix::vec_scale(m,d,0x702630,s);
    crate::matrix::vec_sub(m,e,tech+0x150,d);
    crate::matrix::transform(m,p2,e,basis);
    [p1,p2].into_iter().map(|p|SceneFlash{point:std::array::from_fn(|i|m.f32(p+i as u32*4)),size}).collect()
}
#[cfg(test)]
mod scene_flash_tail_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_scene_flash_tail(){
        for line in include_str!("../../../tools/emu/golden_scene_flash_tail.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let scene=0x2a000000u32;let basis=scene+0x10000;
            let data=bytes(v[0]);let vid:u32=v[3].parse().unwrap();
            m.write(scene+0x2c4,&data[..48]);m.write(scene+0x204,&data[48..]);
            m.write(0x6978cc+vid*0x970,&bytes(v[1]));m.write(0x702630,&bytes(v[2]));
            m.set_u32(scene+0x27b4,v[4].parse().unwrap());m.set_u32(scene+0x28b4,vid);
            scene_geometry_basis(&mut m,scene,basis);
            let got=scene_flash_requests(&mut m,scene,basis,scene+0x20000);
            let mut raw=Vec::new();for f in &got {for x in f.point {raw.extend(x.to_le_bytes());}raw.extend(f.size.to_le_bytes());}
            let want=if v[5]=="-" {Vec::new()}else{bytes(v[5])};
            assert_eq!(raw,want,"{line}");
        }
    }
}

/// `0x549ff0` (`[[this+0x14]+0xc]` is the mode, `x=[[this+0x14]+8]`): returns `(a, b, st0)`.
/// Modes 1 and 2: `st0` is `a` reloaded as `f32`. Mode 3: `a=b=1`, `st0=exp(x*-1/3)`; mode 4: `a=1, b=0`,
/// `st0=exp(-x*x)`; other modes: zeros. REPLACEMENT: the original's exponential is x87 `f2xm1/fscale`;
/// `f64::exp` differs from it by up to 4e-14 relative (tested at 1e-12), `a` and `b` are exact.
pub fn exhaust_parameters(x:f32,mode:u32)->Option<(f32,f32,f64)> {
    match mode {
        3=>return Some((1.0,1.0,(x as f64*(f32::from_bits(0xbeaaaaab) as f64)).exp())),
        4=>return Some((1.0,0.0,(-(x as f64*x as f64)).exp())),
        1|2=>{}
        _=>return Some((0.0,0.0,0.0)),
    }
    let k=if mode==1 {0.6f64}else{0.8};
    let x=x as f64;
    let a=(k*x/(x+5.0f32 as f64)+0.2) as f32;
    let b=((0.4*x)/(x+20.0f32 as f64)+0.6) as f32;
    Some((a,b,a as f64))
}
#[cfg(test)]
mod exhaust_parameter_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_exhaust_parameters(){
        for line in include_str!("../../../tools/emu/golden_exhaust_params.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();
            let x=f32::from_le_bytes(bytes(v[1]).try_into().unwrap());
            let (a,b,r)=exhaust_parameters(x,v[0].parse().unwrap()).unwrap();
            assert_eq!(a.to_le_bytes().to_vec(),bytes(v[2]),"a {line}");
            assert_eq!(b.to_le_bytes().to_vec(),bytes(v[3]),"b {line}");
            assert_eq!(r.to_le_bytes().to_vec(),bytes(v[4]),"ret {line}");
        }
    }
    #[test]
    fn native_exhaust_parameters_exp_modes(){
        for line in include_str!("../../../tools/emu/golden_exhaust_params_exp.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();
            let x=f32::from_le_bytes(bytes(v[1]).try_into().unwrap());
            let (a,b,r)=exhaust_parameters(x,v[0].parse().unwrap()).unwrap();
            assert_eq!(a.to_le_bytes().to_vec(),bytes(v[2]),"a {line}");
            assert_eq!(b.to_le_bytes().to_vec(),bytes(v[3]),"b {line}");
            let want=f64::from_le_bytes(bytes(v[4]).try_into().unwrap());
            assert!((r-want).abs()<=want.abs()*1e-12,"ret {line}: {r} vs {want}");
        }
    }
}

/// One call of `0x4ba660` made by the exhaust branch of `0x551970`.
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct ExhaustEmit {pub position:[f32;3],pub velocity:[f32;3],pub p:f32}

fn ftol(v:f64)->i32 {if v.is_nan()||v.abs()>=2147483648.0 {i32::MIN}else{v as i32}}

/// Exhaust branch of `0x551970` (`0x551ff4..0x55278b`); positions go through the matrix at `scene+0x2c4`
/// (`ebx` of the original) and then `scene+0x204`. `scratch` holds 24 bytes of temporaries. `rand` is the C `rand()`, `high` is the `0x549e60(1)` call
/// made in modes 1 and 2, `emit` is `0x4ba660(position, velocity, p, 1)`; all three run in the original
/// order, so a caller that emits particles from `emit` consumes `rand` exactly as the original does.
/// The x87 intermediates are computed in `f64` (REPLACEMENT: extended precision is not reproduced).
pub fn scene_exhaust_branch(m:&mut Mem,scene:u32,scratch:u32,mut rand:impl FnMut(&mut Mem)->i32,
    mut high:impl FnMut(&mut Mem),mut emit:impl FnMut(&mut Mem,ExhaustEmit)) {
    const K:f64=f32::from_bits(0x38000100) as f64;
    let cfg=m.u32(scene+0x5454);let mode=m.u32(cfg+0xc);
    if !(1..=4).contains(&mode) {return;}
    let vid=m.u32(scene+0x28b4);let rec=vid.wrapping_mul(0x970);
    let t=|m:&Mem,abs:u32|m.f32(rec.wrapping_add(abs));
    let (a,b,st0)=exhaust_parameters(m.f32(cfg+8),mode).unwrap();
    let count=ftol(st0*t(m,0x697db8) as f64*3.0);
    if count<=0 {return;}
    let low=!((t(m,0x697fdc) as f64-t(m,0x6979f8) as f64)>0.0);
    let mut scale=a as f64*t(m,0x697db8) as f64;if low {scale*=6.0;}
    let scale=scale as f32 as f64;
    let p=(b as f64*0.6+0.4) as f32;
    if (mode==1||mode==2) && p as f64>0.7 {high(m);}
    let maxv=[t(m,0x697dc4),t(m,0x697dc8),t(m,0x697dcc)];let minv=[t(m,0x697dd0),t(m,0x697dd4),t(m,0x697dd8)];
    let s2=(t(m,0x697dc0) as f64+1.0) as f32 as f64;
    let ax=|m:&Mem,base:u32,i:u32|m.f32(base+i*4) as f64;
    let (b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11):(f64,f64,f64,f64,f64,f64,f64,f64,f64,f64,f64,f64)=
        (ax(m,scene+0x2c4,0),ax(m,scene+0x2c4,1),ax(m,scene+0x2c4,2),ax(m,scene+0x2c4,3),ax(m,scene+0x2c4,4),ax(m,scene+0x2c4,5),ax(m,scene+0x2c4,6),ax(m,scene+0x2c4,7),ax(m,scene+0x2c4,8),ax(m,scene+0x2c4,9),ax(m,scene+0x2c4,10),ax(m,scene+0x2c4,11));
    for _ in 0..count {
        let r1=((rand(m) as f64*K-0.5)*scale) as f32 as f64;
        let r2=((rand(m) as f64*K-0.5)*scale) as f32 as f64;
        let r3=(rand(m) as f64*K-0.5)*0.3;
        let (a0,a1,a2)=(ax(m,0x702620,0),ax(m,0x702620,1),ax(m,0x702620,2));
        let (c0,c1,c2)=(ax(m,0x702630,0),ax(m,0x702630,1),ax(m,0x702630,2));
        let (d0,d1,d2)=(ax(m,0x702640,0),ax(m,0x702640,1),ax(m,0x702640,2));
        let (t0,t1,t2)=((a0*r2) as f32 as f64,(a1*r2) as f32 as f64,(a2*r2) as f32 as f64);
        let (u0,u1,u2)=((d0*r1) as f32 as f64,(d1*r1) as f32 as f64,(d2*r1) as f32 as f64);
        let q0=(c0*r3) as f32 as f64;let q1=(c1*r3) as f32 as f64;let q2=r3*c2;
        let e0=(q0+t(m,0x697fd8) as f64) as f32 as f64;let e1=(q1+t(m,0x697fdc) as f64) as f32 as f64;let e2=q2+t(m,0x697fe0) as f64;
        let g0=(e0+u0) as f32 as f64;let g1=(u1+e1) as f32 as f64;let g2=e2+u2;
        let mut pos=[(g0+t0) as f32,(t1+g1) as f32,(t2+g2) as f32];
        let mut vel=[(s2*c0+m.f32(scene+0x1b0) as f64) as f32,((c1*s2) as f32 as f64+m.f32(scene+0x1b4) as f64) as f32,((c2*s2) as f32 as f64+m.f32(scene+0x1b8) as f64) as f32];
        let above=(t(m,0x6979f8) as f64-pos[1] as f64) as f64;let d=above as f32;
        if (above<1.0||above.is_nan()) && (d as f64)>-1.0 {
            let w=[(a0+a0) as f32,(a1+a1) as f32,(a2+a2) as f32];
            for i in 0..3 {pos[i]=w[i]+pos[i];}
            let sc=|m:&Mem,o:u32|m.f32(scene+o) as f64;
            let x=(a1*sc(m,0x210)+a2*sc(m,0x21c))+a0*sc(m,0x204);
            let y=((a1*sc(m,0x214)+a2*sc(m,0x220))+a0*sc(m,0x208)) as f32 as f64;
            let z=((a1*sc(m,0x218)+a2*sc(m,0x224))+a0*sc(m,0x20c)) as f32 as f64;
            let dd=(d as f64+1.0) as f32 as f64;
            let v=[(x*dd) as f32,(y*dd) as f32,(z*dd) as f32];
            for i in 0..3 {vel[i]=v[i]+vel[i];}
        }
        let gt=|a:f32,b:f32|a>b;let lt=|a:f32,b:f32|a<b||a.is_nan()||b.is_nan();
        if !low {
            if gt(pos[0],maxv[0]) {pos[0]=maxv[0];}
            if lt(pos[0],minv[0]) {pos[0]=minv[0];}
            if gt(pos[1],maxv[1]) {pos[1]=maxv[1];}
            if lt(pos[1],minv[1]) {pos[1]=minv[1];}
        }else{
            let top=maxv[1] as f64+0.5;
            if (pos[1] as f64)>top {
                pos[1]=top as f32;
                pos[0]=((rand(m) as f64*K-0.5)*maxv[0] as f64) as f32;
            }else if lt(pos[1],minv[1]) {
                pos[1]=minv[1];
                pos[0]=((rand(m) as f64*K-0.5)*maxv[0] as f64) as f32;
            }else{
                if gt(pos[0],0.0) {pos[0]=(maxv[0] as f64+0.1) as f32;}
                if lt(pos[0],0.0) {pos[0]=(minv[0] as f64-0.1) as f32;}
            }
        }
        if gt(pos[2],maxv[2]) {pos[2]=maxv[2];}
        if lt(pos[2],minv[2]) {pos[2]=minv[2];}
        let (x,y,z)=(pos[0] as f64,pos[1] as f64,pos[2] as f64);
        let l=[((z*b6+y*b3)+x*b0)+b9,((y*b4+z*b7)+x*b1)+b10,((y*b5+z*b8)+x*b2)+b11];
        let (src,dst)=(scratch,scratch+12);
        for i in 0..3 {m.set_f32(src+i as u32*4,l[i] as f32);}
        crate::matrix::transform(m,dst,src,scene+0x204);
        let position=std::array::from_fn(|i|m.f32(dst+i as u32*4));
        emit(m,ExhaustEmit{position,velocity:vel,p});
    }
}
#[cfg(test)]
mod scene_exhaust_branch_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    #[test]
    fn native_scene_exhaust_branch(){
        let mut bad=0;
        for line in include_str!("../../../tools/emu/golden_scene_exhaust_branch.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let scene=0x2a000000u32;let basis=scene+0x10000;let cfg2=scene+0x30000;
            let data=bytes(v[0]);let xm=bytes(v[4]);let vid=u32::from_le_bytes(xm[8..12].try_into().unwrap());
            m.write(scene+0x2c4,&data[..48]);m.write(scene+0x204,&data[48..]);m.write(scene+0x1b0,&bytes(v[3]));
            m.write(vid*0x970+0x6979f8,&bytes(v[1]));let ax=bytes(v[2]);m.write(0x702620,&ax[..12]);m.write(0x702630,&ax[12..24]);m.write(0x702640,&ax[24..]);
            m.set_u32(scene+0x28b4,vid);m.set_u32(scene+0x5454,cfg2);m.write(cfg2+8,&xm[..4]);m.set_u32(cfg2+0xc,u32::from_le_bytes(xm[4..8].try_into().unwrap()));
            scene_geometry_basis(&mut m,scene,basis);
            let rands:Vec<i32>=v[5].split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect();let used=std::cell::Cell::new(0usize);
            let mut highs=0;let mut got=Vec::new();
            scene_exhaust_branch(&mut m,scene,scene+0x20000,|_|{let r=rands[used.get()];used.set(used.get()+1);r},|_|highs+=1,|_,e|got.push(e));
            let want_high=if v[6]=="-" {0}else{v[6].split(',').count()};
            assert_eq!(highs,want_high,"{line}");assert_eq!(used.get(),rands.len(),"rand count {line}");
            let want:Vec<Vec<u8>>=if v[7]=="-" {vec![]}else{v[7].split(';').map(bytes).collect()};
            assert_eq!(got.len(),want.len(),"{line}");
            for (g,w) in got.iter().zip(want){
                let mut raw=Vec::new();for x in g.position.iter().chain(g.velocity.iter()) {raw.extend(x.to_le_bytes());}raw.extend(g.p.to_le_bytes());
                if raw!=w {bad+=1;if bad<4 {let f=|b:&[u8]|b.chunks(4).map(|c|f32::from_le_bytes(c.try_into().unwrap())).collect::<Vec<_>>();eprintln!("vid {vid} got {:?} want {:?}",f(&raw),f(&w));}}
            }
        }
        assert_eq!(bad,0);
    }
}

/// Scheduling gate of `0x551970` (`0x551970..0x5519f5`): whether the effect update runs this frame.
/// Needs `[scene+0x51a4] & 0x100`; when `[[0x6cec6c]+0x514] > 1` it runs every `ftol(1/|dt|)`-th frame
/// (`dt = [[0x6ef1fc]+0x1f18]`, counter `[[0x6ef1fc]+0xbb0]`), otherwise every frame.
pub fn scene_effect_scheduled(m:&Mem,scene:u32)->bool {
    if m.u32(scene+0x51a4)&0x100==0 {return false;}
    let cfg=m.u32(0x6cec6c);
    if (m.u32(cfg+0x514) as i32)<=1 {return true;}
    let clock=m.u32(0x6ef1fc);let dt=m.f64(clock+0x1f18);
    if dt==0.0||dt.is_nan() {return false;}
    let every=ftol(1.0/dt.abs()).max(1);
    (m.u32(clock+0xbb0) as i32)%every==0
}
#[cfg(test)]
mod effect_schedule_tests {
    use super::*;
    #[test]
    fn native_effect_schedule(){
        for line in include_str!("../../../tools/emu/golden_effect_schedule.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let scene=0x2a000000u32;let cfg=0x2a010000u32;let clock=0x2a020000u32;
            m.set_u32(0x6cec6c,cfg);m.set_u32(0x6ef1fc,clock);
            m.set_u32(scene+0x51a4,v[0].parse().unwrap());m.set_u32(cfg+0x514,v[1].parse::<i32>().unwrap() as u32);m.set_u32(cfg+0x524,v[2].parse().unwrap());
            let dt:Vec<u8>=(0..16).step_by(2).map(|i|u8::from_str_radix(&v[3][i..i+2],16).unwrap()).collect();m.write(clock+0x1f18,&dt);
            m.set_u32(clock+0xbb0,v[4].parse::<i64>().unwrap() as u32);
            assert_eq!(scene_effect_scheduled(&m,scene),v[5]=="1","{line}");
        }
    }
}

/// The calls `0x551970` makes outward, in the original's order of occurrence.
pub trait SceneEffectEvents {
    /// C `rand()` (`0x61edde`).
    fn rand(&mut self,m:&mut Mem)->i32;
    /// `0x4bac90(point, direction, 0, 1)`: the two scene effects of the first branch.
    fn scene_effect(&mut self,m:&mut Mem,point:[f32;3],direction:[f32;3]);
    /// `0x4bb050(point, direction, intensity, 1)`.
    fn wheel(&mut self,m:&mut Mem,effect:WheelEffect);
    /// `0x549e60(1)` of the exhaust branch.
    fn exhaust_high(&mut self,m:&mut Mem);
    /// `0x4ba660(position, velocity, p, 1)`.
    fn exhaust(&mut self,m:&mut Mem,effect:ExhaustEmit);
    /// `0x4baf90(point, size, 1)`.
    fn flash(&mut self,m:&mut Mem,flash:SceneFlash);
}

/// Whole of `0x551970` except the room query `0x4e5ba7`, whose result the function does not use:
/// schedule gate, scene effects, wheel dust, exhaust, flashes. `scratch` needs 0x100 bytes.
pub fn update_scene_effects(m:&mut Mem,scene:u32,scratch:u32,ev:&mut impl SceneEffectEvents) {
    if !scene_effect_scheduled(m,scene) {return;}
    let basis=scratch;scene_geometry_basis(m,scene,basis);
    for (point,direction) in scene_effect_requests(m,scene,basis,scratch+0x40) {ev.scene_effect(m,point,direction);}
    for w in scene_wheel_requests(m,scene,basis) {ev.wheel(m,w);}
    // the three closures below borrow `ev` one at a time through a cell
    let cell=std::cell::RefCell::new(ev);
    scene_exhaust_branch(m,scene,scratch+0x80,|m|cell.borrow_mut().rand(m),|m|cell.borrow_mut().exhaust_high(m),|m,e|cell.borrow_mut().exhaust(m,e));
    for f in scene_flash_requests(m,scene,basis,scratch+0xa0) {cell.borrow_mut().flash(m,f);}
}
#[cfg(test)]
mod scene_effects_full_tests {
    use super::*;
    fn bytes(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
    fn hex(b:&[u8])->String{b.iter().map(|x|format!("{x:02x}")).collect()}
    struct Log{rands:Vec<i32>,used:usize,events:Vec<String>}
    fn floats(a:&[f32])->Vec<u8>{a.iter().flat_map(|x|x.to_le_bytes()).collect()}
    impl SceneEffectEvents for Log {
        fn rand(&mut self,_:&mut Mem)->i32{let r=self.rands[self.used];self.used+=1;r}
        fn scene_effect(&mut self,_:&mut Mem,p:[f32;3],d:[f32;3]){self.events.push(format!("E:{}",hex(&[floats(&p),floats(&d)].concat())));}
        fn wheel(&mut self,_:&mut Mem,w:WheelEffect){
            let p=match w.point {WheelEffectPoint::Linked(a)=>a.to_le_bytes().to_vec(),WheelEffectPoint::World(p)=>floats(&p)};
            self.events.push(format!("W:{}",hex(&[p,floats(&w.direction),w.intensity.to_le_bytes().to_vec()].concat())));
        }
        fn exhaust_high(&mut self,_:&mut Mem){self.events.push("H".into());}
        fn exhaust(&mut self,_:&mut Mem,e:ExhaustEmit){self.events.push(format!("X:{}",hex(&[floats(&e.position),floats(&e.velocity),e.p.to_le_bytes().to_vec()].concat())));}
        fn flash(&mut self,_:&mut Mem,f:SceneFlash){self.events.push(format!("F:{}",hex(&[floats(&f.point),f.size.to_le_bytes().to_vec()].concat())));}
    }
    #[test]
    fn native_scene_effects_full(){
        for line in include_str!("../../../tools/emu/golden_scene_effects_full.tsv").lines(){
            let v:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let scene=0x2a000000u32;let (cfg,clk,cfg2)=(0x2a100000u32,0x2a110000u32,0x2a120000u32);
            let p:Vec<u32>=v[7].split(',').map(|s|s.parse().unwrap()).collect();let vid=p[0];
            m.set_u32(0x6cec6c,cfg);m.set_u32(0x6ef1fc,clk);m.set_u32(cfg+0x514,0);
            let data=bytes(v[0]);m.write(scene+0x2c4,&data[..48]);m.write(scene+0x204,&data[48..]);m.write(scene+0x1b0,&bytes(v[3]));
            m.write(vid*0x970+0x6978cc,&bytes(v[1]));let ax=bytes(v[2]);m.write(0x702620,&ax[..12]);m.write(0x702630,&ax[12..24]);m.write(0x702640,&ax[24..]);
            m.write(scene+0x27f0,&bytes(v[4]));m.write(scene+0x282c,&bytes(v[5]));m.write(scene+0x4fb0,&bytes(v[6]));
            m.set_u32(scene+0x51a4,p[1]);m.set_u32(scene+0x27b4,p[2]);m.set_u32(scene+0x28b4,vid);
            m.set_u32(scene+0x5460,if p[3]!=0 {p[4]}else{0});m.set_u32(scene+0x28bc,p[5]);m.write(scene+0x2821,&[p[6] as u8]);
            m.set_u32(scene+0x5454,cfg2);m.write(cfg2+8,&bytes(v[8]));m.set_u32(cfg2+0xc,p[7]);
            let rands:Vec<i32>=v[9].split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect();
            let mut log=Log{rands:rands.clone(),used:0,events:vec![]};
            update_scene_effects(&mut m,scene,scene+0x20000,&mut log);
            assert_eq!(log.used,rands.len(),"rand count {line}");
            let want:Vec<String>=if v[10]=="-" {vec![]}else{v[10].split(';').map(String::from).collect()};
            assert_eq!(log.events,want,"{line}");
        }
    }
}

/// Flash flag block of `0x548250` (`0x5489e2..0x548a45`). `event` is the integer at `[physics+0x1228]`,
/// `record_flags` is the byte at `[record+0x108]`. A nonzero event with bit `0x40` set raises `0x10000`
/// (event 2) or `0x8000` (any other value) in `[scene+0x27b4]` and clears the event; otherwise `0x8000`
/// is cleared. `0x10000` is always cleared at the end, so only `0x8000` survives into `0x551970`.
pub fn update_flash_flags(m:&mut Mem,scene:u32,physics:u32,record_flags:u8) {
    let event=m.u32(physics+0x1228);let mut flags=m.u32(scene+0x27b4);
    if event!=0 && record_flags&0x40!=0 {
        flags|=if event==2 {0x10000}else{0x8000};
        m.set_u32(physics+0x1228,0);
    }else{
        flags&=!0x8000;
    }
    m.set_u32(scene+0x27b4,flags&!0x10000);
}
#[cfg(test)]
mod flash_flag_tests {
    use super::*;
    #[test]
    fn native_flash_flags(){
        for line in include_str!("../../../tools/emu/golden_flash_flags.tsv").lines(){
            let v:Vec<u32>=line.trim_end().split('\t').map(|s|s.parse::<i64>().unwrap() as u32).collect();let mut m=Mem::new();let (scene,phys)=(0x2a000000u32,0x2a010000u32);
            m.set_u32(scene+0x27b4,v[0]);m.set_u32(phys+0x1228,v[1]);
            update_flash_flags(&mut m,scene,phys,v[2] as u8);
            assert_eq!(m.u32(scene+0x27b4),v[3],"{line}");assert_eq!(m.u32(phys+0x1228),v[4],"{line}");
        }
    }
}

/// Flag rebuild of `0x54ff80` (`0x550452..0x55054a`): every step `[scene+0x27b4]` gets its bits from the
/// vehicle object. `0x04`/`0x10` from `[+0x51b8]` (1 -> `0x04`, 2 -> `0x10`, else both cleared), `0x08` from
/// `[+0x51b4]`, `0x40` from `[+0x517c]`, `0x20` from `[+0x5178]`, `0x01` from `[+0x4c4]`, `0x400` from
/// `[+0x51a0]` (first branch `0x4bac90`), `0x40000` from `[+0x51c0]`. `dl` (the byte of `[+0xf98]`) is stored at `+0x27b8`.
pub fn update_scene_flag_bits(m:&mut Mem,scene:u32,dl:u8) {
    m.write(scene+0x27b8,&[dl]);
    let mut f=m.u32(scene+0x27b4);
    match m.u32(scene+0x51b8) {1=>f=(f&!0x10)|4,2=>f=(f&!4)|0x10,_=>f&=!0x14}
    let set=|f:&mut u32,off:u32,bit:u32|{if m.u32(scene+off)!=0 {*f|=bit}else{*f&=!bit}};
    set(&mut f,0x51b4,0x08);set(&mut f,0x517c,0x40);set(&mut f,0x5178,0x20);set(&mut f,0x4c4,0x01);
    set(&mut f,0x51a0,0x400);set(&mut f,0x51c0,0x40000);
    m.set_u32(scene+0x27b4,f);
}
#[cfg(test)]
mod scene_flag_bits_tests {
    use super::*;
    #[test]
    fn native_scene_flag_bits(){
        for line in include_str!("../../../tools/emu/golden_scene_flag_bits.tsv").lines(){
            let v:Vec<u32>=line.trim_end().split('\t').map(|s|s.parse::<i64>().unwrap() as u32).collect();let mut m=Mem::new();let scene=0x2a000000u32;
            m.set_u32(scene+0x27b4,v[0]);
            for (o,x) in [0x51b8u32,0x51b4,0x517c,0x5178,0x4c4,0x51a0,0x51c0].into_iter().zip(&v[1..8]) {m.set_u32(scene+o,*x);}
            update_scene_flag_bits(&mut m,scene,v[8] as u8);
            assert_eq!(m.u32(scene+0x27b4),v[9],"{line}");assert_eq!(m.rd8(scene+0x27b8) as u32,v[10],"{line}");
        }
    }
}

/// `0x54a330`: 0.3 below 5, 1.0 above 20 and linear in between (returned in `ST0`, so as `f64`).
pub fn curve_54a330(x: f32) -> f64 {
    if !(x >= 5.0) {
        return f32::from_bits(0x3e99999a) as f64;
    }
    if x > 20.0 {
        return 1.0;
    }
    (x as f64 - 5.0) * f64::from_bits(0x3fa7e4b17e4b17e4) + f64::from_bits(0x3fd3333333333333)
}
#[cfg(test)]
mod curve_54a330_tests {
    use super::*;
    #[test]
    fn native_curve_54a330() {
        for line in include_str!("../../../tools/emu/golden_curve_54a330.tsv").lines() {
            let (x, y) = line.trim_end().split_once('\t').unwrap();
            let h = |s: &str| (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect::<Vec<u8>>();
            let x = f32::from_le_bytes(h(x).try_into().unwrap());
            let want = f64::from_le_bytes(h(y).try_into().unwrap());
            let got = curve_54a330(x);
            assert!(got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()), "{x}: {got} vs {want}");
        }
    }
}

/// One spark velocity as `0x593720` builds it: `p*p1 + q*q1 + r*r1` per component with the original's rounding
/// (x and y through f32 temporaries, the z term of the first product kept in extended precision).
fn spark_velocity(p: [f32; 3], p1: f32, q: [f32; 3], q1: f32, r: [f32; 3], r1: f32) -> [f32; 3] {
    let f = |a: f32, b: f32| (a as f64 * b as f64) as f32;
    let (f2, f3) = ([f(q[0], q1), f(q[1], q1), f(q[2], q1)], [f(r[0], r1), f(r[1], r1), f(r[2], r1)]);
    let xy = |k: usize| {
        let s = (f(p[k], p1) as f64 + f2[k] as f64) as f32;
        (s as f64 + f3[k] as f64) as f32
    };
    let zt = p[2] as f64 * p1 as f64 + f2[2] as f64;
    [xy(0), xy(1), (f3[2] as f64 + zt) as f32]
}

/// `0x593720`: sparks for an impact at `position` with unit normal `normal` and `count` strength.
/// `alloc` is `0x5940d0` (a free spark slot of `this`, 0 when full), `rand` the C `rand`, and `tail` the
/// closing call `0x594c90(this[0x60], position, count*0.01, 0.5, 0.08)` (arguments as raw words).
/// `scratch` needs 48 bytes. x87 temporaries are `f64`; every `fstp dword` of the original is an `as f32`.
pub fn emit_impact_sparks(m: &mut Mem, this: u32, position: u32, normal: u32, count: u32, scratch: u32,
    mut alloc: impl FnMut(&mut Mem, u32) -> u32, mut rand: impl FnMut(&mut Mem) -> i32, mut tail: impl FnMut(&mut Mem, [u32; 5])) {
    const K: f64 = f32::from_bits(0x38000100) as f64;
    if m.u32(this + 0x68) == 0 {
        return;
    }
    let v3 = |m: &Mem, a: u32| [m.f32(a), m.f32(a + 4), m.f32(a + 8)];
    let d = v3(m, normal);
    let df = d.map(|x| x as f64);
    // first axis: the normal against a constant vector (the up vector unless the normal is vertical)
    let c = if !(df[0] < 0.0 || df[0] > 0.0) && !(df[1] < 0.0 || df[1] > 0.0) { v3(m, 0x702640) } else { v3(m, 0x702630) }.map(|x| x as f64);
    let (v1, n1, v2, b2) = (scratch, scratch + 12, scratch + 24, scratch + 36);
    for (i, x) in [c[2] * df[1] - c[1] * df[2], c[0] * df[2] - c[2] * df[0], c[1] * df[0] - c[0] * df[1]].into_iter().enumerate() {
        m.set_f32(v1 + 4 * i as u32, x as f32);
    }
    crate::matrix::vec_unit(m, n1, v1);
    let n = v3(m, n1).map(|x| x as f64);
    for (i, x) in [n[2] * df[1] - n[1] * df[2], n[0] * df[2] - n[2] * df[0], n[1] * df[0] - n[0] * df[1]].into_iter().enumerate() {
        m.set_f32(v2 + 4 * i as u32, x as f32);
    }
    crate::matrix::vec_unit(m, b2, v2);
    let (n, bn) = (v3(m, n1), v3(m, b2));
    let count_f = count as f32;
    let sign = |x: f32| -> f32 {
        if (x as f64).abs() < 1.0 {
            if x <= 0.0 || x.is_nan() { -1.0 } else { 1.0 }
        } else {
            x
        }
    };
    // (second - first) * scale with the first rounded to f32, as `fstp dword` / `fsub dword`
    let pair = |m: &mut Mem, rand: &mut dyn FnMut(&mut Mem) -> i32, scale: f32| -> f32 {
        let a = (rand(m) as f64 * K) as f32;
        let b = rand(m) as f64;
        ((b * K - a as f64) * scale as f64) as f32
    };
    let fill = |m: &mut Mem, slot: u32, v: [f32; 3], rand: &mut dyn FnMut(&mut Mem) -> i32| {
        for i in 0..3u32 {
            m.set_f32(slot + 0x28 + 4 * i, v[i as usize]);
            m.set_f32(slot + 4 + 4 * i, v[i as usize]);
            let z = m.u32(0x702650 + 4 * i);
            m.set_u32(slot + 0x10 + 4 * i, z);
            let p = m.u32(position + 4 * i);
            m.set_u32(slot + 0x34 + 4 * i, p);
            m.set_u32(slot + 0x1c + 4 * i, p);
        }
        m.set_u32(slot + 0x78, 0);
        let t = m.f64(m.u32(this + 0x60) + 0xb98);
        m.set_f32(slot + 0x40, t as f32);
        let r = rand(m) as f64 * f32::from_bits(0x3b480190) as f64;
        let (a, b, c2) = if r < 50.0 || r.is_nan() { (0x6f6668, 0x6f6688, 0x6f6678) } else { (0x6f6698, 0x6f6658, 0x6f6648) };
        for k in 0..4 {
            let (x, y, z) = (m.u32(a + 4 * k), m.u32(b + 4 * k), m.u32(c2 + 4 * k));
            m.set_u32(slot + 0x44 + 4 * k, x);
            m.set_u32(slot + 0x54 + 4 * k, y);
            m.set_u32(slot + 0x64 + 4 * k, z);
        }
        m.set_u32(slot + 0x74, 0x3f800000);
    };
    let limit = (count_f as f64 * f32::from_bits(0x3e4ccccd) as f64) as f32;
    if 0.0 < limit || limit.is_nan() {
        let mut i = 0u32;
        loop {
            let slot = alloc(m, this);
            if slot == 0 {
                break;
            }
            let a = pair(m, &mut rand, 4.0);
            let b = pair(m, &mut rand, 4.0);
            let t = pair(m, &mut rand, 4.0);
            let v = spark_velocity(v3(m, 0x702640), sign(a), v3(m, 0x702620), sign(b), v3(m, 0x702630), t);
            fill(m, slot, v, &mut rand);
            i += 1;
            if !((i as f64) < limit as f64) {
                break;
            }
        }
    }
    let spread = (count_f as f64 * f32::from_bits(0x3d23d70a) as f64 + 1.0) as f32;
    let limit2 = (count_f as f64 * f32::from_bits(0x3f4ccccd) as f64) as f32;
    let mut i = 0u32;
    while (0.0 < limit2 || limit2.is_nan()) && (i == 0 || (i as f64) < limit2 as f64) {
        let slot = alloc(m, this);
        if slot == 0 {
            break;
        }
        let x = pair(m, &mut rand, spread);
        let y = pair(m, &mut rand, spread);
        let mut l = ((rand(m) as f64) * K + 1.0) as f32;
        let (x, y) = (sign(x), sign(y));
        if !(l <= 1.5) {
            l = 1.5;
        }
        let v = spark_velocity(n, x, bn, y, d, l);
        fill(m, slot, v, &mut rand);
        i += 1;
    }
    tail(m, [m.u32(this + 0x60), position, ((count_f as f64 * f32::from_bits(0x3c23d70a) as f64) as f32).to_bits(), 0x3f000000, 0x3da3d70a]);
}
#[cfg(test)]
mod impact_spark_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_impact_sparks() {
        let (this, pos, nrm, clock, slots) = (0x2a00_1000u32, 0x2a00_2000u32, 0x2a00_2100u32, 0x2a00_3000u32, 0x2a01_0000u32);
        for (n, line) in include_str!("../../../tools/emu/golden_impact_sparks.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            m.write(0x6f6648, &bytes(v[0]));
            m.set_u32(this + 0x60, clock);
            m.set_u32(this + 0x68, v[7].parse().unwrap());
            m.write(clock + 0xb98, &bytes(v[1]));
            m.write(pos, &bytes(v[2]));
            m.write(nrm, &bytes(v[3]));
            m.write(slots, &vec![0xaa; 0x100 * 140]);
            let (count, avail): (u32, u32) = (v[4].parse().unwrap(), v[5].parse().unwrap());
            let mut seed: u32 = v[6].parse().unwrap();
            let mut used = 0u32;
            let mut tail = None;
            emit_impact_sparks(&mut m, this, pos, nrm, count, 0x2a02_0000, |_, _| {
                if used >= avail { return 0; }
                used += 1;
                slots + 0x100 * (used - 1)
            }, |_| {
                seed = seed.wrapping_mul(214013).wrapping_add(2531011);
                ((seed >> 16) & 0x7fff) as i32
            }, |_, a| tail = Some(a));
            assert_eq!(used.to_string(), v[8], "case {n}: slots");
            let want_tail = if v[9] == "-" { None } else { Some(v[9].split(',').map(|s| s.parse::<u32>().unwrap()).collect::<Vec<_>>()) };
            assert_eq!(tail.map(|a| a.to_vec()), want_tail.map(|mut t| { t[0] = clock; t[1] = pos; t }), "case {n}: tail");
            let want = bytes(v[10]);
            for i in 0..used as usize {
                let got = m.read(slots + 0x100 * i as u32, 0x80);
                for off in (0..0x80).step_by(4) {
                    let (g, w) = (u32::from_le_bytes(got[off..off + 4].try_into().unwrap()), u32::from_le_bytes(want[i * 0x80 + off..i * 0x80 + off + 4].try_into().unwrap()));
                    assert_eq!(g, w, "case {n} slot {i} +{off:#x}: {:?} vs {:?}", f32::from_bits(g), f32::from_bits(w));
                }
            }
        }
    }
}

/// `0x5940d0` (thiscall): take the head node of the free list `[this+0x68]` and push it on the active
/// list `[this+0x6c]` (nodes link through their first word). Returns the node, 0 when none is free.
pub fn spark_slot_alloc(m: &mut Mem, this: u32) -> u32 {
    let node = m.u32(this + 0x68);
    if node == 0 {
        return 0;
    }
    let next = m.u32(node);
    m.set_u32(this + 0x68, next);
    let active = m.u32(this + 0x6c);
    m.set_u32(node, active);
    m.set_u32(this + 0x6c, node);
    node
}
#[cfg(test)]
mod spark_alloc_tests {
    use super::*;
    #[test]
    fn native_spark_slot_alloc() {
        let (this, nodes) = (0x2a00_1000u32, 0x2a00_2000u32);
        for line in include_str!("../../../tools/emu/golden_spark_alloc.tsv").lines() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let n: u32 = v[0].parse().unwrap();
            let want: Vec<u32> = v[2].split(',').map(|s| s.parse().unwrap()).collect();
            let mut m = Mem::new();
            for i in 0..n {
                m.set_u32(nodes + 0x40 * i, if i + 1 < n { nodes + 0x40 * (i + 1) } else { 0 });
            }
            m.set_u32(this + 0x68, if n > 0 { nodes } else { 0 });
            m.set_u32(this + 0x6c, v[1].parse().unwrap());
            let got = spark_slot_alloc(&mut m, this);
            let mut have = vec![got, m.u32(this + 0x68), m.u32(this + 0x6c)];
            have.extend((0..n).map(|i| m.u32(nodes + 0x40 * i)));
            assert_eq!(have, want, "{line}");
        }
    }
}

/// What the spark flash needs from the scene: the heap (`0x632643`), the light object's base constructors
/// (`0x5f19b0("ThrowLamp", 0x15, 1)` and `0x5ad4c0`) and the registration of the light with the scene
/// (virtual call `[vtable + 0x14](0, lamp)` on `[clock + 0x210]`).
pub trait SparkFlashOwner {
    fn allocate(&mut self, m: &mut Mem, size: u32) -> u32;
    fn construct_lamp(&mut self, m: &mut Mem, lamp: u32);
    fn register_lamp(&mut self, m: &mut Mem, scene: u32, lamp: u32);
}

/// `0x594b60` (thiscall): link `node` after `tail` of the flash list (`0x6f66b0` is the tail).
pub fn spark_flash_link(m: &mut Mem, node: u32, tail: u32) {
    if tail == 0 {
        m.set_u32(0x6f66b0, node);
        return;
    }
    m.set_u32(node, tail);
    let tail_prev = m.u32(tail + 4);
    m.set_u32(node + 4, tail_prev);
    if tail_prev == 0 {
        m.set_u32(0x6f66b0, node);
        m.set_u32(tail + 4, node);
        return;
    }
    m.set_u32(tail_prev, node);
    m.set_u32(tail + 4, node);
}

/// `0x5949f0` (thiscall on the 0x20-byte `node`): a "ThrowLamp" light at `position` of strength `p`.
pub fn construct_spark_flash(m: &mut Mem, node: u32, clock: u32, position: u32, p: f32, a5: u32, a6: u32, owner: &mut impl SparkFlashOwner) {
    for o in [0xc, 0x10, 0x14, 4, 0] {
        m.set_u32(node + o, 0);
    }
    for i in 0..3 {
        let v = m.u32(position + 4 * i);
        m.set_u32(node + 0xc + 4 * i, v);
    }
    let lamp = owner.allocate(m, 0x90);
    owner.construct_lamp(m, lamp);
    m.set_u32(lamp, 0x64d444);
    m.set_u32(lamp + 0x88, 1);
    m.set_u32(lamp + 0x8c, 0);
    m.set_u32(node + 0x18, lamp);
    m.set_u32(lamp + 0x88, 1);
    m.set_u32(lamp + 0x8c, 1);
    let scale = |c: u32| (p as f64 * f32::from_bits(c) as f64) as f32;
    m.set_u32(lamp + 0x38, 2);
    for i in 0..3 {
        let v = m.u32(node + 0xc + 4 * i);
        m.set_u32(lamp + 0x3c + 4 * i, v);
    }
    m.set_u32(lamp + 0x60, 0x3f800000);
    m.set_f32(lamp + 0x6c, p);
    m.set_f32(lamp + 0x70, scale(0x3f70f0e9));
    m.set_f32(lamp + 0x74, scale(0x3f3ebebe));
    m.set_u32(lamp + 0x54, 0x3f800000);
    m.set_u32(lamp + 0x58, 0);
    m.set_u32(lamp + 0x5c, a6);
    m.set_u32(lamp + 0x80, 0x46);
    let scene = m.u32(clock + 0x210);
    if scene != 0 {
        owner.register_lamp(m, scene, lamp);
    }
    m.set_u32(node + 8, a5);
    let t = m.f64(clock + 0xb98);
    m.set_f32(node + 0x1c, t as f32);
}

/// `0x594c90`: a spark flash of strength `p` (nothing below 0.05, at most 1.0) appended to the flash list.
pub fn emit_spark_flash(m: &mut Mem, clock: u32, position: u32, p: f32, a5: u32, a6: u32, owner: &mut impl SparkFlashOwner) {
    if !(p >= f32::from_bits(0x3d4ccccd)) {
        return;
    }
    let node = owner.allocate(m, 0x20);
    if node != 0 {
        let strength = if p > 1.0 { 1.0 } else { p };
        construct_spark_flash(m, node, clock, position, strength, a5, a6, owner);
    }
    let tail = m.u32(0x6f66b0);
    spark_flash_link(m, node, tail);
}
#[cfg(test)]
mod spark_flash_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    struct Owner {
        next: u32,
        registered: Option<(u32, u32, u32)>,
    }
    impl SparkFlashOwner for Owner {
        fn allocate(&mut self, _: &mut Mem, size: u32) -> u32 {
            let a = self.next;
            self.next += (size + 15) & !15;
            a
        }
        fn construct_lamp(&mut self, _: &mut Mem, _: u32) {}
        fn register_lamp(&mut self, _: &mut Mem, scene: u32, lamp: u32) {
            self.registered = Some((scene, 0, lamp));
        }
    }
    #[test]
    fn native_spark_flash() {
        let (heap, clock, pos, scene, vt, list) = (0x2a04_0000u32, 0x2a00_3000u32, 0x2a00_2000u32, 0x2a00_3800u32, 0x2a00_3900u32, 0x2a00_5000u32);
        for (n, line) in include_str!("../../../tools/emu/golden_spark_flash.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.write(heap, &vec![0xcc; 0x400]);
            m.write(pos, &bytes(v[0]));
            let p = f32::from_le_bytes(bytes(v[1]).try_into().unwrap());
            m.write(clock + 0xb98, &bytes(v[4]));
            m.set_u32(clock + 0x210, if v[5] == "1" { scene } else { 0 });
            m.set_u32(scene, vt);
            let (tail, prev): (u32, u32) = (v[6].parse().unwrap(), v[7].parse().unwrap());
            m.set_u32(0x6f66b0, tail);
            m.write(list, &[0x11, 0x11, 0x11, 0x11]);
            m.set_u32(list + 4, prev);
            m.set_u32(list + 0x100, 0x22222222);
            m.set_u32(list + 0x104, 0x33333333);
            let mut owner = Owner { next: heap, registered: None };
            emit_spark_flash(&mut m, clock, pos, p, v[2].parse().unwrap(), v[3].parse().unwrap(), &mut owner);
            let used = (owner.next - heap) as usize;
            assert_eq!(m.read(heap, used), bytes(v[8]), "case {n}: objects");
            assert_eq!(m.read(list, 8), bytes(v[9]), "case {n}: old tail");
            assert_eq!(m.read(list + 0x100, 8), bytes(v[10]), "case {n}: previous");
            assert_eq!(m.read(0x6f66b0, 4), bytes(v[11]), "case {n}: tail pointer");
            let want = if v[12] == "-" { None } else { let t: Vec<u32> = v[12].split(',').map(|s| s.parse().unwrap()).collect(); Some((t[0], t[1], t[2])) };
            assert_eq!(owner.registered, want, "case {n}: registration");
        }
    }
}

/// `0x4ba130(position, vector, send)`: an impact of strength `|vector|`. Above a quarter of the effect
/// setting `0x696030 > 0.5` it optionally announces the impact over the network (`send(id 0x3bd, 0x1c bytes)`
/// when `send` and the session flags `[0x6cec6c]+0x514 == 1` or `+0x524 != 0`) and starts
/// `0x593720(position, vector, min(|vector|*6, 100) truncated)` through `sparks`.
pub fn impact_effect(m: &mut Mem, position: u32, vector: u32, send: bool, mut announce: impl FnMut(&mut Mem, u32, [u8; 0x1c]),
    sparks: impl FnOnce(&mut Mem, u32, u32, u32)) {
    if !(m.f32(0x696030) > 0.5) {
        return;
    }
    let session = m.u32(0x6cec6c);
    if send && (m.i32(session + 0x514) == 1 || m.u32(session + 0x524) != 0) {
        let mut buf = [0u8; 0x1c];
        buf[0] = 1;
        for i in 0..3u32 {
            let (p, v) = (m.u32(position + 4 * i), m.u32(vector + 4 * i));
            buf[4 + 4 * i as usize..8 + 4 * i as usize].copy_from_slice(&p.to_le_bytes());
            buf[0x10 + 4 * i as usize..0x14 + 4 * i as usize].copy_from_slice(&v.to_le_bytes());
        }
        announce(m, 0x3bd, buf);
    }
    let mut n = crate::math::vec_len(m, vector) * 6.0;
    if n > 100.0 {
        n = 100.0;
    }
    sparks(m, position, vector, n as i32 as u32);
}
#[cfg(test)]
mod impact_effect_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_impact_effect() {
        let (pos, vec, cfg, clock) = (0x2a00_0100u32, 0x2a00_0200u32, 0x2a00_0300u32, 0x2a00_0400u32);
        let mut announced_cases = 0;
        for (n, line) in include_str!("../../../tools/emu/golden_impact_effect.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.write(pos, &bytes(v[0]));
            m.write(vec, &bytes(v[1]));
            m.write(0x696030, &bytes(v[2]));
            m.set_u32(0x6cec6c, cfg);
            m.set_u32(cfg + 0x514, v[3].parse().unwrap());
            m.set_u32(cfg + 0x524, v[4].parse().unwrap());
            m.set_u32(0x696224, clock);
            let mut net = None;
            let mut spark = None;
            impact_effect(&mut m, pos, vec, v[5] == "1", |_, id, buf| net = Some((id, buf)), |_, p, vv, c| spark = Some((p, vv, c)));
            let want_net = if v[6] == "-" { None } else { let t: Vec<&str> = v[6].split(',').collect(); Some((clock + 0x2090, t[1].parse::<u32>().unwrap(), t[2].parse::<usize>().unwrap(), t[3].to_owned())) };
            let got_net = net.map(|(id, b)| (clock + 0x2090, id, b.len(), b.iter().map(|x| format!("{x:02x}")).collect::<String>()));
            assert_eq!(got_net.as_ref().map(|g| (g.1, g.2, g.3.clone())), want_net.as_ref().map(|g| (g.1, g.2, g.3.clone())), "case {n}: network");
            let want_spark = if v[7] == "-" { None } else { let t: Vec<u32> = v[7].split(',').map(|s| s.parse().unwrap()).collect(); Some((t[1], t[2], t[3])) };
            assert_eq!(spark, want_spark, "case {n}: sparks");
            announced_cases += got_net.is_some() as u32;
        }
        assert!(announced_cases > 10);
    }
}

/// `0x54fff3..0x5501d7` of `0x54ff80`: the vehicle scene's impact trigger. When the contact code has left a
/// point at `[scene+0x2a3c]` (flag `+0x2a38`) and the global `0x6ef200` is clear, the point is carried through the
/// matrix at `+0x264` into the frame at `+0x1b0 / +0x1bc` (`W`), its direction `U = unit(W)` is tested against the
/// vector at `+0x3e0` and, for a slow enough contact, `impact(point, W)` runs (`0x54ef90`, which is `0x4ba130`
/// with the network announcement). `scratch` needs 36 bytes.
pub fn scene_impact_trigger(m: &mut Mem, scene: u32, scratch: u32, impact: impl FnOnce(&mut Mem, u32, u32)) {
    if m.u32(scene + 0x2a38) == 0 || m.u32(0x6ef200) != 0 {
        return;
    }
    let f = |m: &Mem, o: u32| m.f32(scene + o) as f64;
    let p = [m.f32(scene + 0x2a3c), m.f32(scene + 0x2a40), m.f32(scene + 0x2a44)];
    let (px, py, pz) = (p[0] as f64, p[1] as f64, p[2] as f64);
    let x = pz * f(m, 0x27c) + py * f(m, 0x270) + px * f(m, 0x264) + f(m, 0x288);
    let y = ((px * f(m, 0x268) + pz * f(m, 0x280)) + py * f(m, 0x274) + f(m, 0x28c)) as f32 as f64;
    let z = ((px * f(m, 0x26c) + pz * f(m, 0x284)) + py * f(m, 0x278) + f(m, 0x290)) as f32 as f64;
    let a = (y * f(m, 0x1bc) - x * f(m, 0x1c0)) as f32 as f64;
    let b = (x * f(m, 0x1c4) - z * f(m, 0x1bc)) as f32 as f64;
    let c = (z * f(m, 0x1c0) - y * f(m, 0x1c4)) as f32;
    let w = [(c as f64 + f(m, 0x1b0)) as f32, (b + f(m, 0x1b4)) as f32, (a + f(m, 0x1b8)) as f32];
    let (pp, ww, uu) = (scratch, scratch + 12, scratch + 24);
    for i in 0..3u32 {
        m.set_f32(pp + 4 * i, p[i as usize]);
        m.set_f32(ww + 4 * i, w[i as usize]);
    }
    crate::matrix::vec_unit(m, uu, ww);
    if m.u32(scene + 0x38c) == 0 || m.u32(scene + 0x414) != 0 {
        return;
    }
    let u = [m.f32(uu) as f64, m.f32(uu + 4) as f64, m.f32(uu + 8) as f64];
    let dot = f(m, 0x3e8) * u[2] + f(m, 0x3e4) * u[1] + u[0] * f(m, 0x3e0);
    if dot.abs() < f64::from_bits(0x3fe3333333333333) && f(m, 0x3e8).abs() < 0.5 {
        impact(m, pp, ww);
    }
}
#[cfg(test)]
mod scene_impact_trigger_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_impact_trigger() {
        let scene = 0x2a01_0000u32;
        for (n, line) in include_str!("../../../tools/emu/golden_scene_impact_trigger.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            m.write(scene + 0x264, &bytes(v[0]));
            let frame = bytes(v[1]);
            m.write(scene + 0x1b0, &frame[..12]);
            m.write(scene + 0x1bc, &frame[12..24]);
            m.write(scene + 0x2a3c, &bytes(v[2]));
            m.write(scene + 0x3e0, &bytes(v[3]));
            m.set_u32(scene + 0x2a38, v[4].parse().unwrap());
            m.set_u32(0x6ef200, v[5].parse().unwrap());
            m.set_u32(scene + 0x38c, v[6].parse().unwrap());
            m.set_u32(scene + 0x414, v[7].parse().unwrap());
            let mut got = None;
            scene_impact_trigger(&mut m, scene, 0x2a02_0000, |m, p, w| got = Some((m.read(p, 12), m.read(w, 12))));
            let want = if v[8] == "-" { None } else { let t: Vec<&str> = v[8].split(';').collect(); Some((bytes(t[0]), bytes(t[1]))) };
            assert_eq!(got, want, "case {n}");
        }
    }
}

/// `0x5501d7..0x550452` of `0x54ff80`: smoothing of the body's pitch and roll lean `[scene+0x4f54]` / `[+0x4f58]`
/// from the frame's velocity change. `v = (position - previous) / dt` goes through the matrix at `+0x264`; the
/// lean target is a clamped (+-5 degrees) combination of the local velocity and the local acceleration against
/// the gains `0x6ef2a0` / `0x6ef2a4`, blended with `0.6^(10 dt)` (`pow` is `f64::powf`, a documented stand-in for
/// the CRT's x87 `pow`). Clears the impact flag and the impact point. `scratch` needs 36 bytes.
pub fn scene_body_lean(m: &mut Mem, scene: u32, scratch: u32) {
    let f = |m: &Mem, o: u32| m.f32(scene + o) as f64;
    let dt = crate::globals::dt(m) as f64;
    let dz = f(m, 0x1b8) - f(m, 0x1dc);
    let dy = (f(m, 0x1b4) - f(m, 0x1d8)) as f32 as f64;
    let dx = (f(m, 0x1b0) - f(m, 0x1d4)) as f32 as f64;
    let (vz, vy, vx) = (dz / dt, dy / dt, dx / dt);
    let a = ((vy * f(m, 0x270) + vz * f(m, 0x27c)) + vx * f(m, 0x264)) as f32 as f64;
    let b = ((vx * f(m, 0x268) + vy * f(m, 0x274)) + vz * f(m, 0x280)) as f32 as f64;
    let c0 = ((f(m, 0x27c) * f(m, 0x1e8) + f(m, 0x270) * f(m, 0x1e4)) + f(m, 0x264) * f(m, 0x1e0)) as f32;
    let d = ((f(m, 0x280) * f(m, 0x1e8) + f(m, 0x274) * f(m, 0x1e4)) + f(m, 0x268) * f(m, 0x1e0)) as f32;
    let e = ((f(m, 0x284) * f(m, 0x1e8) + f(m, 0x278) * f(m, 0x1e4)) + f(m, 0x26c) * f(m, 0x1e0)) as f32;
    let (vec1, q) = (scratch, scratch + 12);
    for (i, x) in [c0, d, e].into_iter().enumerate() {
        m.set_f32(vec1 + 4 * i as u32, x);
    }
    crate::carv::fn_004e3132(m, q, vec1, scene + 0x1a4);
    let (qx, qy) = (m.f32(q) as f64, m.f32(q + 4) as f64);
    let s = (f(m, 0x1bc) - qx) as f32 as f64;
    let t = ((f(m, 0x1c0) - qy) / dt) as f32 as f64;
    let s = (s / dt) as f32 as f64;
    let p = (0.6f32 as f64).powf(dt * 10.0);
    let g = |a: u32| m.f32(a) as f64;
    let u = ((b * f32::from_bits(0x3be56042) as f64) - s * g(0x6ef2a0)) as f32;
    let v = t * g(0x6ef2a4) - a * f32::from_bits(0x3ba3d70a) as f64;
    let lim = f32::from_bits(0x3db2b8c2);
    let mut u = u;
    if !(u <= lim) {
        u = lim;
    }
    if u < -lim {
        u = -lim;
    }
    let mut v = v;
    if !(v <= lim as f64) {
        v = lim as f64;
    }
    if v < (-lim) as f64 {
        v = (-lim) as f64;
    }
    let k = 1.0 - p;
    m.set_u32(scene + 0x2a38, 0);
    let old = m.f32(scene + 0x4f54) as f64;
    m.set_f32(scene + 0x4f54, (k * u as f64 + p * old) as f32);
    let old = m.f32(scene + 0x4f58) as f64;
    m.set_f32(scene + 0x4f58, (k * v + p * old) as f32);
    for i in 0..3u32 {
        let z = m.u32(0x702650 + 4 * i);
        m.set_u32(scene + 0x2a3c + 4 * i, z);
    }
}
#[cfg(test)]
mod scene_body_lean_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    /// `pow` is a stand-in (`f64::powf`), so a result may differ from the CRT's by one f32 ulp.
    #[test]
    fn native_scene_body_lean() {
        let (scene, clock) = (0x2a01_0000u32, 0x2a10_0000u32);
        let (mut exact, mut total) = (0, 0);
        for (n, line) in include_str!("../../../tools/emu/golden_scene_body_lean.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            m.write(scene + 0x264, &bytes(v[0]));
            m.write(scene + 0x1b0, &bytes(v[1]));
            m.write(scene + 0x1a4, &bytes(v[2]));
            m.write(scene + 0x1bc, &bytes(v[3]));
            m.write(scene + 0x1d4, &bytes(v[4]));
            m.write(scene + 0x1e0, &bytes(v[5]));
            m.write(scene + 0x4f54, &bytes(v[6]));
            m.write(scene + 0x2a3c, &[0, 0, 0x80, 0x3f, 0, 0, 0, 0x40, 0, 0, 0x40, 0x40]);
            m.set_u32(scene + 0x2a38, 1);
            m.write(0x6ef2a0, &bytes(v[8]));
            m.set_u32(0x6974c4, clock);
            m.write(clock + 0xb90, &bytes(v[7]));
            scene_body_lean(&mut m, scene, 0x2a02_0000);
            let want = bytes(v[9]);
            let got: Vec<u8> = [m.read(scene + 0x4f54, 8), m.read(scene + 0x2a3c, 12), m.read(scene + 0x2a38, 4)].concat();
            assert_eq!(&got[8..], &want[8..], "case {n}: point and flag");
            for i in 0..2 {
                let (g, w) = (u32::from_le_bytes(got[i * 4..i * 4 + 4].try_into().unwrap()), u32::from_le_bytes(want[i * 4..i * 4 + 4].try_into().unwrap()));
                total += 1;
                if g == w {
                    exact += 1;
                } else {
                    assert!((g as i64 - w as i64).abs() <= 1, "case {n} lean {i}: {:?} vs {:?}", f32::from_bits(g), f32::from_bits(w));
                }
            }
        }
        eprintln!("exact {exact} of {total}");
        assert!(exact * 100 >= total * 98, "{exact}/{total}");
    }
}

/// `0x55059f..0x5506cd` of `0x54ff80`: clears the eight wheel dust levels `[scene+0x27f0]` and, while the owner
/// is driving on a track (`[scene+0x2aac] == 1`, `[owner+0x3344] == 0`) faster than 1, sets the level of every
/// wheel standing on a loose surface (surface kind 6, 10 or 11): `0.4 * sqrt(f32(|(|v| - 0.7 |w|)| / 15))`, at
/// full strength above 15, scaled by `sqrt(mass * 1e-4)` for a light vehicle (`[scene+0x19c] < 10000`); kept only
/// above 0.1.
pub fn scene_wheel_levels(m: &mut Mem, scene: u32) {
    for i in 0..8 {
        m.set_u32(scene + 0x27f0 + 4 * i, 0);
    }
    let owner = m.u32(scene + 0x5460);
    if owner == 0 || m.i32(scene + 0x2aac) != 1 || m.u32(owner + 0x3344) != 0 {
        return;
    }
    let x = (m.f32(owner + 0x332c) as f64).abs();
    if !(x > 1.0) {
        return;
    }
    let mut level = 1.0f32;
    if x < 15.0 {
        let y = (m.f32(owner + 0x3330) as f64).abs() * 0.7;
        level = ((x - y).abs() * f64::from_bits(0x3fb1111111111111)) as f32;
    }
    let mut v = (level as f64).sqrt() * f64::from_bits(0x3fd999999999999a);
    let mass = m.f32(scene + 0x19c);
    if mass < f32::from_bits(0x461c4000) {
        v *= (mass as f64 * f64::from_bits(0x3f1a36e2eb1c432d)).sqrt();
    }
    for i in 0..m.i32(scene + 0x28bc).max(0) as u32 {
        let wheel = owner + 0xf8 * i;
        let surface = m.u32(wheel + 0x2bd4);
        if m.u16(wheel + 0x2b68) == 0 || m.u32(owner + 0x3444 + 4 * i) != 0 || surface == 0 || m.u32(wheel + 0x2bdc) == 0 {
            continue;
        }
        if matches!(m.u32(surface + 4), 6 | 10 | 11) && v > 0.1 {
            m.set_f32(scene + 0x27f0 + 4 * i, v as f32);
        }
    }
}
#[cfg(test)]
mod scene_wheel_levels_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_wheel_levels() {
        let (scene, owner, surf) = (0x2a01_0000u32, 0x2a02_0000u32, 0x2a03_0000u32);
        let mut with_levels = 0;
        for (n, line) in include_str!("../../../tools/emu/golden_scene_wheel_levels.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.write(scene + 0x27f0, &[0x77; 32]);
            m.write(owner + 0x332c, &bytes(v[0]));
            m.write(scene + 0x19c, &bytes(v[1]));
            m.set_u32(scene + 0x28bc, v[2].parse().unwrap());
            m.set_u32(scene + 0x5460, if v[3] == "1" { owner } else { 0 });
            m.set_u32(scene + 0x2aac, v[4].parse().unwrap());
            m.set_u32(owner + 0x3344, v[5].parse().unwrap());
            for (i, s) in v[6].split(';').enumerate() {
                let t: Vec<u32> = s.split(',').map(|x| x.parse().unwrap()).collect();
                let i = i as u32;
                m.set_i16(owner + 0xf8 * i + 0x2b68, t[0] as i16);
                m.set_u32(owner + 0x3444 + 4 * i, t[1]);
                m.set_u32(owner + 0xf8 * i + 0x2bd4, if t[3] != 0 { surf + 0x40 * i } else { 0 });
                m.set_u32(owner + 0xf8 * i + 0x2bdc, if t[4] != 0 { 0x1234 } else { 0 });
                m.set_u32(surf + 0x40 * i + 4, t[2]);
            }
            scene_wheel_levels(&mut m, scene);
            let want = bytes(v[7]);
            for i in 0..8usize {
                let (g, w) = (m.u32(scene + 0x27f0 + 4 * i as u32), u32::from_le_bytes(want[i * 4..i * 4 + 4].try_into().unwrap()));
                assert!(g == w || (g as i64 - w as i64).abs() <= 1 && w != 0, "case {n} wheel {i}: {:?} vs {:?}", f32::from_bits(g), f32::from_bits(w));
                with_levels += (w != 0) as u32;
            }
        }
        assert!(with_levels > 50);
    }
}

/// `FUN_00557090` (`__fastcall`, `ecx = scene`): light intensity of the scene in `0..=1`. Starts from
/// `min(f32(1 or 0 when `0x549bf0` is set), f32([scene+0x5080]) or, below `[scene+0x224] < 0.5`, `2 * [scene+0x224]`
/// clamped at 0)`, then folds in nine terms `t = 1 - |a / b| * k` of the integer pairs in the record at
/// `[scene+0x2540]` (k = 0.1, 0.5, 0.5, 0.5, 0.6, 0.6, 0.3, 0.3, 0.3): each `f32(t)` is a candidate minimum
/// (compared before rounding) and `(f32(t) + 1)` joins a running product halved after every step, which is
/// the last candidate. The result is clamped at 0; comparisons treat NaN as "less", as `fnstsw` does.
pub fn scene_light_intensity(m: &mut Mem, scene: u32) -> f64 {
    let lt = |a: f64, b: f64| !(a >= b);
    let mut a = 1.0f32;
    let mut b = m.i32(scene + 0x5080) as f32;
    let d = m.f32(scene + 0x224) as f64;
    if lt(d, 0.5) {
        b = if lt(d, 0.0) { 0.0 } else { (d + d) as f32 };
    }
    let flag_obj = m.u32(scene + 0x5440 + 0x14);
    if m.u32(flag_obj + 0xc) != 0 {
        a = 0.0;
    }
    let mut v = if lt(a as f64, b as f64) { a as f64 } else { b as f64 };
    let rec = m.u32(scene + 0x2540);
    const TERMS: [(u32, f64); 9] = [
        (0x04, 0.1), (0x18, 0.5), (0x2c, 0.5), (0x40, 0.5), (0x54, 0.6), (0x68, 0.6), (0x7c, 0.3), (0x90, 0.3), (0xa4, 0.3),
    ];
    let mut prod = 0.0f32;
    for (i, &(off, k)) in TERMS.iter().enumerate() {
        let q = (m.i32(rec + off) as f64 / m.i32(rec + off + 4) as f64).abs();
        let t = 1.0 - q * k;
        let t32 = t as f32;
        if lt(t, v) {
            v = t32 as f64;
        }
        let p = (t32 as f64 + 1.0) * if i == 0 { 1.0 } else { prod as f64 } * 0.5;
        prod = p as f32;
        if i == TERMS.len() - 1 && lt(p, v) {
            v = prod as f64;
        }
    }
    if lt(v, 0.0) {
        v = 0.0;
    }
    v
}
#[cfg(test)]
mod scene_light_intensity_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_light_intensity() {
        let (scene, rec, flag) = (0x2a01_0000u32, 0x2a03_0000u32, 0x2a04_0000u32);
        let mut nonzero = 0;
        for (n, line) in include_str!("../../../tools/emu/golden_scene_light_intensity.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.set_i32(scene + 0x5080, v[0].parse().unwrap());
            m.write(scene + 0x224, &bytes(v[1]));
            m.set_u32(scene + 0x5440 + 0x14, flag);
            m.set_u32(flag + 0xc, v[2].parse().unwrap());
            m.set_u32(scene + 0x2540, rec);
            for (k, p) in v[3].split(',').enumerate() {
                let (a, b) = p.split_once('/').unwrap();
                m.set_i32(rec + [4u32, 0x18, 0x2c, 0x40, 0x54, 0x68, 0x7c, 0x90, 0xa4][k], a.parse().unwrap());
                m.set_i32(rec + [8u32, 0x1c, 0x30, 0x44, 0x58, 0x6c, 0x80, 0x94, 0xa8][k], b.parse().unwrap());
            }
            let got = scene_light_intensity(&mut m, scene);
            let want = f64::from_le_bytes(bytes(v[4]).try_into().unwrap());
            assert!(got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()), "case {n}: {got:?} vs {want:?}");
            nonzero += (want != 0.0) as u32;
        }
        assert!(nonzero > 50);
    }
}

/// `0x550735..0x550a42` of `0x54ff80` (entered when `0x557090 > 0.4`); `f` is the function's stack frame as it
/// is on entry to the block (every `[esp+X]` of the original is `f + X` plus the pushes in flight, resolved here).
/// Sets `[scene+0x5028] = 1`, turns the game time into the angle `2*pi*frac(t + [scene+0x501c])` (stored at
/// `[scene+0x5020]`, and at `[scene+0x5024]` wrapped below `pi`), prepares the frame with the three squared cosines
/// of the angle and its two ±2*pi/3 shifts, takes the transform `[scene+0x2c4] * [scene+0x204]`, its inverse, the
/// point of `[0x6d2530 + 0x5d0 * [scene+0x28b4]]` carried through it, the unit vector from there to the camera
/// point of `[0x6d2078]`, and the heading `-atan2(..)`. Returns that heading and whether the original goes on at
/// `0x551090` (heading between 1.0472 and 2.0944) instead of `0x550a42`. `cos`/`atan2` are stand-ins for `fcos`/`fpatan`.
pub fn scene_light_ray_setup(m: &mut Mem, scene: u32, f: u32) -> (f64, bool) {
    use std::f64::consts::PI;
    let r = |m: &Mem, o: u32| m.f32(f + o) as f64;
    m.set_u32(scene + 0x5028, 1);
    let t = crate::globals::time_hi(m) + m.f32(scene + 0x501c) as f64;
    let n = t.trunc() as i64 as i32;
    m.set_i32(f + 0x20, n);
    let ang = (((t - n as f64) * 2.0) * PI) as f32;
    m.set_f32(f + 0x14, ang);
    m.set_f32(f + 0x64, ang);
    m.set_f32(scene + 0x5020, ang);
    m.set_f32(scene + 0x5024, ang);
    m.set_f32(f + 0x74, (1.0471975803375244f64).cos() as f32);
    m.set_f32(f + 0x20, (2.094395160675049f64).cos() as f32);
    m.set_f32(f + 0x9c, (1.0 / (1.0 - r(m, 0x74))) as f32);
    let third = 2.094395160675049f32 as f64;
    m.set_f32(f + 0x7c, (ang as f64 - third) as f32);
    m.set_f32(f + 0x1c, (ang as f64 + third) as f32);
    let c = r(m, 0x7c).cos();
    m.set_f32(f + 0x60, r(m, 0x64).cos() as f32);
    m.set_f32(f + 0x48, r(m, 0x1c).cos() as f32);
    if ang as f64 > PI {
        m.set_f32(scene + 0x5024, (ang as f64 - 2.0 * PI) as f32);
    }
    m.set_f32(f + 0x78, (c * c) as f32);
    for o in [0x60, 0x48] {
        let v = r(m, o);
        m.set_f32(f + o, (v * v) as f32);
    }
    crate::matrix::mat_mul(m, f + 0xc0, scene + 0x2c4, scene + 0x204);
    crate::math::mat_inverse(m, f + 0xf0, f + 0xc0);
    let tbl = 0x6d2530 + 0x5d0 * m.u32(scene + 0x28b4);
    let (tx, ty, tz) = (m.f32(tbl) as f64, m.f32(tbl + 4) as f64, m.f32(tbl + 8) as f64);
    let at = |m: &Mem, o: u32| m.f32(f + o) as f64;
    let x = at(m, 0xc0) * tx + at(m, 0xd8) * tz + at(m, 0xcc) * ty + at(m, 0xe4);
    m.set_f32(f + 0x2c, x as f32);
    let y = at(m, 0xdc) * tz + at(m, 0xd0) * ty + at(m, 0xc4) * tx + at(m, 0xe8);
    m.set_f32(f + 0x28, y as f32);
    let z = at(m, 0xe0) * tz + at(m, 0xd4) * ty + at(m, 0xc8) * tx + at(m, 0xec);
    let w = m.u32(0x6d2078);
    let v3 = m.f32(w + 0x5bc) as f64 - z;
    let v2 = m.f32(w + 0x5b8) as f64 - at(m, 0x28);
    let v1 = m.f32(w + 0x5b4) as f64 - at(m, 0x2c);
    m.set_f32(f + 0x50, v1 as f32);
    m.set_f32(f + 0x54, v2 as f32);
    m.set_f32(f + 0x58, v3 as f32);
    for (d, s) in [(0x34, 0x50), (0x38, 0x54), (0x3c, 0x58)] {
        let b = m.u32(f + s);
        m.set_u32(f + d, b);
    }
    crate::matrix::vec_unit(m, f + 0xb4, f + 0x34);
    let a0 = at(m, 0x108) * at(m, 0xbc) + at(m, 0xfc) * at(m, 0xb8) + at(m, 0xf0) * at(m, 0xb4);
    m.set_f32(f + 0xa0, a0 as f32);
    let a1 = at(m, 0x10c) * at(m, 0xbc) + at(m, 0x100) * at(m, 0xb8) + at(m, 0xb4) * at(m, 0xf4);
    m.set_f32(f + 0x14, a1 as f32);
    let a2 = at(m, 0x110) * at(m, 0xbc) + at(m, 0x104) * at(m, 0xb8) + at(m, 0xb4) * at(m, 0xf8);
    m.set_f32(f + 0xa4, a2 as f32);
    let head = -at(m, 0xa0).atan2(at(m, 0x14));
    let take = head.abs() >= 1.0471975803375244 && head.abs() <= 2.094395160675049;
    (head, take)
}
#[cfg(test)]
mod scene_light_ray_setup_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_light_ray_setup() {
        let (scene, world, clock, f) = (0x2a01_0000u32, 0x2a04_0000u32, 0x2a05_0000u32, 0x2a06_0000u32);
        let (mut exact, mut total, mut taken) = (0, 0, 0);
        for (n, line) in include_str!("../../../tools/emu/golden_scene_light_ray_setup.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.set_u32(crate::globals::CLOCK_PTR, clock);
            m.write(clock + 0xb98, &bytes(v[0]));
            m.write(crate::globals::FIXED_STEP_MS, &bytes(&v[1][..4]));
            m.write(crate::globals::FIXED_TIME, &bytes(&v[1][4..]));
            m.write(scene + 0x501c, &bytes(v[2]));
            m.write(scene + 0x204, &bytes(v[3]));
            let idx: u32 = v[4].parse().unwrap();
            m.set_u32(scene + 0x28b4, idx);
            m.write(0x6d2530 + 0x5d0 * idx, &bytes(v[5]));
            m.set_u32(0x6d2078, world);
            m.write(world + 0x5b4, &bytes(v[6]));
            m.write(f, &bytes(v[7]));
            crate::globals::install_constants(&mut m);
            let (head, take) = scene_light_ray_setup(&mut m, scene, f);
            let want_head = f64::from_le_bytes(bytes(v[9]).try_into().unwrap());
            assert!((head - want_head).abs() < 1e-5, "case {n}: heading {head} vs {want_head}");
            if (head.abs() - 1.0471975803375244).abs() > 1e-5 && (head.abs() - 2.094395160675049).abs() > 1e-5 {
                assert_eq!(take, v[8] == "1", "case {n}: branch");
            }
            taken += take as u32;
            let want = bytes(v[10]);
            for i in 0..want.len() / 4 {
                let (g, w) = (m.u32(f + 4 * i as u32), u32::from_le_bytes(want[i * 4..i * 4 + 4].try_into().unwrap()));
                total += 1;
                exact += (g == w) as u32;
                let ok = g == w || (g as i64 - w as i64).abs() <= 2 || (f32::from_bits(g) - f32::from_bits(w)).abs() <= 1e-4;
                assert!(ok, "case {n} frame +{:#x}: {:?} vs {:?}", 4 * i, f32::from_bits(g), f32::from_bits(w));
            }
            let sc = bytes(v[11]);
            for i in 0..3 {
                let (g, w) = (m.u32(scene + 0x5020 + 4 * i), u32::from_le_bytes(sc[i as usize * 4..i as usize * 4 + 4].try_into().unwrap()));
                assert!(g == w || (g as i64 - w as i64).abs() <= 2, "case {n} scene +{:#x}", 0x5020 + 4 * i);
            }
        }
        eprintln!("exact {exact} of {total}, {taken} taken");
        assert!(exact * 100 >= total * 98 && taken > 20);
    }
}

/// `0x550a42..0x550d7c` of `0x54ff80` (continues `scene_light_ray_setup` when it returned `false`; `head` is the
/// heading it returned, still on the FPU stack in the original). `X = (cos(head) - cos(pi/3))` (or
/// `cos(2*pi/3) - cos(head)` for a heading beyond 1.0472) divided by `1 - cos(pi/3)` scales the squared cosines
/// into `f+0x40`, `f+0x2c`, `f+0x28`. Three corner points are then built, one for each of the angles `f+0x7c`,
/// `f+0x64`, `f+0x1c`: the vector `table[i] - 0.03 * sin(angle) * (0x702648, 0x702644, 0x702640)` of the record
/// `0x6d2518 + 0x5d0 * [scene+0x28b4]` (offsets 0xc.., 0x18.., 0x24..) plus `+-0.2` along the vector `0x702620`
/// (`k`, sign from `f+0x14 > 0`), plus the direction `(f+0xa4, f+0x14, f+0xa0)` times `W = (|sin head| + 1) * 0.6`;
/// they land in `f+0x50..`, `f+0x68..` and `f+0x34..` (z, y, x order). `f+0x80..` is `f+0x40` times the vector
/// at `0x6974a8`, and `0x5abc40` builds the translation to the first corner at `f+0x168`.
/// `sin/cos` are stand-ins for `fsin/fcos`.
pub fn scene_light_ray_points(m: &mut Mem, scene: u32, f: u32, head: f64) {
    let r = |m: &Mem, o: u32| m.f32(f + o) as f64;
    let g = |m: &Mem, a: u32| m.f32(a) as f64;
    m.set_f32(f + 0x24, head.sin().abs() as f32);
    let d = if head.abs() < 1.0471975803375244 { head.cos() - r(m, 0x74) } else { r(m, 0x20) - head.cos() };
    let x = d * r(m, 0x9c);
    m.set_f32(f + 0x40, (x * r(m, 0x78)) as f32);
    m.set_f32(f + 0x2c, (x * r(m, 0x60)) as f32);
    m.set_f32(f + 0x28, (x * r(m, 0x48)) as f32);
    let k: f64 = if m.f32(f + 0x14) > 0.0 { f64::from_bits(0x3fc999999999999a) } else { f64::from_bits(0xbfc999999999999a) };
    m.set_f32(f + 0x8c, (k * g(m, 0x702628)) as f32);
    m.set_f32(f + 0x44, (k * g(m, 0x702624)) as f32);
    let k0 = k * g(m, 0x702620);
    let tbl = 0x6d2518 + 0x5d0 * m.u32(scene + 0x28b4);
    let w = (r(m, 0x24) + 1.0) * 0.6000000238418579;
    let products = |m: &mut Mem, slots: [u32; 3]| {
        let v = (r(m, 0xa4) * w) as f32;
        m.set_f32(f + slots[0], v);
        let v = (r(m, 0x14) * w) as f32;
        m.set_f32(f + slots[1], v);
        let v = (r(m, 0xa0) * w) as f32;
        m.set_f32(f + slots[2], v);
    };
    products(m, [0x4c, 0x5c, 0x10]);
    // (angle slot, record offset, scratch slots a/b/c, product slots for a/b/c, output slots for c/b/a)
    let corners: [(u32, u32, [u32; 3], [u32; 3], [u32; 3]); 3] = [
        (0x7c, 0x14, [0x24, 0x30, 0x18], [0x4c, 0x5c, 0x10], [0x50, 0x54, 0x58]),
        (0x64, 0x20, [0x10, 0x18, 0x30], [0x5c, 0x4c, 0x24], [0x68, 0x6c, 0x70]),
        (0x1c, 0x2c, [0x10, 0x18, 0x30], [0x5c, 0x4c, 0x24], [0x34, 0x38, 0x3c]),
    ];
    for (n, &(angle, hi, [sa, sb, sc], [pa, pb, pc], [oc, ob, oa])) in corners.iter().enumerate() {
        if n > 0 {
            products(m, [0x5c, 0x4c, 0x24]);
        }
        let q = r(m, angle).sin() * 0.03;
        m.set_f32(f + sa, (q * g(m, 0x702648)) as f32);
        m.set_f32(f + sb, (q * g(m, 0x702644)) as f32);
        let q2 = q * g(m, 0x702640);
        let v = (g(m, tbl + hi) - r(m, sa)) as f32;
        m.set_f32(f + sa, v);
        let v = (g(m, tbl + hi - 4) - r(m, sb)) as f32;
        m.set_f32(f + sb, v);
        let rr = g(m, tbl + hi - 8) - q2;
        let v = (r(m, sa) + r(m, 0x8c)) as f32;
        m.set_f32(f + sa, v);
        let v = (r(m, sb) + r(m, 0x44)) as f32;
        m.set_f32(f + sb, v);
        m.set_f32(f + sc, (rr + k0) as f32);
        let a = r(m, sa) + r(m, pa);
        let b = r(m, sb) + r(m, pb);
        let c = r(m, sc) + r(m, pc);
        m.set_f32(f + oc, c as f32);
        m.set_f32(f + ob, b as f32);
        m.set_f32(f + oa, a as f32);
    }
    let a = r(m, 0x40);
    m.set_f32(f + 0x80, (a * g(m, 0x6974a8)) as f32);
    m.set_f32(f + 0x84, (a * g(m, 0x6974ac)) as f32);
    m.set_f32(f + 0x88, (a * g(m, 0x6974b0)) as f32);
    let (px, py, pz) = (m.f32(f + 0x50), m.f32(f + 0x54), m.f32(f + 0x58));
    crate::matrix::mat_translation(m, f + 0x168, px, py, pz);
}
#[cfg(test)]
mod scene_light_ray_points_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_light_ray_points() {
        let (scene, world, clock, f) = (0x2a01_0000u32, 0x2a04_0000u32, 0x2a05_0000u32, 0x2a06_0000u32);
        let (mut exact, mut total) = (0, 0);
        for (n, line) in include_str!("../../../tools/emu/golden_scene_light_ray_points.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.set_u32(crate::globals::CLOCK_PTR, clock);
            m.write(clock + 0xb98, &bytes(v[0]));
            m.write(crate::globals::FIXED_STEP_MS, &bytes(&v[1][..4]));
            m.write(crate::globals::FIXED_TIME, &bytes(&v[1][4..]));
            m.write(scene + 0x501c, &bytes(v[2]));
            m.write(scene + 0x204, &bytes(v[3]));
            let idx: u32 = v[4].parse().unwrap();
            m.set_u32(scene + 0x28b4, idx);
            m.write(0x6d2524 + 0x5d0 * idx, &bytes(v[5]));
            m.set_u32(0x6d2078, world);
            m.write(world + 0x5b4, &bytes(v[6]));
            m.write(0x6974a8, &bytes(v[7]));
            m.write(f, &bytes(v[8]));
            crate::globals::install_constants(&mut m);
            let (head, take) = scene_light_ray_setup(&mut m, scene, f);
            assert!(!take, "case {n}");
            scene_light_ray_points(&mut m, scene, f, head);
            let want = bytes(v[11]);
            for i in 0..want.len() / 4 {
                let (g, w) = (m.u32(f + 4 * i as u32), u32::from_le_bytes(want[i * 4..i * 4 + 4].try_into().unwrap()));
                total += 1;
                exact += (g == w) as u32;
                assert!(g == w || (g as i64 - w as i64).abs() <= 2 || (f32::from_bits(g) - f32::from_bits(w)).abs() <= 1e-4, "case {n} frame +{:#x}: {:?} vs {:?}", 4 * i, f32::from_bits(g), f32::from_bits(w));
            }
        }
        eprintln!("exact {exact} of {total}");
        assert!(exact * 100 >= total * 98);
    }
}

/// `0x550d77..0x550f4c` of `0x54ff80` (continues `scene_light_ray_points`): for each of the three corner points
/// a translation to the corner and a scaling by `f+0x40`, `f+0x2c`, `f+0x28` times the vector at `0x6974a8` are
/// multiplied, `scale * translation`, and then by the world transform `f+0xc0` (`scene_light_ray_setup`):
/// the results are `f+0x264`, `f+0x294`, `f+0x2c4`. The frame pairs `f+0x2c = f+0x90`, `f+0x44 = f+0x264` and
/// the counter `f+0x8c = 3` that the original sets before the object-creation loop are written too.
pub fn scene_light_ray_matrices(m: &mut Mem, f: u32) {
    use crate::matrix::{mat_mul, mat_scale, mat_translation};
    let v = |m: &Mem, o: u32| m.f32(f + o);
    let (t, s, p, w) = (f + 0x168, f + 0x198, f + 0x138, f + 0xc0);
    // corner 1: the translation (f+0x168) and the scaling vector (f+0x80..) are already in place
    mat_scale(m, s, v(m, 0x80), v(m, 0x84), v(m, 0x88));
    mat_mul(m, p, s, t);
    mat_mul(m, f + 0x264, p, w);
    // corner 2
    let sc = {
        let x = m.f32(f + 0x2c) as f64;
        [(x * m.f32(0x6974b0) as f64) as f32, (x * m.f32(0x6974ac) as f64) as f32, (x * m.f32(0x6974a8) as f64) as f32]
    };
    m.set_f32(f + 0x50, sc[2]);
    m.set_f32(f + 0x54, sc[1]);
    m.set_f32(f + 0x58, sc[0]);
    mat_translation(m, p, v(m, 0x68), v(m, 0x6c), v(m, 0x70));
    mat_scale(m, s, v(m, 0x50), v(m, 0x54), v(m, 0x58));
    mat_mul(m, t, s, p);
    mat_mul(m, f + 0x294, t, w);
    // corner 3
    let sc = {
        let x = m.f32(f + 0x28) as f64;
        [(x * m.f32(0x6974b0) as f64) as f32, (x * m.f32(0x6974ac) as f64) as f32, (x * m.f32(0x6974a8) as f64) as f32]
    };
    m.set_f32(f + 0x68, sc[2]);
    m.set_f32(f + 0x6c, sc[1]);
    m.set_f32(f + 0x70, sc[0]);
    mat_translation(m, p, v(m, 0x34), v(m, 0x38), v(m, 0x3c));
    mat_scale(m, s, v(m, 0x68), v(m, 0x6c), v(m, 0x70));
    mat_mul(m, t, s, p);
    mat_mul(m, f + 0x2c4, t, w);
    m.set_u32(f + 0x2c, f + 0x90);
    m.set_u32(f + 0x44, f + 0x264);
    m.set_u32(f + 0x8c, 3);
}
#[cfg(test)]
mod scene_light_ray_matrices_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_light_ray_matrices() {
        let (scene, world, clock) = (0x2a01_0000u32, 0x2a04_0000u32, 0x2a05_0000u32);
        let (mut exact, mut total) = (0, 0);
        for (n, line) in include_str!("../../../tools/emu/golden_scene_light_ray_matrices.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.set_u32(crate::globals::CLOCK_PTR, clock);
            m.write(clock + 0xb98, &bytes(v[0]));
            m.write(crate::globals::FIXED_STEP_MS, &bytes(&v[1][..4]));
            m.write(crate::globals::FIXED_TIME, &bytes(&v[1][4..]));
            m.write(scene + 0x501c, &bytes(v[2]));
            m.write(scene + 0x204, &bytes(v[3]));
            let idx: u32 = v[4].parse().unwrap();
            m.set_u32(scene + 0x28b4, idx);
            m.write(0x6d2524 + 0x5d0 * idx, &bytes(v[5]));
            m.set_u32(0x6d2078, world);
            m.write(world + 0x5b4, &bytes(v[6]));
            m.write(0x6974a8, &bytes(v[7]));
            let f = u32::from_str_radix(v[10].trim_start_matches("0x"), 16).unwrap();
            m.write(f, &bytes(v[8]));
            crate::globals::install_constants(&mut m);
            let (head, take) = scene_light_ray_setup(&mut m, scene, f);
            assert!(!take, "case {n}");
            scene_light_ray_points(&mut m, scene, f, head);
            scene_light_ray_matrices(&mut m, f);
            let want = bytes(v[11]);
            for i in 0..want.len() / 4 {
                let (g, w) = (m.u32(f + 4 * i as u32), u32::from_le_bytes(want[i * 4..i * 4 + 4].try_into().unwrap()));
                total += 1;
                exact += (g == w) as u32;
                assert!(g == w || (g as i64 - w as i64).abs() <= 2 || (f32::from_bits(g) - f32::from_bits(w)).abs() <= 1e-4, "case {n} frame +{:#x}: {:?} vs {:?}", 4 * i, f32::from_bits(g), f32::from_bits(w));
            }
        }
        eprintln!("exact {exact} of {total}");
        assert!(exact * 100 >= total * 98);
    }
}

/// `0x551094..0x551474` of `0x54ff80`: the second set of rays, entered with the frame left by
/// `scene_light_ray_setup` (`f+0xc0` world transform, `f+0xf0` its inverse, the cosine terms and angles) and an
/// empty FPU stack. It repeats `scene_light_ray_setup`/`scene_light_ray_points` for the record vector at
/// `0x6d2554 + 0x5d0 * [scene+0x28b4]`: the point carried through `f+0xc0` and its offset to the camera point
/// of `[0x6d2078]` (`f+0x34..`, copied to `f+0x68..`, unit vector at `f+0xa8`), the dot products with `f+0xf0`
/// (`f+0x14`, `f+0x28`; the first one, `A`, stays unrounded on the FPU stack and is used later), the heading
/// `-atan2(A, f+0x14)`, and, when the heading is outside [1.0472, 2.0944], the scaled blend (`f+0x40`, `f+0x44`,
/// `f+0x20`), `k = +-0.2` terms (`f+0x78`, `f+0x74`, `f+0x2c`, now rounded to `f32`) and two corner points
/// (`f+0x68..`, `f+0x50..`; here the record vector is added, not subtracted) up to the first `0x4bdef0` (`vec_set`)
/// call, which stores `(W*A, W*f+0x14, W*f+0x28)` at `f+0x80`. Returns `true` when the heading is in range and the
/// original jumps to `0x551860`. `sin/cos/atan2` are stand-ins.
pub fn scene_light_ray2_points(m: &mut Mem, scene: u32, f: u32) -> bool {
    let r = |m: &Mem, o: u32| m.f32(f + o) as f64;
    let g = |m: &Mem, a: u32| m.f32(a) as f64;
    let tb = 0x6d2554 + 0x5d0 * m.u32(scene + 0x28b4);
    let (tx, ty, tz) = (g(m, tb), g(m, tb + 4), g(m, tb + 8));
    let x = r(m, 0xd8) * tz + r(m, 0xcc) * ty + r(m, 0xc0) * tx + r(m, 0xe4);
    m.set_f32(f + 0x18, x as f32);
    let y = r(m, 0xdc) * tz + r(m, 0xd0) * ty + r(m, 0xc4) * tx + r(m, 0xe8);
    m.set_f32(f + 0x10, y as f32);
    let z = r(m, 0xe0) * tz + r(m, 0xd4) * ty + r(m, 0xc8) * tx + r(m, 0xec);
    let w = m.u32(0x6d2078);
    let v3 = g(m, w + 0x5bc) - z;
    let v2 = g(m, w + 0x5b8) - r(m, 0x10);
    let v1 = g(m, w + 0x5b4) - r(m, 0x18);
    m.set_f32(f + 0x34, v1 as f32);
    m.set_f32(f + 0x38, v2 as f32);
    m.set_f32(f + 0x3c, v3 as f32);
    for (d, s) in [(0x68, 0x34), (0x6c, 0x38), (0x70, 0x3c)] {
        let b = m.u32(f + s);
        m.set_u32(f + d, b);
    }
    crate::matrix::vec_unit(m, f + 0xa8, f + 0x68);
    let a = r(m, 0xa8) * r(m, 0xf0) + r(m, 0xb0) * r(m, 0x108) + r(m, 0xac) * r(m, 0xfc);
    let b = r(m, 0xb0) * r(m, 0x10c) + r(m, 0xac) * r(m, 0x100) + r(m, 0xa8) * r(m, 0xf4);
    m.set_f32(f + 0x14, b as f32);
    let c = r(m, 0xb0) * r(m, 0x110) + r(m, 0xac) * r(m, 0x104) + r(m, 0xa8) * r(m, 0xf8);
    m.set_f32(f + 0x28, c as f32);
    let head = -a.atan2(r(m, 0x14));
    if head.abs() >= 1.0471975803375244 && head.abs() <= 2.094395160675049 {
        return true;
    }
    m.set_f32(f + 0x10, head.sin().abs() as f32);
    let d = if head.abs() < 1.0471975803375244 { head.cos() - r(m, 0x74) } else { r(m, 0x20) - head.cos() };
    let xx = d * r(m, 0x9c);
    m.set_f32(f + 0x40, (xx * r(m, 0x78)) as f32);
    m.set_f32(f + 0x44, (xx * r(m, 0x60)) as f32);
    m.set_f32(f + 0x20, (xx * r(m, 0x48)) as f32);
    let k: f64 = if m.f32(f + 0x14) > 0.0 { f64::from_bits(0x3fc999999999999a) } else { f64::from_bits(0xbfc999999999999a) };
    m.set_f32(f + 0x78, (k * g(m, 0x702628)) as f32);
    m.set_f32(f + 0x74, (k * g(m, 0x702624)) as f32);
    m.set_f32(f + 0x2c, (k * g(m, 0x702620)) as f32);
    let wv = r(m, 0x10) * 0.6000000238418579;
    m.set_f32(f + 0x5c, (r(m, 0x28) * wv) as f32);
    m.set_f32(f + 0x4c, (r(m, 0x14) * wv) as f32);
    m.set_f32(f + 0x24, (a * wv) as f32);
    let t0 = 0x6d2518 + 0x5d0 * m.u32(scene + 0x28b4);
    // (angle slot, record offset of the highest component, scratch slots a/b/c, scratch products for a/b/c, output slots)
    let corners: [(u32, u32, [u32; 3], [u32; 3]); 2] = [
        (0x7c, 0x38, [0x5c, 0x4c, 0x24], [0x68, 0x6c, 0x70]),
        (0x64, 0x44, [0x5c, 0x4c, 0x24], [0x50, 0x54, 0x58]),
    ];
    for (n, &(angle, hi, [pa, pb, pc], [oa, ob, oc])) in corners.iter().enumerate() {
        if n > 0 {
            m.set_f32(f + 0x5c, (r(m, 0x28) * wv) as f32);
            m.set_f32(f + 0x4c, (r(m, 0x14) * wv) as f32);
            m.set_f32(f + 0x24, (a * wv) as f32);
        }
        let q = r(m, angle).sin() * 0.03;
        m.set_f32(f + 0x10, (q * g(m, 0x702648)) as f32);
        m.set_f32(f + 0x18, (q * g(m, 0x702644)) as f32);
        let q2 = q * g(m, 0x702640);
        let v = (r(m, 0x10) + g(m, t0 + hi)) as f32;
        m.set_f32(f + 0x10, v);
        let v = (r(m, 0x18) + g(m, t0 + hi - 4)) as f32;
        m.set_f32(f + 0x18, v);
        let rr = q2 + g(m, t0 + hi - 8);
        let v = (r(m, 0x78) + r(m, 0x10)) as f32;
        m.set_f32(f + 0x10, v);
        let v = (r(m, 0x74) + r(m, 0x18)) as f32;
        m.set_f32(f + 0x18, v);
        let v = (r(m, 0x2c) + rr) as f32;
        m.set_f32(f + 0x30, v);
        let x1 = r(m, pa) + r(m, 0x10);
        let x2 = r(m, 0x18) + r(m, pb);
        let x3 = r(m, 0x30) + r(m, pc);
        m.set_f32(f + oa, x3 as f32);
        m.set_f32(f + ob, x2 as f32);
        m.set_f32(f + oc, x1 as f32);
        if n == 0 {
            // the products are rebuilt for the next corner
            m.set_f32(f + 0x5c, (r(m, 0x28) * wv) as f32);
            m.set_f32(f + 0x4c, (r(m, 0x14) * wv) as f32);
            m.set_f32(f + 0x24, (a * wv) as f32);
        }
    }
    let (px, py, pz) = (((a * wv) as f32), (wv * r(m, 0x14)) as f32, (wv * r(m, 0x28)) as f32);
    m.set_f32(f - 0x4, pz);
    m.set_f32(f - 0x8, py);
    m.set_f32(f - 0xc, px);
    crate::math::vec_set(m, f + 0x80, px, py, pz);
    false
}
#[cfg(test)]
mod scene_light_ray2_points_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_light_ray2_points() {
        let (scene, world) = (0x2a01_0000u32, 0x2a04_0000u32);
        let (mut exact, mut total) = (0, 0);
        for (n, line) in include_str!("../../../tools/emu/golden_scene_light_ray2_points.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            let idx: u32 = v[0].parse().unwrap();
            m.set_u32(scene + 0x28b4, idx);
            m.write(0x6d2548 + 0x5d0 * idx, &bytes(v[1]));
            m.set_u32(0x6d2078, world);
            m.write(world + 0x5b4, &bytes(v[2]));
            let f = u32::from_str_radix(v[4].trim_start_matches("0x"), 16).unwrap();
            m.write(f, &bytes(v[3]));
            crate::globals::install_constants(&mut m);
            assert!(!scene_light_ray2_points(&mut m, scene, f), "case {n}");
            let want = bytes(v[5]);
            for i in 0..want.len() / 4 {
                let off = i as i32 * 4 - 0x40;
                if off < -0xc {
                    continue;
                }
                let a = f.wrapping_add(off as u32);
                let (g, w) = (m.u32(a), u32::from_le_bytes(want[i * 4..i * 4 + 4].try_into().unwrap()));
                total += 1;
                exact += (g == w) as u32;
                assert!(g == w || (g as i64 - w as i64).abs() <= 2 || (f32::from_bits(g) - f32::from_bits(w)).abs() <= 1e-4, "case {n} frame {off:+#x}: {:?} vs {:?}", f32::from_bits(g), f32::from_bits(w));
            }
        }
        eprintln!("exact {exact} of {total}");
        assert!(exact * 100 >= total * 98);
    }
}

/// `0x551474..0x5517ac` of `0x54ff80` (continues `scene_light_ray2_points` after its first `vec_set`): the third
/// corner offset (`0.03 * sin(f+0x1c)` times the axes `0x702640..`, built at `f+0x12c`, plus the record vector
/// at `0x6d2560 + 0x5d0 * [scene+0x28b4]` into `f+0x120`), the three corner positions (`f+0x90`, `f+0x34`),
/// and the three matrices `scale * translation * f+0xc0` at `f+0x1d4`, `f+0x204` and `f+0x234` (the last one
/// scaled by `f+0x20` times the vector at `0x6974a8`, through `0x4bdd60`), with the scratch matrices at
/// `f+0x138..0x354` the original leaves behind. Ends with `f+0x20 = 3`, as the original sets the counter of the
/// object-creation loop that follows.
pub fn scene_light_ray2_matrices(m: &mut Mem, scene: u32, f: u32) {
    use crate::matrix::{mat_mul, mat_scale, mat_translation, vec_scale};
    use crate::math::vec_set;
    let v = |m: &Mem, o: u32| m.f32(f + o);
    let g = |m: &Mem, a: u32| m.f32(a);
    let sum = |a: f32, b: f32| (a as f64 + b as f64) as f32;
    let prod = |a: f32, b: f32| (a as f64 * b as f64) as f32;
    let (s_w, t_w, p_w) = (f + 0xc0, f + 0x138, f + 0x168);
    // third corner
    let s = (v(m, 0x1c) as f64).sin();
    for (d, s2) in [(0x34, 0x80), (0x38, 0x84), (0x3c, 0x88)] {
        let b = m.u32(f + s2);
        m.set_u32(f + d, b);
    }
    let q = s * 0.03;
    vec_set(m, f + 0x12c, (q * g(m, 0x702640) as f64) as f32, (q * g(m, 0x702644) as f64) as f32, (q * g(m, 0x702648) as f64) as f32);
    let tb = 0x6d2560 + 0x5d0 * m.u32(scene + 0x28b4);
    let (x, y, z) = (sum(v(m, 0x12c), g(m, tb)), sum(v(m, 0x130), g(m, tb + 4)), sum(v(m, 0x134), g(m, tb + 8)));
    vec_set(m, f + 0x120, x, y, z);
    let (x, y, z) = (sum(v(m, 0x2c), v(m, 0x120)), sum(v(m, 0x74), v(m, 0x124)), sum(v(m, 0x78), v(m, 0x128)));
    vec_set(m, f + 0x90, x, y, z);
    let (x, y, z) = (sum(v(m, 0x90), v(m, 0x34)), sum(v(m, 0x94), v(m, 0x38)), sum(v(m, 0x98), v(m, 0x3c)));
    vec_set(m, f + 0x34, x, y, z);
    // corner 1
    let f40 = v(m, 0x40);
    vec_set(m, f + 0x90, prod(f40, g(m, 0x6974a8)), prod(f40, g(m, 0x6974ac)), prod(f40, g(m, 0x6974b0)));
    for (corner, dst, p) in [(0x68u32, 0x1d4u32, 0x40u32), (0x50, 0x204, 0x44)] {
        if p == 0x44 {
            let f44 = v(m, 0x44);
            vec_set(m, f + 0x90, prod(f44, g(m, 0x6974a8)), prod(f44, g(m, 0x6974ac)), prod(f44, g(m, 0x6974b0)));
        }
        mat_translation(m, t_w, v(m, corner), v(m, corner + 4), v(m, corner + 8));
        mat_scale(m, f + 0x198, v(m, 0x90), v(m, 0x94), v(m, 0x98));
        mat_mul(m, p_w, f + 0x198, t_w);
        mat_mul(m, f + dst, p_w, s_w);
    }
    // corner 3
    mat_translation(m, f + 0x2f4, v(m, 0x34), v(m, 0x38), v(m, 0x3c));
    vec_scale(m, f + 0x1c8, 0x6974a8, v(m, 0x20));
    mat_scale(m, f + 0x324, v(m, 0x1c8), v(m, 0x1cc), v(m, 0x1d0));
    mat_mul(m, f + 0x354, f + 0x324, f + 0x2f4);
    mat_mul(m, f + 0x234, f + 0x354, s_w);
    m.set_u32(f + 0x20, 3);
}
#[cfg(test)]
mod scene_light_ray2_matrices_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_light_ray2_matrices() {
        let scene = 0x2a01_0000u32;
        let (mut exact, mut total) = (0, 0);
        for (n, line) in include_str!("../../../tools/emu/golden_scene_light_ray2_matrices.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            let idx: u32 = v[0].parse().unwrap();
            m.set_u32(scene + 0x28b4, idx);
            m.write(0x6d2560 + 0x5d0 * idx, &bytes(v[1]));
            m.write(0x6974a8, &bytes(v[2]));
            let f = u32::from_str_radix(v[4].trim_start_matches("0x"), 16).unwrap();
            m.write(f, &bytes(v[3]));
            crate::globals::install_constants(&mut m);
            scene_light_ray2_matrices(&mut m, scene, f);
            let want = bytes(v[5]);
            for i in 0..want.len() / 4 {
                let off = i as i32 * 4 - 0x40;
                if off < 0 {
                    continue;
                }
                let a = f.wrapping_add(off as u32);
                let (g, w) = (m.u32(a), u32::from_le_bytes(want[i * 4..i * 4 + 4].try_into().unwrap()));
                total += 1;
                exact += (g == w) as u32;
                assert!(g == w || (g as i64 - w as i64).abs() <= 2 || (f32::from_bits(g) - f32::from_bits(w)).abs() <= 1e-4, "case {n} frame {off:+#x}: {:?} vs {:?}", f32::from_bits(g), f32::from_bits(w));
            }
        }
        eprintln!("exact {exact} of {total}");
        assert!(exact * 100 >= total * 98);
    }
}

/// `0x542250(x, lim)`: `x` clamped to `-lim..=lim` (NaN gives `-lim`, as the compare order of the original does).
fn clamp_542250(x: f32, lim: f32) -> f64 {
    let (x, lim) = (x as f64, lim as f64);
    if !(x >= -lim) {
        -lim
    } else if x <= lim {
        x
    } else {
        lim
    }
}

/// `__ftol` (`0x61d830`): truncation through `fistp qword`; out of range and NaN give the low half of the
/// "integer indefinite" `0x8000000000000000`.
pub(crate) fn ftol_61d830(v: f64) -> u32 {
    if v.is_nan() || v.abs() >= 9.223372036854775808e18 {
        0
    } else {
        (v as i64) as u32
    }
}

/// `FUN_00541e80` (`__fastcall`, `ecx = packet`): `NetVehIn::put`, packs the vehicle input record into the
/// byte buffer at `+0x1c`/`+0x14`: `[+0x14]`/`[+0x16]` get words copied from `[+8]`/`[+0]`, the flag word
/// `[+0x50]` gets the clamped byte `[+0x54]` shifted by 7 (also copied to `[+0x18]`), then follow the bytes of the
/// control state (gear/steer bytes, one angle (10 bits) and one amount (6 bits) per wheel-like entry, flags,
/// four angle differences, four byte pairs of 127-scaled values, a nibble pair) and, when `[+0x50] & 1`,
/// the pedal section (100- and 255-scaled bytes, a 16-bit value, engine state byte, two more bytes). Returns
/// `false` where the original logs a size or range error (`0x4e260c`). `f2pi` is the angle wrap `0x4e41d9`.
pub fn net_veh_in_put(m: &mut Mem, p: u32) -> bool {
    let mut ok = true;
    let w = m.u16(p + 8);
    m.set_u16(p + 0x14, w);
    let w = m.u16(p);
    m.set_u16(p + 0x16, w);
    if m.rd8(p + 0x54) > 7 {
        m.wr8(p + 0x54, 7);
    }
    let flags = m.u32(p + 0x50) | ((m.rd8(p + 0x54) as u32) << 7);
    m.set_u32(p + 0x50, flags);
    m.set_u32(p + 0x18, flags);
    let kind = m.u32(p + 0x5c);
    let mut bl = if kind <= 4 { kind as u8 } else { 0 };
    let lean = ftol_61d830(m.f32(p + 0x58) as f64 * -0.25) as u8;
    bl = bl.wrapping_sub(lean << 3);
    m.wr8(p + 0x1c, bl);
    let b = m.rd8(p + 0x60);
    m.wr8(p + 0x1d, b);
    let count = m.i32(p + 0x10);
    let mut at = 10u32;
    let byte = |m: &mut Mem, at: u32, v: u8| m.wr8(p + 0x14 + at, v);
    for i in 0..count.max(0) as u32 {
        let b = m.rd8(p + 0x61 + i);
        byte(m, at, b);
        at += 1;
        let ang = crate::chassis::f2pi(m.f32(p + 0x6c + 4 * i));
        let c = clamp_542250(ang, f32::from_bits(0x40490fdb));
        let v = ftol_61d830(c * 2047.0 * f64::from_bits(0x3fd45f306dc9c883)) & 0x3ff;
        byte(m, at, v as u8);
        at += 1;
        byte(m, at, (v >> 8) as u8);
        let c = clamp_542250(m.f32(p + 0x8c + 4 * i), f32::from_bits(0x3f333333));
        let v = ftol_61d830(c * 10.0) as u8;
        let cur = m.rd8(p + 0x14 + at);
        byte(m, at, cur | (v << 2));
        at += 1;
    }
    let (b1, b0, bf, be) = (m.rd8(p + 0xc1), m.rd8(p + 0xc0), m.rd8(p + 0xbf), m.rd8(p + 0xbe));
    let v = ((((b1 << 1) | b0) << 1 | bf) << 1) | be;
    byte(m, at, v);
    at += 1;
    for k in 0..4u32 {
        let d = (m.f32(p + 0xc4) as f64 - m.f32(p + 0xd0 + 12 * k) as f64) as f32;
        let c = clamp_542250(crate::chassis::f2pi(d), f32::from_bits(0x40490fd0));
        byte(m, at, ftol_61d830(c * f64::from_bits(0x40544ad25cec601a)) as u8);
        at += 1;
    }
    for k in 0..4u32 {
        for o in [0xf8u32, 0xfc] {
            let c = clamp_542250(m.f32(p + o + 12 * k), 1.0);
            byte(m, at, ftol_61d830(c * 127.0) as u8);
            at += 1;
        }
    }
    let v = (m.rd8(p + 0xbc) << 4).wrapping_add(m.rd8(p + 0xbd));
    byte(m, at, v);
    at += 1;
    if m.rd8(p + 0x50) & 1 == 0 {
        m.set_u16(p + 0x14, at as u16);
        return at as i32 == m.i32(p + 4);
    }
    let mut put = |m: &mut Mem, at: &mut u32, v: u8| {
        byte(m, *at, v);
        *at += 1;
    };
    let k255 = 255.0;
    let c = clamp_542250(m.f32(p + 0xb0), 100.0);
    put(m, &mut at, ftol_61d830(c * f64::from_bits(0x4004666666666666)) as u8);
    let v = ftol_61d830(m.f32(p + 0xac) as f64 * f32::from_bits(0x3c23d70a) as f64) as u16;
    m.set_u16(p + 0x14 + at, v);
    at += 2;
    let c = clamp_542250(m.f32(p + 0xb4), 100.0);
    put(m, &mut at, ftol_61d830(c * f64::from_bits(0x4004666666666666)) as u8);
    let c = clamp_542250(m.f32(p + 0xb8), 1.0);
    put(m, &mut at, ftol_61d830(c * k255) as u8);
    put(m, &mut at, ftol_61d830(m.f32(p + 0x128) as f64 * k255) as u8);
    let engine = m.i32(p + 0x134);
    if !(0..=7).contains(&engine) {
        ok = false;
    }
    let mut v = (m.rd8(p + 0x13c) << 4) | m.rd8(p + 0x134);
    v <<= 2;
    v |= if m.u32(p + 0x138) != 0 { 0x20 } else { 0 };
    v |= if m.u32(p + 0x130) != 0 { 2 } else { 0 };
    v |= (m.u32(p + 0x12c) != 0) as u8;
    put(m, &mut at, v);
    put(m, &mut at, (m.u32(p + 0x140) != 0) as u8);
    let c = clamp_542250(m.f32(p + 0x144), 1.0);
    put(m, &mut at, ftol_61d830(c * k255) as u8);
    let c = clamp_542250(m.f32(p + 0x148), 1.0);
    put(m, &mut at, ftol_61d830(c * k255) as u8);
    ok && at <= 0x3a && at as i32 == m.i32(p + 8)
}
#[cfg(test)]
mod net_veh_in_put_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_net_veh_in_put() {
        let p = 0x2a01_0000u32;
        let (mut exact, mut total, mut clean) = (0, 0, 0);
        for (n, line) in include_str!("../../../tools/emu/golden_net_veh_in_put.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.write(p, &bytes(v[0]));
            let ok = net_veh_in_put(&mut m, p);
            assert_eq!(ok, v[2] == "0", "case {n}: error report");
            clean += ok as u32;
            let want = bytes(v[1]);
            for (i, w) in want.iter().enumerate() {
                let g = m.rd8(p + i as u32);
                total += 1;
                exact += (g == *w) as u32;
                assert!(g == *w, "case {n} byte {i:#x}: {g:#x} vs {w:#x}");
            }
        }
        eprintln!("exact {exact} of {total}, {clean} clean");
        assert!(clean > 100);
    }
}

/// `0x55186d..0x551895` of `0x54ff80`: once `[scene+0x5090] + [scene+0x508c]` (kept in extended precision) is
/// below the game clock `[[0x6ef1fc]+0xb98]` (`f64`), `[scene+0x5080]` becomes 1.
pub fn scene_light_timeout(m: &mut Mem, scene: u32) {
    let sum = m.f32(scene + 0x5090) as f64 + m.f32(scene + 0x508c) as f64;
    if sum < m.f64(m.u32(0x6ef1fc) + 0xb98) {
        m.set_u32(scene + 0x5080, 1);
    }
}
#[cfg(test)]
mod scene_light_timeout_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_light_timeout() {
        let (scene, clock) = (0x2a01_0000u32, 0x2a04_0000u32);
        for (n, line) in include_str!("../../../tools/emu/golden_scene_light_timeout.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.write(scene + 0x508c, &bytes(v[0]));
            m.set_u32(scene + 0x5080, v[2].parse().unwrap());
            m.set_u32(0x6ef1fc, clock);
            m.write(clock + 0xb98, &bytes(v[1]));
            scene_light_timeout(&mut m, scene);
            assert_eq!(m.u32(scene + 0x5080), v[3].parse::<u32>().unwrap(), "case {n}");
        }
    }
}

/// `0x5ef6d0` (stdcall `(name, kind)`, `ecx = node`): base scene node. The name is copied into a block from
/// `allocate` (`0x632643`, size = length + 1); an empty or null name leaves `[+0xc] = 0`.
pub fn scene_node_base(m: &mut Mem, node: u32, name: u32, kind: u32, allocate: &mut impl FnMut(&mut Mem, u32) -> u32) {
    for o in [4, 8, 0x14, 0x18, 0x1c, 0x20] {
        m.set_u32(node + o, 0);
    }
    m.set_u32(node, 0x652840);
    if name != 0 && m.rd8(name) != 0 {
        let mut n = 0;
        while m.rd8(name + n) != 0 {
            n += 1;
        }
        let p = allocate(m, n + 1);
        m.set_u32(node + 0xc, p);
        let bytes = m.read(name, n as usize + 1);
        m.write(p, &bytes);
    } else {
        m.set_u32(node + 0xc, 0);
    }
    m.wr8(node + 0x28, 0);
    m.set_u32(node + 0x10, kind);
    m.set_u32(node + 0x2c, 0);
    m.set_u32(node + 0x24, 0);
}

/// `0x5f19b0` (stdcall `(name, kind, count)`, `ecx = node`): a base node with a zeroed array of `count`
/// pointers at `[+0x30]` (`[+0x34] = count`), vtable `0x652cb4`.
pub fn scene_node_array(m: &mut Mem, node: u32, name: u32, kind: u32, count: i32, allocate: &mut impl FnMut(&mut Mem, u32) -> u32) {
    scene_node_base(m, node, name, kind, allocate);
    m.set_u32(node, 0x652cb4);
    m.set_i32(node + 0x34, count);
    let arr = if count != 0 { allocate(m, (count as u32).wrapping_mul(4)) } else { 0 };
    m.set_u32(node + 0x30, arr);
    for i in 0..m.i32(node + 0x34).max(0) as u32 {
        m.set_u32(arr + 4 * i, 0);
    }
}

/// `0x551920` (stdcall `(name, parent)`, `ecx = node`): a light node of kind 0x11 attached to `parent`
/// (`[+0x40]`), inheriting its `[+0x24]`; vtable `0x64c7dc`, `[+0x54] = -1`.
pub fn scene_light_node(m: &mut Mem, node: u32, name: u32, parent: u32, allocate: &mut impl FnMut(&mut Mem, u32) -> u32) {
    scene_node_base(m, node, name, 0x11, allocate);
    for o in [0x30, 0x34, 0x38, 0x3c] {
        m.set_u32(node + o, 0);
    }
    m.set_u32(node, 0x64c7dc);
    m.set_u32(node + 0x40, parent);
    for o in [0x44, 0x48, 0x4c, 0x50] {
        m.set_u32(node + o, 0);
    }
    m.set_u32(node + 0x54, 0xffff_ffff);
    let v = m.u32(parent + 0x24);
    m.set_u32(node + 0x24, v);
}

/// `0x5518d0` (`ecx = node`): the transform-carrying node: `scene_node_array(name 0x68d1c0, 6, 1)`, then
/// four zero vectors and the identity transform (`0x7025f0`) from `[+0x40]` on, vtable `0x64d7d0`,
/// `[+0x38] = [+0x3c] = 0`.
pub fn scene_transform_node(m: &mut Mem, node: u32, allocate: &mut impl FnMut(&mut Mem, u32) -> u32) {
    scene_node_array(m, node, 0x68d1c0, 6, 1, allocate);
    for i in 0..12 {
        m.set_u32(node + 0x40 + 4 * i, 0);
    }
    for i in 0..12 {
        let v = m.u32(0x7025f0 + 4 * i);
        m.set_u32(node + 0x40 + 4 * i, v);
    }
    m.set_u32(node, 0x64d7d0);
    m.set_u32(node + 0x38, 0);
    m.set_u32(node + 0x3c, 0);
}
#[cfg(test)]
mod light_ctors_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_light_ctors() {
        let (n, name_at, heap) = (0x2a01_0000u32, 0x2a00_1000u32, 0x2a80_0000u32);
        for (c, line) in include_str!("../../../tools/emu/golden_light_ctors.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            let mut next = heap;
            let mut alloc = |_: &mut Mem, size: u32| {
                let a = next;
                next += (size + 15) & !15;
                a
            };
            let name = bytes(v[1]);
            m.write(name_at, &name);
            m.wr8(name_at + name.len() as u32, 0);
            let nm = if name.is_empty() { 0 } else { name_at };
            m.write(n, &bytes(v[4]));
            m.write(0x68d1c0, &bytes(v[8]));
            m.wr8(0x68d1c0 + bytes(v[8]).len() as u32, 0);
            m.write(0x7025f0, &[0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            let (a1, a2): (u32, i32) = (v[2].parse().unwrap(), v[3].parse().unwrap());
            match v[0] {
                "base" => scene_node_base(&mut m, n, nm, a1, &mut alloc),
                "array" => scene_node_array(&mut m, n, nm, a1, a2, &mut alloc),
                "child" => scene_light_node(&mut m, n, nm, n + 0x80, &mut alloc),
                _ => scene_transform_node(&mut m, n, &mut alloc),
            }
            assert_eq!(m.read(n, 0x100), bytes(v[5]), "case {c} ({}): node", v[0]);
            assert_eq!(m.read(heap, bytes(v[6]).len()), bytes(v[6]), "case {c}: heap");
            assert_eq!(next - heap, v[7].parse::<u32>().unwrap(), "case {c}: allocated");
        }
    }
}

/// `0x5f2190` (thiscall `(matrix, mode)`, `ecx = node`): the node transform at `[+0x40]` is replaced by
/// `matrix` (mode 0), becomes `matrix * old` (mode 1, `0x5abc10`) or has `matrix` applied (mode 2,
/// `0x5abc00`); any other mode leaves it alone.
pub fn scene_node_set_transform(m: &mut Mem, node: u32, matrix: u32, mode: u32) {
    let t = node + 0x40;
    match mode {
        0 => {
            let b = m.read(matrix, 48);
            m.write(t, &b);
        }
        1 => {
            let tmp = 0x7fff_0000;
            crate::matrix::mat_mul(m, tmp, matrix, t);
            let b = m.read(tmp, 48);
            m.write(t, &b);
        }
        2 => crate::matrix::mat_apply(m, t, matrix),
        _ => {}
    }
}
#[cfg(test)]
mod node_transform_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_node_transform() {
        let (n, mat) = (0x2a01_0000u32, 0x2a01_1000u32);
        for (c, line) in include_str!("../../../tools/emu/golden_node_set_transform.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.write(n + 0x40, &bytes(v[1]));
            m.write(mat, &bytes(v[2]));
            scene_node_set_transform(&mut m, n, mat, v[0].parse().unwrap());
            assert_eq!(m.read(n + 0x40, 48), bytes(v[3]), "case {c} mode {}", v[0]);
        }
    }
}

/// What the light-creation loops of `0x54ff80` need from the scene: the heap (`0x632643`) and the virtual
/// registration `[vtable + 0x14](0, light)` on the object at `[0x69622c]`.
pub trait SceneLightOwner {
    fn allocate(&mut self, m: &mut Mem, size: u32) -> u32;
    fn register(&mut self, m: &mut Mem, registry: u32, light: u32);
}

/// `0x550f4c..0x551094` of `0x54ff80`: for each of `count` entries a light node (0x58 bytes, name `0x67334c`,
/// parent `[parent_global]` = `[0x6ef254]` (this loop) or `[0x6ef258]` (the one at `0x5517ac..0x551866`);
/// `scene_light_node`) is stored at `[array]`, given a transform node (0x70 bytes,
/// `scene_transform_node`) at `light+0x44` whose matrix is the 12 floats at `matrices` (mode 0 of
/// `scene_node_set_transform`) and registered with the scene; `array` advances by 4 and `matrices` by 0x30.
/// Allocation failure (a null block) is carried through as the original does (null node, no constructor).
pub fn scene_light_loop(m: &mut Mem, mut array: u32, mut matrices: u32, count: u32, parent_global: u32, owner: &mut impl SceneLightOwner) {
    for _ in 0..count {
        let raw = owner.allocate(m, 0x58);
        let light = if raw != 0 {
            let parent = m.u32(parent_global);
            scene_light_node(m, raw, 0x67334c, parent, &mut |m, n| owner.allocate(m, n));
            raw
        } else {
            0
        };
        m.set_u32(array, light);
        let raw = owner.allocate(m, 0x70);
        let tnode = if raw != 0 {
            scene_transform_node(m, raw, &mut |m, n| owner.allocate(m, n));
            raw
        } else {
            0
        };
        m.set_u32(light + 0x44, tnode);
        scene_node_set_transform(m, tnode, matrices, 0);
        let registry = m.u32(0x69622c);
        owner.register(m, registry, light);
        matrices += 0x30;
        array += 4;
    }
}
#[cfg(test)]
mod scene_light_loop_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    struct Bump {
        next: u32,
        calls: Vec<(u32, u32)>,
    }
    impl SceneLightOwner for Bump {
        fn allocate(&mut self, _: &mut Mem, size: u32) -> u32 {
            let a = self.next;
            self.next += (size + 15) & !15;
            a
        }
        fn register(&mut self, _: &mut Mem, registry: u32, light: u32) {
            self.calls.push((registry, light));
        }
    }
    fn check(golden: &str, global: u32) {
        let (parent, reg, arr, mats, heap) = (0x2a02_0000u32, 0x2a02_1000u32, 0x2a02_3000u32, 0x2a02_4000u32, 0x2a80_0000u32);
        for (c, line) in golden.lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.set_u32(global, parent);
            m.set_u32(0x69622c, reg);
            m.set_u32(parent + 0x24, u32::from_str_radix(v[1], 16).unwrap());
            m.write(mats, &bytes(v[2]));
            for (at, s) in [(0x67334cu32, v[8]), (0x68d1c0, v[9])] {
                m.write(at, &bytes(s));
                m.wr8(at + bytes(s).len() as u32, 0);
            }
            m.write(0x7025f0, &[0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            let mut owner = Bump { next: heap, calls: vec![] };
            scene_light_loop(&mut m, arr, mats, v[0].parse().unwrap(), global, &mut owner);
            assert_eq!(m.read(arr, 16), bytes(v[3]), "case {c}: array");
            let used = v[5].parse::<u32>().unwrap();
            assert_eq!(owner.next - heap, used, "case {c}: allocated");
            assert_eq!(m.read(heap, used as usize), bytes(v[4]), "case {c}: heap");
            let calls: Vec<String> = owner.calls.iter().map(|(r, l)| format!("{r:x}/0/{l:x}")).collect();
            assert_eq!(calls.join(","), v[7], "case {c}: registration");
        }
    }
    #[test]
    fn native_scene_light_loop() {
        check(include_str!("../../../tools/emu/golden_scene_light_loop.tsv"), 0x6ef254);
    }
    #[test]
    fn native_scene_light_loop2() {
        check(include_str!("../../../tools/emu/golden_scene_light_loop2.tsv"), 0x6ef258);
    }
}

/// Walk the intrusive list whose head link is `head` (a pointer to `element + 4`; the next link is at
/// `element + 8`, also pointing to `next + 4`): the first element for which `hit(element)` holds, or 0.
fn find_in_list(m: &Mem, head: u32, hit: impl Fn(u32) -> bool) -> u32 {
    if head == 0 {
        return 0;
    }
    let mut cur = head.wrapping_sub(4);
    while cur != 0 {
        if hit(cur) {
            return cur;
        }
        let next = m.u32(cur + 8);
        if next == 0 {
            return 0;
        }
        cur = next.wrapping_sub(4);
    }
    0
}

/// `0x57c9e0` (via `0x57c9a0` and `0x57c9b0`): the object `id` if it is in the list at `[0x6f653c]` (items are
/// stored `0x178` above the object, the id is the object address), else 0.
pub fn world_object_by_id(m: &Mem, id: u32) -> u32 {
    if id == 0 {
        return 0;
    }
    let head = m.u32(0x6f653c);
    let found = find_in_list(m, head, |e| e.wrapping_sub(0x178) == id);
    if found == 0 { 0 } else { found.wrapping_sub(0x178) }
}

/// `0x53f780` (via `0x53f750`): `p - 0x10` (or 0 for a null `p`) if it is an element of the list at `[0x6ef1f8]`, else 0.
pub fn world_list_find(m: &Mem, p: u32) -> u32 {
    let key = if p == 0 { 0 } else { p.wrapping_sub(0x10) };
    find_in_list(m, m.u32(0x6ef1f8), |e| e == key)
}
#[cfg(test)]
mod world_lookup_tests {
    use super::*;
    #[test]
    fn native_world_lookup() {
        for (c, line) in include_str!("../../../tools/emu/golden_world_lookup.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let hex = |s: &str| u32::from_str_radix(s, 16).unwrap();
            let elems: Vec<u32> = v[2].split(',').filter(|s| !s.is_empty()).map(hex).collect();
            let mut m = Mem::new();
            let head = if v[0] == "57c9e0" { 0x6f653c } else { 0x6ef1f8 };
            m.set_u32(head, hex(v[3]));
            for (i, a) in elems.iter().enumerate() {
                m.set_u32(a + 8, if i + 1 < elems.len() { elems[i + 1] + 4 } else { 0 });
            }
            let arg = hex(v[4]);
            let got = if v[0] == "57c9e0" { world_object_by_id(&m, arg) } else { world_list_find(&m, arg) };
            assert_eq!(got, hex(v[5]), "case {c} ({})", v[0]);
        }
    }
}

/// `0x57df70` (thiscall, `ecx = object`, one stack argument): copy the 12 floats (transform) at `object+0x224` to `dst`.
pub fn object_matrix_copy(m: &mut Mem, object: u32, dst: u32) {
    let b = m.read(object + 0x224, 48);
    m.write(dst, &b);
}
#[cfg(test)]
mod object_matrix_copy_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_object_matrix_copy() {
        let (o, b) = (0x2a01_0000u32, 0x2a01_2000u32);
        for (c, line) in include_str!("../../../tools/emu/golden_object_matrix_copy.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.write(o, &bytes(v[0]));
            object_matrix_copy(&mut m, o, b);
            assert_eq!(m.read(b, 0x40), bytes(v[1]), "case {c}");
        }
    }
}

/// `0x4018b0` (thiscall, one stack argument): append the C string `s` to the text buffer at `buf`
/// (`+0xc` data, `+8` capacity, `+0x10` length); stops when the buffer is full. Without data nothing happens.
pub fn text_buffer_append_str(m: &mut Mem, buf: u32, s: u32) {
    if m.u32(buf + 0xc) == 0 {
        return;
    }
    let mut p = s;
    while m.rd8(p) != 0 {
        let len = m.i32(buf + 0x10);
        if len >= m.i32(buf + 8) {
            break;
        }
        let data = m.u32(buf + 0xc);
        m.wr8(data.wrapping_add(len as u32), m.rd8(p));
        m.set_i32(buf + 0x10, len + 1);
        p += 1;
    }
}

/// `0x401870` (thiscall `(src, n)`): append `n` bytes to the text buffer if they fit strictly below the capacity.
pub fn text_buffer_append_bytes(m: &mut Mem, buf: u32, src: u32, n: i32) {
    let data = m.u32(buf + 0xc);
    if data == 0 {
        return;
    }
    let len = m.i32(buf + 0x10);
    if len.wrapping_add(n) >= m.i32(buf + 8) {
        return;
    }
    let bytes = m.read(src, n.max(0) as usize);
    m.write(data.wrapping_add(len as u32), &bytes);
    m.set_i32(buf + 0x10, len.wrapping_add(n));
}
#[cfg(test)]
mod text_buffer_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_text_buffer() {
        let (t, d, s) = (0x2a01_0000u32, 0x2a01_1000u32, 0x2a01_2000u32);
        for (c, line) in include_str!("../../../tools/emu/golden_text_buffer.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.set_u32(t + 8, v[1].parse().unwrap());
            m.set_u32(t + 0xc, if v[3] == "1" { d } else { 0 });
            m.set_u32(t + 0x10, v[2].parse().unwrap());
            m.write(d, &bytes(v[4]));
            m.write(s, &bytes(v[5]));
            if v[0] == "4018b0" {
                text_buffer_append_str(&mut m, t, s);
            } else {
                text_buffer_append_bytes(&mut m, t, s, v[6].parse().unwrap());
            }
            assert_eq!(m.read(t, 0x14), bytes(v[7]), "case {c} ({}): buffer", v[0]);
            assert_eq!(m.read(d, 64), bytes(v[8]), "case {c} ({}): data", v[0]);
        }
    }
}

/// `0x5506d4..0x551866` of `0x54ff80` (the light-ray block, up to the call of `0x54f040`): glue of the pieces
/// above with the original's control flow. Exits early (`[scene+0x5028] = 0`) unless `[scene+0x51bc] != 0`, the
/// record `0x6d2518 + 0x5d0 * [scene+0x28b4]` has its first word set, both `[0x6ef254]` and `[0x6ef258]` are
/// set, and `scene_light_intensity > 0.4`. Otherwise `scene_light_ray_setup`; a heading outside [1.0472, 2.0944]
/// builds the first ray set (`points`, `matrices`, `scene_light_loop` on `[0x6ef254]`); then the second set
/// (`scene_light_ray2_points`; when its heading is in range the block ends, else `matrices` and the loop on
/// `[0x6ef258]`). `f` is the frame of the function.
pub fn scene_light_rays(m: &mut Mem, scene: u32, f: u32, owner: &mut impl SceneLightOwner) {
    let idx = m.u32(scene + 0x28b4);
    let gate = m.u32(scene + 0x51bc) != 0
        && m.u32(0x6d2518 + 0x5d0 * idx) != 0
        && m.u32(0x6ef254) != 0
        && m.u32(0x6ef258) != 0
        && scene_light_intensity(m, scene) > 0.4;
    if !gate {
        m.set_u32(scene + 0x5028, 0);
        return;
    }
    let (head, take) = scene_light_ray_setup(m, scene, f);
    if !take {
        scene_light_ray_points(m, scene, f, head);
        scene_light_ray_matrices(m, f);
        scene_light_loop(m, f + 0x90, f + 0x264, 3, 0x6ef254, owner);
    }
    if scene_light_ray2_points(m, scene, f) {
        return;
    }
    scene_light_ray2_matrices(m, scene, f);
    scene_light_loop(m, f + 0x90, f + 0x1d4, 3, 0x6ef258, owner);
}
#[cfg(test)]
mod scene_light_rays_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    struct Bump {
        next: u32,
        calls: Vec<(u32, u32)>,
    }
    impl SceneLightOwner for Bump {
        fn allocate(&mut self, _: &mut Mem, size: u32) -> u32 {
            let a = self.next;
            self.next += (size + 15) & !15;
            a
        }
        fn register(&mut self, _: &mut Mem, registry: u32, light: u32) {
            self.calls.push((registry, light));
        }
    }
    const AREAS: [(u32, u32); 6] = [(0x200, 0x2f8), (0x28b0, 0x28b8), (0x2540, 0x2548), (0x5000, 0x5100), (0x51b8, 0x51c0), (0x5440, 0x5460)];
    #[test]
    fn native_scene_light_rays() {
        let (scene, world, clock, rec, flag, p1, p2, reg, heap) =
            (0x2a01_0000u32, 0x2a04_0000u32, 0x2a05_0000u32, 0x2a03_0000u32, 0x2a03_8000u32, 0x2a06_0000u32, 0x2a06_1000u32, 0x2a06_2000u32, 0x2a80_0000u32);
        let (mut exact, mut total, mut registered) = (0, 0, 0);
        for (c, line) in include_str!("../../../tools/emu/golden_scene_light_rays.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let f = u32::from_str_radix(v[0].trim_start_matches("0x"), 16).unwrap();
            let idx: u32 = v[1].parse().unwrap();
            let mut m = Mem::new();
            m.write(f, &bytes(v[2]));
            let mut at = 0;
            let areas = bytes(v[3]);
            for (a, b) in AREAS {
                m.write(scene + a, &areas[at..at + (b - a) as usize]);
                at += (b - a) as usize;
            }
            m.write(0x6d2518 + 0x5d0 * idx, &bytes(v[4]));
            m.set_u32(0x6d2078, world);
            m.write(world + 0x5b4, &bytes(v[5]));
            let pv = bytes(v[6]);
            m.write(p1 + 0x24, &pv[..4]);
            m.write(p2 + 0x24, &pv[4..]);
            m.write(0x6974a8, &bytes(v[7]));
            m.write(rec, &bytes(v[8]));
            m.set_u32(scene + 0x2540, rec);
            m.set_u32(0x6974c4, clock);
            m.write(clock + 0xb98, &bytes(v[9]));
            m.write(flag + 0xc, &bytes(v[10]));
            m.set_u32(scene + 0x5440 + 0x14, flag);
            let ptrs = bytes(v[11]);
            m.set_u32(0x6ef254, if ptrs[0] != 0 { p1 } else { 0 });
            m.set_u32(0x6ef258, if ptrs[4] != 0 { p2 } else { 0 });
            m.set_u32(0x69622c, reg);
            m.write(0x7025f0, &[0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            m.write(0x702620, &[0, 0, 0, 0, 0, 0, 0x80, 0x3f, 0, 0, 0, 0]);
            m.write(0x702630, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x3f]);
            m.write(0x702640, &[0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0]);
            m.write(0x702650, &[0; 12]);
            for (at, s) in [(0x67334cu32, v[17]), (0x68d1c0, v[18])] {
                m.write(at, &bytes(s));
                m.wr8(at + bytes(s).len() as u32, 0);
            }
            let mut owner = Bump { next: heap, calls: vec![] };
            scene_light_rays(&mut m, scene, f, &mut owner);
            let used = v[15].parse::<u32>().unwrap();
            assert_eq!(owner.next - heap, used, "case {c}: allocated");
            assert_eq!(m.read(heap, used as usize), bytes(v[14]), "case {c}: heap");
            let got: Vec<String> = owner.calls.iter().map(|(r, l)| format!("{r:x}/0/{l:x}")).collect();
            assert_eq!(got.join(","), v[16], "case {c}: registrations");
            registered += !got.is_empty() as u32;
            let (mut at, after) = (0, bytes(v[12]));
            for (a, b) in AREAS {
                for k in 0..(b - a) {
                    assert_eq!(m.rd8(scene + a + k), after[at], "case {c}: scene +{:#x}", a + k);
                    at += 1;
                }
            }
            let want = bytes(v[13]);
            for i in 0..want.len() / 4 {
                let (g, w) = (m.u32(f + 4 * i as u32), u32::from_le_bytes(want[i * 4..i * 4 + 4].try_into().unwrap()));
                total += 1;
                exact += (g == w) as u32;
            }
        }
        eprintln!("frame words exact {exact} of {total}, {registered} cases with registrations");
        assert!(registered > 100);
    }
}

/// `0x58d170` (thiscall, one stack argument `out`): the next point of a vehicle's point list. The entries with a
/// non-null flag at `[this+0xf28+4*i]` (`i < [this+0xdf8]`) count; from the cursor `[this+0xe00]` on, the first
/// one is copied (12 bytes from `this+0xef8+12*i`) to `out` and the cursor moves past it. When none is left the
/// cursor is reset to 0 and the result is 0; otherwise the result is the new cursor.
pub fn vehicle_next_point(m: &mut Mem, this: u32, out: u32) -> u32 {
    let count = m.i32(this + 0xdf8);
    let mut i = m.i32(this + 0xe00);
    if i < count {
        loop {
            if m.u32(this.wrapping_add((i as u32).wrapping_mul(4)).wrapping_add(0xf28)) != 0 {
                let src = this.wrapping_add((i as u32).wrapping_mul(12)).wrapping_add(0xef8);
                let b = m.read(src, 12);
                m.write(out, &b);
                m.set_i32(this + 0xe00, i + 1);
                return (i + 1) as u32;
            }
            i += 1;
            m.set_i32(this + 0xe00, i);
            if i >= count {
                break;
            }
        }
    }
    m.set_u32(this + 0xe00, 0);
    0
}

/// `0x53f7b0`: `p - 0x10` for a vehicle of kind 2 (`[p+0x188] == 2` and `[p+0xdf0] == 2`), else 0.
pub fn vehicle_second_kind(m: &Mem, p: u32) -> u32 {
    if p != 0 && m.u32(p + 0x188) == 2 && m.u32(p + 0xdf0) == 2 {
        p.wrapping_sub(0x10)
    } else {
        0
    }
}

/// `0x57de90` (thiscall, `out`): the 12-float transform at `this+0x700` when `[this+0x5a0]` is set, else the one
/// at `this+0x1f4`, copied to `out`.
pub fn object_matrix_select(m: &mut Mem, this: u32, out: u32) {
    let src = if m.u32(this + 0x5a0) != 0 { this + 0x700 } else { this + 0x1f4 };
    let b = m.read(src, 48);
    m.write(out, &b);
}
#[cfg(test)]
mod small_leaves_tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_small_leaves() {
        let (o, d) = (0x2a01_0000u32, 0x2a01_4000u32);
        for (c, line) in include_str!("../../../tools/emu/golden_small_leaves.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            m.write(o, &bytes(v[1]));
            let arg = u32::from_str_radix(v[2], 16).unwrap();
            let res = match v[0] {
                "58d170" => vehicle_next_point(&mut m, o, d),
                "53f7b0" => vehicle_second_kind(&m, arg),
                _ => {
                    object_matrix_select(&mut m, o, d);
                    0
                }
            };
            if v[0] == "53f7b0" {
                assert_eq!(res, u32::from_str_radix(v[3], 16).unwrap(), "case {c}: result");
            }
            if v[0] == "58d170" {
                assert_eq!(res, u32::from_str_radix(v[3], 16).unwrap(), "case {c}: result");
            }
            assert_eq!(m.read(d, 0x40), bytes(v[4]), "case {c} ({}): out", v[0]);
            assert_eq!(m.read(o + 0xdf8, 12), bytes(v[5]), "case {c} ({}): state", v[0]);
        }
    }
}

/// `0x5d2500`: the bucket of a name in a name table: the sum of its bytes modulo 500 (0 for a null name).
pub fn name_table_bucket(m: &Mem, name: u32) -> u32 {
    if name == 0 {
        return 0;
    }
    let mut sum = 0u32;
    let mut a = name;
    while m.rd8(a) != 0 {
        sum = sum.wrapping_add(m.rd8(a) as u32);
        a += 1;
    }
    sum % 500
}

/// `0x5d2720` (thiscall, `table` = `ecx`): the value stored under `name` (bucket array at `table + 8`, chain link `+8`,
/// name pointer `+0xc`, value `+0x10`); `None` for a null name or a name not in the table.
pub fn name_table_find(m: &Mem, table: u32, name: u32) -> Option<u32> {
    if name == 0 {
        return None;
    }
    let mut node = m.u32(m.u32(table + 8) + 4 * name_table_bucket(m, name));
    while node != 0 {
        let (mut a, mut b) = (name, m.u32(node + 0xc));
        let same = loop {
            let (x, y) = (m.rd8(a), m.rd8(b));
            if x != y {
                break false;
            }
            if x == 0 {
                break true;
            }
            a += 1;
            b += 1;
        };
        if same {
            return Some(m.u32(node + 0x10));
        }
        node = m.u32(node + 8);
    }
    None
}

/// `0x5ef2b0` with a null owner (its `ecx` argument 0): the value of `name` in the global name table `0x71e820`, or 0.
/// With an owner the original calls its virtual `+0x20`; that branch is not ported.
pub fn global_name_lookup(m: &Mem, name: u32) -> u32 {
    name_table_find(m, 0x71e820, name).unwrap_or(0)
}

#[cfg(test)]
mod name_table_tests {
    use super::*;
    #[test]
    fn native_name_table() {
        for (c, line) in include_str!("../../../tools/emu/golden_name_table.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let hex = |s: &str| u32::from_str_radix(s, 16).unwrap();
            let mut m = Mem::new();
            let base = 0x0100_0000u32;
            let (table, strs, query) = (base, base + 0x20000, base + 0x30000);
            m.set_u32(0x71e828, table);
            let mut tails: std::collections::HashMap<u32, u32> = Default::default();
            for (i, rec) in v[0].split(',').filter(|s| !s.is_empty()).enumerate() {
                let f: Vec<&str> = rec.split(':').collect();
                let (node, name, val) = (base + 0x2000 + 0x20 * i as u32, f[1], hex(f[2]));
                let sp = strs + 0x40 * i as u32;
                m.write(sp, name.as_bytes());
                m.wr8(sp + name.len() as u32, 0);
                m.set_u32(node + 0xc, sp);
                m.set_u32(node + 0x10, val);
                let b = name.bytes().map(|c| c as u32).sum::<u32>() % 500;
                match tails.insert(b, node) {
                    Some(prev) => m.set_u32(prev + 8, node),
                    None => m.set_u32(table + 4 * b, node),
                }
            }
            m.write(query, v[1].as_bytes());
            m.wr8(query + v[1].len() as u32, 0);
            let arg = if hex(v[2]) != 0 { query } else { 0 };
            assert_eq!(global_name_lookup(&m, arg), hex(v[3]), "case {c} ({})", v[1]);
        }
    }
}

/// `0x54f040` (maneuver chooser, `ecx = scene`), from its start to the list lookup `0x54f2fc`: the gates and the
/// vectors it works on. `f` is the function's frame (`esp` after its pushes; the offsets are the original's `[esp + X]`).
/// Returns `false` where the original returns 0, `true` where it goes on (to `0x53f780` at `0x54f303`).
pub fn maneuver_prologue(m: &mut Mem, scene: u32, f: u32) -> bool {
    use crate::math::vec_len;
    let clock = m.f64(m.u32(0x6ef1fc) + 0xb98);
    // fcompp of the limit `[+0x5088]` against the time since `[+0x5084]`: C0 (limit below, or unordered) / not C0|C3 (limit above)
    let elapsed = |m: &Mem| clock - m.f32(scene + 0x5084) as f64;
    let expired = |m: &Mem| !(m.f32(scene + 0x5088) as f64 >= elapsed(m));
    let waiting = |m: &Mem| m.f32(scene + 0x5088) as f64 > elapsed(m);
    m.set_u32(scene + 0x5078, 0);
    if m.u32(scene + 0x507c) != 0 || expired(m) {
        m.set_u32(scene + 0x5074, 0);
        m.set_u32(scene + 0x5094, 0);
        m.set_u32(scene + 0x5098, 0);
    }
    m.set_u32(f + 0xb0, 0);
    m.set_u32(f + 0x30, 0);
    if m.u32(scene + 0x4c4) == 0 || m.u32(scene + 0x390) == 0 || m.i32(m.u32(0x6cec6c) + 0x514) > 1 || m.u32(scene + 0x184) != 0 {
        return false;
    }
    let obj = world_object_by_id(m, m.u32(scene + 0x414));
    m.set_u32(f + 0x9c, obj);
    if obj == 0 {
        return false;
    }
    if m.u32(scene + 0x507c) != 0 && waiting(m) {
        return false;
    }
    m.set_u32(scene + 0x507c, 0);
    m.set_u32(scene + 0x5088, 0x41200000);
    let (c0, c1, c2) = (m.f32(0x702620) as f64, m.f32(0x702624) as f64, m.f32(0x702628) as f64);
    for i in 0..3 {
        m.set_u32(f + 0x34 + 4 * i, m.u32(obj + 0x1c4 + 4 * i));
    }
    // the three projections of the scene's axes on (c0, c1, c2): rows 0x234/0x240/0x24c, 0x238/0x244/0x250, 0x23c/0x248/0x254
    for (k, (a, b, c)) in [(0x24c, 0x240, 0x234), (0x250, 0x244, 0x238), (0x254, 0x248, 0x23c)].into_iter().enumerate() {
        let v = (c2 * m.f32(scene + a) as f64 + c1 * m.f32(scene + b) as f64) + c0 * m.f32(scene + c) as f64;
        m.set_f32(f + 0x58 + 4 * k as u32, v as f32);
    }
    m.set_f32(f + 0xbc, vec_len(m, f + 0x34) as f32);
    object_matrix_copy(m, obj, f + 0x1a8);
    let mut d = [0f32; 3];
    for i in 0..3u32 {
        m.set_u32(f + 0x4c + 4 * i, m.u32(obj + 0x1a0 + 4 * i));
        d[i as usize] = (m.f32(f + 0x4c + 4 * i) as f64 - m.f32(f + 0x34 + 4 * i) as f64) as f32;
        m.set_f32(f + [0x84, 0x88, 0x8c][i as usize], d[i as usize]);
    }
    m.set_f32(f + 0x20, d[0]);
    m.set_f32(f + 0x24, d[1]);
    m.set_f32(f + 0x28, d[2]);
    let len = vec_len(m, f + 0x20);
    if len > 100.0 {
        return false;
    }
    if !(len >= 2.0) {
        return false;
    }
    let mut dot = 0.0f64;
    let mut terms = [0.0f64; 3];
    for i in 0..3u32 {
        let (p, q) = (m.f32(f + 0x34 + 4 * i) as f64, m.f32(scene + 0x1d4 + 4 * i) as f64);
        let (a, b) = (p + q, q - p);
        if i == 2 {
            m.set_f32(f + 0x28, b as f32);
        }
        terms[i as usize] = a * b;
    }
    dot += terms[2] + terms[1];
    dot += terms[0];
    if dot > 0.0 {
        m.set_u32(f + 0xb0, 6);
    }
    true
}

#[cfg(test)]
mod maneuver_prologue_tests {
    use super::*;
    #[test]
    fn native_maneuver_prologue() {
        for (c, line) in include_str!("../../../tools/emu/golden_maneuver_prologue.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let bytes = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() };
            // the emulator's addresses (tools/emu/gen_maneuver_prologue.py)
            let (scene, clock_obj, glb, objs_base, f) = (0x2a01_0000u32, 0x2a03_0000u32, 0x2a03_1000u32, 0x2a04_0000u32, 0x2a05_0000u32);
            let mut m = Mem::new();
            let mut at = 0;
            let sc = bytes(v[2]);
            for (a, b) in [(0x180u32, 0x264u32), (0x390, 0x398), (0x410, 0x418), (0x4c0, 0x4c8), (0x5070, 0x50a0)] {
                m.write(scene + a, &sc[at..at + (b - a) as usize]);
                at += (b - a) as usize;
            }
            m.set_u32(0x6ef1fc, clock_obj);
            m.write(clock_obj + 0xb98, &bytes(v[3]));
            m.write(0x702620, &bytes(v[4]));
            m.set_u32(0x6cec6c, glb);
            m.write(glb + 0x514, &bytes(v[6]));
            m.write(0x6f653c, &bytes(v[7]));
            let objs = bytes(v[8]);
            for i in 0..objs.len() / 0xf0 {
                m.write(objs_base + 0x1000 * (i as u32 + 1) + 0x170, &objs[0xf0 * i..0xf0 * (i + 1)]);
            }
            m.write(f, &bytes(v[1]));
            let go = maneuver_prologue(&mut m, scene, f);
            let want_stop: u32 = v[0].parse().unwrap();
            assert_eq!(go, want_stop == 0, "case {c} stop");
            assert_eq!(m.read(f, 0x200), bytes(v[10]), "case {c} frame");
            assert_eq!(m.read(scene + 0x5070, 0x30), bytes(v[11]), "case {c} scene");
        }
    }
}
