//! Named public places from the EWM catalog Q42, continuous 3x3 multi-tile
//! geospatial grid, and NSW/EcoPunk corridor dataset-driven placement generator.
//! Display frame is Y-up; tiles are (floor(x/100), floor(z/100)). Not XOR tile ids.

#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use super::catalog::{Catalog, CatalogEntry, Place};
#[cfg(target_arch = "wasm32")]
use super::instances::InstanceState64;
#[cfg(target_arch = "wasm32")]
use super::ocs_frames::ScaleRegime;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(not(target_arch = "wasm32"))]
#[path = "instances.rs"]
mod instances;
#[cfg(not(target_arch = "wasm32"))]
pub use instances::InstanceState64;

#[path = "../../../../production/corridor/mesh.rs"]
mod corridor_mesh;

/// Authored forestry-track mesh. Vertex y is `ground::height_at` plus crown,
/// shoulder, or cut-side drain. `surface_at` reads this mesh.
const ROAD_MESH: &[u8] = include_bytes!("../../../../production/corridor/forestry-road.bin");

pub use corridor_mesh::{road_vertices, RoadSurface, RoadVertex};

pub const DEFAULT_RADIUS_M: f32 = 80.0;
pub const TILE_METRES: f32 = 100.0;
pub const MAX_WINDOW: usize = 64;

/// Scale-aware tile edge length in metres for each OCS `ScaleRegime`.
#[cfg(target_arch = "wasm32")]
pub fn tile_size_for_regime(regime: ScaleRegime) -> f64 {
    match regime {
        ScaleRegime::LocalEnu => TILE_METRES as f64,
        ScaleRegime::PlanetaryWgs84 => 100_000.0,
        ScaleRegime::HeliocentricIcrf => 1.0e9,
        ScaleRegime::GalacticCosmos => 1.0e14,
    }
}

/// Scale-aware streaming/query radius in metres for each OCS `ScaleRegime`.
#[cfg(target_arch = "wasm32")]
pub fn radius_for_regime(regime: ScaleRegime) -> f64 {
    match regime {
        ScaleRegime::LocalEnu => DEFAULT_RADIUS_M as f64,
        ScaleRegime::PlanetaryWgs84 => 250_000.0,
        ScaleRegime::HeliocentricIcrf => 5.0e10,
        ScaleRegime::GalacticCosmos => 1.0e16,
    }
}

/// 64-bit floor-division tile index `(ix, iz)` for any OCS `ScaleRegime`.
#[cfg(target_arch = "wasm32")]
pub fn tile_xz_for_regime(x_m: f64, z_m: f64, regime: ScaleRegime) -> (i64, i64) {
    let tile = tile_size_for_regime(regime);
    ((x_m / tile).floor() as i64, (z_m / tile).floor() as i64)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
}

pub fn camera_pose(target: [f32; 3], yaw: f32, pitch: f32) -> CameraPose {
    CameraPose {
        x: target[0],
        y: target[1],
        z: target[2],
        yaw,
        pitch,
    }
}

pub fn tile_xz(x: f32, z: f32) -> (i32, i32) {
    ((x / TILE_METRES).floor() as i32, (z / TILE_METRES).floor() as i32)
}

#[cfg(target_arch = "wasm32")]
pub fn place_from_catalog(catalog: &Catalog) -> Result<&Place, JsValue> {
    catalog.place()
}

#[cfg(target_arch = "wasm32")]
pub fn bbox_wsen(place: &Place) -> [f64; 4] {
    place.bbox_wsen
}

/// Circle in the Y-up x/z plane versus an AABB, expanded by `radius` metres.
pub fn aabb_overlaps(min: [f32; 3], max: [f32; 3], pose: &CameraPose, radius: f32) -> bool {
    let nx = pose.x.clamp(min[0], max[0]);
    let nz = pose.z.clamp(min[2], max[2]);
    let dx = pose.x - nx;
    let dz = pose.z - nz;
    dx * dx + dz * dz <= radius * radius
}

#[cfg(target_arch = "wasm32")]
pub fn tile_membership(entry: &CatalogEntry, pose: &CameraPose) -> bool {
    let (ix, iy) = tile_xz(pose.x, pose.z);
    entry.tiles.iter().any(|t| *t == (ix, iy))
}

/// Circle-vs-AABB (plus radius) or catalog tile membership of the camera target.
#[cfg(target_arch = "wasm32")]
pub fn entry_overlaps(entry: &CatalogEntry, pose: &CameraPose, radius: f32) -> bool {
    entry
        .aabb
        .as_ref()
        .map(|aabb| aabb_overlaps(aabb.min, aabb.max, pose, radius))
        .unwrap_or(false)
        || tile_membership(entry, pose)
}

