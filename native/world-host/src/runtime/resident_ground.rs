//! Homepage soil and eco-intent grasses.
//!
//! The bare URL keeps the campsite props from first paint and replaces the
//! olive habitat and the measured DEM with `catchment-ground.10d`. Themeda,
//! Poa, and Lomandra are the meshes already copied under
//! `/production/ground/flora/`. This path does not fetch `/intenteco/` or any
//! other host. `?ewm=1` still uses the catchment streaming loop instead.

use super::instances::Placement;
use super::understorey::{self, ResidentTufts, Species};
use super::woody::{self, ResidentWood};
use super::{bytes, err, hash, text, Asset, Scene};
use qualia_core_db::render::compile_10d::decode_10d_mesh;
use qualia_core_db::vibe_host::PoetSnapshot;
use qualia_core_db::QualiaPortal;
use serde_json::{json, Value};
use wasm_bindgen::JsValue;

const TRACK_ID: &str = "wr-forestry-track";

const GROUND_ID: &str = "catchment-ground";
const GROUND_URL: &str = "/production/ground/catchment-ground.10d";
const GROUND_SHA: &str = "b782bfa73ce391ebbd504f7b27151048c53feb0fd93b201b2963f2fb42aeff5e";

struct FloraFile {
    id: &'static str,
    species: Species,
    url: &'static str,
    sha256: &'static str,
}

const FLORA: &[FloraFile] = &[
    FloraFile {
        id: "wr-themeda-triandra",
        species: Species::ThemedaTriandra,
        url: "/production/ground/flora/themeda-triandra-aboveground-lod1.10d",
        sha256: "17caaeec11d63108d75f14482aceb726dfde17b5e9d56a8f4747ba20dde821c1",
    },
    FloraFile {
        id: "wr-poa-labillardierei",
        species: Species::PoaLabillardierei,
        url: "/production/ground/flora/poa-labillardierei-aboveground-lod1.10d",
        sha256: "9871744f10406fa173dd9171ce02c44d130df45fbfd29b50c3b0c44c902e71f2",
    },
    FloraFile {
        id: "wr-lomandra-longifolia",
        species: Species::LomandraLongifolia,
        url: "/production/ground/flora/lomandra-longifolia-aboveground-lod1.10d",
        sha256: "72c34d82e660a9b7a9b15826febc141f87a58530d1d33f9ca7e6a79a71e91ddb",
    },
];

struct Loaded {
    id: String,
    raw: Vec<u8>,
    colour: [f32; 4],
    sha256: String,
    species: Option<Species>,
}

pub struct Mounted {
    pub upload: Value,
    pub instances: Value,
    pub tufts: ResidentTufts,
    pub wood: ResidentWood,
    pub asset_receipts: Vec<Value>,
    pub total_bytes: usize,
    pub road: Value,
}

fn replaced_on_the_homepage(id: &str) -> bool {
    super::stream::is_legacy_replaced_by_catchment(id) || id == "canberra-reference-terrain"
}

fn placements_for(species: Species, id: &str) -> Vec<Placement> {
    understorey::TUFTS
        .iter()
        .filter(|seed| seed.species == species)
        .enumerate()
        .map(|(index, seed)| {
            let y = super::ground::height_at(seed.trunk_x, seed.trunk_z);
            let half = seed.yaw * 0.5;
            Placement {
                id: format!("{id}-{index:03}"),
                translation: [seed.trunk_x, y, seed.trunk_z],
                rotation: [0.0, half.sin(), 0.0, half.cos()],
                scale: [1.0, 1.0, 1.0],
            }
        })
        .collect()
}

/// The soil mesh cites these PNG maps. Unbound maps draw as white.
async fn bind_ground_textures(portal: &mut QualiaPortal, ground: &[u8]) -> Result<(), JsValue> {
    let refs = super::textures::collect_material_texture_refs(ground);
    if refs.is_empty() {
        return Err(err("Catchment soil mesh has no colour maps"));
    }
    for (digest, role, cutoff) in refs {
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        let url = format!("/production/ground/textures/sha256/{hex}");
        let encoded = bytes(&url, 4 * 1024 * 1024).await?;
        if hash(&encoded) != hex {
            return Err(err(format!("Soil texture digest mismatch: {hex}")));
        }
        if encoded.len() < 8 || &encoded[..8] != b"\x89PNG\r\n\x1a\n" {
            return Err(err(format!("Soil texture is not a PNG: {hex}")));
        }
        let bound = portal.load_texture_encoded(&digest, "image/png", &encoded, role, cutoff)?;
        if !bound {
            return Err(err(format!("Soil texture was not bound: {hex}")));
        }
    }
    Ok(())
}

