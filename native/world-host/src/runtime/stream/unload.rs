//! v1 omit-and-rebuild of the resident GPU set on place change.
//!
//! Ids omitted from the new composed window leave the GPU because
//! `QualiaPortal::load_scene_assets` rebuilds from that window. This is not
//! QualiaDB PR10 `unload_scene_parts`. Fetch of missing members stays
//! `FetchRangeService` whole-entry Range; the host does not reimplement a
//! Range client. No streaming server.
//!
//! Included from `selection.rs` (`#[path]`) so this file compiles without a
//! `stream/mod.rs` edit. Integrator `select_place` / `take_ewm_tile_job`:
//! abort in-flight Range, `plan_place_change`, `prepare_window`, `admit_prepared`.

#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use super::super::super::catalog::Catalog;
#[cfg(target_arch = "wasm32")]
use super::LiveSelection;
use std::collections::{BTreeMap, BTreeSet};

/// v1 GPU unload: omit from the new window and rebuild. Not PR10 scene-part unload.
pub const V1_UNLOAD: &str = "omit-and-rebuild";

const PACKAGES: &str = "/packages/";
const PRODUCTION: &str = "/production/";
const PLACE_VOLUMES: &str = "/data/ewm/places/";
const INTENTECO: &str = "/intenteco/";

/// Same-origin static members: organs under `/packages/`, `/production/`, or
/// `/intenteco/`, and place volumes under `/data/ewm/places/`.
/// Rejects `..`, `//`, and `\`.
pub fn static_member_url(url: &str) -> bool {
    if !url.starts_with('/') || url.starts_with("//") || url.contains("..") || url.contains('\\') {
        return false;
    }
    url.starts_with(PACKAGES)
        || url.starts_with(PRODUCTION)
        || url.starts_with(PLACE_VOLUMES)
        || url.starts_with(INTENTECO)
}

/// Resident ids that are absent from the next window. They leave the GPU on rebuild.
pub fn omitted_ids(resident: &[String], next: &[String]) -> Vec<String> {
    let keep: BTreeSet<&str> = next.iter().map(String::as_str).collect();
    resident
        .iter()
        .filter(|id| !keep.contains(id.as_str()))
        .cloned()
        .collect()
}

/// Next-window ids that are not currently resident. Fetch these via FetchRangeService.
pub fn missing_ids(resident: &[String], next: &[String]) -> Vec<String> {
    let have: BTreeSet<&str> = resident.iter().map(String::as_str).collect();
    next.iter()
        .filter(|id| !have.contains(id.as_str()))
        .cloned()
        .collect()
}

/// Abort the in-flight Range GET before composing a new place window.
#[cfg(target_arch = "wasm32")]
pub fn abort_in_flight(controller: Option<&web_sys::AbortController>) {
    if let Some(controller) = controller {
        controller.abort();
    }
}

/// Place-change plan for the integrator: new window, omitted (leave GPU),
/// missing (whole-entry Range). Caller then `prepare_window` + `admit_prepared`
/// so `load_scene_assets` rebuilds the resident set.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
pub struct PlaceChange {
    pub place_id: String,
    pub window: LiveSelection,
    pub omitted: Vec<String>,
    pub missing: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
pub fn plan_place_change(
    catalog: &Catalog,
    place_id: &str,
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    resident: &[String],
) -> PlaceChange {
    let window = super::compose_window(catalog, place_id, target, yaw, pitch);
    let omitted = omitted_ids(resident, &window.ids);
    let missing = missing_ids(resident, &window.ids);
    PlaceChange {
        place_id: window.place_id.clone(),
        window,
        omitted,
        missing,
    }
}

// ---------------------------------------------------------------------------
// Tier-bounded LRU GPU Residency Manager & Incremental Add/Drop
// ---------------------------------------------------------------------------

/// Discrete Level-of-Detail tier for streamed scene assets.
/// `Lod0` is highest detail (nearest camera); `Lod2` is lowest detail (farthest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LodLevel {
    Lod0 = 0,
    Lod1 = 1,
    Lod2 = 2,
}

impl LodLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lod0 => "lod0",
            Self::Lod1 => "lod1",
            Self::Lod2 => "lod2",
        }
    }

    pub const fn is_higher_detail_than(self, other: Self) -> bool {
        (self as u8) < (other as u8)
    }

    pub const fn is_lower_detail_than(self, other: Self) -> bool {
        (self as u8) > (other as u8)
    }
}

