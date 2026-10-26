fn main() {
    let path = std::env::args_os().nth(1).expect("font path");
    let data = std::fs::read(path).unwrap();
    let font = ht2_formats::font::Font::parse(&data).unwrap();
    if std::env::args().any(|a| a == "--draw") {
        use std::io::BufRead;
        for line in std::io::stdin().lock().lines() {
            let line = line.unwrap();
            let fields: Vec<_> = line.split(' ').collect();
            let number = |i: usize| fields[i].parse::<i32>().unwrap();
            let bytes: Vec<u8> = fields[4].as_bytes().chunks_exact(2).map(|p|
                u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap()).collect();
            let pixels = font.draw_mask((80, 64), (number(0), number(1)), number(2), &bytes, number(3) != 0);
            println!("{}", pixels.iter().map(|p| if *p == 0 { '0' } else { '1' }).collect::<String>());
        }
        return;
    }
    if std::env::args().any(|a| a == "--measure") {
        use std::io::BufRead;
        for line in std::io::stdin().lock().lines() {
            let line = line.unwrap();
            let bytes: Vec<u8> = line.as_bytes().chunks_exact(2).map(|pair|
                u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()).collect();
            println!("{} {}", font.text_width(&bytes, true), font.text_width(&bytes, false));
        }
        return;
    }
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
