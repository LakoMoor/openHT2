//! wgpu renderer. Holds any number of objects (loaded scenes) that each have a model matrix.
//! Works against a window surface or an offscreen texture.

use crate::scene::{Look, Scene};
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use ht2_formats::b3d::Batch;
use wgpu::util::DeviceExt;

/// Convert original Direct3D row-vector uploads to the shader's column-vector
/// convention. Both APIs use 0..1 depth; the original view already changes axes.
pub fn original_view_projection(view:&[f32;16],projection:&[f32;16])->Mat4 {
    Mat4::from_cols_array(projection)*Mat4::from_cols_array(view)
}

const SHADER: &str = r#"
struct Object { mvp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> obj: Object;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;
@group(1) @binding(2) var<uniform> tint: vec4<f32>;

struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>) -> VOut {
    var o: VOut;
    o.pos = obj.mvp * vec4<f32>(p, 1.0);
    o.uv = uv;
    return o;
}

@fragment fn fs(i: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, smp, i.uv) * tint;
    if (c.a < 0.5) { discard; }
    return vec4<f32>(c.rgb, 1.0);
}

@fragment fn fs_blend(i: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, smp, i.uv) * tint;
    if (c.a < 0.01) { discard; }
    return c;
}
"#;

/// The 2D layer: coloured triangles already in clip space.
const HUD_SHADER: &str = r#"
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) color: vec4<f32> };
@vertex fn vs(@location(0) p: vec2<f32>, @location(1) c: vec4<f32>) -> VOut {
    var o: VOut;
    o.pos = vec4<f32>(p, 0.0, 1.0);
    o.color = c;
    return o;
}
@fragment fn fs(i: VOut) -> @location(0) vec4<f32> { return i.color; }
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct HudVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

struct DrawBatch {
    /// Textures with partial transparency are drawn in a second, blended pass.
    blend: bool,
    vbuf: wgpu::Buffer,
    count: u32,
    bind: wgpu::BindGroup,
}

struct Object {
    batches: Vec<DrawBatch>,
    mvp: wgpu::Buffer,
    bind: wgpu::BindGroup,
    model: Mat4,
    visible: bool,
    cockpit: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectId(usize);

/// The textures of a scene uploaded once, for objects made later with [`Renderer::add_parts`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TexturesId(usize);

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    pipeline_blend: wgpu::RenderPipeline,
    obj_layout: wgpu::BindGroupLayout,
    mat_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    objects: Vec<Object>,
    texture_sets: Vec<Vec<wgpu::TextureView>>,
    depth: Option<(wgpu::TextureView, (u32, u32))>,
    hud_pipeline: wgpu::RenderPipeline,
    hud: Vec<crate::hud::Rect>,
    movie: crate::movie::Layer,
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
        count: None,
    }
}

