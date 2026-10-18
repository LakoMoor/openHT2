use ht2_formats::{b3d, res, txr, way};
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
        "b3d-ls" => b3d_ls(&data),
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
