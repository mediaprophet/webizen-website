//! Woodland scene graph from admitted Q42VOL v4 CBOR-LD SuperBlocks of NQuins.
//! Turtle is the producer source. Visitors never read the Q42 file.
//! This module never fetches scene.json.
use super::instances::Placement;
use super::knowledge::{QuinIndex, REL};
use super::textures::Texture;
use super::{Asset, Scene, Water};
use qualia_core_db::q42::q42_volume::WasmSliceQ42Reader;
use qualia_core_db::vibe_host::PoetSnapshot;
use std::collections::BTreeMap;

const SCENE_WORLD: &str = "https://webizen.au/scene/world";
const SCENE_ASSET: &str = "https://webizen.au/scene/asset";
const SCENE_PLACEMENT: &str = "https://webizen.au/scene/placement";
const SCENE_TEXTURE: &str = "https://webizen.au/scene/texture";

pub struct SceneGraph {
    pub scene: Scene,
    pub water_url: String,
    pub water_sha256: String,
    pub logic_url: Option<String>,
}

pub fn from_host(host: &PoetSnapshot) -> Result<SceneGraph, String> {
    from_index(&QuinIndex::from_host(host)?)
}

pub fn project(bytes: &[u8]) -> Result<SceneGraph, String> {
    let reader = WasmSliceQ42Reader::open(bytes).map_err(|e| e.to_string())?;
    if !reader.is_unified_volume() || reader.version() != 4 {
        return Err("Expected native Q42VOL v4 scene graph".into());
    }
    let quins = reader.collect_all_quins().map_err(|e| e.to_string())?;
    if quins.is_empty() || quins.len() > 50_000 {
        return Err("Scene graph exceeds host budget".into());
    }
    let mut names = BTreeMap::new();
    let mut scratch = vec![0; 1024 * 1024];
    let mut text = vec![0; 65536];
    for q in &quins {
        for hash in [q.subject, q.predicate, q.object] {
            if names.contains_key(&hash) {
                continue;
            }
            let n = reader
                .lookup_lexicon_hash_into(hash, &mut scratch, &mut text)
                .map_err(|e| e.to_string())?
                .ok_or("Missing native Q42 lexical term")?;
            names.insert(
                hash,
                std::str::from_utf8(&text[..n])
                    .map_err(|e| e.to_string())?
                    .to_string(),
            );
        }
    }
    let mut index: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for q in &quins {
        index
            .entry((names[&q.subject].clone(), names[&q.predicate].clone()))
            .or_default()
            .push(names[&q.object].clone());
    }
    from_index(&QuinIndex {
        index,
        rows: quins.len(),
    })
}

