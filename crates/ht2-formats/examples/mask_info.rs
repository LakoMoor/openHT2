fn main() {
 let data=std::fs::read(std::env::args_os().nth(1).unwrap()).unwrap();
 let r=ht2_formats::res::Res::parse(&data).unwrap();
 let out=std::path::PathBuf::from(std::env::args_os().nth(2).unwrap());
 std::fs::create_dir_all(&out).unwrap();
 for section in r.sections {if let ht2_formats::res::Section::Files{entries,..}=section {for e in entries {let name=e.name().replace('\\',"/");let base=name.rsplit('/').next().unwrap();if base.starts_with("icon") && base.ends_with(".msk") {
 let mask=ht2_formats::msk::Mask::parse(e.data).unwrap();
 let raw:Vec<u8>=mask.pixels.iter().flat_map(|p|p.unwrap_or(0xdead).to_le_bytes()).collect();
 std::fs::write(out.join(base),e.data).unwrap();std::fs::write(out.join(format!("{base}.raw")),raw).unwrap();
 println!("{base} {}x{}",mask.width,mask.height);
 }}}}
}
