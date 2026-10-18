//! Loads a `.b3d` together with the `.RES` that supplies its textures and materials.

use ht2_formats::{b3d, res, txr};
use std::{collections::HashMap, fs, path::Path};

/// How a material looks: a texture, or a flat tint.
#[derive(Clone)]
pub enum Look {
    Texture(usize),
    Flat([f32; 3]),
}

pub struct Scene {
    pub textures: Vec<txr::Texture>,
    /// One entry per `.b3d` material index.
    pub looks: Vec<Look>,
    pub batches: Vec<b3d::Batch>,
    /// `.b3d` material names, indexed like `looks` and `Batch::material`.
    pub material_names: Vec<String>,
    pub center: [f32; 3],
    pub radius: f32,
}

/// `name kind N attrs...` -> (kind, N)
fn parse_material(token: &str) -> Option<(&str, usize)> {
    let mut w = token.split_whitespace();
    w.next()?;
    let kind = w.next()?;
    Some((kind, w.next()?.parse().ok()?))
}

/// Load a model and the textures and materials of its `.res`, with the geometry of the whole
/// file (see [`load`] to pick a part). Several parts can share one `Scene`'s textures.
pub fn load(b3d_path: &Path, only: Option<&str>) -> Result<Scene, String> {
    let (model, mut scene) = load_assets(b3d_path)?;
    let batches = model.triangles_in(only);
    if batches.is_empty() {
        return Err(match only {
            Some(n) => format!("no triangles under a block named {n:?}"),
            None => "model has no triangles".into(),
        });
    }
    scene.set_batches(batches);
    Ok(scene)
}

impl Scene {
    /// Replace the geometry and recompute the bounds.
    pub fn set_batches(&mut self, batches: Vec<b3d::Batch>) {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in batches.iter().flat_map(|b| &b.verts) {
            for i in 0..3 {
                lo[i] = lo[i].min(v.pos[i]);
                hi[i] = hi[i].max(v.pos[i]);
            }
        }
        if !batches.is_empty() {
            self.center = std::array::from_fn(|i| (lo[i] + hi[i]) / 2.0);
            self.radius = (0..3).map(|i| (hi[i] - lo[i]) / 2.0).fold(0.0f32, f32::max).max(0.001);
        }
        self.batches = batches;
    }
}

/// The parsed model plus a `Scene` holding its textures and materials but no geometry yet.
pub fn load_assets(b3d_path: &Path) -> Result<(b3d::B3d, Scene), String> {
    let model = b3d::parse(&fs::read(b3d_path).map_err(|e| format!("{}: {e}", b3d_path.display()))?)
        .map_err(|e| e.to_string())?;

    let res_path = find_sibling(b3d_path, "res").ok_or("no matching .res next to the model")?;
    let res_data = fs::read(&res_path).map_err(|e| e.to_string())?;
    let res = res::Res::parse(&res_data).map_err(|e| e.to_string())?;

    let mut textures = Vec::new();
    // 1-based texture number in the RES -> index into `textures`.
    let mut tex_slot: HashMap<usize, usize> = HashMap::new();
    let (mut colors, mut materials) = (&[][..], &[][..]);
    for s in &res.sections {
        match s {
            res::Section::Files { kind: "TEXTUREFILES", entries } => {
                for (i, e) in entries.iter().enumerate() {
                    if let Ok(t) = txr::decode(e.data) {
                        tex_slot.insert(i + 1, textures.len());
                        textures.push((t, e.raw_name.contains("ttx")));
                    }
                }
            }
            res::Section::Tokens { kind: "COLORS", tokens } => colors = tokens,
            res::Section::Tokens { kind: "MATERIALS", tokens } => materials = tokens,
            _ => {}
        }
    }

    let by_name: HashMap<&str, (&str, usize)> = materials
        .iter()
        .filter_map(|t| Some((t.split_whitespace().next()?, parse_material(t)?)))
        .collect();

    let mut keyed = vec![false; textures.len()];
    let looks = model
        .materials
        .iter()
        .map(|name| match by_name.get(name.as_str()) {
            Some(&("tex", n)) => tex_slot.get(&n).map_or(Look::Flat([1.0, 0.0, 1.0]), |&i| Look::Texture(i)),
            Some(&("ttx", n)) => tex_slot.get(&n).map_or(Look::Flat([1.0, 0.0, 1.0]), |&i| {
                keyed[i] = true;
                Look::Texture(i)
            }),
            Some(&("col", n)) => {
                let g = colors.get(n.wrapping_sub(1)).and_then(|c| c.parse::<f32>().ok()).unwrap_or(0.5);
                Look::Flat([g; 3])
            }
            _ => Look::Flat([1.0, 0.0, 1.0]),
        })
        .collect();

    // Colour-keyed textures: make the corner pixel's colour transparent.
    let textures = textures
        .into_iter()
        .zip(keyed)
        .map(|((mut t, _), key)| {
            if key && t.rgba.len() >= 4 {
                let k: [u8; 3] = t.rgba[..3].try_into().unwrap();
                for px in t.rgba.chunks_exact_mut(4) {
                    if px[..3] == k {
                        px[3] = 0;
                    }
                }
            }
            t
        })
        .collect();

    let material_names = model.materials.clone();
    let scene = Scene { textures, looks, batches: Vec::new(), material_names, center: [0.0; 3], radius: 1.0 };
    Ok((model, scene))
}

/// Case-insensitive sibling with the same stem (`aa.b3d` -> `aa.res` / `AA.RES`).
fn find_sibling(path: &Path, ext: &str) -> Option<std::path::PathBuf> {
    let stem = path.file_stem()?.to_str()?.to_lowercase();
    fs::read_dir(path.parent()?).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(ext))
            && p.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.to_lowercase() == stem)
    })
}
