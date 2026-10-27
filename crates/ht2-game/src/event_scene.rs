//! B3D-to-runtime adapter for point events. Construction rejects node classes
//! whose references/state are not connected yet, instead of inventing traversal.
use ht2_formats::b3d::{Data,Node};
use ht2_sim::{mem::Mem,scene_events::{self,EventHooks}};
use std::collections::HashMap;

/// 0x5d27c0 tolerates unresolved engine-defined names; lookup clears the slot.
fn optional_name(name:&str)->bool {name.starts_with("$$")}
fn binding(definitions:&HashMap<String,u32>,name:&str)->Option<u32> {
    definitions.get(name).copied().or_else(||optional_name(name).then_some(0))
}

pub struct EventScene {
    memory:Mem,
    root:u32,
    next:u32,
    switches:HashMap<String,u32>,
    references:Vec<(u32,String)>,
    nodes:HashMap<String,u32>,
    links:Vec<(u32,String,String)>,
    groups:Vec<(u32,String)>,
}
impl EventScene {
    pub fn from_node(node:&Node)->Result<Self,String> {
        Self::from_nodes(std::slice::from_ref(node),&node.name)
    }
    /// Build all supplied definitions before resolving links. The selected root
    /// alone is visited by query; other roots can supply spaces and key owners.
    pub fn from_nodes(nodes:&[Node],root_name:&str)->Result<Self,String> {
        Self::from_roots(nodes.iter(),root_name)
    }
    /// COMMON contains shared key owners (including ILLUM_LEVEL_00..15).
    /// Load definitions without requiring renderable triangles or textures.
    pub fn from_map_files(game:&std::path::Path,files:&[ht2_formats::b3d::B3d],root_name:&str)->Result<Self,String> {
        let path=game.join("COMMON").join("COMMON.B3D");
        let common=ht2_formats::b3d::parse(&std::fs::read(&path).map_err(|e|format!("{}: {e}",path.display()))?).map_err(|e|e.to_string())?;
        Self::from_roots(common.nodes.iter().chain(files.iter().flat_map(|f|f.nodes.iter())),root_name)
    }
    fn from_roots<'a>(nodes:impl IntoIterator<Item=&'a Node>,root_name:&str)->Result<Self,String> {
        let mut out=Self{memory:Mem::new(),root:0,next:0x4200_0000,switches:HashMap::new(),references:Vec::new(),nodes:HashMap::new(),links:Vec::new(),groups:Vec::new()};
        for node in nodes {
            let address=out.add(node)?;
            if out.root==0 && node.name==root_name {out.root=address;}
        }
        if out.root==0 {return Err(format!("event root not found: {root_name}"));}
        for (address,name) in &out.references {
            let target=binding(&out.switches,name).ok_or_else(||format!("unresolved event switch reference: {name}"))?;
            out.memory.set_u32(*address+0x3c,target);
        }
        for (address,space,group) in &out.links {
            let target=if group.trim_matches(' ').is_empty(){0}else{binding(&out.nodes,group).ok_or_else(||format!("unresolved event group: {group}"))?};
            let transform=if space.trim_matches(' ').is_empty(){0}else{binding(&out.nodes,space).ok_or_else(||format!("unresolved event space: {space}"))?};
            if transform!=0 && out.memory.u32(transform+0x10)!=6 {return Err(format!("event link space is not a space: {space}"));}
            out.memory.set_u32(*address+0x40,target);out.memory.set_u32(*address+0x44,transform);
        }
        for (address,name) in &out.groups {
            if name.is_empty() {continue;}
            let space=binding(&out.nodes,name).ok_or_else(||format!("unresolved event group space: {name}"))?;
            if space!=0 && out.memory.u32(space+0x10)!=6 {return Err(format!("event group space is not a space: {name}"));}
            out.memory.set_u32(*address+0x3c,space);
        }
        Ok(out)
    }
    fn alloc(&mut self,bytes:u32)->u32 {let p=self.next;self.next+=(bytes+15)&!15;p}
    fn words(&mut self,words:&[u32])->u32 {
        let p=self.alloc(words.len() as u32*4);
        for (i,v) in words.iter().enumerate(){self.memory.set_u32(p+i as u32*4,*v);}p
    }
    fn volume(&mut self,node:u32,offset:u32,kind:u32,geometry:&[f32]) {
        let data=self.words(&geometry.iter().map(|x|x.to_bits()).collect::<Vec<_>>());
        self.memory.set_u32(node+offset,kind);self.memory.set_u32(node+offset+8,data);
    }
    fn command(&mut self,node:u32,surface:[u32;2],params:&[u32]) {
        let payload=self.words(params);
        for (i,v) in [surface[0],surface[1],params.len() as u32,payload].into_iter().enumerate(){self.memory.set_u32(node+0x30+i as u32*4,v);}
    }
    fn add(&mut self,n:&Node)->Result<u32,String> {
        let node=self.alloc(0x80);
        self.nodes.entry(n.name.clone()).or_insert(node);
        let (kind,lists)=match (&n.data,n.kind) {
            (_,19)=>(15,1),
            (_,2)=>(7,3),
            (_,4)=>{
                let space=n.names.first().ok_or_else(||format!("missing event group space name: {}",n.name))?;
                self.groups.push((node,space.clone()));(4,1)
            }
            (_,5) if n.name.starts_with("env")=>(23,0),
            (_,5) if n.name.starts_with("lamp")=>(21,1),
            (_,5)=>(5,1),
            (Data::Space{axes,pos,flag},24)=>{
                let matrix=axes.iter().flatten().chain(pos.iter()).copied().collect::<Vec<_>>();
                for (i,value) in matrix.iter().enumerate(){self.memory.set_f32(node+0x40+i as u32*4,*value);}
                self.memory.set_u32(node+0x38,if *flag==1 {1}else{0});(6,1)
            }
            (_,18)=>{
                if n.names.len()!=2 {return Err(format!("missing event link names: {}",n.name));}
                self.links.push((node,n.names[0].clone(),n.names[1].clone()));(17,0)
            }
            (Data::Switch{cases,key},21)=>{
                if let Some(target)=n.name.strip_prefix("refer_") {self.references.push((node,target.to_owned()));}
                let lists=usize::try_from(*cases).map_err(|_|format!("negative switch case count in {}",n.name))?;
                if lists>65536 {return Err(format!("switch case count too large in {}",n.name));}
                self.memory.set_i32(node+0x38,*key);
                self.switches.entry(n.name.clone()).or_insert(node);(18,lists)
            }
            (Data::Split{plane},9|22)=>{self.volume(node,0x38,5,plane);(8,2)}
            (Data::VolumeBranch{volume_kind,geometry,..},10|11)=>{self.volume(node,0x38,*volume_kind,geometry);(8,2)}
            (Data::CollisionVolume{volume_kind,geometry,surface,params,..},13..=17)=>{
                self.volume(node,0x40,*volume_kind,geometry);self.command(node,*surface,params);(9,0)
            }
            (Data::CollisionPlane{plane,surface,params,..},12)=>{
                self.volume(node,0x40,5,plane);self.command(node,*surface,params);(9,0)
            }
            // Point predicate rejects these two volume kinds, so no payload is read.
            (Data::Collision{..},23)=>{self.volume(node,0x40,6,&[]);(9,0)}
            (Data::Wall{..},20)=>{self.volume(node,0x40,7,&[]);(9,0)}
            (_,21)=>return Err(format!("event adapter requires reference/state binding: {} (B3D {})",n.name,n.kind)),
            // Other runtime classes are ignored by 0x5dbf00, including spaces,
            // geometry and cameras. Their children must not be searched either.
            _=>(0,0),
        };
        self.memory.set_u32(node+0x10,kind);
        if lists!=0 {
            let mut heads=vec![0;lists];let mut tails=vec![0;lists];
            for child in &n.children {
                if child.list as usize>=lists {return Err(format!("invalid child list {} in {}",child.list,n.name));}
                let id=self.add(child)?;let list=child.list as usize;
                if tails[list]==0 {heads[list]=id;}else{self.memory.set_u32(tails[list]+8,id);self.memory.set_u32(id+4,tails[list]);}
                // Transform hierarchy follows spaces, not room/group ownership.
                if self.memory.u32(id+0x10)!=6 || kind==6 {self.memory.set_u32(id+0x2c,node);}
                tails[list]=id;
            }
            let table=self.words(&heads);self.memory.set_u32(node+0x30,table);self.memory.set_u32(node+0x34,lists as u32);
        }
        Ok(node)
    }
    /// Invoke synchronously: callback can change scene memory just as the
    /// original does; commands and payloads retain their lifetime in this scene.
    pub fn query(&mut self,point:[f32;3],user:u32,hooks:&mut impl EventHooks) {
        scene_events::query(&mut self.memory,self.root,point,0,user,hooks);
    }
    /// Install owned scene records into the simulation address space once.
    /// Scene addresses stay stable, so callbacks can pass payloads directly to
    /// the vehicle command receiver without copying or a second address space.
    pub fn install(&self,m:&mut Mem) {
        const BASE:u32=0x4200_0000;
        m.write(BASE,&self.memory.read(BASE,(self.next-BASE) as usize));
    }
    /// Bind an already-owned scene vehicle's embedded node to the selected
    /// tree. The owner chooses the root; this does not guess nearest rooms.
    pub fn bind_vehicle(&self,m:&mut Mem,vehicle:u32,root_name:&str)->Result<(),String> {
        let root=self.nodes.get(root_name).copied().ok_or_else(||format!("event vehicle root not found: {root_name}"))?;
        m.set_u32(vehicle+0x110+0x2c,root);Ok(())
    }
    pub fn query_in(&self,m:&mut Mem,root_name:&str,point:[f32;3],user:u32,hooks:&mut impl EventHooks)->Result<(),String> {
        let root=self.nodes.get(root_name).copied().ok_or_else(||format!("event query root not found: {root_name}"))?;
        scene_events::query(m,root,point,0,user,hooks);Ok(())
    }
    /// Update the owning switch key. Event traversal reads it on each query;
    /// unlike rendering, it does not clamp the selection to the case count.
    pub fn set_switch(&mut self,name:&str,key:i32)->bool {
        let Some(node)=self.switches.get(name).copied() else{return false;};
        self.memory.set_i32(node+0x38,key);true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn b3d_event_reaches_vehicle_receiver_in_shared_memory() {
        use ht2_sim::ai_scene::{self,ObstacleSourceHooks,SceneCommandHooks,CommandHeap};
        struct Receiver;
        impl ObstacleSourceHooks for Receiver {
            fn player_kind(&mut self,_:&mut Mem,_:u32)->i32 {0}
            fn world_transform(&mut self,_:&mut Mem,_:u32)->[f32;12]{[1.,0.,0.,0.,1.,0.,0.,0.,1.,0.,0.,2.]}
        }
        impl SceneCommandHooks for Receiver {fn height_modifier(&mut self,_:&mut Mem)->bool{false}}
        impl CommandHeap for Receiver {
            fn allocate(&mut self,m:&mut Mem,bytes:u32)->u32{m.alloc(bytes)}
            fn release(&mut self,_:&mut Mem,_:u32){}
        }
        impl EventHooks for Receiver {
            fn event(&mut self,m:&mut Mem,_:u32,command:u32,vehicle:u32){ai_scene::receive_scene_command(m,command,vehicle,&mut Receiver,self);}
        }
        let event=Node{name:"Height".into(),kind:13,names:vec![],data:Data::CollisionVolume{sphere:[0.;4],volume_kind:0,geometry:vec![],surface:[16,0],params:vec![3f32.to_bits()]},children:vec![],list:0};
        let room=Node{name:"Room".into(),kind:19,names:vec![],data:Data::Other,children:vec![event],list:0};
        let scene=EventScene::from_node(&room).unwrap();let mut m=Mem::new();scene.install(&mut m);
        let vehicle=0x1a000000;let player=0x1b000000;m.set_u32(vehicle+0x509c,player);
        scene.query_in(&mut m,"Room",[0.,0.,2.],vehicle,&mut Receiver).unwrap();
        assert_eq!(m.u32(player+0x60),0x20000);
        assert!(scene.query_in(&mut m,"Missing",[0.;3],vehicle,&mut Receiver).is_err());
    }
    #[test]
    fn optional_names_match_native_resolver() {
        for line in include_str!("../../../tools/emu/golden_optional_event_names.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();
            assert_eq!(optional_name(p[0]),p[1]=="1");assert_eq!(p[2],"0");
            assert_eq!(binding(&HashMap::new(),p[0]),if p[1]=="1" {Some(0)}else{None});
            let known=HashMap::from([(p[0].to_owned(),123)]);assert_eq!(binding(&known,p[0]),Some(123));
        }
    }
    struct Commands(Vec<(u32,u32,Vec<u32>)>);
    impl EventHooks for Commands {
        fn event(&mut self,m:&mut Mem,_:u32,command:u32,user:u32){
            assert_eq!(user,456);let data=m.u32(command+12);
            self.0.push((m.u32(command),m.u32(command+4),(0..m.u32(command+8)).map(|i|m.u32(data+i*4)).collect()));
        }
    }
    #[test]
    fn unresolved_binding_is_reported() {
        let node=Node{name:"LinkedGroup".into(),kind:18,names:vec!["Space".into(),"Group".into()],data:Data::Other,children:vec![],list:0};
        assert!(EventScene::from_node(&node).err().unwrap().contains("Group"));
        let node=Node{name:"refer_Missing".into(),kind:21,names:vec![],data:Data::Switch{cases:0,key:0},children:vec![],list:0};
        assert!(EventScene::from_node(&node).err().unwrap().contains("Missing"));
    }
    #[test]
    #[ignore="requires HT2_GAME installed game data"]
    fn installed_scene_bindings_with_common() {
        let game=std::env::var("HT2_GAME").expect("HT2_GAME");
        let (_,_,files)=crate::load_scenes(std::path::Path::new(&game),"ENV").unwrap();
        assert!(!files.is_empty());
        let scene=EventScene::from_map_files(std::path::Path::new(&game),&files,&files[0].nodes[0].name).unwrap();
        assert!(scene.switches.contains_key("ILLUM_LEVEL_14"));
        assert!(scene.root!=0);
        let mut memory=Mem::new();scene.install(&mut memory);
        let mut rooms=scene.nodes.iter().filter(|(_,address)|scene.memory.u32(**address+0x10)==15).map(|(name,_)|name.clone()).collect::<Vec<_>>();rooms.sort();
        assert!(!rooms.is_empty());let mut delivered=0;
        for name in &rooms {
            let mut commands=Commands(Vec::new());
            scene.query_in(&mut memory,name,[0.,0.,0.],456,&mut commands).unwrap();delivered+=commands.0.len();
        }
        println!("installed ENV: {} room queries, {delivered} commands at origin",rooms.len());
    }
    #[test]
    fn b3d_point_events_match_native_pipeline() {
        for data in [include_str!("../../../tools/emu/golden_b3d_event_pipeline.tsv"),include_str!("../../../tools/emu/golden_b3d_switch_events.tsv"),include_str!("../../../tools/emu/golden_b3d_refer_events.tsv"),include_str!("../../../tools/emu/golden_b3d_link_events.tsv"),include_str!("../../../tools/emu/golden_b3d_group_events.tsv")] {
        for (case,line) in data.lines().enumerate(){
            for separate_definitions in [false,true] {
            let p:Vec<_>=line.split('\t').collect();let bytes=(0..p[0].len()).step_by(2).map(|i|u8::from_str_radix(&p[0][i..i+2],16).unwrap()).collect::<Vec<_>>();
            let mut b3d=ht2_formats::b3d::parse(&bytes).unwrap();
            if separate_definitions {
                let children=&mut b3d.nodes[0].children;
                if children.len()!=2 || !matches!(children[1].kind,21|24) {continue;}
                let definition=children.remove(1);b3d.nodes.push(definition);
            }
            let mut scene=EventScene::from_nodes(&b3d.nodes,&b3d.nodes[0].name).unwrap();
            if p.len()>4 {assert!(scene.set_switch("refer_Source",99));}
            let mut commands=Commands(Vec::new());scene.query([3.,4.,5.],456,&mut commands);
            let actual=commands.0.iter().map(|(kind,sub,params)|format!("{kind},{sub},{}",params.iter().map(|x|x.to_string()).collect::<Vec<_>>().join(","))).collect::<Vec<_>>().join(";");
            assert_eq!(actual,p[1],"case {case}");
            if p.len()>2 {
                assert!(scene.set_switch(if p.len()>4 {p[4]}else{"Branch"},p[2].parse().unwrap()));
                let mut commands=Commands(Vec::new());scene.query([3.,4.,5.],456,&mut commands);
                let actual=commands.0.iter().map(|(kind,sub,params)|format!("{kind},{sub},{}",params.iter().map(|x|x.to_string()).collect::<Vec<_>>().join(","))).collect::<Vec<_>>().join(";");
                assert_eq!(actual,p[3],"updated case {case}");
            }
            }
        }
        }
    }
}
