//! Original camera matrix/projection path. The caller still supplies a stand-in
//! chase/cab pose; original camera tracking and per-module settings remain pending.
use glam::{Mat4,Vec3};
use ht2_sim::{matrix,mem::Mem,scene_state};

pub struct Frame {
    pub view:[f32;16],
    pub projection:[f32;16],
    pub viewport:[u32;4],
}
impl Frame {
    pub fn combined(&self)->Mat4 {
        ht2_render::render::original_view_projection(&self.view,&self.projection)
    }
}

/// Use the original startup lens (controller +0xbe8/+0xbf0), scene near/far,
/// hierarchy lookup, inverse, axis conversion and projection for a supplied pose.
pub fn from_pose(eye:Vec3,target:Vec3,up:Vec3,size:(u32,u32))->Frame {
    // The pose helper uses right/up/back; original transform nodes use
    // right/forward/up. This conversion does not apply a projection.
    let pose=Mat4::look_at_rh(eye,target,up).inverse();
    let axes=[pose.x_axis.truncate(),-pose.z_axis.truncate(),pose.y_axis.truncate(),eye];
    let mut memory=Mem::new();let (scene,camera,node)=(0x2a000000,0x2a010000,0x2a020000);
    matrix::put12(&mut memory,node+0x40,&std::array::from_fn(|i|axes[i/3][i%3]));
    matrix::put12(&mut memory,ht2_sim::globals::MAT_IDENTITY,&matrix::IDENTITY);
    memory.set_u32(node+0x38,1);memory.set_u32(camera+0x50,node);
    memory.set_f32(camera+0x58,0.8);memory.set_f32(camera+0x54,0.6);
    memory.set_u32(0x720908,1);
    memory.set_f64(scene+0x2110,1.);memory.set_f64(scene+0x2118,100000.);
    memory.set_f64(scene+0x2148,1.);
    memory.set_i32(scene+0x7a8,(size.0.wrapping_sub(1) as i32).wrapping_shl(19));
    memory.set_i32(scene+0x7ac,(size.1.wrapping_sub(1) as i32).wrapping_shl(19));
    let clip=[0,0,size.0 as i32,size.1 as i32];
    scene_state::prepare_camera_node(&mut memory,camera,scene,clip);
    let projection=scene_state::projection(&mut memory,scene,clip);
    let view=scene_state::device_view(&mut memory,scene+0x1070);
    Frame{view,projection:projection.matrix,viewport:projection.viewport}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_pose_projects_forward_right_and_up_with_original_lens() {
        for eye in [Vec3::ZERO,Vec3::new(130.,-70.,20.)] {
            for forward in [Vec3::Y,Vec3::new(2.,3.,0.5).normalize()] {
            let right_axis=forward.cross(Vec3::Z).normalize();let up_axis=right_axis.cross(forward);
            for size in [(800,600),(1280,720),(801,601),(3840,2160)] {
                let frame=from_pose(eye,eye+forward*10.,up_axis,size);
                assert_eq!(frame.viewport,[0,0,size.0,size.1]);
                let project=|delta:Vec3|frame.combined().project_point3(eye+delta);
                let center=project(forward*10.);
                assert!(center.x.abs()<1e-5 && center.y.abs()<1e-5 && center.z>0. && center.z<1.);
                let right=project(forward*10.+right_axis);
                let up=project(forward*10.+up_axis);
                assert!((right.x-0.25).abs()<1e-5,"{eye:?} {forward:?} {right:?}");
                assert!((up.y-1./3.).abs()<1e-5);
                assert!(project(forward*0.5).z<0.);
            }
            }
        }
    }
}