/// Entries whose AABB (plus radius) or tile contains the camera target. Windowed to 64.
#[cfg(target_arch = "wasm32")]
pub fn overlapping_entries<'a>(
    entries: &'a [CatalogEntry],
    pose: &CameraPose,
    radius: f32,
) -> Vec<&'a CatalogEntry> {
    let mut out = Vec::new();
    for entry in entries {
        if entry_overlaps(entry, pose, radius) {
            out.push(entry);
            if out.len() >= MAX_WINDOW {
                break;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Continuous 3x3 Multi-Tile Geospatial Grid & LRU Eviction Tracker
// ---------------------------------------------------------------------------

/// Integer tile coordinate `(tx, ty, zoom)` on the local 100 m ENU grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileCoord {
    pub tx: i32,
    pub ty: i32,
    pub zoom: u8,
}

impl TileCoord {
    pub const fn new(tx: i32, ty: i32, zoom: u8) -> Self {
        Self { tx, ty, zoom }
    }

    /// Map a world-space `(x, z)` metre coordinate to its containing [`TileCoord`].
    pub fn from_world_xz(x: f32, z: f32, zoom: u8) -> Self {
        let (tx, ty) = tile_xz(x, z);
        Self { tx, ty, zoom }
    }

    /// Minimum `(x, z)` corner of this tile in world metres.
    pub fn world_origin_xz(self) -> (f32, f32) {
        (self.tx as f32 * TILE_METRES, self.ty as f32 * TILE_METRES)
    }

    /// Deterministic 3x3 active ring centered on `self` (9 tiles, row-major `dy`, `dx`).
    pub fn ring_3x3(self) -> [TileCoord; 9] {
        [
            TileCoord::new(self.tx - 1, self.ty - 1, self.zoom),
            TileCoord::new(self.tx, self.ty - 1, self.zoom),
            TileCoord::new(self.tx + 1, self.ty - 1, self.zoom),
            TileCoord::new(self.tx - 1, self.ty, self.zoom),
            TileCoord::new(self.tx, self.ty, self.zoom),
            TileCoord::new(self.tx + 1, self.ty, self.zoom),
            TileCoord::new(self.tx - 1, self.ty + 1, self.zoom),
            TileCoord::new(self.tx, self.ty + 1, self.zoom),
            TileCoord::new(self.tx + 1, self.ty + 1, self.zoom),
        ]
    }
}

/// Diff produced when the camera/player/vehicle updates the 3x3 active tile ring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileRingDelta {
    pub center: TileCoord,
    pub active: Vec<TileCoord>,
    pub entering: Vec<TileCoord>,
    pub departing: Vec<TileCoord>,
    pub lru_evicted: Vec<TileCoord>,
}

/// Stateful 3x3 active tile ring with LRU eviction tracking for departing tiles.
#[derive(Clone, Debug)]
pub struct ActiveTileRing {
    center: Option<TileCoord>,
    active: Vec<TileCoord>,
    /// Most-recently-used at the back; least-recently-used at index 0.
    lru_resident: Vec<TileCoord>,
    max_resident_tiles: usize,
}

impl Default for ActiveTileRing {
    fn default() -> Self {
        Self::new(9)
    }
}

impl ActiveTileRing {
    /// Construct a ring tracker retaining at most `max_resident_tiles` (clamped >= 9).
    pub fn new(max_resident_tiles: usize) -> Self {
        Self {
            center: None,
            active: Vec::new(),
            lru_resident: Vec::new(),
            max_resident_tiles: max_resident_tiles.max(9),
        }
    }

    pub fn center(&self) -> Option<TileCoord> {
        self.center
    }

    pub fn active_tiles(&self) -> &[TileCoord] {
        &self.active
    }

    pub fn resident_tiles(&self) -> &[TileCoord] {
        &self.lru_resident
    }

    /// Update the active 3x3 ring for world position `(x, z)` at `zoom`.
    /// Returns entering tiles, departing tiles, and any LRU-evicted tiles beyond `max_resident_tiles`.
    pub fn update_for_position(&mut self, x: f32, z: f32, zoom: u8) -> TileRingDelta {
        let next_center = TileCoord::from_world_xz(x, z, zoom);
        if self.center == Some(next_center) {
            return TileRingDelta {
                center: next_center,
                active: self.active.clone(),
                entering: Vec::new(),
                departing: Vec::new(),
                lru_evicted: Vec::new(),
            };
        }

        let next_ring: Vec<TileCoord> = next_center.ring_3x3().to_vec();
        let entering: Vec<TileCoord> = next_ring
            .iter()
            .copied()
            .filter(|t| !self.active.contains(t))
            .collect();
        let departing: Vec<TileCoord> = self
            .active
            .iter()
            .copied()
            .filter(|t| !next_ring.contains(t))
            .collect();

        // Touch all active ring tiles as most-recently-used.
        for &tile in &next_ring {
            if let Some(pos) = self.lru_resident.iter().position(|t| *t == tile) {
                self.lru_resident.remove(pos);
            }
            self.lru_resident.push(tile);
        }

        // Evict least-recently-used non-active tiles if residency exceeds budget.
        let mut lru_evicted = Vec::new();
        while self.lru_resident.len() > self.max_resident_tiles {
            if let Some(evict_idx) = self
                .lru_resident
                .iter()
                .position(|t| !next_ring.contains(t))
            {
                lru_evicted.push(self.lru_resident.remove(evict_idx));
            } else {
                break;
            }
        }

        self.center = Some(next_center);
        self.active = next_ring.clone();

        TileRingDelta {
            center: next_center,
            active: next_ring,
            entering,
            departing,
            lru_evicted,
        }
    }
}

// ---------------------------------------------------------------------------
// NSW / EcoPunk Forest-to-Town-Outskirts Corridor & Dataset-Driven Placement
// ---------------------------------------------------------------------------

/// Multi-tile ecological and land-use zone along the Talbingo-Tumut corridor
/// (`data/ewm/datasets/corridor-gis-botany.q42`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CorridorZone {
    ForestCampsite,
    RiparianCreek,
    MontaneSlope,
    RoadCorridor,
    TownOutskirtsCommunityGrounds,
}

impl CorridorZone {
    pub const fn as_str(self) -> &'static str {
        match self {
            CorridorZone::ForestCampsite => "ForestCampsite",
            CorridorZone::RiparianCreek => "RiparianCreek",
            CorridorZone::MontaneSlope => "MontaneSlope",
            CorridorZone::RoadCorridor => "RoadCorridor",
            CorridorZone::TownOutskirtsCommunityGrounds => "TownOutskirtsCommunityGrounds",
        }
    }
}