fn push_asset(assets: &js_sys::Array, item: &Loaded) -> Result<(), JsValue> {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &obj,
        &"bytes".into(),
        &js_sys::Uint8Array::from(item.raw.as_slice()),
    )?;
    js_sys::Reflect::set(&obj, &"id".into(), &item.id.clone().into())?;
    for (name, value) in ["r", "g", "b", "a"].iter().zip(item.colour) {
        js_sys::Reflect::set(&obj, &(*name).into(), &JsValue::from_f64(value as f64))?;
    }
    assets.push(&obj);
    Ok(())
}

/// Replace the first-paint ground with the catchment mesh and the three tuft species.
pub async fn mount(
    portal: &mut QualiaPortal,
    scene: &mut Scene,
    prior: &Value,
    overlay: Option<&super::macros::Live>,
    host: &mut PoetSnapshot,
) -> Result<Mounted, JsValue> {
    let parts = prior["parts"]
        .as_array()
        .ok_or(err("Homepage soil mount needs the first-paint part list"))?;
    let mut loaded = Vec::new();
    let mut saw_camper = false;
    for part in parts {
        let id = part["id"]
            .as_str()
            .ok_or(err("First-paint part is missing an id"))?;
        if replaced_on_the_homepage(id) {
            continue;
        }
        if id.starts_with("vintage-windup-camper") {
            saw_camper = true;
        }
        let asset = scene
            .assets
            .iter()
            .find(|asset| asset.id == id)
            .ok_or_else(|| err(format!("First-paint asset is not in the woodland scene: {id}")))?;
        text("native-world-status", &format!("Opening the campsite — {id}"));
        let raw = bytes(&asset.url, 64 * 1024 * 1024).await?;
        if hash(&raw) != asset.sha256 {
            return Err(err(format!("Asset digest mismatch: {id}")));
        }
        super::admission::check(id, &raw)?;
        loaded.push(Loaded {
            id: id.to_string(),
            raw,
            colour: asset.colour,
            sha256: asset.sha256.clone(),
            species: None,
        });
    }
    if !saw_camper {
        return Err(err(
            "Homepage soil mount could not keep the campsite van",
        ));
    }
    text(
        "native-world-status",
        "Opening the soil, the creek bank, and the grasses…",
    );
    let ground = bytes(GROUND_URL, 64 * 1024 * 1024).await?;
    if hash(&ground) != GROUND_SHA {
        return Err(err("Catchment soil digest mismatch"));
    }
    super::admission::check(GROUND_ID, &ground)?;
    bind_ground_textures(portal, &ground).await?;
    let ground_mesh = decode_10d_mesh(&ground).map_err(|_| err("Could not read the catchment soil mesh"))?;
    let ground_positions = ground_mesh.positions;
    loaded.push(Loaded {
        id: GROUND_ID.to_string(),
        raw: ground,
        colour: [1.0, 1.0, 1.0, 1.0],
        sha256: GROUND_SHA.to_string(),
        species: None,
    });
    for flora in FLORA {
        let raw = bytes(flora.url, 64 * 1024 * 1024).await?;
        if hash(&raw) != flora.sha256 {
            return Err(err(format!("Tuft digest mismatch: {}", flora.id)));
        }
        super::admission::check(flora.id, &raw)?;
        loaded.push(Loaded {
            id: flora.id.to_string(),
            raw,
            colour: [1.0, 1.0, 1.0, 1.0],
            sha256: flora.sha256.to_string(),
            species: Some(flora.species),
        });
    }
    let fetched_trees = woody::fetch().await?;
    for tree in &fetched_trees {
        loaded.push(Loaded {
            id: tree.id.clone(),
            raw: tree.admitted.clone(),
            colour: [1.0, 1.0, 1.0, 1.0],
            sha256: tree.source_sha.clone(),
            species: None,
        });
    }
    text(
        "native-world-status",
        "Asking the corridor ontology what the track is made of…",
    );
    let assigned = super::surface_field::assign(host, super::gis_join::road_mesh_bytes()).await?;
    // The mesh shader lights base_color with the scene sun. portal.load_emf reads the
    // version-4 electric-field envelope and does not shade this mesh. A refusal leaves
    // the Q42EMF64 record in the receipt and does not paint a substitute colour.
    let portal_field = match portal.load_emf(&assigned.emf) {
        Ok(value) => {
            let text = js_sys::JSON::stringify(&value)
                .ok()
                .and_then(|value| value.as_string())
                .unwrap_or_default();
            serde_json::from_str::<Value>(&text).unwrap_or(json!({"loaded": true}))
        }
        Err(error) => json!({
            "loaded": false,
            "reason": error.as_string().unwrap_or_else(|| {
                "The field reader refused the track field".to_string()
            }),
        }),
    };
    let mut road = assigned.receipt;
    road["portal_field"] = portal_field;
    let mesh_sha = road["mesh_sha256"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    loaded.push(Loaded {
        id: TRACK_ID.to_string(),
        raw: assigned.mesh,
        colour: [1.0, 1.0, 1.0, 1.0],
        sha256: mesh_sha,
        species: None,
    });
    let assets = js_sys::Array::new();
    let mut total_bytes = 0usize;
    let mut asset_receipts = Vec::new();
    for item in &loaded {
        total_bytes += item.raw.len();
        push_asset(&assets, item)?;
        asset_receipts.push(json!({
            "id": item.id,
            "sha256": item.sha256,
            "resident_ground": item.id == GROUND_ID || item.species.is_some(),
            "woody": item.id.starts_with("wr-eucalyptus"),
            "track": item.id == TRACK_ID
        }));
    }
    if total_bytes > 256 * 1024 * 1024 {
        return Err(err("Homepage scene exceeds its asset byte budget"));
    }
    let uploaded = portal.load_scene_assets(&assets)?;
    js_sys::Reflect::set(
        &web_sys::window().unwrap(),
        &"__webizenNativeUpload".into(),
        &uploaded,
    )?;
    let refused = js_sys::Reflect::get(&uploaded, &"organs_refused".into())?
        .as_f64()
        .unwrap_or(0.0);
    if refused != 0.0 {
        return Err(err(format!(
            "Homepage soil mount refused {refused} assets"
        )));
    }
    let upload: Value = serde_json::from_str(
        &js_sys::JSON::stringify(&uploaded)?
            .as_string()
            .ok_or(err("Invalid homepage soil upload receipt"))?,
    )
    .map_err(err)?;
    if upload["material_pipeline"].as_bool() != Some(true) {
        return Err(err("Homepage soil could not bind its colour maps"));
    }
    let admitted = portal.scene_render_receipt()?;
    let got_indices = js_sys::Reflect::get(&admitted, &"index_count".into())?
        .as_f64()
        .unwrap_or(0.0) as u64;
    let authored_triangles = upload["total_triangles"].as_u64().unwrap_or(0);
    if got_indices != authored_triangles * 3 {
        return Err(err(
            "Homepage soil mount reduced the authored scene",
        ));
    }
    scene.assets.push(Asset {
        id: GROUND_ID.to_string(),
        url: GROUND_URL.to_string(),
        colour: [1.0, 1.0, 1.0, 1.0],
        sha256: GROUND_SHA.to_string(),
        placements: Vec::new(),
    });
    for flora in FLORA {
        let placements = placements_for(flora.species, flora.id);
        if placements.is_empty() {
            return Err(err(format!("No tufts placed for {}", flora.id)));
        }
        scene.assets.push(Asset {
            id: flora.id.to_string(),
            url: flora.url.to_string(),
            colour: [1.0, 1.0, 1.0, 1.0],
            sha256: flora.sha256.to_string(),
            placements,
        });
    }
    let instances = super::instances::bind_resident(portal, scene, &upload, overlay)?;
    let parts = upload["parts"]
        .as_array()
        .ok_or(err("Homepage soil upload is missing part spans"))?;
    let mut meshes = Vec::with_capacity(3);
    for flora in FLORA {
        let item = loaded
            .iter()
            .find(|item| item.id == flora.id)
            .ok_or(err("Tuft bytes were not kept"))?;
        let mesh = decode_10d_mesh(&item.raw).map_err(|_| err(format!("Could not read {}", flora.id)))?;
        let part = parts
            .iter()
            .find(|part| part["id"].as_str() == Some(flora.id))
            .ok_or_else(|| err(format!("Tuft mesh was not admitted: {}", flora.id)))?;
        let vertex_start = part["vertex_start"]
            .as_u64()
            .ok_or(err("Tuft vertex span is missing"))? as u32;
        let vertex_count = part["vertex_count"].as_u64().unwrap_or(0) as usize;
        if vertex_count != mesh.positions.len() {
            return Err(err(format!(
                "{} vertex span does not match the eco-intent mesh",
                flora.id
            )));
        }
        meshes.push((flora.species, vertex_start, mesh.positions));
    }
    let tufts = ResidentTufts::from_meshes(meshes).map_err(err)?;
    let ground_part = parts
        .iter()
        .find(|part| part["id"].as_str() == Some(GROUND_ID))
        .ok_or(err("Catchment soil mesh was not admitted"))?;
    let soil_start = ground_part["vertex_start"].as_u64().unwrap_or(0) as u32;
    let soil_count = ground_part["vertex_count"].as_u64().unwrap_or(0) as usize;
    let wood = woody::install(
        portal,
        &fetched_trees,
        parts,
        ground_positions,
        soil_start,
        soil_count,
    )?;
    Ok(Mounted {
        upload,
        instances,
        tufts,
        wood,
        asset_receipts,
        total_bytes,
        road,
    })
}
