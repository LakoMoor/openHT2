//! RGBA movie presentation. Aspect-preserving fit is provisional until the original
//! AVI window's scaling policy is verified; it is not claimed as a port of that policy.
pub struct Layer {
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    image: Option<(wgpu::Texture, wgpu::BindGroup, (u32, u32))>,
}
impl Layer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("movie"), source: wgpu::ShaderSource::Wgsl(r#"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
struct Out { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> Out {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: Out;
    o.pos = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    return o;
}
@fragment fn fs(i: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(image, smp, i.uv).rgb, 1.0);
}
"#.into()) });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("movie"), layout: None,
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: Default::default(), multiview: None, cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("movie"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        Self { pipeline, sampler, image: None }
    }
    pub fn set_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32), rgba: &[u8]) -> Result<(), String> {
        let bytes = (size.0 as usize).checked_mul(size.1 as usize).and_then(|n| n.checked_mul(4)).ok_or("movie frame size overflow")?;
        let max = device.limits().max_texture_dimension_2d;
        if size.0 == 0 || size.1 == 0 || size.0 > max || size.1 > max || bytes != rgba.len() { return Err("invalid movie frame dimensions or data length".into()); }
        if self.image.as_ref().map(|i| i.2) != Some(size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("movie frame"), size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
            let view = texture.create_view(&Default::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("movie frame"), layout: &self.pipeline.get_bind_group_layout(0), entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) }] });
            self.image = Some((texture, bind, size));
        }
        queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &self.image.as_ref().unwrap().0, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, rgba, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size.0 * 4), rows_per_image: Some(size.1) }, wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 });
        Ok(())
    }
    pub fn clear(&mut self) { self.image = None; }
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, size: (u32, u32)) -> bool {
        let Some((_, bind, image)) = &self.image else { return false };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("movie"), color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })], depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
        let scale = (size.0 as f32 / image.0 as f32).min(size.1 as f32 / image.1 as f32);
        let (w, h) = (image.0 as f32 * scale, image.1 as f32 * scale);
        pass.set_viewport((size.0 as f32 - w) * 0.5, (size.1 as f32 - h) * 0.5, w, h, 0.0, 1.0);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
        true
    }
}
