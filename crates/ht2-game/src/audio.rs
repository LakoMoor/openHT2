//! PCM output for movie playback. This backend owns the buffer until the device releases it.
//! It does not alter the system volume or silently resample the source.

/// Decode uncompressed PCM WAVE into the same signed 16-bit representation
/// used by the audio backend. RIFF chunks can appear in arbitrary order.
pub fn decode_wave(data:&[u8])->Result<ht2_formats::avi::PcmAudio,String> {
    let u16at=|b:&[u8],i:usize|->u16 {u16::from_le_bytes(b[i..i+2].try_into().unwrap())};
    let u32at=|b:&[u8],i:usize|->u32 {u32::from_le_bytes(b[i..i+4].try_into().unwrap())};
    if data.len()<12 || &data[..4]!=b"RIFF" || &data[8..12]!=b"WAVE" {return Err("not a RIFF WAVE".into());}
    let end=(u32at(data,4) as usize).checked_add(8).ok_or("RIFF size overflow")?;
    if end<12 || end>data.len(){return Err("truncated RIFF".into());}
    let (mut fmt,mut samples)=(None,None);let mut offset=12usize;
    while offset<end {
        if end-offset<8{return Err("truncated chunk header".into());}
        let size=u32at(data,offset+4) as usize;let start=offset+8;
        let stop=start.checked_add(size).ok_or("chunk size overflow")?;
        if stop>end{return Err("truncated chunk".into());}
        match &data[offset..offset+4] {b"fmt "=>fmt=Some(&data[start..stop]),b"data"=>samples=Some(&data[start..stop]),_=>{}}
        offset=stop.checked_add(size&1).ok_or("chunk padding overflow")?;
        if offset>end{return Err("truncated chunk padding".into());}
    }
    let fmt=fmt.ok_or("missing fmt")?;let bytes=samples.ok_or("missing data")?;
    if fmt.len()<16{return Err("short PCM format".into());}
    let channels=u16at(fmt,2);let rate=u32at(fmt,4);let bits=u16at(fmt,14);let align=u16at(fmt,12);
    if u16at(fmt,0)!=1 || !(1..=2).contains(&channels) || rate==0 || !matches!(bits,8|16) || align!=channels*(bits/8) || bytes.len()%usize::from(align)!=0 {return Err("unsupported PCM format".into());}
    let samples=if bits==8 {bytes.iter().map(|&b|(i16::from(b)-128)*256).collect()}else{bytes.chunks_exact(2).map(|b|i16::from_le_bytes([b[0],b[1]])).collect()};
    Ok(ht2_formats::avi::PcmAudio{channels,sample_rate:rate,samples})
}

/// The original scene binds antiradarSound and falls back to hornSound.
/// Read only the installed archive; no original audio is included in the repo.
pub fn load_radar_sound(game:&std::path::Path)->Result<ht2_formats::avi::PcmAudio,String> {
    let path=game.join("COMMON").join("common1.res");let bytes=std::fs::read(&path).map_err(|e|format!("{}: {e}",path.display()))?;
    let archive=ht2_formats::res::Res::parse(&bytes).map_err(|e|e.to_string())?;
    let mut radar=None;let mut horn=None;
    for section in archive.sections {if let ht2_formats::res::Section::Files{kind,entries}=section {if kind=="SOUNDFILES" {for entry in entries {
        if entry.name().eq_ignore_ascii_case("snd\\antiradar.wav") {radar=Some(entry.data);}
        if entry.name().eq_ignore_ascii_case("snd\\horn.wav") {horn=Some(entry.data);}
    }}}}
    decode_wave(radar.or(horn).ok_or("radar and horn sound not found")?)
}

