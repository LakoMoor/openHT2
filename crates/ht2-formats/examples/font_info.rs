fn main() {
    let path = std::env::args_os().nth(1).expect("font path");
    let data = std::fs::read(path).unwrap();
    let font = ht2_formats::font::Font::parse(&data).unwrap();
    if std::env::args().any(|a| a == "--dump") {
        for (code, g) in font.glyphs.iter().enumerate() {
            let bits: String = g.coverage.iter().map(|&p| if p == 0 { '0' } else { '1' }).collect();
            println!("{code} {} {} {bits}", g.left, g.advance);
        }
        return;
    }
    println!("cell={} height={} glyphs={}", font.cell_width, font.height, font.glyphs.len());
    for code in [32usize, 65, 192, 255] {
        let g = &font.glyphs[code];
        println!("{code}: left={} advance={} pixels={}", g.left, g.advance, g.coverage.iter().filter(|&&p| p != 0).count());
    }
}
