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

use super::super::super::catalog::Catalog;
use super::LiveSelection;
use std::collections::BTreeSet;

/// v1 GPU unload: omit from the new window and rebuild. Not PR10 scene-part unload.
pub const V1_UNLOAD: &str = "omit-and-rebuild";

const PACKAGES: &str = "/packages/";
const PRODUCTION: &str = "/production/";
const PLACE_VOLUMES: &str = "/data/ewm/places/";

/// Same-origin static members: organs under `/packages/` or `/production/`,
/// place volumes under `/data/ewm/places/`. Rejects `..`, `//`, and `\`.
pub fn static_member_url(url: &str) -> bool {
    if !url.starts_with('/') || url.starts_with("//") || url.contains("..") || url.contains('\\') {
        return false;
    }
    url.starts_with(PACKAGES) || url.starts_with(PRODUCTION) || url.starts_with(PLACE_VOLUMES)
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
pub fn abort_in_flight(controller: Option<&web_sys::AbortController>) {
    if let Some(controller) = controller {
        controller.abort();
    }
}

/// Place-change plan for the integrator: new window, omitted (leave GPU),
/// missing (whole-entry Range). Caller then `prepare_window` + `admit_prepared`
/// so `load_scene_assets` rebuilds the resident set.
#[derive(Clone, Debug)]
pub struct PlaceChange {
    pub place_id: String,
    pub window: LiveSelection,
    pub omitted: Vec<String>,
    pub missing: Vec<String>,
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn static_member_url_allows_packages_production_and_place_volumes() {
        assert!(static_member_url(
            "/packages/by-sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));
        assert!(static_member_url(
            "/production/native-world/assets/habitat-terrain.10d"
        ));
        assert!(static_member_url("/data/ewm/places/river.q42"));
        assert!(!static_member_url("/data/ewm/first-paint.json"));
        assert!(!static_member_url("https://example.invalid/river.q42"));
        assert!(!static_member_url("/data/ewm/places/../catalog.q42"));
        assert!(!static_member_url("//data/ewm/places/river.q42"));
    }
}
