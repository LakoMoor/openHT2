//! End-menu backgrounds and company-rating text from the user's game data.
//! Buttons and multiplayer outcomes still require porting.
use std::path::Path;

#[derive(Default)]
pub struct Buttons {
    pub cursor: Option<(i32,i32)>,
    pub captured: Option<usize>,
    pub bits: [u32;3],
    /// Routed actions awaiting their original menu transition implementation.
    pub action: Option<u32>,
}
impl Buttons {
    pub fn event(&mut self, rects: &[(i32,i32,i32,i32)], message: u32) {
        let inside = |i: usize| self.cursor.is_some_and(|(x,y)| {
            let r = rects[i]; x >= r.0 && y >= r.1 && x <= r.2 && y <= r.3
        });
        let target = self.captured.or_else(|| (0..rects.len()).find(|&i| inside(i)));
        if let Some(i) = target {
            let hit = inside(i);
            if message == 0x200 { button_move(&mut self.bits[i],self.captured == Some(i),hit); }
            else {
                let id = if rects.len() == 3 { [0xa8,0xac,0xa3][i] } else { [0xa8,0xa3][i] };
                let routed = button_mouse(id,false,&mut self.bits[i],message,hit);
                if message == 0x201 { self.captured = Some(i); }
                if message == 0x202 {
                    if self.captured == Some(i) && routed & 0xe000 == 0x8000 { self.action = Some(id); }
                    self.captured = None;
                }
            }
        }
    }
}

pub fn button_rects(game: &Path, mode: u32, family: u32, size: (u32,u32)) -> Result<Vec<(i32,i32,i32,i32)>,String> {
    let t = background(game,mode,family)?;
    let offset = ((t.width as i32-size.0 as i32)/2,(t.height as i32-size.1 as i32)/2);
    let names: &[&str] = if mode == 2 { &["ow_rest","ow_cont","ow_main"] } else { &["o_rest","o_main"] };
    let sizes: Vec<_> = names.iter().map(|n| {
        let b = resource(game,family,&format!("m_{family}\\{n}_a.txr"))?;
        let t = ht2_formats::txr::decode(&b).map_err(|e|e.to_string())?;
        Ok((t.width as i32-1,t.height as i32-1))
    }).collect::<Result<_,String>>()?;
    let widths: Vec<_> = sizes.iter().map(|p|p.0).collect();
    Ok(button_positions(&widths,mode==2).iter().zip(sizes).map(|(x,(w,h))| {
        let (x,y)=(589-offset.0+x,687-offset.1);(x,y,x+w,y+h)
    }).collect())
}

/// Mouse-message branch of the original button (`0x5bfa80`). `inside` comes
/// from the widget's hit test; the returned word is the original routed message.
pub fn button_mouse(id: u32, disabled: bool, bits: &mut u32, message: u32,
    inside: bool) -> u32 {
    match message {
        0x201 => {
            if disabled { return message; }
            *bits |= 2;
            id | 0xc000
        }
        0x202 => {
            if disabled { return message; }
            if inside { *bits &= !2; id | 0x8000 } else { id | 0xa000 }
        }
        _ => message,
    }
}

/// Captured mouse movement (`0x5bfb30`); ordinary hover only returns a hit.
pub fn button_move(bits: &mut u32, captured: bool, inside: bool) -> bool {
    if captured {
        if inside { *bits |= 2; } else { *bits &= !2; }
        true
    } else { inside }
}

/// `0x51ef...` builds groups by prepending children; `0x5bdee0` lays them out.
pub fn button_positions(widths: &[i32], winning: bool) -> Vec<i32> {
    let gap = if winning { 0x91 } else { 0x122 } - widths[0];
    let mut x = 0;
    widths.iter().map(|&w| { let at = x; x += w + gap; at }).collect()
}

