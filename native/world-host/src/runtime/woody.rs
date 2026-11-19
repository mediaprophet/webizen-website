//! Existing eco-intent eucalypts on the homepage canvas.
//!
//! The bytes stay in `C:\github\eco-intent`. The page fetches them from the
//! same-origin mount `/production/woody/`, which is that tree and not a copy
//! in this repository, and not `/intenteco/`. Each root vertex is fitted once
//! with [`super::ground::fit_soil_joint`] and then copied. Wind moves the
//! crown about the fitted plate. The frame loop does not evaluate Vibe.

use super::ground;
use super::instances::{
    contour_demonstration_site, fit_tree, wind_and_root_mechanics, GumSeed, WoodySpecies,
};
use super::{bytes, err, hash, text};
use qualia_core_db::container_10d::{
    attach_material_section, seal_whole_file_crc32c, MaterialRecord, SubmeshRange,
};
use qualia_core_db::render::compile_10d::decode_10d_mesh;
use qualia_core_db::QualiaPortal;
use wasm_bindgen::JsValue;

const REVEAL_RADIUS_M: f64 = 16.0;
const PIT_DEPTH_M: f64 = 8.0;
/// Aboveground lods carry the lignotuber fins up to about 1.4 m. That wood is
/// fitted to the soil once and held under the surface. The bole above it leans.
const COLLAR_BAND_M: f64 = 1.45;
const COLLAR_BURY_M: f64 = 0.20;

struct Source {
    species: WoodySpecies,
    slug: &'static str,
    crown_sha: &'static str,
    root_sha: &'static str,
}

const SOURCES: &[Source] = &[
    Source {
        species: WoodySpecies::Camaldulensis,
        slug: "eucalyptus-camaldulensis",
        crown_sha: "99079f19d9a224ccf1be0d820912f83a860696d7e314bac33e0b9b0c25a19593",
        root_sha: "dc1b3efbf8ba44a07b54e6f3702049e13a41af1527e26836c71dd3e311720967",
    },
    Source {
        species: WoodySpecies::Melliodora,
        slug: "eucalyptus-melliodora",
        crown_sha: "ffa0cbf89eb0c919efa641bb42c43e5ddfe8653b962d693700f32ca64bf1a8eb",
        root_sha: "b568509edf806bcd26d09e3da5a100a3dd088b07ba4020e5861cb104db96622e",
    },
    Source {
        species: WoodySpecies::Dalrympleana,
        slug: "eucalyptus-dalrympleana",
        crown_sha: "be1a2b5de7c1f8d2136e8cba1606faf385546919c9b6251ce6b2b014e59a42f4",
        root_sha: "6f9f4a44dcb569fe173073191259b0f5036976405d236c25d4a1a4f1d91747e8",
    },
    Source {
        species: WoodySpecies::Regnans,
        slug: "eucalyptus-regnans",
        crown_sha: "941e1b66504815f7e7f52b982e1ad285f91c86291c804fa2b8d564142802671c",
        root_sha: "4f2eac958f5ecfefe73420b5ac573d55956edd9343443ea2eb40f2cd7619f1c1",
    },
];

pub struct FetchedPart {
    pub id: String,
    pub admitted: Vec<u8>,
    pub source_sha: String,
    pub species: WoodySpecies,
    pub roots: bool,
    pub positions: Vec<[f32; 3]>,
}

struct Span {
    vertex_start: u32,
    authored: Vec<[f32; 3]>,
    scratch: Vec<f32>,
}

struct Tree {
    species: WoodySpecies,
    yaw: f64,
    trunk_x: f64,
    trunk_z: f64,
    collar_y: f64,
    plate_authored: [f64; 3],
    plate_world: [f64; 3],
    crown_ref: f64,
    crown: Span,
    roots: Span,
}

pub struct ResidentWood {
    trees: Vec<Tree>,
    soil_start: u32,
    soil_rest: Vec<f32>,
    soil_scratch: Vec<f32>,
    xray: bool,
    look_yaw: f32,
    proof_target: [f32; 3],
    pub tip_shift_m: f32,
}

fn yaw_xz(x: f64, z: f64, yaw: f64) -> (f64, f64) {
    let (s, c) = yaw.sin_cos();
    (x * c + z * s, -x * s + z * c)
}

fn lean_xy(offset: [f64; 3], angle: f64) -> [f64; 3] {
    let (s, c) = angle.sin_cos();
    [
        offset[0] * c + offset[1] * s,
        -offset[0] * s + offset[1] * c,
        offset[2],
    ]
}