/// GPU residency byte and triangle caps aligned with `runtime::graphics::DeviceTier`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TierBudget {
    pub tier_id: &'static str,
    pub max_bytes: u64,
    pub max_triangles: u64,
}

impl TierBudget {
    /// Desktop WebGPU primary tier: 384 MiB / 650k triangles.
    pub const fn tier_a() -> Self {
        Self {
            tier_id: "tier-a",
            max_bytes: 384 * 1024 * 1024,
            max_triangles: 650_000,
        }
    }

    /// Desktop WebGL2 / Mobile WebGPU tier: 192 MiB / 320k triangles.
    pub const fn tier_b() -> Self {
        Self {
            tier_id: "tier-b",
            max_bytes: 192 * 1024 * 1024,
            max_triangles: 320_000,
        }
    }

    /// Mobile WebGL2 conservative tier: 96 MiB / 150k triangles.
    pub const fn tier_c() -> Self {
        Self {
            tier_id: "tier-c",
            max_bytes: 96 * 1024 * 1024,
            max_triangles: 150_000,
        }
    }

    /// Resolve a `TierBudget` from tier identifiers such as `"tier-a"`, `"TierA"`,
    /// `"tier-b"`, `"TierB"`, `"tier-c"`, or `"TierC"`. Unknown tiers fall back
    /// conservatively to `TierC`.
    pub fn for_tier(tier_id: &str) -> Self {
        let norm = tier_id.trim().to_ascii_lowercase();
        match norm.as_str() {
            "tier-a" | "tiera" | "tier_a" | "a" => Self::tier_a(),
            "tier-b" | "tierb" | "tier_b" | "b" => Self::tier_b(),
            "tier-c" | "tierc" | "tier_c" | "c" => Self::tier_c(),
            _ => Self::tier_c(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl From<&super::super::super::graphics::TierBudget> for TierBudget {
    fn from(b: &super::super::super::graphics::TierBudget) -> Self {
        Self {
            tier_id: b.tier_id,
            max_bytes: b.max_gpu_memory_bytes as u64,
            max_triangles: b.max_resident_triangles,
        }
    }
}

/// One asset currently resident in GPU memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResidentAsset {
    pub id: String,
    pub lod_level: LodLevel,
    pub bytes: u64,
    pub triangles: u64,
    pub last_used_frame: u64,
    pub pinned: bool,
}

/// One asset requested by the active camera/place window at a target LOD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesiredAsset {
    pub id: String,
    pub lod_level: LodLevel,
    pub bytes: u64,
    pub triangles: u64,
    pub pinned: bool,
}

/// A resident asset whose LOD level should be swapped in-place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LodTransition {
    pub id: String,
    pub from_lod: LodLevel,
    pub to_lod: LodLevel,
    pub bytes: u64,
    pub triangles: u64,
}

/// Minimal incremental GPU residency transition computed by [`compute_incremental_delta`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResidencyDelta {
    pub to_add: Vec<DesiredAsset>,
    pub to_drop: Vec<String>,
    pub to_upgrade_lod: Vec<LodTransition>,
    pub to_downgrade_lod: Vec<LodTransition>,
    pub projected_bytes: u64,
    pub projected_triangles: u64,
}

impl ResidencyDelta {
    pub fn is_empty(&self) -> bool {
        self.to_add.is_empty()
            && self.to_drop.is_empty()
            && self.to_upgrade_lod.is_empty()
            && self.to_downgrade_lod.is_empty()
    }
}

/// Snapshot of current GPU residency against the active [`TierBudget`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResidencyStats {
    pub tier_id: &'static str,
    pub resident_count: usize,
    pub total_bytes: u64,
    pub total_triangles: u64,
    pub max_bytes: u64,
    pub max_triangles: u64,
}

/// LRU GPU residency ledger bounded by the active [`TierBudget`].
#[derive(Clone, Debug)]
pub struct GpuResidencyLedger {
    pub budget: TierBudget,
    entries: BTreeMap<String, ResidentAsset>,
}

impl GpuResidencyLedger {
    pub fn new(budget: TierBudget) -> Self {
        Self {
            budget,
            entries: BTreeMap::new(),
        }
    }

    pub fn set_budget(&mut self, budget: TierBudget) {
        self.budget = budget;
    }

