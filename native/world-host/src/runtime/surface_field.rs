//! The forestry track's vectors take their electromagnetic properties from
//! the surface ontology. The ontology names the mixture. The index and the
//! Fresnel factor are derived here. Nothing in this module is a paint table.

use super::gis_join::RoadSurface;
use super::knowledge::QuinIndex;
use super::{err, hash};
use qualia_core_db::container_10d::mesh_section::{self, encode_mesh_section};
use qualia_core_db::container_10d::{
    attach_material_section, encode_container, seal_whole_file_crc32c, AlignmentTier,
    Container10dHeader, MaterialRecord, SectionInput, SectionTableError, SectionType, SubmeshRange,
};
use qualia_core_db::q42::emf_container::{EmfCellRecord64, EmfContainerWriter, EMF_BRICK_CELLS};
use qualia_core_db::render::assets::Mesh;
use qualia_core_db::vibe_host::PoetSnapshot;
use qualia_core_db::q_hash;
use serde_json::{json, Value};
use wasm_bindgen::JsValue;

#[path = "../../../../production/corridor/surface_mix.rs"]
mod surface_mix;

use surface_mix::{derive_mixture, MixPart, Mixture};

const ONTOLOGY_URL: &str = "/production/corridor/surface.q42";
const ONTOLOGY_SHA: &str = "114a17ce397e7c61d5895210796e97fcb18de32fdf7d8c83ced79101923e4d57";
const AIR: &str = "https://webizen.au/matter/air";
const DIRT: &str = "https://webizen.au/corridor/dirt";
const GRAVEL: &str = "https://webizen.au/corridor/gravel";
const SEALED: &str = "https://webizen.au/corridor/sealed";
/// Keeps the formed surface from fighting the soil mesh. Not a colour.
const CLEARANCE_M: f32 = 0.02;

pub struct AssignedTrack {
    pub mesh: Vec<u8>,
    pub emf: Vec<u8>,
    pub receipt: Value,
}

struct Body {
    iri: &'static str,
    surface: RoadSurface,
    mixture: Mixture,
}

pub async fn assign(host: &mut PoetSnapshot, road: &[u8]) -> Result<AssignedTrack, JsValue> {
    let ontology = super::bytes(ONTOLOGY_URL, 1024 * 1024).await?;
    if hash(&ontology) != ONTOLOGY_SHA {
        return Err(err("Surface ontology digest mismatch"));
    }
    super::knowledge::admit(host, &ontology, "corridor", "surface.q42").map_err(err)?;
    let index = QuinIndex::from_host(host).map_err(err)?;
    let bodies = [
        resolve(&index, DIRT, RoadSurface::Dirt)?,
        resolve(&index, GRAVEL, RoadSurface::Gravel)?,
        resolve(&index, SEALED, RoadSurface::Sealed)?,
    ];
    let (vertices, indices) = super::gis_join::road_vertices(road)
        .ok_or(err("Forestry track mesh did not yield vectors"))?;
    encode(&bodies, &vertices, &indices)
}

fn resolve(index: &QuinIndex, iri: &'static str, surface: RoadSurface) -> Result<Body, JsValue> {
    let parts = index.rel(iri, "hasPart");
    if parts.is_empty() {
        return Err(err(format!("Surface ontology has no parts for {iri}")));
    }
    let mut mix_parts = Vec::with_capacity(parts.len());
    for part in &parts {
        let constituent = index
            .one(part, "constituent")
            .map_err(|e| err(format!("{iri}: {e}")))?;
        mix_parts.push(MixPart {
            fraction: number(index, part, "fraction")?,
            refractive_index: number(index, &constituent, "refractiveIndex")?,
            porosity: number(index, &constituent, "porosity")?,
        });
    }
    let pore = index
        .one(iri, "poreFluid")
        .map_err(|e| err(format!("{iri}: {e}")))?;
    let mixture = derive_mixture(
        &mix_parts,
        number(index, &pore, "refractiveIndex")?,
        number(index, iri, "saturation")?,
        number(index, AIR, "refractiveIndex")?,
    )
    .ok_or_else(|| err(format!("Surface ontology mixture for {iri} is not usable")))?;
    Ok(Body {
        iri,
        surface,
        mixture,
    })
}

