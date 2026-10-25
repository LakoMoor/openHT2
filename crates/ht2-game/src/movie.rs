//! Streaming movie decoder backend. FFmpeg is an explicit external runtime dependency;
//! source movies are read in place. No executable path from game data is ever executed.
use std::{ffi::OsStr, io::{Read, ErrorKind}, path::Path, process::{Child, Command, Stdio}, sync::mpsc::{self, Receiver, TryRecvError}, thread::{self, JoinHandle}, time::Duration};

pub struct Frame {
    pub number: u64,
    pub time: Duration,
    pub rgba: Vec<u8>,
}
pub struct Decoder {
    pub width: u32,
    pub height: u32,
    child: Child,
    frames: Option<Receiver<Result<Frame, String>>>,
    reader: Option<JoinHandle<()>>,
    errors: Option<JoinHandle<String>>,
    complete: bool,
}
fn read_frame(reader: &mut impl Read, size: usize) -> Result<Option<Vec<u8>>, String> {
    let mut bytes = vec![0; size];
    let mut used = 0;
    while used < size {
        match reader.read(&mut bytes[used..]) {
            Ok(0) if used == 0 => return Ok(None),
            Ok(0) => return Err(format!("partial decoded movie frame: {used}/{size} bytes")),
            Ok(n) => used += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => {},
            Err(e) => return Err(format!("movie frame read: {e}")),
        }
    }
    Ok(Some(bytes))
}
impl Decoder {
    /// `ffmpeg` comes from application configuration/PATH, never from the movie file.
    pub fn open(ffmpeg: &OsStr, path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("read movie: {e}"))?;
        let movie = ht2_formats::avi::parse(&data).map_err(|e| format!("movie AVI: {e}"))?;
        let video = movie.streams.iter().find(|s| s.kind == *b"vids").ok_or("movie has no video")?;
        if video.rate == 0 || video.scale == 0 || movie.width == 0 || movie.height == 0 {
            return Err("invalid movie dimensions or frame rate".into());
        }
        let size = (movie.width as usize).checked_mul(movie.height as usize).and_then(|n| n.checked_mul(4)).filter(|&n| n <= 64 * 1024 * 1024).ok_or("movie frame is too large")?;
        let (width, height, rate, scale, start) = (movie.width, movie.height, video.rate, video.scale, video.start);
        let mut command = Command::new(ffmpeg);
        command.args(["-nostdin", "-v", "error", "-i"]).arg(path)
            .args(["-map", "0:v:0", "-an", "-sn", "-dn", "-fps_mode", "cfr", "-r"]).arg(format!("{rate}/{scale}"))
            .args(["-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"])
            .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)] {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW: this is a decoder, not a UI.
        }
        let mut child = command.spawn().map_err(|e| format!("start movie decoder: {e}"))?;
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let errors = thread::spawn(move || {
            let mut kept = Vec::new();
            let mut block = [0u8; 4096];
            loop {
                match stderr.read(&mut block) {
                    Ok(0) => break,
                    Ok(n) => { let remaining = 8192usize.saturating_sub(kept.len()); kept.extend_from_slice(&block[..n.min(remaining)]); },
                    Err(e) if e.kind() == ErrorKind::Interrupted => {},
                    Err(_) => break,
                }
            }
            String::from_utf8_lossy(&kept).into_owned()
        });
        // Two queued frames bound decoded memory independently of movie duration.
        let (tx, frames) = mpsc::sync_channel(2);
        let reader = thread::spawn(move || {
            let mut number = 0u64;
            loop {
                match read_frame(&mut stdout, size) {
                    Ok(Some(rgba)) => {
                        let ticks = (u128::from(number) + u128::from(start)) * u128::from(scale);
                        let nanos = ticks * 1_000_000_000 / u128::from(rate);
                        let Ok(nanos) = u64::try_from(nanos) else { let _ = tx.send(Err("movie timestamp overflow".into())); break };
                        if tx.send(Ok(Frame { number, time: Duration::from_nanos(nanos), rgba })).is_err() { break; }
                        number += 1;
                    },
                    Ok(None) => break,
                    Err(e) => { let _ = tx.send(Err(e)); break; },
                }
            }
        });
        Ok(Self { width, height, child, frames: Some(frames), reader: Some(reader), errors: Some(errors), complete: false })
    }
    /// Non-blocking: None means not ready or complete; `complete()` distinguishes them.
    pub fn poll(&mut self) -> Result<Option<Frame>, String> {
        if self.complete { return Ok(None); }
        match self.frames.as_ref().unwrap().try_recv() {
            Ok(frame) => frame.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                let Some(status) = self.child.try_wait().map_err(|e| format!("movie decoder status: {e}"))? else { return Ok(None) };
                self.complete = true;
                if let Some(reader) = self.reader.take() { let _ = reader.join(); }
                let diagnostic = self.errors.take().and_then(|t| t.join().ok()).unwrap_or_default();
                if status.success() { Ok(None) } else { Err(format!("movie decoder failed ({status}): {diagnostic}")) }
            },
        }
    }
    pub fn complete(&self) -> bool { self.complete }
}
impl Drop for Decoder {
    fn drop(&mut self) {
        self.frames.take(); // release a worker blocked on the bounded queue before joining
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(t) = self.reader.take() { let _ = t.join(); }
        if let Some(t) = self.errors.take() { let _ = t.join(); }
    }
}