/// Ecological stratum for dataset-driven placement and clearance rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EcologicalStratum {
    CanopyTree,
    SubcanopyTree,
    Shrub,
    GrassTussock,
    Fauna,
}

/// ALA/SVTM occurrence-weighted species entry for a corridor tile.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesWeight {
    pub species_slug: &'static str,
    pub stratum: EcologicalStratum,
    pub ala_weight: f32,
}

/// Cleared oval paddock, gravel loop, and shelter pad layout at the
/// Town-Outskirts Community Grounds destination tile.
#[derive(Clone, Debug, PartialEq)]
pub struct CommunityGroundsFeatures {
    pub cleared_oval_center_local: [f32; 2],
    pub cleared_oval_radius_m: f32,
    pub gravel_loop_inner_m: f32,
    pub gravel_loop_outer_m: f32,
    pub shelter_pad_local: [f32; 2],
    pub shelter_pad_radius_m: f32,
}

/// Dataset-driven profile for a 100 m corridor tile, derived from
/// `corridor-gis-botany.q42` and `nsw-ecopunk-sources.q42`.
#[derive(Clone, Debug, PartialEq)]
pub struct DatasetTileProfile {
    pub tile: TileCoord,
    pub zone: CorridorZone,
    pub dominant_pct: &'static str,
    pub mean_elevation_m: f32,
    pub mean_slope_deg: f32,
    /// Optional local Z coordinate (`0..100 m`) of the road centreline crossing this tile.
    pub road_centreline_z_local: Option<f32>,
    /// Half-width of the road carriageway in metres (kept clear of trees and shrubs).
    pub road_half_width_m: f32,
    /// Optional cleared paddock/oval + gravel loop + shelter pad specification.
    pub community_grounds: Option<CommunityGroundsFeatures>,
    /// Occurrence-weighted species list from ALA Biocache & NSW SVTM PCTs.
    pub species_weights: Vec<SpeciesWeight>,
}

/// Classify a tile along the Forest Campsite -> Country Road Corridor -> Town-Outskirts
/// Community Grounds transect.
pub fn classify_corridor_tile(tile: TileCoord) -> CorridorZone {
    match (tile.tx, tile.ty) {
        (0, 0) => CorridorZone::ForestCampsite,
        (1, 0) | (2, 0) => CorridorZone::RoadCorridor,
        (tx, 0) if tx >= 3 => CorridorZone::TownOutskirtsCommunityGrounds,
        (tx, ty) if tx <= 0 && ty >= 1 => CorridorZone::RiparianCreek,
        _ => CorridorZone::MontaneSlope,
    }
}

