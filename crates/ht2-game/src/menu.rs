//! End-menu backgrounds loaded from the user's game data. Widgets and text are still
//! missing; displaying the backdrop is not a complete original menu.
use std::path::Path;

/// Original `0x574310`: at most 60 eligible companies, descending share, with
/// the human first on ties. Reserve the final visible row for the human if needed.
pub fn rating_rows(players: &[(u32, u32, f64)], human: usize, limit: usize) -> Vec<usize> {
    let mut order: Vec<usize> = players.iter().enumerate()
        .filter(|(_, p)| matches!(p.0, 1 | 2) && p.1 & 0x800 == 0)
        .take(60).map(|(i, _)| i).collect();
    if order.len() > 1 {
        loop {
            let mut changed = false;
            for i in 1..order.len() {
                let (a, b) = (players[order[i - 1]], players[order[i]]);
                if a.2 < b.2 || (a.2 == b.2 && b.0 == 1) {
                    order.swap(i - 1, i);
                    changed = true;
                }
            }
            if !changed { break; }
        }
    }
    let mut rows = Vec::new();
    let mut seen_human = false;
    for &i in &order {
        if rows.len() >= limit { break; }
        rows.push(i);
        seen_human |= players[i].0 == 1;
        if limit >= 2 && rows.len() == limit - 1 && !seen_human { break; }
    }
    if !seen_human { rows.push(human); }
    rows
}

/// A byte-string draw request from `0x574190`; font is the menu FON and color
/// is palette entry 6. Palette conversion and company logos belong to composition.
#[derive(Debug, PartialEq)]
pub struct RatingText {
    pub origin: (i32, i32),
    pub limit: Option<i32>,
    pub bytes: Vec<u8>,
}

/// Text columns of one company-rating row. The caller supplies the original
/// byte-encoded name; the share and fleet are `getPlayerData` +0x68/+0x70.
pub fn rating_text(origin: (i32, i32), name: &[u8], share: f64, members: i32,
    fleet: f64) -> [RatingText; 4] {
    let (x, y) = origin;
    let percentage = (share * 100.0).trunc() as i32;
    [
        RatingText { origin: (x.wrapping_add(0x2d), y.wrapping_add(0x17)), limit: Some(0x46), bytes: name.to_vec() },
        RatingText { origin: (x.wrapping_add(0x87), y.wrapping_add(0x17)), limit: None, bytes: format!("{percentage}%").into_bytes() },
        RatingText { origin: (x.wrapping_add(0xdc), y.wrapping_add(0x17)), limit: None, bytes: members.to_string().into_bytes() },
        RatingText { origin: (x.wrapping_add(0x10e), y.wrapping_add(0x17)), limit: None, bytes: format!("{fleet:4.1}").into_bytes() },
    ]
}

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
    fn company_order_matches_original() {
        for line in include_str!("../../../tools/emu/golden_rating_rows.tsv").lines() {
            let fields: Vec<_> = line.split('\t').collect();
            let parse = |s: &str| s.parse::<usize>().unwrap();
            let players: Vec<_> = fields[2].split(';').filter(|s| !s.is_empty()).map(|s| {
                let p: Vec<_> = s.split(',').collect();
                (p[0].parse().unwrap(), p[1].parse().unwrap(), p[2].parse().unwrap())
            }).collect();
            let expected: Vec<_> = fields[3].split(',').filter(|s| !s.is_empty()).map(parse).collect();
            assert_eq!(rating_rows(&players, parse(fields[0]), parse(fields[1])), expected);
        }
    }
    #[test]
    fn rating_columns_match_original() {
        for line in include_str!("../../../tools/emu/golden_rating_text.tsv").lines() {
            let p: Vec<_> = line.split('\t').collect();
            assert_eq!(p.len(), 9);
            let x = p[0].parse().unwrap();
            let y = p[1].parse().unwrap();
            let runs = rating_text((x,y), b"TEST", p[2].parse().unwrap(), p[3].parse().unwrap(), p[4].parse().unwrap());
            for (run, expected) in runs.iter().zip(&p[5..]) {
                let fields: Vec<_> = expected.splitn(4, ',').collect();
                assert_eq!(run.origin, (fields[0].parse().unwrap(), fields[1].parse().unwrap()));
                assert_eq!(run.limit.unwrap_or(-1), fields[2].parse::<i32>().unwrap());
                assert_eq!(run.bytes, fields[3].as_bytes());
            }
        }
    }
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