fn compose_buttons(game: &Path, mode: u32, family: u32, size: (u32,u32),
    offset: (i32,i32), rgba: &mut [u8], input: Option<&Buttons>) -> Result<(), String> {
    let names: &[&str] = if mode == 2 { &["ow_rest", "ow_cont", "ow_main"] }
        else { &["o_rest", "o_main"] };
    let textures: Vec<_> = names.iter().map(|name| {
        let data = resource(game, family, &format!("m_{family}\\{name}_a.txr"))?;
        ht2_formats::txr::decode(&data).map_err(|e| e.to_string())
    }).collect::<Result<_,String>>()?;
    let widths: Vec<_> = textures.iter().map(|t| t.width as i32 - 1).collect();
    for (i,(base, x)) in textures.iter().zip(button_positions(&widths, mode == 2)).enumerate() {
        let pressed = input.is_some_and(|b|b.bits[i]&2!=0);
        let alternate = if pressed {
            Some(ht2_formats::txr::decode(&resource(game,family,&format!("m_{family}\\{}_ap.txr",names[i]))?).map_err(|e|e.to_string())?)
        } else { None };
        let t = alternate.as_ref().unwrap_or(base);
        let (x,y) = (0x24d - offset.0 + x, 0x2af - offset.1);
        for dy in 0..t.height as i32 {
            for dx in 0..t.width as i32 {
                let (px,py) = (x+dx,y+dy);
                if px < 0 || py < 0 || px >= size.0 as i32 || py >= size.1 as i32 { continue; }
                let src = (dy as usize * t.width as usize + dx as usize)*4;
                let dst = (py as usize * size.0 as usize + px as usize)*4;
                rgba[dst..dst+4].copy_from_slice(&t.rgba[src..src+4]);
            }
        }
    }
    Ok(())
}

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
    ht2_formats::txr::decode(&resource(game, family, &name)?).map_err(|e| e.to_string())
}
fn resource(game: &Path, family: u32, name: &str) -> Result<Vec<u8>, String> {
    let path = game.join("COMMON").join(format!("esc{family}.RES"));
    let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let archive = ht2_formats::res::Res::parse(&data).map_err(|e| e.to_string())?;
    for section in &archive.sections {
        if let ht2_formats::res::Section::Files { entries, .. } = section {
            if let Some(entry) = entries.iter().find(|e| e.name().eq_ignore_ascii_case(&name)) {
                return Ok(entry.data.to_vec());
            }
        }
    }
    Err(format!("menu resource missing: {name}"))
}