    pub fn budget(&self) -> TierBudget {
        self.budget
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<&ResidentAsset> {
        self.entries.get(id)
    }

    pub fn resident_ids(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    pub fn total_bytes(&self) -> u64 {
        self.entries.values().map(|a| a.bytes).sum()
    }

    pub fn total_triangles(&self) -> u64 {
        self.entries.values().map(|a| a.triangles).sum()
    }

    pub fn stats(&self) -> ResidencyStats {
        ResidencyStats {
            tier_id: self.budget.tier_id,
            resident_count: self.entries.len(),
            total_bytes: self.total_bytes(),
            total_triangles: self.total_triangles(),
            max_bytes: self.budget.max_bytes,
            max_triangles: self.budget.max_triangles,
        }
    }

    pub fn lod_map(&self) -> BTreeMap<String, LodLevel> {
        self.entries
            .iter()
            .map(|(id, asset)| (id.clone(), asset.lod_level))
            .collect()
    }

    pub fn upsert(&mut self, asset: ResidentAsset) {
        self.entries.insert(asset.id.clone(), asset);
    }

    pub fn touch(&mut self, id: &str, frame: u64) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.last_used_frame = frame;
        }
    }

    pub fn remove(&mut self, id: &str) -> Option<ResidentAsset> {
        self.entries.remove(id)
    }

    /// Apply a computed [`ResidencyDelta`] to the ledger after GPU upload/eviction succeeds.
    pub fn apply_delta(&mut self, delta: &ResidencyDelta, current_frame: u64) {
        for id in &delta.to_drop {
            self.entries.remove(id);
        }
        for swap in delta
            .to_upgrade_lod
            .iter()
            .chain(delta.to_downgrade_lod.iter())
        {
            if let Some(entry) = self.entries.get_mut(&swap.id) {
                entry.lod_level = swap.to_lod;
                entry.bytes = swap.bytes;
                entry.triangles = swap.triangles;
                entry.last_used_frame = current_frame;
            }
        }
        for added in &delta.to_add {
            self.entries.insert(
                added.id.clone(),
                ResidentAsset {
                    id: added.id.clone(),
                    lod_level: added.lod_level,
                    bytes: added.bytes,
                    triangles: added.triangles,
                    last_used_frame: current_frame,
                    pinned: added.pinned,
                },
            );
        }
    }
}