/// Combined media backend; the original completion/menu policy remains to be ported.
pub struct Player {
    decoder: Decoder,
    pending: Option<Frame>,
    pcm: Option<ht2_formats::avi::PcmAudio>,
    #[cfg(windows)]
    audio: Option<crate::audio::Playback>,
    start: Option<std::time::Instant>,
    duration: Duration,
    pub finished: bool,
}
impl Player {
    pub fn open(ffmpeg: &OsStr, path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("movie audio: {e}"))?;
        let m = ht2_formats::avi::parse(&data).map_err(|e| e.to_string())?;
        let v = m.streams.iter().find(|s| s.kind == *b"vids").ok_or("no movie video")?;
        if v.rate == 0 { return Err("zero movie rate".into()); }
        let duration = Duration::from_secs_f64(f64::from(v.length) * f64::from(v.scale) / f64::from(v.rate));
        let pcm = m.streams.iter().position(|s| s.kind == *b"auds").map(|s| m.pcm_audio(s).map_err(|e| e.to_string())).transpose()?;
        #[cfg(not(windows))]
        if pcm.is_some() { return Err("movie audio backend unavailable on this platform".into()); }
        Ok(Self { decoder: Decoder::open(ffmpeg, path)?, pending: None, pcm, #[cfg(windows)] audio: None, start: None, duration, finished: false })
    }
    pub fn update(&mut self, renderer: &mut ht2_render::Renderer) -> Result<(), String> {
        if self.finished { return Ok(()); }
        if self.pending.is_none() { self.pending = self.decoder.poll()?; }
        if self.start.is_none() {
            let Some(frame) = self.pending.take() else {
                if self.decoder.complete() { return Err("movie produced no frames".into()); }
                return Ok(());
            };
            renderer.set_movie_frame((self.decoder.width, self.decoder.height), &frame.rgba)?;
            #[cfg(windows)]
            if let Some(pcm) = self.pcm.take() { self.audio = Some(crate::audio::Playback::play(pcm)?); }
            self.start = Some(std::time::Instant::now());
        }
        #[cfg(windows)]
        let time = match &self.audio { Some(audio) => audio.position()?, None => self.start.unwrap().elapsed() };
        #[cfg(not(windows))]
        let time = self.start.unwrap().elapsed();
        loop {
            if self.pending.is_none() { self.pending = self.decoder.poll()?; }
            let Some(frame) = &self.pending else { break };
            if frame.time > time { break; }
            let frame = self.pending.take().unwrap();
            renderer.set_movie_frame((self.decoder.width, self.decoder.height), &frame.rgba)?;
        }
        #[cfg(windows)]
        let ended = match &self.audio { Some(audio) => audio.finished()?, None => time >= self.duration };
        #[cfg(not(windows))]
        let ended = time >= self.duration;
        self.finished = self.decoder.complete() && self.pending.is_none() && ended;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn movie_layer_pixels_and_clear() {
        let instance = wgpu::Instance::default();
        let (_, device, queue) = pollster::block_on(ht2_render::request_device(&instance, None)).unwrap();
        let mut renderer = ht2_render::Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
        renderer.set_movie_frame((2, 1), &[255, 0, 0, 255, 0, 0, 255, 255]).unwrap();
        let pixels = renderer.screenshot((4, 4), glam::Mat4::IDENTITY);
        let pixel = |x: usize, y: usize| &pixels[(y * 4 + x) * 4..(y * 4 + x + 1) * 4];
        assert_eq!(pixel(0, 0), [0, 0, 0, 255]);
        assert_eq!(pixel(0, 1), [255, 0, 0, 255]);
        assert_eq!(pixel(3, 1), [0, 0, 255, 255]);
        assert_eq!(pixel(3, 3), [0, 0, 0, 255]);
        assert!(renderer.set_movie_frame((2, 1), &[0]).is_err());
        renderer.clear_movie();
        let scene = renderer.screenshot((4, 4), glam::Mat4::IDENTITY);
        assert_ne!(pixels, scene);
    }
    #[test]
    fn refuses_truncated_raw_frame() {
        assert!(read_frame(&mut &b"123"[..], 4).is_err());
        assert!(read_frame(&mut &b""[..], 4).unwrap().is_none());
        assert_eq!(read_frame(&mut &b"1234"[..], 4).unwrap().unwrap(), b"1234");
    }
    #[test]
    #[ignore = "requires GPU, HT2_FFMPEG, HT2_TEST_MOVIE and HT2_MOVIE_SHOT"]
    fn render_decoded_movie_frame() {
        let exe = std::env::var_os("HT2_FFMPEG").unwrap();
        let path = std::env::var_os("HT2_TEST_MOVIE").unwrap();
        let shot = std::env::var_os("HT2_MOVIE_SHOT").unwrap();
        let mut d = Decoder::open(&exe, Path::new(&path)).unwrap();
        let until = std::time::Instant::now() + Duration::from_secs(10);
        let frame = loop {
            if let Some(frame) = d.poll().unwrap() {
                if frame.number >= 12 { break frame; }
                continue;
            }
            assert!(!d.complete() && std::time::Instant::now() < until);
            thread::sleep(Duration::from_millis(1));
        };
        let instance = wgpu::Instance::default();
        let (_, device, queue) = pollster::block_on(ht2_render::request_device(&instance, None)).unwrap();
        let mut renderer = ht2_render::Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
        renderer.set_movie_frame((d.width, d.height), &frame.rgba).unwrap();
        let pixels = renderer.screenshot((d.width, d.height), glam::Mat4::IDENTITY);
        assert_eq!(pixels, frame.rgba, "native-size movie pixels changed in GPU presentation");
        ht2_render::save_png(Path::new(&shot), (d.width, d.height), &pixels).unwrap();
    }
    #[test]
    #[cfg(windows)]
    #[ignore = "requires GPU, audio device, HT2_FFMPEG and HT2_TEST_MOVIE; audio is silenced"]
    fn synchronized_player_finishes() {
        let exe = std::env::var_os("HT2_FFMPEG").unwrap();
        let path = std::env::var_os("HT2_TEST_MOVIE").unwrap();
        let mut p = Player::open(&exe, Path::new(&path)).unwrap();
        p.pcm.as_mut().unwrap().samples.fill(0);
        let instance = wgpu::Instance::default();
        let (_, device, queue) = pollster::block_on(ht2_render::request_device(&instance, None)).unwrap();
        let mut renderer = ht2_render::Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
        let until = std::time::Instant::now() + p.duration + Duration::from_secs(10);
        while !p.finished {
            p.update(&mut renderer).unwrap();
            assert!(std::time::Instant::now() < until, "A/V player did not finish");
            thread::sleep(Duration::from_millis(5));
        }
        assert!(p.start.unwrap().elapsed() >= p.duration.saturating_sub(Duration::from_millis(100)));
        assert!(p.decoder.complete());
        assert!(p.audio.as_ref().unwrap().finished().unwrap());
    }
    #[test]
    #[ignore = "requires HT2_FFMPEG and HT2_TEST_MOVIE"]
    fn decode_real_movie_and_cancel() {
        let exe = std::env::var_os("HT2_FFMPEG").unwrap();
        let path = std::env::var_os("HT2_TEST_MOVIE").unwrap();
        let mut d = Decoder::open(&exe, Path::new(&path)).unwrap();
        let data = std::fs::read(&path).unwrap();
        let m = ht2_formats::avi::parse(&data).unwrap();
        let limit = std::time::Instant::now() + Duration::from_secs(30);
        let mut count = 0;
        while !d.complete() {
            if let Some(frame) = d.poll().unwrap() {
                assert_eq!(frame.number, count);
                assert_eq!(frame.rgba.len(), d.width as usize * d.height as usize * 4);
                assert_eq!(frame.time, Duration::from_nanos(count * 1_000_000_000 / 12));
                count += 1;
            } else { thread::sleep(Duration::from_millis(1)); }
            assert!(std::time::Instant::now() < limit, "decoder did not finish");
        }
        assert_eq!(count, u64::from(m.frames));
        let blocked = Decoder::open(&exe, Path::new(&path)).unwrap();
        thread::sleep(Duration::from_millis(100));
        drop(blocked); // worker may be blocked sending the third frame; must terminate
    }
}
