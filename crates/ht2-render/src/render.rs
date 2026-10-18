//! wgpu renderer. Holds any number of objects (loaded scenes) that each have a model matrix.
//! Works against a window surface or an offscreen texture.

use crate::scene::{Look, Scene};
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use ht2_formats::b3d::Batch;
use wgpu::util::DeviceExt;

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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectId(usize);

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
    depth: Option<(wgpu::TextureView, (u32, u32))>,
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
        Self { device, queue, pipeline, pipeline_blend, obj_layout, mat_layout, sampler, white, objects: Vec::new(), depth: None }
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
        self.objects.push(Object { batches, mvp, bind, model: Mat4::IDENTITY });
        ObjectId(self.objects.len() - 1)
    }

    pub fn set_model(&mut self, id: ObjectId, model: Mat4) {
        self.objects[id.0].model = model;
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
        for o in &self.objects {
            let mvp = view_proj * o.model;
            self.queue.write_buffer(&o.mvp, 0, bytemuck::cast_slice(&mvp.to_cols_array()));
        }
        self.depth_view(size);
        let depth = &self.depth.as_ref().unwrap().0;
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.45, g: 0.6, b: 0.8, a: 1.0 }),
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
            for blended in [false, true] {
                pass.set_pipeline(if blended { &self.pipeline_blend } else { &self.pipeline });
                for o in &self.objects {
                    pass.set_bind_group(0, &o.bind, &[]);
                    for b in o.batches.iter().filter(|b| b.blend == blended) {
                        pass.set_bind_group(1, &b.bind, &[]);
                        pass.set_vertex_buffer(0, b.vbuf.slice(..));
                        pass.draw(0..b.count, 0..1);
                    }
                }
            }
        }
        self.queue.submit([enc.finish()]);
    }

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
