//! End-menu backgrounds loaded from the user's game data. Widgets and text are still
//! missing; displaying the backdrop is not a complete original menu.
use std::path::Path;

/// `0x51f380`, resources initialized by the end-screen constructor in `0x51f...`.
pub fn background_name(mode: u32, family: u32) -> Result<String, String> {
    let name = match mode { 0 => "BackLoose2", 1 => "BackLoose1", 2 => "BackWin", 5 => "BackLoose5", _ => return Err(format!("menu mode {mode} background not ported")) };
    if family != 15 && family != 16 { return Err("unsupported menu texture family".into()); }
    Ok(format!("m_{family}\\{name}.txr"))
}
pub fn background(game: &Path, mode: u32, family: u32) -> Result<ht2_formats::txr::Texture, String> {
    let name = background_name(mode, family)?;
    let path = game.join("COMMON").join(format!("esc{family}.RES"));
    let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let archive = ht2_formats::res::Res::parse(&data).map_err(|e| e.to_string())?;
    for section in &archive.sections {
        if let ht2_formats::res::Section::Files { entries, .. } = section {
            if let Some(entry) = entries.iter().find(|e| e.name().eq_ignore_ascii_case(&name)) {
                return ht2_formats::txr::decode(entry.data).map_err(|e| format!("{name}: {e}"));
            }
        }
    }
    Err(format!("menu texture missing: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_original_end_screen_names() {
        assert_eq!(background_name(2, 16).unwrap(), "m_16\\BackWin.txr");
        assert_eq!(background_name(5, 15).unwrap(), "m_15\\BackLoose5.txr");
        assert!(background_name(3, 16).is_err());
    }
    #[test]
    #[ignore = "requires HT2_GAME game data"]
    fn render_installed_end_background() {
        let game = std::env::var_os("HT2_GAME").unwrap();
        let t = background(Path::new(&game), 2, 16).unwrap();
        assert_eq!((t.width, t.height), (1600, 1200));
        let instance = wgpu::Instance::default();
        let (_, device, queue) = pollster::block_on(ht2_render::request_device(&instance, None)).unwrap();
        let mut renderer = ht2_render::Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
        renderer.set_movie_frame((t.width, t.height), &t.rgba).unwrap();
        let pixels = renderer.screenshot((800, 600), glam::Mat4::IDENTITY);
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/end-menu.png");
        ht2_render::save_png(&path, (800, 600), &pixels).unwrap();
    }
}