fn from_index(index: &QuinIndex) -> Result<SceneGraph, String> {
    let world = index.subject_of_kind(SCENE_WORLD)?;
    let schema = index.text(&world, "schema")?;
    if schema != "webizen.native-world/v1" {
        return Err("Unsupported native world scene schema".into());
    }
    let mut assets = Vec::new();
    for uri in index.rel(&world, "hasAsset") {
        let kind = index
            .many(&uri, &format!("{REL}kind"))
            .into_iter()
            .next()
            .unwrap_or_default();
        if kind != SCENE_ASSET {
            return Err("Scene hasAsset must name an asset".into());
        }
        let index_n = index.u64(&uri, "index").unwrap_or(0);
        let colour = index.f32x4(&uri, ["colourR", "colourG", "colourB", "colourA"])?;
        let mut placements = Vec::new();
        for p_uri in index.rel(&uri, "hasPlacement") {
            let p_kind = index
                .many(&p_uri, &format!("{REL}kind"))
                .into_iter()
                .next()
                .unwrap_or_default();
            if p_kind != SCENE_PLACEMENT {
                return Err("Scene hasPlacement must name a placement".into());
            }
            let p_index = index.u64(&p_uri, "index").unwrap_or(0);
            placements.push((
                p_index,
                Placement {
                    id: index.text(&p_uri, "id")?,
                    translation: index.f64x3(&p_uri, "translation")?,
                    rotation: index.f64x4(
                        &p_uri,
                        ["rotationX", "rotationY", "rotationZ", "rotationW"],
                    )?,
                    scale: index.f64x3(&p_uri, "scale")?,
                },
            ));
        }
        placements.sort_by_key(|(n, _)| *n);
        assets.push((
            index_n,
            Asset {
                id: index.text(&uri, "id")?,
                url: index.text(&uri, "url")?,
                sha256: index.text(&uri, "sha256")?,
                colour,
                placements: placements.into_iter().map(|(_, v)| v).collect(),
            },
        ));
    }
    assets.sort_by_key(|(n, _)| *n);
    if assets.is_empty() || assets.len() > 512 {
        return Err("Unsupported native world artifact manifest".into());
    }
    let mut textures = Vec::new();
    for uri in index.rel(&world, "hasTexture") {
        let kind = index
            .many(&uri, &format!("{REL}kind"))
            .into_iter()
            .next()
            .unwrap_or_default();
        if kind != SCENE_TEXTURE {
            return Err("Scene hasTexture must name a texture".into());
        }
        let index_n = index.u64(&uri, "index").unwrap_or(0);
        textures.push((
            index_n,
            Texture {
                url: index.text(&uri, "url")?,
                sha256: index.text(&uri, "sha256")?,
                mime_type: index.text(&uri, "mimeType")?,
                role: index.u64(&uri, "role").unwrap_or(0) as u8,
                cutoff: index.number(&uri, "cutoff").ok().map(|n| n as f32),
            },
        ));
    }
    textures.sort_by_key(|(n, _)| *n);
    Ok(SceneGraph {
        scene: Scene {
            schema,
            assets: assets.into_iter().map(|(_, v)| v).collect(),
            textures: textures.into_iter().map(|(_, v)| v).collect(),
            water: None,
            triangle_count: index.u64(&world, "triangleCount").ok(),
            vertex_count: index.u64(&world, "vertexCount").ok(),
            expanded_triangle_count: None,
        },
        water_url: index.text(&world, "waterGeometry")?,
        water_sha256: index.text(&world, "waterSha256")?,
        logic_url: index
            .text(&world, "hasLogicVolume")
            .ok()
            .filter(|url| !url.is_empty()),
    })
}

pub fn water_from_bytes(bytes: &[u8]) -> Result<Water, String> {
    if bytes.len() < 12 || &bytes[..4] != b"WTR1" {
        return Err("Expected WTR1 water geometry".into());
    }
    let npos = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| "Invalid water header")?)
        as usize;
    let pos_end = 8usize
        .checked_add(npos.checked_mul(4).ok_or("Water positions overflow")?)
        .ok_or("Water positions overflow")?;
    if bytes.len() < pos_end + 4 {
        return Err("Water geometry is truncated".into());
    }
    let mut positions = Vec::with_capacity(npos);
    for i in 0..npos {
        let start = 8 + i * 4;
        let value = f32::from_le_bytes(
            bytes[start..start + 4]
                .try_into()
                .map_err(|_| "Invalid water position")?,
        );
        if !value.is_finite() {
            return Err("Water position is not finite".into());
        }
        positions.push(value);
    }
    let nidx = u32::from_le_bytes(
        bytes[pos_end..pos_end + 4]
            .try_into()
            .map_err(|_| "Invalid water index count")?,
    ) as usize;
    let idx_end = (pos_end + 4)
        .checked_add(nidx.checked_mul(4).ok_or("Water indices overflow")?)
        .ok_or("Water indices overflow")?;
    if bytes.len() != idx_end {
        return Err("Water geometry length mismatch".into());
    }
    let mut indices = Vec::with_capacity(nidx);
    for i in 0..nidx {
        let start = pos_end + 4 + i * 4;
        indices.push(u32::from_le_bytes(
            bytes[start..start + 4]
                .try_into()
                .map_err(|_| "Invalid water index")?,
        ));
    }
    if positions.len() < 9 || indices.len() < 3 {
        return Err("Water geometry is empty".into());
    }
    Ok(Water { positions, indices })
}