/// Centered backdrop and rating text, following the single-player branch of
/// `0x51f760`. Buttons and multiplayer outcomes are not yet composed.
pub fn screen(game: &Path, mode: u32, family: u32, size: (u32, u32),
    world: Option<&ht2_sim::world::World>) -> Result<Vec<u8>, String> {
    screen_input(game,mode,family,size,world,None)
}
pub fn screen_input(game: &Path, mode: u32, family: u32, size: (u32, u32),
    world: Option<&ht2_sim::world::World>, input: Option<&Buttons>) -> Result<Vec<u8>, String> {
    let t = background(game, mode, family)?;
    let (w, h) = size;
    if w == 0 || h == 0 || w > 8192 || h > 8192 { return Err("unsupported menu surface size".into()); }
    let ox = (t.width as i32 - w as i32) / 2;
    let oy = (t.height as i32 - h as i32) / 2;
    let mut rgba = vec![0; w as usize * h as usize * 4];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let dst = (y as usize * w as usize + x as usize) * 4;
            rgba[dst + 3] = 255;
            let (sx, sy) = (x + ox, y + oy);
            if sx >= 0 && sy >= 0 && sx < t.width as i32 && sy < t.height as i32 {
                let src = (sy as usize * t.width as usize + sx as usize) * 4;
                rgba[dst..dst + 4].copy_from_slice(&t.rgba[src..src + 4]);
            }
        }
    }
    compose_buttons(game, mode, family, size, (ox,oy), &mut rgba,input)?;
    if !matches!(mode, 1 | 2 | 5) { return Ok(rgba); }
    let Some(world) = world else { return Ok(rgba) };
    let Some(human) = world.players.iter().position(|p| p.kind == 1) else { return Ok(rgba) };
    let data = std::fs::read(game.join("MENU.FON")).map_err(|e| e.to_string())?;
    let font = ht2_formats::font::Font::parse(&data).map_err(|e| e.to_string())?;
    let players: Vec<_> = world.players.iter().map(|p| (p.kind, p.flags, p.share)).collect();
    for (row, index) in rating_rows(&players, human, 3).into_iter().enumerate() {
        let p = &world.players[index];
        let name = p.name.and_then(|i| world.names.get(i)).map_or("", |n| n.name.as_str());
        // Invert the same byte decoder used when reading names.sch. Unsupported
        // Unicode cannot occur in names parsed from the installed byte strings.
        let name: Vec<u8> = name.chars().map(|c| (0u16..=255).find(|&b|
            ht2_formats::sdte::cp1251_to_string(&[b as u8]).starts_with(c)).unwrap_or(63) as u8).collect();
        let runs = rating_text((0x26e - ox, 0x1b8 - oy + row as i32 * 0x21),
            &name, p.share, p.members.len() as i32, p.fleet);
        for run in runs {
            let limit = run.limit.unwrap_or_else(|| font.text_width(&run.bytes, true).wrapping_add(1));
            let mask = font.draw_mask((w as u16, h as u16), run.origin, limit, &run.bytes, true);
            for (pixel, mask) in rgba.chunks_exact_mut(4).zip(mask) {
                // Entry 6 of the base palette (0x5ba240). Dynamic palette changes
                // and the DirectDraw font path still require differential checking.
                if mask != 0 { pixel.copy_from_slice(&[0, 255, 0, 255]); }
            }
        }
        if p.flags & 0x1000 != 0 {
            if let Some(logo) = p.logo.filter(|&i| i < 8) {
                let bytes = resource(game, family, &format!("m_{family}\\icon{}.msk", logo + 1))?;
                let mask = ht2_formats::msk::Mask::parse(&bytes).map_err(|e| e.to_string())?;
                let (x, y) = (0x26e - ox, 0x1b8 - oy + row as i32 * 0x21);
                for dy in 0..i32::from(mask.height) {
                    for dx in 0..i32::from(mask.width) {
                        let (px, py) = (x + dx, y + dy);
                        if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 { continue; }
                        if let Some(v) = mask.pixels[dy as usize * usize::from(mask.width) + dx as usize] {
                            // Resource families 15/16 carry RGB555/RGB565 respectively.
                            let (r, g, b) = if family == 15 { ((v >> 10) & 31, (v >> 5) & 31, v & 31) }
                                else { ((v >> 11) & 31, (v >> 5) & 63, v & 31) };
                            let rgb = [(r * 255 / 31) as u8, (g * 255 / if family == 15 { 31 } else { 63 }) as u8, (b * 255 / 31) as u8, 255];
                            let dst = (py as usize * w as usize + px as usize) * 4;
                            rgba[dst..dst + 4].copy_from_slice(&rgb);
                        }
                    }
                }
            }
        }
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GPU"]
    fn renderer_accepts_prepared_scene_viewport() {
        let instance=wgpu::Instance::default();
        let (_,device,queue)=pollster::block_on(ht2_render::request_device(&instance,None)).unwrap();
        let mut renderer=ht2_render::Renderer::new(device,queue,wgpu::TextureFormat::Rgba8Unorm);
        let target=renderer.device.create_texture(&wgpu::TextureDescriptor {
            label:Some("prepared viewport integration"),size:wgpu::Extent3d{width:64,height:48,depth_or_array_layers:1},
            mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::Rgba8Unorm,
            usage:wgpu::TextureUsages::RENDER_ATTACHMENT,view_formats:&[],
        });
        let view=target.create_view(&Default::default());
        renderer.device.push_error_scope(wgpu::ErrorFilter::Validation);
        renderer.draw_viewport(&view,(64,48),glam::Mat4::IDENTITY,[7,5,40,30]).unwrap();
        assert!(renderer.draw_viewport(&view,(64,48),glam::Mat4::IDENTITY,[63,0,2,2]).is_err());
        assert!(renderer.draw_viewport(&view,(64,48),glam::Mat4::IDENTITY,[0,0,0,2]).is_err());
        renderer.device.poll(wgpu::Maintain::Wait);
        assert!(pollster::block_on(renderer.device.pop_error_scope()).is_none());
    }
    #[test]
    fn routed_click_requires_capture_and_inside_release() {
        let rects=[(0,0,20,20),(30,0,50,20),(60,0,80,20)];
        let mut b=Buttons::default();
        b.cursor=Some((40,10));
        b.event(&rects,0x202);
        assert_eq!(b.action,None);
        b.event(&rects,0x201);
        assert_eq!(b.bits[1]&2,2);
        b.cursor=Some((90,10));
        b.event(&rects,0x200);
        assert_eq!(b.bits[1]&2,0);
        b.event(&rects,0x202);
        assert_eq!(b.action,None);
        b.cursor=Some((40,10));
        b.event(&rects,0x201);
        b.event(&rects,0x202);
        assert_eq!(b.action,Some(0xac));
        assert_eq!(b.captured,None);
    }
    #[test]
    fn button_mouse_matches_original() {
        for line in include_str!("../../../tools/emu/golden_button_mouse.txt").lines() {
            let p: Vec<u32> = line.split_whitespace().map(|s| s.parse().unwrap()).collect();
            let mut bits = p[2];
            assert_eq!(button_mouse(p[0],p[1]!=0,&mut bits,p[3],p[4]!=0),p[5]);
            assert_eq!(bits,p[6]);
        }
        for line in include_str!("../../../tools/emu/golden_button_move.txt").lines() {
            let p: Vec<u32> = line.split_whitespace().map(|s| s.parse().unwrap()).collect();
            let mut bits = p[0];
            assert_eq!(button_move(&mut bits,p[1]!=0,p[2]!=0),p[3]!=0);
            assert_eq!(bits,p[4]);
        }
    }
    #[test]
    fn button_layout_matches_original() {
        for line in include_str!("../../../tools/emu/golden_end_buttons.tsv").lines() {
            let p: Vec<_> = line.split('\t').collect();
            let nums = |s: &str| s.split(',').map(|x| x.parse::<i32>().unwrap()).collect::<Vec<_>>();
            assert_eq!(button_positions(&nums(p[1]), p[0] == "1"), nums(p[2]));
        }
    }
    #[test]
    #[ignore = "requires HT2_GAME game data and GPU"]
    fn render_rating_from_world() {
        let game = std::env::var_os("HT2_GAME").unwrap();
        let game = Path::new(&game);
        let mut world = ht2_sim::world::World::new(std::rc::Rc::new(Default::default()),
            vec![], vec![], Default::default(), vec![], Default::default(), 0);
        world.create_players_regular(&[0; 12], None);
        world.names.push(ht2_sim::world::DriverName { name: "Иван".into(), role: 1, care: 1.0, wander: 1.0 });
        world.players[0].name = Some(0);
        world.players[0].share = 0.51;
        world.players[0].fleet = 5100.0;
        world.players[0].flags |= 0x1000;
        world.players[0].logo = Some(0);
        let mut ai = world.players[0].clone();
        ai.kind = 2;
        ai.share = 0.49;
        ai.fleet = 4900.0;
        world.players.push(ai);
        let rgba = screen(game, 2, 16, (800,600), Some(&world)).unwrap();
        let mut buttons=Buttons::default();
        buttons.bits[1]=2;
        let pressed=screen_input(game,2,16,(800,600),Some(&world),Some(&buttons)).unwrap();
        assert_ne!(pressed,rgba);
        let base = screen(game, 2, 16, (800,600), None).unwrap();
        assert!(rgba.chunks_exact(4).zip(base.chunks_exact(4)).filter(|(a,b)| a != b).count() > 100);
        let t = background(game, 2, 16).unwrap();
        assert_eq!(&base[..4], &t.rgba[(300*1600+400)*4..(300*1600+400)*4+4]);
        let instance = wgpu::Instance::default();
        let (_, device, queue) = pollster::block_on(ht2_render::request_device(&instance, None)).unwrap();
        let mut renderer = ht2_render::Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
        renderer.set_movie_frame((800,600), &rgba).unwrap();
        let pixels = renderer.screenshot((800,600), glam::Mat4::IDENTITY);
        assert_eq!(pixels, rgba);
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/end-rating.png");
        ht2_render::save_png(&path, (800,600), &pixels).unwrap();
    }
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
