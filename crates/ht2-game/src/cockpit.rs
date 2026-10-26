//! Original CABINES.B3D geometry and instrument nodes. The pose still follows
//! the prototype's seat camera; cabin selection and indicator keyframes remain pending.
use glam::{Mat4,Vec3};
use ht2_formats::b3d::Node;
use ht2_render::{scene,Renderer,ObjectId};
use std::path::Path;

pub struct Cockpit {
    body:ObjectId,
    needles:Vec<(ObjectId,Mat4,usize)>,
    tank:f32,
}
fn remove(nodes:&mut Vec<Node>,names:&[String]) {
    nodes.retain(|n|!names.contains(&n.name));
    for n in nodes {remove(&mut n.children,names);}
}
fn transform(axes:[[f32;3];3],pos:[f32;3])->Mat4 {
    Mat4::from_cols(Vec3::from(axes[0]).extend(0.),Vec3::from(axes[1]).extend(0.),Vec3::from(axes[2]).extend(0.),Vec3::from(pos).extend(1.))
}
impl Cockpit {
    pub fn load(renderer:&mut Renderer,game:&Path,prefix:&str,fallback:Option<&str>,tank:f32)->Result<Self,String> {
        let (mut model,mut assets)=scene::load_assets(&game.join("COMMON/CABINES.B3D"))?;
        let prefix=select_prefix(prefix,fallback,|name|model.nodes.iter().any(|n|n.name==format!("{name}Cab")))
            .ok_or_else(||format!("no original cockpit {prefix}Cab or configured fallback"))?;
        let root=format!("{prefix}Cab");
        if !model.nodes.iter().any(|n|n.name==root){return Err(format!("no original cockpit {root}"));}
        let textures=renderer.upload_textures(&assets);
        let mut needles=Vec::new();let mut excluded=Vec::new();
        for (index,suffix) in ["Tacho","Speed","FuelArrow","Rule"].iter().enumerate(){
            let name=format!("{prefix}{suffix}");let space=format!("{name}Space");
            if let Some((axes,pos))=model.space(&space){
                let geometry=model.triangles_in(Some(&name));
                if !geometry.is_empty(){
                    let id=renderer.add_parts(textures,&assets,&[&geometry])[0];
                    needles.push((id,transform(axes,pos),index));excluded.push(name);
                }
            }
        }
        remove(&mut model.nodes,&excluded);
        assets.set_batches(model.triangles_in(Some(&root)));
        if assets.batches.is_empty(){return Err(format!("empty original cockpit {root}"));}
        let body=renderer.add_parts(textures,&assets,&[&assets.batches])[0];
        let result=Self{body,needles,tank};result.hide(renderer);Ok(result)
    }
    pub fn hide(&self,renderer:&mut Renderer){
        renderer.set_visible(self.body,false);
        for (id,_,_) in &self.needles {renderer.set_visible(*id,false);}
    }
    pub fn update(&self,renderer:&mut Renderer,mount:Mat4,speed_kmh:f32,rev_per_second:f32,fuel:f32,steering:f32){
        let mut memory=ht2_sim::mem::Mem::new();
        let bases=[0x2a000000,0x2a000040,0x2a000080];let out=[0x2a000100,0x2a000140,0x2a000180];
        for (_,base,index) in self.needles.iter().filter(|(_,_,i)|*i<3) {
            let values=[base.x_axis.truncate(),base.y_axis.truncate(),base.z_axis.truncate(),base.w_axis.truncate()];
            ht2_sim::matrix::put12(&mut memory,bases[*index],&std::array::from_fn(|i|values[i/3][i%3]));
        }
        ht2_sim::instruments::cabin_needles(&mut memory,bases,out,speed_kmh,rev_per_second,if self.tank>0. {fuel/self.tank}else{0.});
        renderer.set_visible(self.body,true);renderer.set_model(self.body,mount);
        for (id,base,index) in &self.needles {
            renderer.set_visible(*id,true);
            if *index==3 {
                let values=[base.x_axis.truncate(),base.y_axis.truncate(),base.z_axis.truncate(),base.w_axis.truncate()];
                ht2_sim::matrix::put12(&mut memory,0x2a000200,&std::array::from_fn(|i|values[i/3][i%3]));
                ht2_sim::instruments::cabin_steering(&mut memory,0x2a000200,0x2a000240,steering);
                let v:[f32;12]=std::array::from_fn(|i|memory.f32(0x2a000240+i as u32*4));
                renderer.set_model(*id,mount*transform([[v[0],v[1],v[2]],[v[3],v[4],v[5]],[v[6],v[7],v[8]]],[v[9],v[10],v[11]]));continue;
            }
            let v:[f32;12]=std::array::from_fn(|i|memory.f32(out[*index]+i as u32*4));
            let transformed=transform([[v[0],v[1],v[2]],[v[3],v[4],v[5]],[v[6],v[7],v[8]]],[v[9],v[10],v[11]]);
            // Type-4/36 geometry is in its named space's local coordinates.
            renderer.set_model(*id,mount * transformed);
        }
    }
}

fn select_prefix<'a>(primary:&'a str,fallback:Option<&'a str>,mut exists:impl FnMut(&str)->bool)->Option<&'a str> {
    if exists(primary){Some(primary)}else{fallback.filter(|name|exists(name))}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_cabin_fallback_lookups(){
        for line in include_str!("../../../tools/emu/golden_cabin_fallback.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let mut requests=Vec::new();
            let selected=select_prefix(p[0],Some(p[1]),|name|{requests.push(format!("{name}Cab"));name==p[0] && p[2]=="1" || name==p[1] && p[3]=="1"});
            assert_eq!(requests.join(","),p[4]);
            assert_eq!(selected.unwrap_or(""),p[5]);
        }
    }
}