fn seed_for(species: WoodySpecies) -> GumSeed {
    let site = contour_demonstration_site();
    match species {
        WoodySpecies::Camaldulensis => site.seed,
        WoodySpecies::Melliodora => GumSeed {
            species,
            trunk_x: -36.0,
            trunk_z: 42.0,
            yaw: 0.4,
        },
        WoodySpecies::Dalrympleana => GumSeed {
            species,
            trunk_x: -6.0,
            trunk_z: 18.0,
            yaw: 1.2,
        },
        WoodySpecies::Regnans => GumSeed {
            species,
            trunk_x: -40.0,
            trunk_z: 0.0,
            yaw: 2.1,
        },
    }
}

/// Clear the refuse flag, reseal, and add one double-sided vertex-colour material.
/// The eco-intent file on disk is not modified.
fn admit(bytes: Vec<u8>, id: &str) -> Result<Vec<u8>, JsValue> {
    if bytes.len() < 64 {
        return Err(err(format!("{id} is shorter than a 10d header")));
    }
    let mut data = bytes;
    let flags = u16::from_le_bytes([data[6], data[7]]);
    let cleared = flags & !1;
    data[6] = (cleared & 0xff) as u8;
    data[7] = (cleared >> 8) as u8;
    seal_whole_file_crc32c(&mut data);
    let mesh = decode_10d_mesh(&data).map_err(|_| err(format!("Could not read {id}")))?;
    let index_count = u32::try_from(mesh.triangles.len().saturating_mul(3))
        .map_err(|_| err(format!("{id} index count does not fit")))?;
    let mut material = MaterialRecord::legacy_default();
    material.double_sided = true;
    material.multiply_vertex_color = true;
    material.wind_animated = false;
    let range = SubmeshRange {
        first_index: 0,
        index_count,
        material_id: material.id,
        semantic_id: 0,
    };
    attach_material_section(&data, &[material], &[range], index_count)
        .map_err(|error| err(format!("{id} material: {error:?}")))
}

async fn fetch_part(species: WoodySpecies, slug: &str, sha: &str, roots: bool) -> Result<FetchedPart, JsValue> {
    let kind = if roots { "rhizosphere" } else { "aboveground-lod1" };
    let id = format!(
        "wr-{slug}-{}",
        if roots { "roots" } else { "crown" }
    );
    let url = format!("/production/woody/{slug}/dist/lods/{slug}-{kind}.10d");
    text("native-world-status", &format!("Opening {slug}"));
    let raw = bytes(&url, 8 * 1024 * 1024).await?;
    if hash(&raw) != sha {
        return Err(err(format!("Eucalypt digest mismatch: {id}")));
    }
    let admitted = admit(raw, &id)?;
    super::admission::check(&id, &admitted)?;
    let mesh = decode_10d_mesh(&admitted).map_err(|_| err(format!("Could not read {id}")))?;
    Ok(FetchedPart {
        id,
        admitted,
        source_sha: sha.to_string(),
        species,
        roots,
        positions: mesh.positions,
    })
}

pub async fn fetch() -> Result<Vec<FetchedPart>, JsValue> {
    let mut parts = Vec::with_capacity(SOURCES.len() * 2);
    for source in SOURCES {
        parts.push(fetch_part(source.species, source.slug, source.crown_sha, false).await?);
        parts.push(fetch_part(source.species, source.slug, source.root_sha, true).await?);
    }
    Ok(parts)
}

fn span_from(part: &FetchedPart, vertex_start: u32, vertex_count: usize) -> Result<Span, JsValue> {
    if vertex_count != part.positions.len() || vertex_count == 0 {
        return Err(err(format!(
            "{} vertex span does not match the eco-intent mesh",
            part.id
        )));
    }
    Ok(Span {
        vertex_start,
        authored: part.positions.clone(),
        scratch: vec![0.0; vertex_count * 3],
    })
}