fn number(index: &QuinIndex, subject: &str, predicate: &str) -> Result<f64, JsValue> {
    index
        .number(subject, predicate)
        .map_err(|e| err(format!("{subject}: {e}")))
}

fn encode(
    bodies: &[Body; 3],
    vertices: &[super::gis_join::RoadVertex],
    indices: &[u32],
) -> Result<AssignedTrack, JsValue> {
    let mut materials: Vec<MaterialRecord> = bodies
        .iter()
        .map(|body| {
            let mut material = MaterialRecord::legacy_default();
            material.id = q_hash(body.iri);
            material.double_sided = true;
            material.multiply_vertex_color = false;
            material.wind_animated = false;
            material.metallic = 0.0;
            material.roughness = 1.0;
            let r = body.mixture.fresnel_r as f32;
            material.base_color = [r, r, r, 1.0];
            material
        })
        .collect();
    materials.sort_by_key(|material| material.id);
    let legacy = MaterialRecord::legacy_default().id;
    if materials.windows(2).any(|pair| pair[0].id == pair[1].id)
        || materials
            .iter()
            .any(|material| material.id == 0 || material.id == legacy)
    {
        return Err(err("Surface material identities collided"));
    }

    let mut grouped: Vec<Vec<[u32; 3]>> = vec![Vec::new(); materials.len()];
    for triangle in indices.chunks_exact(3) {
        let surface = vertices
            .get(triangle[0] as usize)
            .map(|vertex| vertex.surface)
            .ok_or(err("Forestry track index is outside the vectors"))?;
        let body = bodies
            .iter()
            .find(|body| body.surface == surface)
            .ok_or(err("Forestry track vector has no ontology body"))?;
        let slot = materials
            .iter()
            .position(|material| material.id == q_hash(body.iri))
            .ok_or(err("Surface material was not built"))?;
        grouped[slot].push([triangle[0], triangle[1], triangle[2]]);
    }

    let mut triangles = Vec::with_capacity(indices.len() / 3);
    let mut ranges = Vec::with_capacity(materials.len());
    let mut cursor = 0u32;
    for (material, group) in materials.iter().zip(grouped.iter()) {
        if group.is_empty() {
            return Err(err("A queried track surface has no triangles"));
        }
        let first_index = cursor;
        let index_count = u32::try_from(group.len() * 3)
            .map_err(|_| err("Track index count does not fit"))?;
        triangles.extend(group.iter().copied());
        cursor = cursor
            .checked_add(index_count)
            .ok_or(err("Track index count does not fit"))?;
        ranges.push(SubmeshRange {
            first_index,
            index_count,
            material_id: material.id,
            semantic_id: material.id,
        });
    }

    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    let positions: Vec<[f32; 3]> = vertices
        .iter()
        .map(|vertex| {
            let position = [vertex.x, vertex.y + CLEARANCE_M, vertex.z];
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
            position
        })
        .collect();
    let mesh = Mesh {
        positions,
        triangles,
        min,
        max,
    };
    let payload_len = mesh_section::encoded_len(mesh.positions.len(), mesh.triangles.len());
    let mut payload = vec![0u8; payload_len];
    let written = encode_mesh_section(&mesh, &mut payload)
        .map_err(|error| err(format!("Track vectors were not encoded: {error:?}")))?;
    payload.truncate(written);
    let header = Container10dHeader::proposed();
    let input = [SectionInput {
        section_type: SectionType::QuantizedMesh,
        alignment_tier: AlignmentTier::Page,
        stride: 0,
        element_count: 0,
        payload: &payload,
    }];
    let needed = match encode_container(&header, &input, &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Ok(_) => return Err(err("Track container size was not reported")),
        Err(error) => return Err(err(format!("Track container: {error:?}"))),
    };
    let mut container = vec![0u8; needed];
    encode_container(&header, &input, &mut container)
        .map_err(|error| err(format!("Track container: {error:?}")))?;
    seal_whole_file_crc32c(&mut container);
    let flags = u16::from_le_bytes([container[6], container[7]]) & !1;
    container[6] = (flags & 0xff) as u8;
    container[7] = (flags >> 8) as u8;
    seal_whole_file_crc32c(&mut container);
    let index_count = u32::try_from(mesh.triangles.len() * 3)
        .map_err(|_| err("Track index count does not fit"))?;
    let admitted = attach_material_section(&container, &materials, &ranges, index_count)
        .map_err(|error| err(format!("Track material: {error:?}")))?;
    super::admission::check("wr-forestry-track", &admitted)?;

    let emf = write_field(&mesh.positions, vertices, bodies)?;
    let receipt = json!({
        "ontology": ONTOLOGY_URL,
        "ontology_sha256": ONTOLOGY_SHA,
        "assignment": "vertex index i carries the mixture of the body named by vector i",
        "extinction_k": 0.0,
        "spectral_sigma": 0.0,
        "illuminant": "scene sun",
        "reflectance": "normal-incidence Fresnel factor of the volume-weighted refractive index",
        "visible": "equal RGB channels of that Fresnel factor, lit by the scene sun. Pigment extinction is absent, so dirt, gravel, and sealed stay near the same dark grey",
        "field_container": "Q42EMF64",
        "field_version": 2,
        "bodies": bodies.iter().map(|body| json!({
            "surface": body.surface.as_str(),
            "iri": body.iri,
            "n_eff": body.mixture.n_eff,
            "fresnel_r": body.mixture.fresnel_r,
            "porosity": body.mixture.porosity,
        })).collect::<Vec<_>>(),
        "emf_sha256": hash(&emf),
        "mesh_sha256": hash(&admitted),
        "vertices": vertices.len(),
    });
    Ok(AssignedTrack {
        mesh: admitted,
        emf,
        receipt,
    })
}