/// Compute the minimal incremental add/drop/LOD-swap delta between the current
/// [`GpuResidencyLedger`] and `desired_window`, evicting least-recently-used
/// non-pinned assets whenever the active [`TierBudget`] would be exceeded.
pub fn compute_incremental_delta(
    ledger: &GpuResidencyLedger,
    desired_window: &[DesiredAsset],
    current_frame: u64,
) -> ResidencyDelta {
    // Deduplicate desired window preserving caller priority order (first wins).
    let mut desired_order: Vec<&DesiredAsset> = Vec::with_capacity(desired_window.len());
    let mut desired_map: BTreeMap<&str, &DesiredAsset> = BTreeMap::new();
    for item in desired_window {
        if desired_map.insert(item.id.as_str(), item).is_none() {
            desired_order.push(item);
        }
    }

    // Build projected resident map starting from current ledger state.
    let mut projected: BTreeMap<String, ResidentAsset> = ledger.entries.clone();
    let mut to_add: Vec<DesiredAsset> = Vec::new();
    let mut to_upgrade_lod: Vec<LodTransition> = Vec::new();
    let mut to_downgrade_lod: Vec<LodTransition> = Vec::new();

    for desired in &desired_order {
        if let Some(existing) = projected.get_mut(&desired.id) {
            existing.last_used_frame = current_frame;
            existing.pinned = desired.pinned;
            if desired.lod_level.is_higher_detail_than(existing.lod_level) {
                to_upgrade_lod.push(LodTransition {
                    id: desired.id.clone(),
                    from_lod: existing.lod_level,
                    to_lod: desired.lod_level,
                    bytes: desired.bytes,
                    triangles: desired.triangles,
                });
                existing.lod_level = desired.lod_level;
                existing.bytes = desired.bytes;
                existing.triangles = desired.triangles;
            } else if desired.lod_level.is_lower_detail_than(existing.lod_level) {
                to_downgrade_lod.push(LodTransition {
                    id: desired.id.clone(),
                    from_lod: existing.lod_level,
                    to_lod: desired.lod_level,
                    bytes: desired.bytes,
                    triangles: desired.triangles,
                });
                existing.lod_level = desired.lod_level;
                existing.bytes = desired.bytes;
                existing.triangles = desired.triangles;
            } else {
                existing.bytes = desired.bytes;
                existing.triangles = desired.triangles;
            }
        } else {
            projected.insert(
                desired.id.clone(),
                ResidentAsset {
                    id: desired.id.clone(),
                    lod_level: desired.lod_level,
                    bytes: desired.bytes,
                    triangles: desired.triangles,
                    last_used_frame: current_frame,
                    pinned: desired.pinned,
                },
            );
            to_add.push((*desired).clone());
        }
    }

    let mut projected_bytes: u64 = projected.values().map(|a| a.bytes).sum();
    let mut projected_triangles: u64 = projected.values().map(|a| a.triangles).sum();
    let max_bytes = ledger.budget.max_bytes;
    let max_triangles = ledger.budget.max_triangles;

    let mut to_drop: Vec<String> = Vec::new();

    // Phase 1: Evict least-recently-used non-pinned assets that are NOT in desired_window.
    if projected_bytes > max_bytes || projected_triangles > max_triangles {
        let mut lru_candidates: Vec<(u64, String)> = projected
            .values()
            .filter(|a| !a.pinned && !desired_map.contains_key(a.id.as_str()))
            .map(|a| (a.last_used_frame, a.id.clone()))
            .collect();
        // Oldest last_used_frame first; deterministic tie-break by id.
        lru_candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        for (_, id) in lru_candidates {
            if projected_bytes <= max_bytes && projected_triangles <= max_triangles {
                break;
            }
            if let Some(removed) = projected.remove(&id) {
                projected_bytes = projected_bytes.saturating_sub(removed.bytes);
                projected_triangles = projected_triangles.saturating_sub(removed.triangles);
                to_drop.push(id);
            }
        }
    }

    // Phase 2: If the desired window itself still exceeds the tier budget after all
    // non-desired LRU assets are evicted, defer lowest-priority non-pinned additions
    // (reverse order of desired_window) so the GPU stays strictly within tier caps.
    if projected_bytes > max_bytes || projected_triangles > max_triangles {
        let mut idx = to_add.len();
        while idx > 0 && (projected_bytes > max_bytes || projected_triangles > max_triangles) {
            idx -= 1;
            if to_add[idx].pinned {
                continue;
            }
            let cancelled = to_add.remove(idx);
            if let Some(removed) = projected.remove(&cancelled.id) {
                projected_bytes = projected_bytes.saturating_sub(removed.bytes);
                projected_triangles = projected_triangles.saturating_sub(removed.triangles);
            }
        }
    }

    // Phase 3: If an in-place LOD upgrade still overflows the tier budget after all
    // non-desired LRU evictions and deferred additions, hold non-pinned upgrades at
    // their previous LOD so budget invariants remain intact.
    if projected_bytes > max_bytes || projected_triangles > max_triangles {
        let mut idx = to_upgrade_lod.len();
        while idx > 0 && (projected_bytes > max_bytes || projected_triangles > max_triangles) {
            idx -= 1;
            let id = to_upgrade_lod[idx].id.clone();
            let Some(prev) = ledger.entries.get(&id) else {
                continue;
            };
            let is_pinned = projected.get(&id).map(|a| a.pinned).unwrap_or(false);
            if is_pinned {
                continue;
            }
            to_upgrade_lod.remove(idx);
            if let Some(entry) = projected.get_mut(&id) {
                projected_bytes = projected_bytes.saturating_sub(entry.bytes) + prev.bytes;
                projected_triangles =
                    projected_triangles.saturating_sub(entry.triangles) + prev.triangles;
                entry.lod_level = prev.lod_level;
                entry.bytes = prev.bytes;
                entry.triangles = prev.triangles;
            }
        }
    }

    ResidencyDelta {
        to_add,
        to_drop,
        to_upgrade_lod,
        to_downgrade_lod,
        projected_bytes,
        projected_triangles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;

    #[test]
    fn omit_and_rebuild_is_not_pr10_unload_scene_parts() {
        assert_eq!(V1_UNLOAD, "omit-and-rebuild");
        let src = include_str!("unload.rs");
        assert!(src.contains("load_scene_assets"));
        assert!(src.contains("not"));
        assert!(src.contains("unload_scene_parts"));
        assert!(!src.contains(&format!("fn bytes_{}", "range")));
        assert!(src.contains("FetchRangeService"));
    }

    #[test]
    fn omitted_ids_are_those_absent_from_the_next_window() {
        let resident = vec![
            "vintage-windup-camper--chassis".into(),
            "habitat-terrain".into(),
        ];
        let next = vec!["habitat-terrain".into(), "tidal-vessel--bronze".into()];
        let omitted = omitted_ids(&resident, &next);
        assert!(omitted.contains(&"vintage-windup-camper--chassis".into()));
        assert!(!omitted.contains(&"habitat-terrain".into()));
        let missing = missing_ids(&resident, &next);
        assert!(missing.contains(&"tidal-vessel--bronze".into()));
        assert!(!missing.contains(&"habitat-terrain".into()));
    }

    #[test]
    fn static_member_url_allows_packages_production_place_volumes_and_intenteco() {
        assert!(static_member_url(
            "/packages/by-sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));
        assert!(static_member_url(
            "/production/native-world/assets/habitat-terrain.10d"
        ));
        assert!(static_member_url("/data/ewm/places/river.q42"));
        assert!(static_member_url(
            "/intenteco/eucalyptus-blakelyi/aboveground_lod1.10d"
        ));
        assert!(!static_member_url("/data/ewm/first-paint.json"));
        assert!(!static_member_url("https://example.invalid/river.q42"));
        assert!(!static_member_url("/data/ewm/places/../catalog.q42"));
        assert!(!static_member_url("/intenteco/../secrets.10d"));
        assert!(!static_member_url("//data/ewm/places/river.q42"));
    }

    #[test]
    fn tier_budgets_match_tier_a_b_c_specs() {
        let a = TierBudget::for_tier("TierA");
        let b = TierBudget::for_tier("tier-b");
        let c = TierBudget::for_tier("TierC");
        assert_eq!(a.max_bytes, 384 * MIB);
        assert_eq!(a.max_triangles, 650_000);
        assert_eq!(b.max_bytes, 192 * MIB);
        assert_eq!(b.max_triangles, 320_000);
        assert_eq!(c.max_bytes, 96 * MIB);
        assert_eq!(c.max_triangles, 150_000);
    }

    #[test]
    fn lru_eviction_under_tier_c_bytes_and_triangles_respects_pinned_assets() {
        let mut ledger = GpuResidencyLedger::new(TierBudget::tier_c());
        // Pinned terrain base (frame 1, oldest, must NEVER be evicted).
        ledger.upsert(ResidentAsset {
            id: "habitat-terrain".into(),
            lod_level: LodLevel::Lod1,
            bytes: 20 * MIB,
            triangles: 30_000,
            last_used_frame: 1,
            pinned: true,
        });
        // Two unpinned warm assets from prior tiles: frame 2 (older) and frame 6 (newer).
        ledger.upsert(ResidentAsset {
            id: "old-flora-a".into(),
            lod_level: LodLevel::Lod1,
            bytes: 35 * MIB,
            triangles: 50_000,
            last_used_frame: 2,
            pinned: false,
        });
        ledger.upsert(ResidentAsset {
            id: "old-flora-b".into(),
            lod_level: LodLevel::Lod1,
            bytes: 30 * MIB,
            triangles: 45_000,
            last_used_frame: 6,
            pinned: false,
        });

        // Move to a new tile requesting terrain (pinned) + new-flora-c (35 MiB / 55k tris).
        // Total before eviction = 120 MiB (> 96 MiB) and 180k tris (> 150k tris).
        let desired = vec![
            DesiredAsset {
                id: "habitat-terrain".into(),
                lod_level: LodLevel::Lod1,
                bytes: 20 * MIB,
                triangles: 30_000,
                pinned: true,
            },
            DesiredAsset {
                id: "new-flora-c".into(),
                lod_level: LodLevel::Lod1,
                bytes: 35 * MIB,
                triangles: 55_000,
                pinned: false,
            },
        ];

        let delta = compute_incremental_delta(&ledger, &desired, 10);
        assert_eq!(delta.to_add.len(), 1);
        assert_eq!(delta.to_add[0].id, "new-flora-c");
        // Only the oldest unpinned asset (old-flora-a, frame 2) should be evicted;
        // habitat-terrain is pinned and old-flora-b stays warm within 96 MiB / 150k tris!
        assert_eq!(delta.to_drop, vec!["old-flora-a".to_string()]);
        assert_eq!(delta.projected_bytes, 85 * MIB);
        assert_eq!(delta.projected_triangles, 130_000);
        assert!(delta.projected_bytes <= TierBudget::tier_c().max_bytes);
        assert!(delta.projected_triangles <= TierBudget::tier_c().max_triangles);

        ledger.apply_delta(&delta, 10);
        assert!(ledger.get("habitat-terrain").is_some());
        assert!(ledger.get("old-flora-a").is_none());
        assert!(ledger.get("old-flora-b").is_some());
        assert!(ledger.get("new-flora-c").is_some());
        assert_eq!(ledger.total_bytes(), 85 * MIB);
        assert_eq!(ledger.total_triangles(), 130_000);
    }

    #[test]
    fn incremental_delta_tracks_lod_upgrades_and_downgrades() {
        let mut ledger = GpuResidencyLedger::new(TierBudget::tier_a());
        ledger.upsert(ResidentAsset {
            id: "eucalyptus-blakelyi".into(),
            lod_level: LodLevel::Lod2,
            bytes: 4 * MIB,
            triangles: 4_000,
            last_used_frame: 3,
            pinned: false,
        });
        ledger.upsert(ResidentAsset {
            id: "acacia-dealbata".into(),
            lod_level: LodLevel::Lod0,
            bytes: 18 * MIB,
            triangles: 36_000,
            last_used_frame: 3,
            pinned: false,
        });

        let desired = vec![
            DesiredAsset {
                id: "eucalyptus-blakelyi".into(),
                lod_level: LodLevel::Lod0,
                bytes: 20 * MIB,
                triangles: 42_000,
                pinned: false,
            },
            DesiredAsset {
                id: "acacia-dealbata".into(),
                lod_level: LodLevel::Lod1,
                bytes: 8 * MIB,
                triangles: 14_000,
                pinned: false,
            },
        ];

        let delta = compute_incremental_delta(&ledger, &desired, 4);
        assert!(delta.to_add.is_empty());
        assert!(delta.to_drop.is_empty());
        assert_eq!(delta.to_upgrade_lod.len(), 1);
        assert_eq!(delta.to_upgrade_lod[0].id, "eucalyptus-blakelyi");
        assert_eq!(delta.to_upgrade_lod[0].from_lod, LodLevel::Lod2);
        assert_eq!(delta.to_upgrade_lod[0].to_lod, LodLevel::Lod0);
        assert_eq!(delta.to_downgrade_lod.len(), 1);
        assert_eq!(delta.to_downgrade_lod[0].id, "acacia-dealbata");
        assert_eq!(delta.to_downgrade_lod[0].from_lod, LodLevel::Lod0);
        assert_eq!(delta.to_downgrade_lod[0].to_lod, LodLevel::Lod1);
        assert_eq!(delta.projected_bytes, 28 * MIB);
        assert_eq!(delta.projected_triangles, 56_000);
    }

    #[test]
    fn tier_c_triangle_pressure_alone_triggers_lru_eviction() {
        let mut ledger = GpuResidencyLedger::new(TierBudget::tier_c());
        ledger.upsert(ResidentAsset {
            id: "water-base".into(),
            lod_level: LodLevel::Lod1,
            bytes: 2 * MIB,
            triangles: 20_000,
            last_used_frame: 1,
            pinned: true,
        });
        ledger.upsert(ResidentAsset {
            id: "dense-shrub-old".into(),
            lod_level: LodLevel::Lod1,
            bytes: 5 * MIB,
            triangles: 90_000,
            last_used_frame: 2,
            pinned: false,
        });

        let desired = vec![
            DesiredAsset {
                id: "water-base".into(),
                lod_level: LodLevel::Lod1,
                bytes: 2 * MIB,
                triangles: 20_000,
                pinned: true,
            },
            DesiredAsset {
                id: "dense-tree-new".into(),
                lod_level: LodLevel::Lod1,
                bytes: 6 * MIB,
                triangles: 80_000,
                pinned: false,
            },
        ];

        // Bytes are tiny (13 MiB << 96 MiB), but triangles = 190k > 150k!
        let delta = compute_incremental_delta(&ledger, &desired, 8);
        assert_eq!(delta.to_drop, vec!["dense-shrub-old".to_string()]);
        assert_eq!(delta.projected_triangles, 100_000);
        assert!(delta.projected_triangles <= TierBudget::tier_c().max_triangles);
        assert!(!delta.is_empty());
        ledger.apply_delta(&delta, 8);
        let stats = ledger.stats();
        assert_eq!(stats.tier_id, "tier-c");
        assert_eq!(stats.resident_count, 2);
        assert_eq!(stats.total_triangles, 100_000);
        assert_eq!(ledger.lod_map().get("dense-tree-new"), Some(&LodLevel::Lod1));
        let noop = compute_incremental_delta(&ledger, &desired, 9);
        assert!(noop.is_empty());
    }
}
