use ht2_formats::{b3d, res, tech, txr, way};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [cmd, path, ..] = args.as_slice() else {
        eprintln!("usage: ht2-tool <res-ls|way-ls> <file>");
        return ExitCode::from(2);
    };
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return fail(format!("{path}: {e}")),
    };
    let r = match cmd.as_str() {
        "res-ls" => res_ls(&data),
        "way-ls" => way_ls(&data),
        "way-rooms" => way_rooms(&data),
        "b3d-ls" => b3d_ls(&data),
        "tech-ls" => tech_ls(&data),
        "sdte-cat" => ht2_formats::sdte::decrypt(&data).map_err(|e| e.to_string()).map(|b| println!("{}", ht2_formats::sdte::cp1251_to_string(&b))),
        "b3d-batches" => b3d_batches(&data),
        "txr-png" => match args.get(2) {
            Some(out) => txr_png(&data, out),
            None => Err("usage: ht2-tool txr-png <file.res> <out-dir>".into()),
        },
        _ => Err(format!("unknown command {cmd}")),
    };
    r.map_or_else(fail, |()| ExitCode::SUCCESS)
}

fn fail(msg: String) -> ExitCode {
    eprintln!("error: {msg}");
    ExitCode::FAILURE
}

fn res_ls(data: &[u8]) -> Result<(), String> {
    let r = res::Res::parse(data).map_err(|e| e.to_string())?;
    for s in &r.sections {
        match s {
            res::Section::Files { kind, entries } => {
                println!("{kind} {}", entries.len());
                for e in entries {
                    let dim = res::txr_info(e.data)
                        .map(|t| format!(" {}x{}", t.width, t.height))
                        .unwrap_or_default();
                    println!("  {:>9}  {}{dim}", e.data.len(), e.raw_name);
                }
            }
            res::Section::Tokens { kind, tokens } => println!("{kind} {}", tokens.len()),
        }
    }
    Ok(())
}

/// Decode every 3D-world texture in a `.RES` file to `<out>/<name>.png`.
fn txr_png(data: &[u8], out: &str) -> Result<(), String> {
    let r = res::Res::parse(data).map_err(|e| e.to_string())?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let (mut ok, mut skipped) = (0, 0);
    for s in &r.sections {
        let res::Section::Files { kind: "TEXTUREFILES", entries } = s else { continue };
        for e in entries {
            let Ok(t) = txr::decode(e.data) else {
                skipped += 1;
                continue;
            };
            let stem: String = e.name().chars().map(|c| if c.is_alphanumeric() || c == '.' { c } else { '_' }).collect();
            let file = fs::File::create(format!("{out}/{stem}.png")).map_err(|e| e.to_string())?;
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), t.width, t.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().map_err(|e| e.to_string())?;
            w.write_image_data(&t.rgba).map_err(|e| e.to_string())?;
            ok += 1;
        }
    }
    println!("{ok} written, {skipped} skipped (unsupported variant)");
    Ok(())
}

fn b3d_ls(data: &[u8]) -> Result<(), String> {
    fn walk(ns: &[b3d::Node], depth: usize) {
        for n in ns {
            let extra = match &n.data {
                b3d::Data::Space { axes, pos } => format!(" axes={axes:?} pos={pos:?}"),
                b3d::Data::Switch { cases, key } => format!(" cases={cases} key={key}"),
                b3d::Data::CollisionVolume { volume_kind, geometry, surface, params, .. } =>
                    format!(" volume={volume_kind} geometry={geometry:?} surface={surface:?} params={params:?}"),
                _ => String::new(),
            };
            println!("{:indent$}[{}] {} {:?}{extra}", "", n.kind, n.name, n.names, indent = depth * 2);
            walk(&n.children, depth + 1);
        }
    }
    let m = b3d::parse(data).map_err(|e| e.to_string())?;
    println!("{} materials", m.materials.len());
    walk(&m.nodes, 0);
    let tris: usize = m.triangles().iter().map(|b| b.verts.len() / 3).sum();
    println!("{tris} triangles");
    Ok(())
}

/// Per-material triangle counts and bounding boxes, to find out what a model is made of.
fn b3d_batches(data: &[u8]) -> Result<(), String> {
    let m = b3d::parse(data).map_err(|e| e.to_string())?;
    for b in m.triangles() {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in &b.verts {
            for i in 0..3 {
                lo[i] = lo[i].min(v.pos[i]);
                hi[i] = hi[i].max(v.pos[i]);
            }
        }
        let name = m.materials.get(b.material as usize).map_or("?", String::as_str);
        println!("{:>4} {:<24} {:>6} tris  x {:>8.0}..{:<8.0} y {:>8.0}..{:<8.0} z {:>7.0}..{:<7.0}", b.material, name, b.verts.len() / 3, lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]);
    }
    Ok(())
}

fn tech_ls(data: &[u8]) -> Result<(), String> {
    println!("{:<20} {:<14} {:>6} {:>6} {:>7} {:>7} {:>6}", "name", "model prefix", "mass", "hp", "price", "wheelR", "gears");
    for t in tech::parse(data).map_err(|e| e.to_string())? {
        println!("{:<20} {:<14} {:>6.0} {:>6} {:>7} {:>7.2} {:>6}", t.name, t.prefix, t.mass, t.horse_power, t.price, t.wheel_radius, t.num_gears);
    }
    Ok(())
}

fn way_rooms(data: &[u8]) -> Result<(), String> {
    let chunks = way::parse(data).map_err(|e| e.to_string())?;
    let (mut nodes, mut segs, mut length) = (0, 0, 0.0f64);
    for r in way::rooms(&chunks) {
        let len: f64 = r.segments.iter().map(|s| s.points.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt()).sum::<f64>()).sum();
        println!("{:<16} {:>4} nodes {:>4} segments {:>8.0} m of road", r.name, r.nodes.len(), r.segments.len(), len);
        nodes += r.nodes.len();
        segs += r.segments.len();
        length += len;
    }
    println!("total: {nodes} nodes, {segs} segments, {length:.0} m");
    Ok(())
}

fn way_ls(data: &[u8]) -> Result<(), String> {
    fn walk(cs: &[way::Chunk], depth: usize) {
        for c in cs {
            let name = c.as_cstr().filter(|s| s.len() > 1 && s.is_ascii()).unwrap_or("");
            println!("{:indent$}{} ({} bytes) {name}", "", c.tag_str(), c.data.len(), indent = depth * 2);
            walk(&c.children, depth + 1);
        }
    }
    walk(&way::parse(data).map_err(|e| e.to_string())?, 0);
    Ok(())
}