fn write_field(
    positions: &[[f32; 3]],
    vertices: &[super::gis_join::RoadVertex],
    bodies: &[Body; 3],
) -> Result<Vec<u8>, JsValue> {
    let bricks = vertices.len().div_ceil(EMF_BRICK_CELLS);
    if bricks == 0 || bricks > 4 {
        return Err(err("Track field does not fit the electromagnetic brick grid"));
    }
    let origin = positions.first().copied().unwrap_or([0.0, 0.0, 0.0]);
    let mut writer = EmfContainerWriter::new(origin, 1.0, [bricks as u32, 1, 1], 0.0);
    for brick in 0..bricks {
        let mut cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
        for local in 0..EMF_BRICK_CELLS {
            let index = brick * EMF_BRICK_CELLS + local;
            let Some(vertex) = vertices.get(index) else {
                cells[local].confidence_alpha = 0.0;
                continue;
            };
            let body = bodies
                .iter()
                .find(|body| body.surface == vertex.surface)
                .ok_or(err("Track field vector has no ontology body"))?;
            cells[local].refractive_n = body.mixture.n_eff as f32;
            cells[local].extinction_k = 0.0;
            cells[local].spectral_sigma = 0.0;
            cells[local].wavelength_nm = 0.0;
            cells[local].confidence_alpha = 1.0;
        }
        writer
            .add_brick_chunk(brick as u64, &cells, 0, 0, 0)
            .map_err(|error| err(format!("Track field brick: {error:?}")))?;
    }
    writer
        .to_bytes()
        .map_err(|error| err(format!("Track field: {error:?}")))
}