impl Renderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let obj_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX)],
        });
        let mat_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                uniform_entry(2, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&obj_layout, &mat_layout],
            push_constant_ranges: &[],
        });
        let make_pipeline = |entry: &str, blend: Option<wgpu::BlendState>, depth_write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GpuVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: depth_write,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let pipeline = make_pipeline("fs", None, true);
        let pipeline_blend = make_pipeline("fs_blend", Some(wgpu::BlendState::ALPHA_BLENDING), false);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let white = upload(&device, &queue, 1, 1, &[255; 4]);
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("hud"), source: wgpu::ShaderSource::Wgsl(HUD_SHADER.into()) });
        let hud_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], push_constant_ranges: &[] });
        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud"),
            layout: Some(&hud_layout),
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<HudVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let movie = crate::movie::Layer::new(&device, format);
        Self { device, queue, pipeline, pipeline_blend, obj_layout, mat_layout, sampler, white, objects: Vec::new(), texture_sets: Vec::new(), depth: None, hud_pipeline, hud: Vec::new(), movie }
    }

    /// Upload a scene's geometry and textures and return a handle to place it with [`set_model`](Self::set_model).
    pub fn add(&mut self, scene: &Scene) -> ObjectId {
        self.add_many(scene, &[&scene.batches])[0]
    }

    /// Upload the textures of `scene` once and make one object per geometry part, all drawn
    /// with that scene's materials (e.g. a vehicle body and its wheels).
    pub fn add_many(&mut self, scene: &Scene, parts: &[&[Batch]]) -> Vec<ObjectId> {
        let tex_views: Vec<_> = scene.textures.iter().map(|t| upload(&self.device, &self.queue, t.width, t.height, &t.rgba)).collect();
        parts.iter().map(|part| self.build_object(scene, &tex_views, part)).collect()
    }

    /// Upload the textures of `scene` for [`add_parts`](Self::add_parts).
    pub fn upload_textures(&mut self, scene: &Scene) -> TexturesId {
        let views = scene.textures.iter().map(|t| upload(&self.device, &self.queue, t.width, t.height, &t.rgba)).collect();
        self.texture_sets.push(views);
        TexturesId(self.texture_sets.len() - 1)
    }

    /// Like [`add_many`](Self::add_many) with textures uploaded before (`scene` gives the
    /// materials).
    pub fn add_parts(&mut self, textures: TexturesId, scene: &Scene, parts: &[&[Batch]]) -> Vec<ObjectId> {
        let views = std::mem::take(&mut self.texture_sets[textures.0]);
        let ids = parts.iter().map(|part| self.build_object(scene, &views, part)).collect();
        self.texture_sets[textures.0] = views;
        ids
    }

    /// Draw the object or not.
    pub fn set_visible(&mut self, id: ObjectId, visible: bool) {
        self.objects[id.0].visible = visible;
    }

    fn build_object(&mut self, scene: &Scene, tex_views: &[wgpu::TextureView], part: &[Batch]) -> ObjectId {
        let batches = part
            .iter()
            .filter(|b| !b.verts.is_empty())
            .map(|b| {
                let verts: Vec<GpuVertex> = b.verts.iter().map(|v| GpuVertex { pos: v.pos, uv: v.uv }).collect();
                let mut blend = false;
                let (view, tint) = match scene.looks.get(b.material as usize) {
                    Some(Look::Texture(i)) => {
                        blend = scene.textures[*i].is_translucent();
                        (&tex_views[*i], [1.0; 4])
                    }
                    Some(Look::Flat(c)) => (&self.white, [c[0], c[1], c[2], 1.0]),
                    None => (&self.white, [1.0, 0.0, 1.0, 1.0]),
                };
                let tint_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&tint),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.mat_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                        wgpu::BindGroupEntry { binding: 2, resource: tint_buf.as_entire_binding() },
                    ],
                });
                DrawBatch {
                    blend,
                    vbuf: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&verts),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                    count: verts.len() as u32,
                    bind,
                }
            })
            .collect();
        let mvp = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mvp"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.obj_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: mvp.as_entire_binding() }],
        });
        self.objects.push(Object { batches, mvp, bind, model: Mat4::IDENTITY, visible: true, cockpit: false });
        ObjectId(self.objects.len() - 1)
    }

    /// What the 2D layer shows over the scene from the next frame on (window pixels).
    pub fn set_hud(&mut self, rects: Vec<crate::hud::Rect>) {
        self.hud = rects;
    }

    pub fn set_model(&mut self, id: ObjectId, model: Mat4) {
        self.objects[id.0].model = model;
    }

    /// Draw cabin geometry after the world with its own depth buffer contents.
    pub fn set_cockpit(&mut self, id: ObjectId, cockpit: bool) {
        self.objects[id.0].cockpit = cockpit;
    }

    fn depth_view(&mut self, size: (u32, u32)) {
        if self.depth.as_ref().is_none_or(|(_, s)| *s != size) {
            let t = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((t.create_view(&Default::default()), size));
        }
    }

    /// Draw every object with the combined `proj * view` matrix.
    pub fn draw(&mut self, target: &wgpu::TextureView, size: (u32, u32), view_proj: Mat4) {
        self.draw_region(target,size,view_proj,None);
    }

    /// Draw the scene with an explicitly prepared viewport. HUD and movie layers
    /// retain their full-surface coordinates. Invalid regions are reported before
    /// issuing GPU commands instead of silently changing the original viewport.
    pub fn draw_viewport(&mut self,target:&wgpu::TextureView,size:(u32,u32),view_proj:Mat4,
        viewport:[u32;4])->Result<(),String> {
        let [x,y,w,h]=viewport;
        if w==0 || h==0 || x.checked_add(w).is_none_or(|end|end>size.0)
            || y.checked_add(h).is_none_or(|end|end>size.1) {
            return Err("scene viewport is outside the render target".into());
        }
        self.draw_region(target,size,view_proj,Some(viewport));Ok(())
    }

    /// Consume the original backend's view/projection commands and viewport.
    pub fn draw_original_view(&mut self,target:&wgpu::TextureView,size:(u32,u32),
        view:&[f32;16],projection:&[f32;16],viewport:[u32;4])->Result<(),String> {
        self.draw_viewport(target,size,original_view_projection(view,projection),viewport)
    }

    fn draw_region(&mut self, target: &wgpu::TextureView, size: (u32, u32), view_proj: Mat4, viewport:Option<[u32;4]>) {
        let mut movie_encoder = self.device.create_command_encoder(&Default::default());
        if self.movie.draw(&mut movie_encoder, target, size) {
            self.queue.submit([movie_encoder.finish()]);
            return;
        }
        for o in &self.objects {
            let mvp = view_proj * o.model;
            self.queue.write_buffer(&o.mvp, 0, bytemuck::cast_slice(&mvp.to_cols_array()));
        }
        self.depth_view(size);
        let depth = &self.depth.as_ref().unwrap().0;
        let mut enc = self.device.create_command_encoder(&Default::default());
        for cockpit in [false, true] {
            if cockpit && !self.objects.iter().any(|o| o.visible && o.cockpit) { continue; }
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if cockpit { "cockpit" } else { "world" }),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if cockpit { wgpu::LoadOp::Load } else { wgpu::LoadOp::Clear(wgpu::Color { r: 0.45, g: 0.6, b: 0.8, a: 1.0 }) },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if !cockpit {
                if let Some([x,y,w,h])=viewport {pass.set_viewport(x as f32,y as f32,w as f32,h as f32,0.,1.);}
            }
            for blended in [false, true] {
                pass.set_pipeline(if blended { &self.pipeline_blend } else { &self.pipeline });
                for o in self.objects.iter().filter(|o| o.visible && o.cockpit == cockpit) {
                    pass.set_bind_group(0, &o.bind, &[]);
                    for b in o.batches.iter().filter(|b| b.blend == blended) {
                        pass.set_bind_group(1, &b.bind, &[]);
                        pass.set_vertex_buffer(0, b.vbuf.slice(..));
                        pass.draw(0..b.count, 0..1);
                    }
                }
            }
        }
        if !self.hud.is_empty() {
            let (w, h) = (size.0 as f32, size.1 as f32);
            let clip = |x: f32, y: f32| [x / w * 2.0 - 1.0, 1.0 - y / h * 2.0];
            let mut verts = Vec::with_capacity(self.hud.len() * 6);
            for r in &self.hud {
                let (a, b, c, d) = (clip(r.x, r.y), clip(r.x + r.w, r.y), clip(r.x + r.w, r.y + r.h), clip(r.x, r.y + r.h));
                for p in [a, b, c, a, c, d] {
                    verts.push(HudVertex { pos: p, color: r.color });
                }
            }
            let vbuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("hud"), contents: bytemuck::cast_slice(&verts), usage: wgpu::BufferUsages::VERTEX });
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.hud_pipeline);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.draw(0..verts.len() as u32, 0..1);
        }
        self.queue.submit([enc.finish()]);
    }

    /// Render offscreen and return tightly packed RGBA8.
    pub fn set_movie_frame(&mut self, size: (u32, u32), rgba: &[u8]) -> Result<(), String> {
        self.movie.set_frame(&self.device, &self.queue, size, rgba)
    }
    pub fn clear_movie(&mut self) { self.movie.clear(); }

    /// Render offscreen and return tightly packed RGBA8.
    pub fn screenshot(&mut self, size: (u32, u32), view_proj: Mat4) -> Vec<u8> {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shot"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.draw(&tex.create_view(&Default::default()), size, view_proj);

        let row = (size.0 * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (row * size.1) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(size.1) },
            },
            wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        );
        self.queue.submit([enc.finish()]);
        buf.slice(..).map_async(wgpu::MapMode::Read, |r| r.expect("map screenshot buffer"));
        self.device.poll(wgpu::Maintain::Wait);
        let mapped = buf.slice(..).get_mapped_range();
        mapped.chunks(row as usize).flat_map(|r| &r[..(size.0 * 4) as usize]).copied().collect()
    }
}

