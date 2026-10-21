//! Scene loading and wgpu rendering shared by the viewer and the game.

pub mod render;
pub mod scene;

pub use render::{ObjectId, Renderer, TexturesId};
pub use scene::Scene;

/// Pick a GPU and open a device. `surface` is `None` for offscreen rendering.
pub async fn request_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: surface,
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        })
        .await
        .ok_or("no suitable GPU adapter")?;
    let (device, queue) =
        adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await.map_err(|e| e.to_string())?;
    Ok((adapter, device, queue))
}

/// Write tightly packed RGBA8 to a PNG file.
pub fn save_png(path: &std::path::Path, size: (u32, u32), rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size.0, size.1);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut w| w.write_image_data(rgba)).map_err(|e| e.to_string())
}