/// Build the dataset-backed [`DatasetTileProfile`] for `tile`, parameterized by
/// the real NSW FSDF / ALA Biocache / SVTM corridor descriptors in `corridor-gis-botany.q42`.
pub fn corridor_tile_profile(tile: TileCoord) -> DatasetTileProfile {
    let zone = classify_corridor_tile(tile);
    match zone {
        CorridorZone::ForestCampsite => DatasetTileProfile {
            tile,
            zone,
            dominant_pct: "pct-3376",
            mean_elevation_m: 442.0,
            mean_slope_deg: 6.5,
            road_centreline_z_local: Some(18.0),
            road_half_width_m: 4.5,
            community_grounds: None,
            species_weights: vec![
                SpeciesWeight { species_slug: "eucalyptus-viminalis", stratum: EcologicalStratum::CanopyTree, ala_weight: 0.34 },
                SpeciesWeight { species_slug: "eucalyptus-delegatensis", stratum: EcologicalStratum::CanopyTree, ala_weight: 0.20 },
                SpeciesWeight { species_slug: "acacia-dealbata", stratum: EcologicalStratum::SubcanopyTree, ala_weight: 0.32 },
                SpeciesWeight { species_slug: "dicksonia-antarctica", stratum: EcologicalStratum::Shrub, ala_weight: 0.14 },
                SpeciesWeight { species_slug: "poa-sieberiana", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.45 },
                SpeciesWeight { species_slug: "lomandra-longifolia", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.35 },
                SpeciesWeight { species_slug: "vombatus-ursinus", stratum: EcologicalStratum::Fauna, ala_weight: 0.40 },
                SpeciesWeight { species_slug: "macropus-giganteus", stratum: EcologicalStratum::Fauna, ala_weight: 0.35 },
            ],
        },
        CorridorZone::RiparianCreek => DatasetTileProfile {
            tile,
            zone,
            dominant_pct: "pct-3355",
            mean_elevation_m: 418.0,
            mean_slope_deg: 11.0,
            road_centreline_z_local: None,
            road_half_width_m: 0.0,
            community_grounds: None,
            species_weights: vec![
                SpeciesWeight { species_slug: "eucalyptus-viminalis", stratum: EcologicalStratum::CanopyTree, ala_weight: 0.42 },
                SpeciesWeight { species_slug: "acacia-dealbata", stratum: EcologicalStratum::SubcanopyTree, ala_weight: 0.38 },
                SpeciesWeight { species_slug: "dicksonia-antarctica", stratum: EcologicalStratum::Shrub, ala_weight: 0.45 },
                SpeciesWeight { species_slug: "lomandra-longifolia", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.55 },
                SpeciesWeight { species_slug: "poa-sieberiana", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.25 },
                SpeciesWeight { species_slug: "ornithorhynchus-anatinus", stratum: EcologicalStratum::Fauna, ala_weight: 0.30 },
                SpeciesWeight { species_slug: "wallabia-bicolor", stratum: EcologicalStratum::Fauna, ala_weight: 0.45 },
            ],
        },
        CorridorZone::RoadCorridor => DatasetTileProfile {
            tile,
            zone,
            dominant_pct: if tile.tx <= 1 { "pct-3376" } else { "pct-3373" },
            mean_elevation_m: if tile.tx <= 1 { 435.0 } else { 412.0 },
            mean_slope_deg: if tile.tx <= 1 { 8.0 } else { 5.5 },
            road_centreline_z_local: Some(50.0),
            road_half_width_m: 6.5,
            community_grounds: None,
            species_weights: vec![
                SpeciesWeight { species_slug: "eucalyptus-viminalis", stratum: EcologicalStratum::CanopyTree, ala_weight: 0.30 },
                SpeciesWeight { species_slug: "eucalyptus-blakelyi", stratum: EcologicalStratum::CanopyTree, ala_weight: 0.28 },
                SpeciesWeight { species_slug: "acacia-dealbata", stratum: EcologicalStratum::SubcanopyTree, ala_weight: 0.26 },
                SpeciesWeight { species_slug: "bursaria-spinosa", stratum: EcologicalStratum::Shrub, ala_weight: 0.24 },
                SpeciesWeight { species_slug: "themeda-triandra", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.50 },
                SpeciesWeight { species_slug: "poa-sieberiana", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.40 },
                SpeciesWeight { species_slug: "macropus-giganteus", stratum: EcologicalStratum::Fauna, ala_weight: 0.55 },
                SpeciesWeight { species_slug: "wallabia-bicolor", stratum: EcologicalStratum::Fauna, ala_weight: 0.30 },
            ],
        },
        CorridorZone::TownOutskirtsCommunityGrounds => DatasetTileProfile {
            tile,
            zone,
            dominant_pct: "pct-3373",
            mean_elevation_m: 396.0,
            mean_slope_deg: 2.0,
            road_centreline_z_local: Some(12.0),
            road_half_width_m: 5.0,
            community_grounds: Some(CommunityGroundsFeatures {
                cleared_oval_center_local: [54.0, 56.0],
                cleared_oval_radius_m: 26.0,
                gravel_loop_inner_m: 26.0,
                gravel_loop_outer_m: 32.0,
                shelter_pad_local: [20.0, 56.0],
                shelter_pad_radius_m: 8.0,
            }),
            species_weights: vec![
                SpeciesWeight { species_slug: "eucalyptus-blakelyi", stratum: EcologicalStratum::CanopyTree, ala_weight: 0.48 },
                SpeciesWeight { species_slug: "banksia-marginata", stratum: EcologicalStratum::Shrub, ala_weight: 0.32 },
                SpeciesWeight { species_slug: "bursaria-spinosa", stratum: EcologicalStratum::Shrub, ala_weight: 0.28 },
                SpeciesWeight { species_slug: "themeda-triandra", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.60 },
                SpeciesWeight { species_slug: "lomandra-longifolia", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.30 },
                SpeciesWeight { species_slug: "macropus-giganteus", stratum: EcologicalStratum::Fauna, ala_weight: 0.65 },
            ],
        },
        CorridorZone::MontaneSlope => DatasetTileProfile {
            tile,
            zone,
            dominant_pct: "pct-3391",
            mean_elevation_m: 488.0,
            mean_slope_deg: 22.5,
            road_centreline_z_local: None,
            road_half_width_m: 0.0,
            community_grounds: None,
            species_weights: vec![
                SpeciesWeight { species_slug: "eucalyptus-pauciflora", stratum: EcologicalStratum::CanopyTree, ala_weight: 0.52 },
                SpeciesWeight { species_slug: "acacia-dealbata", stratum: EcologicalStratum::SubcanopyTree, ala_weight: 0.30 },
                SpeciesWeight { species_slug: "banksia-marginata", stratum: EcologicalStratum::Shrub, ala_weight: 0.22 },
                SpeciesWeight { species_slug: "poa-sieberiana", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.55 },
                SpeciesWeight { species_slug: "lomandra-longifolia", stratum: EcologicalStratum::GrassTussock, ala_weight: 0.25 },
                SpeciesWeight { species_slug: "vombatus-ursinus", stratum: EcologicalStratum::Fauna, ala_weight: 0.35 },
                SpeciesWeight { species_slug: "phascolarctos-cinereus", stratum: EcologicalStratum::Fauna, ala_weight: 0.15 },
            ],
        },
    }
}

/// Deterministic placement emitted by [`generate_tile_placements`].
#[derive(Clone, Debug, PartialEq)]
pub struct TileInstancePlacement {
    pub species_slug: &'static str,
    pub stratum: EcologicalStratum,
    pub world_pos: [f32; 3],
    pub slope_deg: f32,
    pub state64: InstanceState64,
}

#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[inline]
fn unit_f32(state: &mut u64) -> f32 {
    let bits = splitmix64(state) >> 40;
    (bits as f32) * (1.0 / ((1u32 << 24) as f32))
}