fn upload(device: &wgpu::Device, queue: &wgpu::Queue, w: u32, h: u32, rgba: &[u8]) -> wgpu::TextureView {
    device
        .create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba,
        )
        .create_view(&Default::default())
}

#[cfg(test)]
mod original_camera_tests {
    use super::*;
    #[test]
    fn uploaded_matrices_preserve_original_clip_coordinates() {
        let parse=|s:&str|-> [f32;16] {
            s.split(',').map(|v|f32::from_bits(v.parse().unwrap())).collect::<Vec<_>>().try_into().unwrap()
        };
        for (view,projection) in include_str!("../../../tools/emu/golden_scene_device_view.tsv").lines()
            .zip(include_str!("../../../tools/emu/golden_scene_projection.tsv").lines()) {
            let v=parse(view.split('\t').nth(1).unwrap());
            let p=parse(projection.split('\t').nth(6).unwrap());
            let combined=original_view_projection(&v,&p);
            for point in [[0.,0.,0.,1.],[1.,2.,3.,1.],[-30.,70.,12.,1.],[0.,1.,0.,0.]] {
                let row_mul=|x:[f32;4],m:&[f32;16]|-> [f64;4] {
                    std::array::from_fn(|col|(0..4).map(|row|x[row] as f64*m[row*4+col] as f64).sum())
                };
                let camera=row_mul(point,&v);
                let expected:[f64;4]=std::array::from_fn(|col|(0..4).map(|row|camera[row]*p[row*4+col] as f64).sum());
                let actual=(combined*glam::Vec4::from_array(point)).to_array();
                for i in 0..4 {
                    let scale=(0..4).flat_map(|r|(0..4).map(move |k| (point[r] as f64*v[r*4+k] as f64*p[k*4+i] as f64).abs())).sum::<f64>().max(1.);
                    assert!((actual[i] as f64-expected[i]).abs()<=scale*2e-6);
                }
            }
        }
    }
}