pub fn install(
    portal: &mut QualiaPortal,
    parts: &[FetchedPart],
    upload_parts: &[serde_json::Value],
    soil_positions: Vec<[f32; 3]>,
    soil_start: u32,
    soil_vertex_count: usize,
) -> Result<ResidentWood, JsValue> {
    if soil_positions.len() != soil_vertex_count || soil_vertex_count == 0 {
        return Err(err("Homepage soil vertex span does not match the catchment mesh"));
    }
    let site = contour_demonstration_site();
    let mut trees = Vec::with_capacity(SOURCES.len());
    for source in SOURCES {
        let seed = seed_for(source.species);
        let fitted = fit_tree(seed);
        let plate = fitted.joints[fitted.plate];
        let crown = parts
            .iter()
            .find(|part| part.species == source.species && !part.roots)
            .ok_or_else(|| err(format!("Missing crown mesh for {}", source.slug)))?;
        let roots = parts
            .iter()
            .find(|part| part.species == source.species && part.roots)
            .ok_or_else(|| err(format!("Missing root mesh for {}", source.slug)))?;
        let crown_row = upload_parts
            .iter()
            .find(|row| row["id"].as_str() == Some(crown.id.as_str()))
            .ok_or_else(|| err(format!("Crown mesh was not admitted: {}", crown.id)))?;
        let root_row = upload_parts
            .iter()
            .find(|row| row["id"].as_str() == Some(roots.id.as_str()))
            .ok_or_else(|| err(format!("Root mesh was not admitted: {}", roots.id)))?;
        trees.push(Tree {
            species: source.species,
            yaw: seed.yaw,
            trunk_x: seed.trunk_x,
            trunk_z: seed.trunk_z,
            collar_y: ground::height_at(seed.trunk_x, seed.trunk_z),
            plate_authored: plate.authored,
            plate_world: plate.world,
            crown_ref: source.species.crown_ref_m(),
            crown: span_from(
                crown,
                crown_row["vertex_start"].as_u64().unwrap_or(0) as u32,
                crown_row["vertex_count"].as_u64().unwrap_or(0) as usize,
            )?,
            roots: span_from(
                roots,
                root_row["vertex_start"].as_u64().unwrap_or(0) as u32,
                root_row["vertex_count"].as_u64().unwrap_or(0) as usize,
            )?,
        });
    }
    let dx = site.rise_tip[0] - site.gully_tip[0];
    let dz = site.rise_tip[2] - site.gully_tip[2];
    let across = (-dz, dx);
    let len = (across.0 * across.0 + across.1 * across.1).sqrt().max(1.0e-6);
    let look_x = across.0 / len;
    let look_z = across.1 / len;
    let look_yaw = (-look_x as f32).atan2(-look_z as f32);
    let mut soil_rest = Vec::with_capacity(soil_positions.len() * 3);
    for position in &soil_positions {
        soil_rest.extend_from_slice(position);
    }
    let soil_scratch = soil_rest.clone();
    let proof_target = [
        site.seed.trunk_x as f32,
        (site.collar_y + 6.0) as f32,
        site.seed.trunk_z as f32,
    ];
    let mut wood = ResidentWood {
        trees,
        soil_start,
        soil_rest,
        soil_scratch,
        xray: false,
        look_yaw,
        proof_target,
        tip_shift_m: 0.0,
    };
    wood.prime(portal);
    Ok(wood)
}

impl ResidentWood {
    /// `gum=1` framing. `roots` looks down into the cut that reveals the laterals.
    pub fn proof_view(&self, roots: bool) -> ([f32; 3], f32, f32, f32) {
        if roots {
            (
                [
                    self.proof_target[0],
                    self.proof_target[1] - 4.0,
                    self.proof_target[2],
                ],
                self.look_yaw,
                0.72,
                26.0,
            )
        } else {
            (self.proof_target, self.look_yaw, 0.38, 36.0)
        }
    }

    fn write_span(portal: &mut QualiaPortal, span: &Span) {
        portal.write_part_vertices(span.vertex_start, &span.scratch);
    }

