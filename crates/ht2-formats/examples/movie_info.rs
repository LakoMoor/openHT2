//! Inspect a game's BNE/AVI movie without executing or copying game assets.
fn main() {
    let path = std::env::args_os().nth(1).expect("movie path");
    let data = std::fs::read(path).unwrap();
    let m = ht2_formats::avi::parse(&data).unwrap();
    println!("{}x{} frames={} us/frame={}", m.width, m.height, m.frames, m.microseconds_per_frame);
    for (i, s) in m.streams.iter().enumerate() {
        let packets: Vec<_> = m.packets.iter().filter(|p| p.stream == i).collect();
        println!("{i}: {:?} {:?} scale={} rate={} length={} packets={} bytes={}", String::from_utf8_lossy(&s.kind), String::from_utf8_lossy(&s.handler), s.scale, s.rate, s.length, packets.len(), packets.iter().map(|p| p.data.len()).sum::<usize>());
        if s.kind == *b"auds" {
            let pcm = m.pcm_audio(i).unwrap();
            println!("PCM channels={} rate={} samples={}", pcm.channels, pcm.sample_rate, pcm.samples.len());
            if let Some(path) = std::env::args_os().nth(2) {
                let bytes: Vec<u8> = pcm.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                std::fs::write(path, bytes).unwrap();
            }
        }
    }
}
