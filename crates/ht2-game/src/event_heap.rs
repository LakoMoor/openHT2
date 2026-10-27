//! Application-owned allocator for scene command lifetimes. This replaces the
//! CRT allocation boundary; it does not reproduce king.exe's CRT heap addresses.
use std::collections::{BTreeMap,HashMap};
use ht2_sim::{mem::Mem,ai_scene::{self,CommandHeap,SceneCommandHooks}};

#[derive(Default)]
pub struct EventHeap {
    live:HashMap<u32,u32>,
    free:BTreeMap<u32,Vec<u32>>,
}
impl CommandHeap for EventHeap {
    fn allocate(&mut self,m:&mut Mem,bytes:u32)->u32 {
        let size=bytes.max(1).checked_add(15).expect("command allocation overflow")&!15;
        let address=self.free.get_mut(&size).and_then(Vec::pop).unwrap_or_else(||m.alloc(size));
        assert!(self.live.insert(address,size).is_none(),"command allocation overlaps live block");address
    }
    fn release(&mut self,_:&mut Mem,address:u32) {
        if address==0{return;}
        let size=self.live.remove(&address).expect("command free without heap ownership");
        self.free.entry(size).or_default().push(address);
    }
}
impl EventHeap {
    /// Explicit scene-event portion of the original AI update. Earlier motion
    /// synchronization/mode decisions remain the scene vehicle owner's job.
    pub fn update(&mut self,m:&mut Mem,vehicle:u32,player:u32,flags:u32,source:&mut impl SceneCommandHooks) {
        let driver=m.u32(vehicle+0x5460);
        ai_scene::query_vehicle_commands(m,vehicle,source,self);
        ai_scene::update_obstacle_command_cached(m,vehicle,driver,player,flags,&mut ai_scene::OwnedObstacle{heap:self});
    }
    /// Deliver scene commands to the live trajectory-world player represented
    /// by `player_id`. The caller owns the identity and scene attachment.
    pub fn update_player(&mut self,m:&mut Mem,vehicle:u32,player_id:u32,player:&mut ht2_sim::world::AiPlayer,flags:u32,source:&mut impl SceneCommandHooks) {
        struct Tracked<'a,S>{source:&'a mut S,id:u32,marked:bool}
        impl<S:SceneCommandHooks> ai_scene::ObstacleSourceHooks for Tracked<'_,S> {
            fn player_kind(&mut self,m:&mut Mem,vehicle:u32)->i32{self.source.player_kind(m,vehicle)}
            fn world_transform(&mut self,m:&mut Mem,body:u32)->[f32;12]{self.source.world_transform(m,body)}
        }
        impl<S:SceneCommandHooks> SceneCommandHooks for Tracked<'_,S> {
            fn height_modifier(&mut self,m:&mut Mem)->bool{self.source.height_modifier(m)}
            fn mark_player(&mut self,m:&mut Mem,player:u32){
                assert_eq!(player,self.id,"scene player identity mismatch");
                self.source.mark_player(m,player);self.marked=true;
            }
        }
        let driver=m.u32(vehicle+0x5460);
        let mut tracked=Tracked{source,id:player_id,marked:false};
        ai_scene::query_vehicle_commands(m,vehicle,&mut tracked,self);
        player.scene_remove_pending|=tracked.marked;
        ai_scene::update_obstacle_command_cached(m,vehicle,driver,player_id,flags,&mut ai_scene::PlayerObstacle{heap:self,player_id,player});
    }
    pub fn clear_vehicle(&mut self,m:&mut Mem,vehicle:u32) {
        for slot in 0..2 {ai_scene::replace_command(m,vehicle+0x5570,0,slot,self);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore="requires HT2_GAME installed game data"]
    fn scene_commands_update_live_world_player_and_reuse_heap() {
        struct Source;
        impl ai_scene::ObstacleSourceHooks for Source {
            fn player_kind(&mut self,_:&mut Mem,_:u32)->i32{0}
            fn world_transform(&mut self,_:&mut Mem,_:u32)->[f32;12]{ht2_sim::matrix::IDENTITY}
        }
        impl SceneCommandHooks for Source {}
        let game=std::path::PathBuf::from(std::env::var("HT2_GAME").unwrap());
        let loaded=crate::load_world(&game,"ENV",None).unwrap();let net=loaded.ground.roads.clone();
        let mut world=ht2_sim::world::World::new_game(&game,&game.join("ENV"),net,1.,None);
        let player=world.players.iter_mut().find(|p|p.kind!=0 && p.kind!=1).unwrap();
        let mut m=Mem::new();let mut heap=EventHeap::default();let vehicle=0x1a000000;let root=0x42000000;let id=0x1b000000;let driver=0x1c000000;let flags=0x1d000000;
        m.set_u32(vehicle+0x13c,root);m.set_u32(root+0x10,9);m.set_u32(root+0x30,29);m.set_u32(vehicle+0x5460,driver);m.set_u32(0x696338,0x1e000000);
        for _ in 0..10 {
            heap.update_player(&mut m,vehicle,id,player,flags,&mut Source);
            assert!(!player.full_speed);assert_eq!(player.drag,100.);assert!(heap.live.is_empty());
        }
        m.set_u32(root+0x10,0);heap.update_player(&mut m,vehicle,id,player,flags,&mut Source);
        assert!(player.full_speed);assert_eq!(player.drag,-1.);assert_eq!(heap.free.values().map(Vec::len).sum::<usize>(),3);
        m.set_u32(root+0x10,9);m.set_u32(root+0x30,0xcc0);m.set_u32(vehicle+0x509c,id);
        heap.update_player(&mut m,vehicle,id,player,flags,&mut Source);
        assert!(player.scene_remove_pending);assert_ne!(m.u32(id+0x60)&0x20000,0);
    }
    #[test]
    fn event_query_preserves_driver_captured_before_callbacks() {
        struct Source {vehicle:u32,driver:u32}
        impl ai_scene::ObstacleSourceHooks for Source {
            fn player_kind(&mut self,_:&mut Mem,_:u32)->i32{0}
            fn world_transform(&mut self,m:&mut Mem,_:u32)->[f32;12]{m.set_u32(self.vehicle+0x5460,self.driver);ht2_sim::matrix::IDENTITY}
        }
        impl SceneCommandHooks for Source {}
        let mut m=Mem::new();let mut heap=EventHeap::default();let vehicle=0x1a000000;let root=0x42000000;let player=0x1b000000;let driver=0x1c000000;let other=0x1c010000;let flags=0x1d000000;
        m.set_u32(vehicle+0x13c,root);m.set_u32(root+0x10,9);m.set_u32(root+0x30,29);m.set_u32(vehicle+0x5460,driver);m.set_u32(0x696338,0x1e000000);
        heap.update(&mut m,vehicle,player,flags,&mut Source{vehicle,driver:other});
        assert_eq!(m.f64(driver+0x40b0),-1.);assert_eq!(m.f64(other+0x40b0),100.);
        assert_eq!(m.f64(player+0x3d8),-1.);assert_eq!(m.rd8(player+0x3d0),1);
    }
    #[test]
    fn persistent_owner_queries_delivers_and_consumes() {
        struct Source;
        impl ai_scene::ObstacleSourceHooks for Source {
            fn player_kind(&mut self,_:&mut Mem,_:u32)->i32{0}
            fn world_transform(&mut self,_:&mut Mem,_:u32)->[f32;12]{ht2_sim::matrix::IDENTITY}
        }
        impl SceneCommandHooks for Source {}
        let mut m=Mem::new();let mut heap=EventHeap::default();
        let vehicle=0x1a000000;let root=0x42000000;let player=0x1b000000;let driver=0x1c000000;let flags=0x1d000000;
        m.set_u32(vehicle+0x13c,root);m.set_u32(root+0x10,9);m.set_u32(root+0x30,29);m.set_u32(root+0x40,0);
        m.set_u32(vehicle+0x5460,driver);m.set_u32(0x696338,0x1e000000);
        for _ in 0..1000 {
            heap.update(&mut m,vehicle,player,flags,&mut Source);
            assert_eq!(m.f64(player+0x3d8),100.);assert_eq!(m.rd8(player+0x3d0),0);
            assert!(heap.live.is_empty());
        }
        assert_eq!(heap.free.values().map(Vec::len).sum::<usize>(),3);
        m.set_u32(root+0x10,0);heap.update(&mut m,vehicle,player,flags,&mut Source);
        assert_eq!(m.f64(player+0x3d8),-1.);assert_eq!(m.rd8(player+0x3d0),1);
    }
    #[test]
    fn repeated_queue_rotation_reuses_owned_blocks() {
        let mut m=Mem::new();let mut heap=EventHeap::default();let vehicle=0x1a000000;
        let mut used=std::collections::HashSet::new();
        for pass in 0..1000 {
            for subcode in 0..3 {
                let temporary=heap.allocate(&mut m,4);used.insert(temporary);
                ai_scene::queue_prepared_obstacle(&mut m,vehicle,subcode,(pass+1) as f32,temporary,&mut heap);
                let command=m.u32(vehicle+0x5570);used.insert(command);used.insert(m.u32(command+12));
            }
            heap.clear_vehicle(&mut m,vehicle);
            assert!(heap.live.is_empty());assert_eq!(m.u32(vehicle+0x5570),0);assert_eq!(m.u32(vehicle+0x5574),0);
        }
        // Two queued records/payloads and the current temporary all fit in five
        // blocks; no unbounded growth across repeated scene-vehicle lifetimes.
        assert_eq!(used.len(),5);
    }
    #[test]
    #[should_panic(expected="command free without heap ownership")]
    fn double_free_is_rejected() {
        let mut m=Mem::new();let mut h=EventHeap::default();let p=h.allocate(&mut m,4);h.release(&mut m,p);h.release(&mut m,p);
    }
}