fn tile_seed(tile: TileCoord, seed: u64) -> u64 {
    let tx = (tile.tx as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let ty = (tile.ty as i64 as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let zm = (tile.zoom as u64).wrapping_mul(0x94d0_49bb_1331_11eb);
    seed ^ tx ^ ty.rotate_left(19) ^ zm.rotate_left(41)
}

fn pick_weighted_species<'a>(
    candidates: &[&'a SpeciesWeight],
    rng: &mut u64,
) -> Option<&'a SpeciesWeight> {
    let total: f32 = candidates.iter().map(|s| s.ala_weight.max(0.01)).sum();
    if total <= 0.0 || candidates.is_empty() {
        return None;
    }
    let mut target = unit_f32(rng) * total;
    for &cand in candidates {
        let w = cand.ala_weight.max(0.01);
        if target <= w {
            return Some(cand);
        }
        target -= w;
    }
    candidates.last().copied()
}

fn is_position_clear_for_stratum(
    lx: f32,
    lz: f32,
    stratum: EcologicalStratum,
    profile: &DatasetTileProfile,
) -> bool {
    // The authored track uses the same half-widths as the tile clearances
    // (4.5 / 5.0 / 6.5). Those tile fields stay as they are; this is an
    // extra keep-out so the new carriageway and pad are not planted.
    let (ox, oz) = profile.tile.world_origin_xz();
    let extra = match stratum {
        EcologicalStratum::CanopyTree | EcologicalStratum::SubcanopyTree => 3.0,
        EcologicalStratum::Shrub => 1.5,
        EcologicalStratum::GrassTussock => 0.5,
        EcologicalStratum::Fauna => 1.0,
    };
    if corridor_mesh::plant_blocked(ROAD_MESH, ox as f64 + lx as f64, oz as f64 + lz as f64, extra)
    {
        return false;
    }

    // 1. Road centreline clearance
    if let Some(rz) = profile.road_centreline_z_local {
        let dist_road = (lz - rz).abs();
        let clearance = match stratum {
            EcologicalStratum::CanopyTree | EcologicalStratum::SubcanopyTree => {
                profile.road_half_width_m + 3.0
            }
            EcologicalStratum::Shrub => profile.road_half_width_m + 1.5,
            EcologicalStratum::GrassTussock => profile.road_half_width_m + 0.5,
            EcologicalStratum::Fauna => profile.road_half_width_m + 1.0,
        };
        if dist_road < clearance {
            return false;
        }
    }

    // 2. Community Grounds cleared oval paddock, gravel loop, and shelter pad
    if let Some(cg) = &profile.community_grounds {
        let dx_o = lx - cg.cleared_oval_center_local[0];
        let dz_o = lz - cg.cleared_oval_center_local[1];
        let dist_oval = (dx_o * dx_o + dz_o * dz_o).sqrt();

        let dx_s = lx - cg.shelter_pad_local[0];
        let dz_s = lz - cg.shelter_pad_local[1];
        let dist_shelter = (dx_s * dx_s + dz_s * dz_s).sqrt();

        match stratum {
            EcologicalStratum::CanopyTree | EcologicalStratum::SubcanopyTree => {
                if dist_oval < cg.gravel_loop_outer_m + 2.0
                    || dist_shelter < cg.shelter_pad_radius_m + 3.0
                {
                    return false;
                }
            }
            EcologicalStratum::Shrub => {
                if dist_oval < cg.gravel_loop_outer_m + 0.5
                    || dist_shelter < cg.shelter_pad_radius_m + 1.5
                {
                    return false;
                }
            }
            EcologicalStratum::GrassTussock => {
                // Keep gravel loop and shelter pad clear of tall tussocks;
                // allow mown/native grass verges outside gravel loop.
                if (dist_oval >= cg.gravel_loop_inner_m && dist_oval <= cg.gravel_loop_outer_m)
                    || dist_shelter < cg.shelter_pad_radius_m
                {
                    return false;
                }
            }
            EcologicalStratum::Fauna => {
                if dist_shelter < cg.shelter_pad_radius_m {
                    return false;
                }
            }
        }
    }

    // 3. Forest Campsite central camp clearing around (50, 50)
    if profile.zone == CorridorZone::ForestCampsite {
        let dx = lx - 50.0;
        let dz = lz - 50.0;
        let d = (dx * dx + dz * dz).sqrt();
        if matches!(
            stratum,
            EcologicalStratum::CanopyTree | EcologicalStratum::SubcanopyTree
        ) && d < 12.0
        {
            return false;
        }
    }

    true
}

/// Deterministically generate location-appropriate [`InstanceState64`] placements
/// for a 100 m corridor tile (trees, long grasses/tussocks, shrubs, and fauna),
/// respecting road centreline and Community Grounds clearings.
pub fn generate_tile_placements(
    tile: TileCoord,
    profile: &DatasetTileProfile,
    seed: u64,
) -> Vec<TileInstancePlacement> {
    let mut rng = tile_seed(tile, seed);
    let (ox, oz) = tile.world_origin_xz();

    let (tree_budget, shrub_budget, grass_budget, fauna_budget) = match profile.zone {
        CorridorZone::ForestCampsite => (24usize, 14usize, 36usize, 4usize),
        CorridorZone::RiparianCreek => (26usize, 20usize, 42usize, 4usize),
        CorridorZone::MontaneSlope => (28usize, 14usize, 34usize, 3usize),
        CorridorZone::RoadCorridor => (16usize, 12usize, 32usize, 4usize),
        CorridorZone::TownOutskirtsCommunityGrounds => (12usize, 10usize, 28usize, 4usize),
    };

    let tree_species: Vec<&SpeciesWeight> = profile
        .species_weights
        .iter()
        .filter(|s| {
            matches!(
                s.stratum,
                EcologicalStratum::CanopyTree | EcologicalStratum::SubcanopyTree
            )
        })
        .collect();
    let shrub_species: Vec<&SpeciesWeight> = profile
        .species_weights
        .iter()
        .filter(|s| s.stratum == EcologicalStratum::Shrub)
        .collect();
    let grass_species: Vec<&SpeciesWeight> = profile
        .species_weights
        .iter()
        .filter(|s| s.stratum == EcologicalStratum::GrassTussock)
        .collect();
    let fauna_species: Vec<&SpeciesWeight> = profile
        .species_weights
        .iter()
        .filter(|s| s.stratum == EcologicalStratum::Fauna)
        .collect();

    let mut out = Vec::with_capacity(tree_budget + shrub_budget + grass_budget + fauna_budget);

    let strata_plans: [(usize, &[&SpeciesWeight]); 4] = [
        (tree_budget, &tree_species),
        (shrub_budget, &shrub_species),
        (grass_budget, &grass_species),
        (fauna_budget, &fauna_species),
    ];

    for (budget, candidates) in strata_plans {
        if candidates.is_empty() || budget == 0 {
            continue;
        }
        let mut placed = 0usize;
        let max_attempts = budget * 6;
        for _ in 0..max_attempts {
            if placed >= budget {
                break;
            }
            let Some(chosen) = pick_weighted_species(candidates, &mut rng) else {
                break;
            };
            let lx = 4.0 + unit_f32(&mut rng) * (TILE_METRES - 8.0);
            let lz = 4.0 + unit_f32(&mut rng) * (TILE_METRES - 8.0);
            if !is_position_clear_for_stratum(lx, lz, chosen.stratum, profile) {
                continue;
            }

            let inst_seed = splitmix64(&mut rng);
            let age = 0.25 + 0.75 * unit_f32(&mut rng);
            let height_scale = 0.8 + 0.45 * unit_f32(&mut rng);
            let site_vigor = 0.85 + 0.30 * unit_f32(&mut rng);
            let wind_stress = (profile.mean_slope_deg / 45.0).clamp(0.05, 0.75);
            let state64 = InstanceState64::new(
                inst_seed,
                age,
                height_scale,
                site_vigor,
                [wind_stress, 0.12, 0.08, 0.05],
            );

            // Simple deterministic elevation estimate along tile slope.
            let slope_tan = profile.mean_slope_deg.to_radians().tan();
            let elev_y = profile.mean_elevation_m + (lx - 50.0) * slope_tan * 0.25;

            out.push(TileInstancePlacement {
                species_slug: chosen.species_slug,
                stratum: chosen.stratum,
                world_pos: [ox + lx, elev_y, oz + lz],
                slope_deg: profile.mean_slope_deg,
                state64,
            });
            placed += 1;
        }
    }

    out
}

/// Driving surface at world `(x, z)`.
///
/// On the forestry-track mesh this is dirt, gravel, or sealed. Off the mesh,
/// wet bank, open water, and low pebble are creek. Everything else is grass.
/// Centreline samples of the mesh are never grass.
pub fn surface_at(x: f64, z: f64) -> RoadSurface {
    corridor_mesh::surface_at_with(ROAD_MESH, x, z, off_road_is_creek(x, z))
}

/// Centreline of the authored track, in the same metre frame as `ground::height_at`.
pub fn road_centreline() -> Vec<corridor_mesh::CentrelineSample> {
    corridor_mesh::centreline(ROAD_MESH)
}

/// Bytes of `production/corridor/forestry-road.bin`.
pub fn road_mesh_bytes() -> &'static [u8] {
    ROAD_MESH
}