#[cfg(test)]
mod wave_tests {
    use super::*;
    #[test]
    fn decodes_unsigned_pcm_and_rejects_truncation() {
        let mut data=b"RIFF".to_vec();data.extend(40u32.to_le_bytes());data.extend(b"WAVEfmt ");data.extend(16u32.to_le_bytes());
        data.extend(1u16.to_le_bytes());data.extend(1u16.to_le_bytes());data.extend(11025u32.to_le_bytes());data.extend(11025u32.to_le_bytes());data.extend(1u16.to_le_bytes());data.extend(8u16.to_le_bytes());data.extend(b"data");data.extend(4u32.to_le_bytes());data.extend([0,128,255,64]);
        let pcm=decode_wave(&data).unwrap();assert_eq!((pcm.channels,pcm.sample_rate),(1,11025));assert_eq!(pcm.samples,[-32768,0,32512,-16384]);
        assert!(decode_wave(&data[..data.len()-1]).is_err());
    }
    #[test]
    #[ignore="requires HT2_GAME installed game data"]
    fn loads_installed_radar_sound() {
        let game=std::path::PathBuf::from(std::env::var("HT2_GAME").unwrap());let pcm=load_radar_sound(&game).unwrap();
        assert_eq!((pcm.channels,pcm.sample_rate),(1,11025));assert!(!pcm.samples.is_empty());
    }
}
#[cfg(windows)]
mod windows {
    use ht2_formats::avi::PcmAudio;
    use std::{mem::size_of, ptr::{null, null_mut}};
    use windows_sys::Win32::{Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT}, Media::Audio::*, System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject}};

    pub struct Playback {
        device: HWAVEOUT,
        event: HANDLE,
        header: Box<WAVEHDR>,
        samples: Box<[i16]>,
        prepared: bool,
        channels: u16,
        rate: u32,
    }
    fn check(code: u32, operation: &str) -> Result<(), String> {
        if code == 0 { Ok(()) } else { Err(format!("{operation}: WinMM error {code}")) }
    }
    fn format(pcm: &PcmAudio) -> Result<WAVEFORMATEX, String> {
        if pcm.channels == 0 || pcm.channels > 2 || pcm.sample_rate == 0 || pcm.samples.len() % usize::from(pcm.channels) != 0 {
            return Err("invalid PCM channels, rate or sample frame".into());
        }
        let align = pcm.channels * 2;
        let rate = pcm.sample_rate.checked_mul(u32::from(align)).ok_or("PCM rate overflow")?;
        let _ = u32::try_from(pcm.samples.len().checked_mul(2).ok_or("PCM size overflow")?).map_err(|_| "PCM buffer too large")?;
        if pcm.samples.is_empty() { return Err("empty PCM buffer".into()); }
        Ok(WAVEFORMATEX { wFormatTag: 1, nChannels: pcm.channels, nSamplesPerSec: pcm.sample_rate, nAvgBytesPerSec: rate, nBlockAlign: align, wBitsPerSample: 16, cbSize: 0 })
    }
    impl Playback {
        pub fn play(pcm: PcmAudio) -> Result<Self, String> {
            let format = format(&pcm)?;
            let event = unsafe { CreateEventW(null(), 1, 0, null()) };
            if event.is_null() { return Err("cannot create audio completion event".into()); }
            let mut device = null_mut();
            if let Err(e) = check(unsafe { waveOutOpen(&mut device, WAVE_MAPPER, &format, event as usize, 0, CALLBACK_EVENT) }, "waveOutOpen") {
                unsafe { CloseHandle(event); }
                return Err(e);
            }
            let mut samples = pcm.samples.into_boxed_slice();
            let header = Box::new(WAVEHDR { lpData: samples.as_mut_ptr().cast(), dwBufferLength: (samples.len() * 2) as u32, dwBytesRecorded: 0, dwUser: 0, dwFlags: 0, dwLoops: 0, lpNext: null_mut(), reserved: 0 });
            let mut out = Self { device, event, header, samples, prepared: false, channels: pcm.channels, rate: pcm.sample_rate };
            check(unsafe { waveOutPrepareHeader(out.device, &mut *out.header, size_of::<WAVEHDR>() as u32) }, "waveOutPrepareHeader")?;
            out.prepared = true;
            // Opening a device may signal the same event: clear it before submitting audio.
            if unsafe { ResetEvent(event) } == 0 { return Err("cannot reset audio completion event".into()); }
            check(unsafe { waveOutWrite(out.device, &mut *out.header, size_of::<WAVEHDR>() as u32) }, "waveOutWrite")?;
            Ok(out)
        }
        pub fn finished(&self) -> Result<bool, String> {
            match unsafe { WaitForSingleObject(self.event, 0) } {
                WAIT_OBJECT_0 => Ok(true), WAIT_TIMEOUT => Ok(false), _ => Err("audio completion wait failed".into()),
            }
        }
        pub fn pause(&self) -> Result<(), String> { check(unsafe { waveOutPause(self.device) }, "waveOutPause") }
        pub fn resume(&self) -> Result<(), String> { check(unsafe { waveOutRestart(self.device) }, "waveOutRestart") }
        /// Device sample position is the video clock; rendering speed must not alter audio.
        pub fn position(&self) -> Result<std::time::Duration, String> {
            use windows_sys::Win32::Media::{MMTIME, TIME_SAMPLES, TIME_BYTES, TIME_MS};
            let mut time: MMTIME = unsafe { std::mem::zeroed() };
            time.wType = TIME_SAMPLES;
            check(unsafe { waveOutGetPosition(self.device, &mut time, size_of::<MMTIME>() as u32) }, "waveOutGetPosition")?;
            let seconds = unsafe { match time.wType {
                TIME_SAMPLES => f64::from(time.u.sample) / f64::from(self.rate),
                TIME_BYTES => f64::from(time.u.cb) / (f64::from(self.rate) * f64::from(self.channels) * 2.0),
                TIME_MS => f64::from(time.u.ms) / 1000.0,
                _ => return Err("unsupported audio position unit".into()),
            }};
            Ok(std::time::Duration::from_secs_f64(seconds))
        }
    }
    impl Drop for Playback {
        fn drop(&mut self) {
            // Reset returns all buffers before unprepare/free. On a driver failure retain
            // the allocations rather than free memory which might still be in use.
            let reset = unsafe { waveOutReset(self.device) };
            let release = if self.prepared && reset == 0 { unsafe { waveOutUnprepareHeader(self.device, &mut *self.header, size_of::<WAVEHDR>() as u32) } } else { reset };
            if release != 0 {
                let header = std::mem::replace(&mut self.header, Box::new(unsafe { std::mem::zeroed() }));
                let samples = std::mem::replace(&mut self.samples, Box::new([]));
                std::mem::forget((header, samples));
                // Keep the callback event alive too if ownership could not be returned.
                eprintln!("audio driver did not release its buffer: {release}");
                return;
            }
            unsafe { waveOutClose(self.device); CloseHandle(self.event); }
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn rejects_bad_pcm_before_opening_device() {
            for pcm in [PcmAudio { channels: 0, sample_rate: 22050, samples: vec![0] }, PcmAudio { channels: 2, sample_rate: 22050, samples: vec![0] }, PcmAudio { channels: 1, sample_rate: 0, samples: vec![0] }] {
                assert!(Playback::play(pcm).is_err());
            }
        }
        #[test]
        #[ignore = "opens the default audio device and plays 50 ms of silence"]
        fn silent_device_smoke() {
            let audio = Playback::play(PcmAudio { channels: 1, sample_rate: 22050, samples: vec![0; 1103] }).unwrap();
            audio.pause().unwrap();
            audio.resume().unwrap();
            let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while !audio.finished().unwrap() {
                assert!(std::time::Instant::now() < until, "audio never completed");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
#[cfg(windows)]
pub use windows::Playback;