    fn fill_crown(tree: &mut Tree, t_s: f64, wind_speed_ms: f64) -> f32 {
        let wind = wind_and_root_mechanics(tree.species, t_s, wind_speed_ms, 0.35, 0.42);
        let hold = 1.0 - wind.damping_ratio;
        let mut tip = 0.0_f32;
        for (index, local) in tree.crown.authored.iter().enumerate() {
            let authored = [local[0] as f64, local[1] as f64, local[2] as f64];
            let collar = authored[1] <= COLLAR_BAND_M;
            let rest = if collar {
                let buried = [authored[0], authored[1].min(-COLLAR_BURY_M), authored[2]];
                ground::fit_soil_joint([tree.trunk_x, tree.trunk_z], tree.yaw, buried)
            } else {
                let off = [
                    authored[0] - tree.plate_authored[0],
                    authored[1] - tree.plate_authored[1],
                    authored[2] - tree.plate_authored[2],
                ];
                let (x, z) = yaw_xz(off[0], off[2], tree.yaw);
                [
                    tree.plate_world[0] + x,
                    tree.plate_world[1] + off[1],
                    tree.plate_world[2] + z,
                ]
            };
            let posed = if collar || wind_speed_ms <= 0.0 {
                rest
            } else {
                let off = [
                    authored[0] - tree.plate_authored[0],
                    authored[1] - tree.plate_authored[1],
                    authored[2] - tree.plate_authored[2],
                ];
                let reach = (off[1] / tree.crown_ref).clamp(0.0, 1.6);
                let radius = (off[0] * off[0] + off[2] * off[2]).sqrt();
                let bend = if radius < 0.45 {
                    0.35 * wind.bough_bend_rad + wind.bole_deflection_rad
                } else if radius < 2.2 {
                    wind.bough_bend_rad
                } else {
                    wind.bough_bend_rad + wind.sprig_flutter_rad
                };
                let leaned = lean_xy(off, bend * hold * reach);
                let (x, z) = yaw_xz(leaned[0], leaned[2], tree.yaw);
                [
                    tree.plate_world[0] + x,
                    tree.plate_world[1] + leaned[1],
                    tree.plate_world[2] + z,
                ]
            };
            if authored[1] > tree.crown_ref * 0.75 {
                let shift = ((posed[0] - rest[0]).powi(2)
                    + (posed[1] - rest[1]).powi(2)
                    + (posed[2] - rest[2]).powi(2))
                .sqrt();
                tip = tip.max(shift as f32);
            }
            let at = index * 3;
            tree.crown.scratch[at] = posed[0] as f32;
            tree.crown.scratch[at + 1] = posed[1] as f32;
            tree.crown.scratch[at + 2] = posed[2] as f32;
        }
        tip
    }

    fn fill_roots(tree: &mut Tree, shown: bool) {
        for (index, local) in tree.roots.authored.iter().enumerate() {
            let world = if shown {
                let authored = [local[0] as f64, local[1] as f64, local[2] as f64];
                ground::fit_soil_joint([tree.trunk_x, tree.trunk_z], tree.yaw, authored)
            } else {
                [tree.trunk_x, tree.collar_y - 4.0, tree.trunk_z]
            };
            let at = index * 3;
            tree.roots.scratch[at] = world[0] as f32;
            tree.roots.scratch[at + 1] = world[1] as f32;
            tree.roots.scratch[at + 2] = world[2] as f32;
        }
    }

    fn write_soil(&mut self, portal: &mut QualiaPortal) {
        if !self.xray {
            portal.write_part_vertices(self.soil_start, &self.soil_rest);
            return;
        }
        self.soil_scratch.copy_from_slice(&self.soil_rest);
        let count = self.soil_scratch.len() / 3;
        for index in 0..count {
            let x = self.soil_scratch[index * 3] as f64;
            let z = self.soil_scratch[index * 3 + 2] as f64;
            for tree in &self.trees {
                let dx = x - tree.trunk_x;
                let dz = z - tree.trunk_z;
                if dx * dx + dz * dz <= REVEAL_RADIUS_M * REVEAL_RADIUS_M {
                    self.soil_scratch[index * 3 + 1] = (tree.collar_y - PIT_DEPTH_M) as f32;
                    break;
                }
            }
        }
        portal.write_part_vertices(self.soil_start, &self.soil_scratch);
    }

    fn prime(&mut self, portal: &mut QualiaPortal) {
        self.tip_shift_m = 0.0;
        for tree in &mut self.trees {
            Self::fill_crown(tree, 0.0, 0.0);
            Self::fill_roots(tree, false);
            Self::write_span(portal, &tree.crown);
            Self::write_span(portal, &tree.roots);
        }
        self.write_soil(portal);
    }

    pub fn pose(&mut self, portal: &mut QualiaPortal, t_s: f64, wind_speed_ms: f64) {
        let mut tip = 0.0_f32;
        for tree in &mut self.trees {
            tip = tip.max(Self::fill_crown(tree, t_s, wind_speed_ms));
            Self::write_span(portal, &tree.crown);
        }
        self.tip_shift_m = tip;
    }

    pub fn set_lens(&mut self, portal: &mut QualiaPortal, active: bool) {
        self.xray = active;
        for tree in &mut self.trees {
            Self::fill_roots(tree, active);
            Self::write_span(portal, &tree.roots);
        }
        self.write_soil(portal);
    }
}