/// `road=1` framing. The eye sits south of a centreline sample so the camp stays in frame.
/// Gum and root framing win when those queries are present. The default camera is unchanged.
pub fn road_proof_view() -> Option<([f32; 3], f32, f32, f32)> {
    let samples = road_centreline();
    let sample = samples.iter().find(|sample| sample.chainage_m >= 8.0)?;
    Some((
        [sample.x, sample.y + 1.2, sample.z],
        std::f32::consts::PI,
        0.55,
        48.0,
    ))
}

fn off_road_is_creek(x: f64, z: f64) -> bool {
    if !x.is_finite() || !z.is_finite() {
        return false;
    }
    let y = super::ground::height_at(x, z);
    let mat = super::ground::material_at(x, z);
    y < 0.24
        || mat == super::ground::GroundMaterial::WetBank
        || (mat == super::ground::GroundMaterial::Pebble && y < 1.2)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_arch = "wasm32")]
    use super::super::catalog::Aabb;

    #[cfg(target_arch = "wasm32")]
    fn unit_entry() -> CatalogEntry {
        CatalogEntry {
            entry_id: 1,
            scene_id: "unit".into(),
            place_id: "canberra-public-reference".into(),
            url: "/production/unit.10d".into(),
            sha256: "00".into(),
            bytes: 1,
            triangles: 1,
            vertices: 1,
            role: "Base".into(),
            obligation: "Mandatory".into(),
            aabb: Some(Aabb {
                min: [-0.5, -0.5, -0.5],
                max: [0.5, 0.5, 0.5],
            }),
            tiles: vec![(0, 0)],
        }
    }

    #[test]
    fn tile_xz_is_floor_division_not_xor() {
        assert_eq!(TILE_METRES, 100.0);
        assert_eq!(tile_xz(0.0, 0.0), (0, 0));
        assert_eq!(tile_xz(100.0, 0.0), (1, 0));
        assert_eq!(tile_xz(-0.1, -0.1), (-1, -1));
        assert_eq!(tile_xz(100.0, 200.0), (1, 2));
        assert_eq!(tile_xz(250.0, -150.0), (2, -2));
    }

    #[test]
    fn camera_pose_copies_xz_yaw_pitch_without_zoom() {
        let pose = camera_pose([12.5, 99.0, -3.25], 1.5, -0.25);
        assert_eq!(pose.x, 12.5);
        assert_eq!(pose.z, -3.25);
        assert_eq!(pose.yaw, 1.5);
        assert_eq!(pose.pitch, -0.25);
        assert_eq!(pose.y, 99.0);
    }

    #[test]
    fn aabb_overlaps_unit_at_origin_within_80m() {
        let pose = camera_pose([0.0, 0.0, 0.0], 0.0, 0.0);
        assert!(aabb_overlaps(
            [-0.5, -0.5, -0.5],
            [0.5, 0.5, 0.5],
            &pose,
            DEFAULT_RADIUS_M
        ));
        assert!(!aabb_overlaps(
            [200.0, -1.0, 200.0],
            [201.0, 1.0, 201.0],
            &pose,
            DEFAULT_RADIUS_M
        ));
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn overlapping_entries_windows_to_max_window() {
        let pose = camera_pose([0.0, 0.0, 0.0], 0.0, 0.0);
        let entries: Vec<CatalogEntry> = (0..MAX_WINDOW + 1).map(|_| unit_entry()).collect();
        let window = overlapping_entries(&entries, &pose, DEFAULT_RADIUS_M);
        assert_eq!(MAX_WINDOW, 64);
        assert_eq!(window.len(), MAX_WINDOW);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn scale_regime_tile_and_radius_are_monotonic() {
        assert_eq!(tile_size_for_regime(ScaleRegime::LocalEnu), 100.0);
        assert_eq!(radius_for_regime(ScaleRegime::LocalEnu), 80.0);
        assert!(
            tile_size_for_regime(ScaleRegime::LocalEnu)
                < tile_size_for_regime(ScaleRegime::PlanetaryWgs84)
        );
        assert!(
            tile_size_for_regime(ScaleRegime::PlanetaryWgs84)
                < tile_size_for_regime(ScaleRegime::HeliocentricIcrf)
        );
        assert!(
            tile_size_for_regime(ScaleRegime::HeliocentricIcrf)
                < tile_size_for_regime(ScaleRegime::GalacticCosmos)
        );
        assert!(
            radius_for_regime(ScaleRegime::LocalEnu)
                < radius_for_regime(ScaleRegime::PlanetaryWgs84)
        );
        assert!(
            radius_for_regime(ScaleRegime::PlanetaryWgs84)
                < radius_for_regime(ScaleRegime::HeliocentricIcrf)
        );
        assert!(
            radius_for_regime(ScaleRegime::HeliocentricIcrf)
                < radius_for_regime(ScaleRegime::GalacticCosmos)
        );
        assert_eq!(
            tile_xz_for_regime(
                88_500_030_005.59,
                47_161_666_865.26,
                ScaleRegime::HeliocentricIcrf
            ),
            (88, 47)
        );
    }

    #[test]
    fn active_tile_ring_3x3_tracks_entering_departing_and_lru_eviction() {
        let mut ring = ActiveTileRing::new(9);

        // Initial spawn at Forest Campsite (tile 0, 0) -> 9 entering, 0 departing.
        let d0 = ring.update_for_position(50.0, 50.0, 0);
        assert_eq!(d0.center, TileCoord::new(0, 0, 0));
        assert_eq!(d0.active.len(), 9);
        assert_eq!(d0.entering.len(), 9);
        assert!(d0.departing.is_empty());
        assert!(d0.lru_evicted.is_empty());

        // Moving inside the same tile produces a no-op delta.
        let d_same = ring.update_for_position(75.0, 20.0, 0);
        assert!(d_same.entering.is_empty());
        assert!(d_same.departing.is_empty());

        // Stepping east into RoadCorridor (tile 1, 0) -> 3 entering (tx=2), 3 departing (tx=-1).
        let d1 = ring.update_for_position(150.0, 50.0, 0);
        assert_eq!(d1.center, TileCoord::new(1, 0, 0));
        assert_eq!(d1.active.len(), 9);
        assert_eq!(d1.entering.len(), 3);
        assert_eq!(d1.departing.len(), 3);
        assert!(d1.entering.iter().all(|t| t.tx == 2));
        assert!(d1.departing.iter().all(|t| t.tx == -1));
        assert_eq!(d1.lru_evicted.len(), 3);
        assert!(d1.lru_evicted.iter().all(|t| t.tx == -1));
    }

    #[test]
    fn corridor_classification_spans_forest_campsite_road_and_community_grounds() {
        assert_eq!(
            classify_corridor_tile(TileCoord::new(0, 0, 0)),
            CorridorZone::ForestCampsite
        );
        assert_eq!(
            classify_corridor_tile(TileCoord::new(0, 1, 0)),
            CorridorZone::RiparianCreek
        );
        assert_eq!(
            classify_corridor_tile(TileCoord::new(1, 0, 0)),
            CorridorZone::RoadCorridor
        );
        assert_eq!(
            classify_corridor_tile(TileCoord::new(2, 0, 0)),
            CorridorZone::RoadCorridor
        );
        assert_eq!(
            classify_corridor_tile(TileCoord::new(1, 1, 0)),
            CorridorZone::MontaneSlope
        );
        assert_eq!(
            classify_corridor_tile(TileCoord::new(3, 0, 0)),
            CorridorZone::TownOutskirtsCommunityGrounds
        );
    }

    #[test]
    fn dataset_driven_placements_are_deterministic_and_keep_roads_and_oval_clear() {
        let road_tile = TileCoord::new(1, 0, 0);
        let road_profile = corridor_tile_profile(road_tile);
        let p1 = generate_tile_placements(road_tile, &road_profile, 0xCAFE_BABE);
        let p2 = generate_tile_placements(road_tile, &road_profile, 0xCAFE_BABE);
        assert_eq!(p1, p2, "placement generation must be strictly deterministic");
        assert!(!p1.is_empty());

        // Road centreline at z_local = 50.0, half_width = 6.5 -> no trees within 9.5 m of centreline.
        let rz = road_profile.road_centreline_z_local.unwrap();
        for inst in &p1 {
            let lz = inst.world_pos[2] - road_tile.world_origin_xz().1;
            if matches!(
                inst.stratum,
                EcologicalStratum::CanopyTree | EcologicalStratum::SubcanopyTree
            ) {
                assert!(
                    (lz - rz).abs() >= road_profile.road_half_width_m + 3.0 - 1e-4,
                    "tree {} encroached on road centreline at lz={lz}",
                    inst.species_slug
                );
            }
        }

        // Community Grounds (3, 0) must keep cleared oval paddock + gravel loop + shelter pad free of trees!
        let cg_tile = TileCoord::new(3, 0, 0);
        let cg_profile = corridor_tile_profile(cg_tile);
        let cg_placements = generate_tile_placements(cg_tile, &cg_profile, 0x15B0_2026);
        let cg = cg_profile.community_grounds.as_ref().unwrap();
        let (ox, oz) = cg_tile.world_origin_xz();

        let mut has_tree = false;
        let mut has_grass = false;
        let mut has_fauna = false;
        for inst in &cg_placements {
            let lx = inst.world_pos[0] - ox;
            let lz = inst.world_pos[2] - oz;
            match inst.stratum {
                EcologicalStratum::CanopyTree | EcologicalStratum::SubcanopyTree => {
                    has_tree = true;
                    let dist_oval = ((lx - cg.cleared_oval_center_local[0]).powi(2)
                        + (lz - cg.cleared_oval_center_local[1]).powi(2))
                    .sqrt();
                    let dist_shelter = ((lx - cg.shelter_pad_local[0]).powi(2)
                        + (lz - cg.shelter_pad_local[1]).powi(2))
                    .sqrt();
                    assert!(
                        dist_oval >= cg.gravel_loop_outer_m + 2.0 - 1e-4,
                        "tree placed inside Community Grounds cleared oval/gravel loop (dist={dist_oval})"
                    );
                    assert!(
                        dist_shelter >= cg.shelter_pad_radius_m + 3.0 - 1e-4,
                        "tree placed inside Community Grounds shelter pad (dist={dist_shelter})"
                    );
                }
                EcologicalStratum::GrassTussock => has_grass = true,
                EcologicalStratum::Fauna => has_fauna = true,
                EcologicalStratum::Shrub => {}
            }
        }
        assert!(has_tree, "Community Grounds perimeter must still include trees");
        assert!(has_grass, "Community Grounds must include native grass tussocks");
        assert!(has_fauna, "Community Grounds must include native fauna");
    }

    #[test]
    fn centreline_is_dirt_gravel_or_sealed_and_old_half_widths_stay() {
        assert_eq!(
            corridor_tile_profile(TileCoord::new(0, 0, 0)).road_half_width_m,
            4.5
        );
        assert_eq!(
            corridor_tile_profile(TileCoord::new(1, 0, 0)).road_half_width_m,
            6.5
        );
        assert_eq!(
            corridor_tile_profile(TileCoord::new(2, 0, 0)).road_half_width_m,
            6.5
        );
        assert_eq!(
            corridor_tile_profile(TileCoord::new(3, 0, 0)).road_half_width_m,
            5.0
        );
        let samples = road_centreline();
        assert!(samples.len() >= 40, "authored centreline missing");
        let mut dirt = false;
        let mut gravel = false;
        let mut sealed = false;
        let mut prev = 0u8;
        for sample in &samples {
            let surface = surface_at(sample.x as f64, sample.z as f64);
            assert_eq!(surface, sample.surface);
            assert!(
                matches!(
                    surface,
                    RoadSurface::Dirt | RoadSurface::Gravel | RoadSurface::Sealed
                ),
                "centreline surface {}",
                surface.as_str()
            );
            dirt |= surface == RoadSurface::Dirt;
            gravel |= surface == RoadSurface::Gravel;
            sealed |= surface == RoadSurface::Sealed;
            let rank = match surface {
                RoadSurface::Dirt => 0,
                RoadSurface::Gravel => 1,
                RoadSurface::Sealed => 2,
                _ => 9,
            };
            assert!(rank >= prev, "surface order");
            prev = rank;
            let half = sample.half_width_m;
            assert!(
                (half - 4.5).abs() < 1e-3
                    || (half - 5.0).abs() < 1e-3
                    || (half - 6.5).abs() < 1e-3,
                "{half}"
            );
        }
        assert!(dirt && gravel && sealed);
        assert_eq!(surface_at(-22.0, 28.0), RoadSurface::Grass);
        assert_ne!(surface_at(0.0, 56.0).as_str(), "grass");
    }

    #[test]
    fn authored_track_keeps_trees_off_the_carriageway() {
        let mut checked = 0usize;
        for (tx, ty) in [(-1, 0), (-1, 1)] {
            let tile = TileCoord::new(tx, ty, 0);
            let profile = corridor_tile_profile(tile);
            let placed = generate_tile_placements(tile, &profile, 0xB0A3_2026);
            let (ox, oz) = tile.world_origin_xz();
            for sample in road_centreline() {
                if sample.x < ox
                    || sample.x >= ox + TILE_METRES
                    || sample.z < oz
                    || sample.z >= oz + TILE_METRES
                {
                    continue;
                }
                checked += 1;
                for inst in &placed {
                    if !matches!(
                        inst.stratum,
                        EcologicalStratum::CanopyTree | EcologicalStratum::SubcanopyTree
                    ) {
                        continue;
                    }
                    let dx = inst.world_pos[0] - sample.x;
                    let dz = inst.world_pos[2] - sample.z;
                    let dist = (dx * dx + dz * dz).sqrt();
                    assert!(
                        dist + 1e-3 >= sample.half_width_m + 3.0,
                        "tree {} on the carriageway at {:.1} m",
                        inst.species_slug,
                        dist
                    );
                }
            }
        }
        assert!(checked > 10, "centreline did not cross the camp tiles");
    }
}
