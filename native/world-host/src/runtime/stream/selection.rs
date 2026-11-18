//! Camera-move SelectionIntent recompile (after PR6a) plus per-place windows.
//!
//! Debounce on TILE change (`floor(x/100)`, `floor(z/100)`), not every frame
//! blend. Zoom is unused. Window ≤64: live place Base, then overlapping
//! Enhancement for that place. Field-studio Base is the woodland first-paint 24.
//! A selected live place does not keep global first-paint as an immortal
//! invariant. Does not scrape live GIS. Does not enable `streaming.default`.
//! Woodland scene graph is Q42. Flag off fails closed; it does not load scene.json.
//!
//! Integrator stores [`LiveSelection`] on `World` and calls [`fetch_window`]
//! from the frame loop when [`tile_changed`]. `select_place` aborts the prior
//! Range controller, [`compose_window`] for the live place, then
//! [`prepare_window`] / [`admit_prepared`] so `load_scene_assets` rebuilds.
//! Caller owns the AbortController. Do not hold a `World` borrow across the await.

#![allow(dead_code)]

#[path = "unload.rs"]
pub mod unload;
pub use unload::{
    abort_in_flight, missing_ids, omitted_ids, plan_place_change, static_member_url, PlaceChange,
    V1_UNLOAD,
};

use super::super::catalog::{Catalog, CatalogEntry};
use super::super::gis_join::{
    camera_pose, entry_overlaps, overlapping_entries, tile_xz, DEFAULT_RADIUS_M, MAX_WINDOW,
};
use super::super::{admission, err, hash, text, textures};
use super::{family_from_magic, parse_hex48};
use qualia_core_db::bundle::stream::descriptor::CandidateIdentity;
use qualia_core_db::bundle::stream::http::FallbackPolicy;
use qualia_core_db::bundle::stream::plan::{
    DependencyEdge, DependencyReason, EntryFamily, Inventory, Obligation, PlanEntry, PlanEntrySpec,
    PrincipalSelection, RepresentationClass, ResourceEstimate, Schedule, SelectionIntent,
    SelectionRequest,
};
use qualia_core_db::net::disclosure::NetworkDisclosureRegistry;
use qualia_core_db::net::fetch_range_wasm::FetchRangeService;
use qualia_core_db::QualiaPortal;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use wasm_bindgen::JsValue;

const DEFAULT_COLOUR: [f32; 4] = [0.55, 0.6, 0.45, 1.0];
const FIELD_STUDIO_PLACE: &str = "field-studio";

/// Live camera-tile window. `ids` are catalog `scene_id` strings, ≤64.
#[derive(Clone, Debug, Default)]
pub struct LiveSelection {
    pub tile: (i32, i32),
    pub ids: Vec<String>,
    pub place_id: String,
}

/// GPU rebuild of a live window. Triangle check uses the sum of those rows'
/// catalog `triangles`, not `scene.triangle_count` and not the first-paint 127688
/// unless this window happens to be that subset.
pub struct LiveWindow {
    pub uploaded: JsValue,
    pub upload_receipt: Value,
    pub asset_receipts: Vec<Value>,
    pub total_bytes: usize,
    pub declared_triangles: u64,
}

/// HTTP + admit bytes for a live window. Does not touch the portal, so the
/// frame loop can keep ticking the resident set while this runs.
pub struct PreparedWindow {
    pub assets: js_sys::Array,
    pub asset_receipts: Vec<Value>,
    pub total_bytes: usize,
    pub declared_triangles: u64,
}

impl LiveSelection {
    /// Boot fallback only: empty overlap may use the catalog first-paint list
    /// (still ≤64). Do not apply after a live place Base is selected.
    pub fn or_first_paint(mut self, first_paint: &[String]) -> Self {
        if self.ids.is_empty() {
            self.ids = first_paint.iter().take(MAX_WINDOW).cloned().collect();
            if self.place_id.is_empty() {
                self.place_id = FIELD_STUDIO_PLACE.into();
            }
        }
        self
    }
}

/// Overlapping catalog entries for the camera target, windowed to ≤64.
/// Prefers `scene_id` strings. Empty overlap stays empty; use
/// [`LiveSelection::or_first_paint`] or [`live_selection_from_catalog`] when
/// the catalog first-paint list is provided.
pub fn live_selection(
    entries: &[CatalogEntry],
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
) -> LiveSelection {
    let pose = camera_pose(target, yaw, pitch);
    let tile = tile_xz(pose.x, pose.z);
    let ids: Vec<String> = overlapping_entries(entries, &pose, DEFAULT_RADIUS_M)
        .into_iter()
        .map(|e| e.scene_id.clone())
        .take(MAX_WINDOW)
        .collect();
    LiveSelection {
        tile,
        ids,
        place_id: String::new(),
    }
}

/// Compose the live window for a PublicPlace: that place's Base, then
/// overlapping Enhancement for that place, cap 64.
///
/// Once a live place is selected, the window is that place's Base — not an
/// immortal global first-paint. Field-studio Base is the woodland first-paint
/// 24, so boot and field-studio travel keep today's four-tuple. Travelling to
/// river drops camper organs unless river Base names them.
pub fn compose_window(
    catalog: &Catalog,
    place_id: &str,
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
) -> LiveSelection {
    let pose = camera_pose(target, yaw, pitch);
    let tile = tile_xz(pose.x, pose.z);
    let live_id = if place_id.is_empty() {
        FIELD_STUDIO_PLACE
    } else {
        place_id
    };
    let base = catalog.live_place_base(live_id);
    let woodland_base = base_is_woodland_first_paint(catalog, &base);
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    for id in &base {
        if ids.len() >= MAX_WINDOW {
            break;
        }
        if seen.insert(id.clone()) {
            ids.push(id.clone());
        }
    }
    for entry in &catalog.entries {
        if ids.len() >= MAX_WINDOW {
            break;
        }
        if entry.role != "Enhancement" {
            continue;
        }
        if seen.contains(&entry.scene_id) {
            continue;
        }
        if !entry_overlaps(entry, &pose, DEFAULT_RADIUS_M) {
            continue;
        }
        if !woodland_base && !enhancement_belongs_to_place(catalog, live_id, entry) {
            continue;
        }
        seen.insert(entry.scene_id.clone());
        ids.push(entry.scene_id.clone());
    }
    LiveSelection {
        tile,
        ids,
        place_id: live_id.to_string(),
    }
}

/// Field-studio / boot window. Camera-move keeps this signature until the
/// integrator passes the live place into [`compose_window`].
pub fn live_selection_from_catalog(
    catalog: &Catalog,
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
) -> LiveSelection {
    compose_window(catalog, FIELD_STUDIO_PLACE, target, yaw, pitch)
}

fn base_is_woodland_first_paint(catalog: &Catalog, base: &[String]) -> bool {
    let paint: BTreeSet<&str> = catalog.first_paint.iter().map(String::as_str).collect();
    let live: BTreeSet<&str> = base.iter().map(String::as_str).collect();
    live == paint
}

fn enhancement_belongs_to_place(catalog: &Catalog, place_id: &str, entry: &CatalogEntry) -> bool {
    if entry.place_id == place_id {
        return true;
    }
    catalog
        .place_named(place_id)
        .map(|place| place.enhancement.iter().any(|id| id == &entry.scene_id))
        .unwrap_or(false)
}

pub fn tile_changed(prev: (i32, i32), next: (i32, i32)) -> bool {
    prev != next
}

/// Fetch, admit, and upload a live camera-move window (≤64).
///
/// Caller owns `controller`. On a new tile, abort the previous controller then
/// construct a fresh `web_sys::AbortController` before calling this again.
/// This function aborts `controller` on any error.
///
/// `scene_colours` is `(scene_id, rgba)` from the authored scene; unknown ids
/// use the woodland default. Same-origin `/packages/`, `/production/`, or
/// `/data/ewm/places/` via [`FetchRangeService`] whole-entry Range.
/// Does not enable `streaming.default`. Does not scrape live GIS.
pub async fn fetch_window(
    portal: &mut QualiaPortal,
    catalog: &Catalog,
    scene_ids: &[String],
    controller: &web_sys::AbortController,
    scene_colours: &[(String, [f32; 4])],
    scene_textures: &[textures::Texture],
) -> Result<LiveWindow, JsValue> {
    let prepared =
        prepare_window(catalog, scene_ids, controller, scene_colours, scene_textures).await?;
    admit_prepared(portal, prepared)
}

/// Range-then-GET the live id list. Caller then [`admit_prepared`] on the portal.
pub async fn prepare_window(
    catalog: &Catalog,
    scene_ids: &[String],
    controller: &web_sys::AbortController,
    scene_colours: &[(String, [f32; 4])],
    scene_textures: &[textures::Texture],
) -> Result<PreparedWindow, JsValue> {
    let result =
        prepare_window_inner(catalog, scene_ids, controller, scene_colours, scene_textures).await;
    if result.is_err() {
        controller.abort();
    }
    result
}

pub fn admit_prepared(
    portal: &mut QualiaPortal,
    prepared: PreparedWindow,
) -> Result<LiveWindow, JsValue> {
    if prepared.assets.length() == 0 {
        return Ok(LiveWindow {
            uploaded: JsValue::NULL,
            upload_receipt: json!({"empty": true, "streamed": true}),
            asset_receipts: prepared.asset_receipts,
            total_bytes: prepared.total_bytes,
            declared_triangles: prepared.declared_triangles,
        });
    }
    let uploaded = portal.load_scene_assets(&prepared.assets)?;
    let mut upload_receipt: Value = serde_json::from_str(
        &js_sys::JSON::stringify(&uploaded)?
            .as_string()
            .ok_or(err("Invalid native upload receipt"))?,
    )
    .map_err(err)?;
    let authored_triangles = js_sys::Reflect::get(&uploaded, &"total_triangles".into())?
        .as_f64()
        .unwrap_or(0.) as u64;
    // load_scene_assets already replaced the GPU mesh. Keep this receipt so
    // bind_resident / Pack-up rebind partition the mesh that is actually
    // resident. A catalog triangle mismatch used to return Err here and leave
    // w.upload on the previous first-paint parts ("instance group partition
    // leaves unowned geometry or records").
    upload_receipt["declared_triangles"] = json!(prepared.declared_triangles);
    upload_receipt["authored_triangles"] = json!(authored_triangles);
    if authored_triangles != prepared.declared_triangles {
        upload_receipt["triangle_mismatch"] = json!(true);
        upload_receipt["triangle_mismatch_note"] = json!(
            "Native GPU subset triangle total differs from the live window catalog sum"
        );
    }
    Ok(LiveWindow {
        uploaded,
        upload_receipt,
        asset_receipts: prepared.asset_receipts,
        total_bytes: prepared.total_bytes,
        declared_triangles: prepared.declared_triangles,
    })
}

fn role_of(row: &CatalogEntry) -> RepresentationClass {
    match row.role.as_str() {
        "Enhancement" => RepresentationClass::Enhancement,
        "Control" => RepresentationClass::Control,
        "Metadata" => RepresentationClass::Metadata,
        "Alternative" => RepresentationClass::Alternative,
        _ => RepresentationClass::Base,
    }
}

fn obligation_of(row: &CatalogEntry) -> Obligation {
    if row.obligation == "Optional" {
        Obligation::Optional
    } else {
        Obligation::Mandatory
    }
}

async fn prepare_window_inner(
    catalog: &Catalog,
    scene_ids: &[String],
    controller: &web_sys::AbortController,
    scene_colours: &[(String, [f32; 4])],
    scene_textures: &[textures::Texture],
) -> Result<PreparedWindow, JsValue> {
    if catalog.schema != "webizen.ewm-catalog/v1" {
        return Err(err("Unsupported EWM catalog"));
    }
    if catalog.streaming_default {
        return Err(err("EWM catalog must keep streaming.default false"));
    }
    if catalog.inner_ranges {
        return Err(err("EWM catalog must keep inner_ranges false"));
    }
    let mut seen = BTreeSet::new();
    let ids: Vec<String> = scene_ids
        .iter()
        .filter(|id| seen.insert((*id).clone()))
        .cloned()
        .collect();
    if ids.len() > MAX_WINDOW {
        return Err(err("EWM window exceeds 64"));
    }
    if ids.is_empty() {
        return Ok(PreparedWindow {
            assets: js_sys::Array::new(),
            asset_receipts: Vec::new(),
            total_bytes: 0,
            declared_triangles: 0,
        });
    }

    let identity = CandidateIdentity::from_parts(
        parse_hex48(&catalog.candidate.descriptor_sha384)?,
        parse_hex48(&catalog.candidate.context_sha384)?,
        catalog.candidate.generation,
        catalog.candidate.entries,
    )
    .map_err(err)?;

    let mut plan_specs = Vec::new();
    let mut selected = Vec::new();
    for id in &ids {
        let row = catalog
            .entry(id)
            .cloned()
            .ok_or_else(|| err(format!("Live-window id missing from catalog: {id}")))?;
        let url = row.url.as_str();
        if !static_member_url(url) {
            return Err(err(
                "Live-window url must be same-origin /packages/, /production/, or /data/ewm/places/",
            ));
        }
        plan_specs.push(PlanEntrySpec {
            candidate: identity,
            entry_id: row.entry_id,
            family: EntryFamily::TenD,
            role: role_of(&row),
            interval: None,
            timebase: None,
            estimate: ResourceEstimate::new(row.bytes, 0, 0),
            obligation: obligation_of(&row),
        });
        selected.push(row);
    }
    if plan_specs.len() > MAX_WINDOW {
        return Err(err("EWM window exceeds 64"));
    }
    let plan_entries: Vec<PlanEntry> = plan_specs
        .into_iter()
        .map(PlanEntry::new)
        .collect::<Result<_, _>>()
        .map_err(err)?;
    let mut edges = Vec::new();
    for edge in &catalog.edges {
        let Some(from) = plan_entries.iter().find(|e| e.entry_id() == edge.from_entry_id) else {
            continue;
        };
        let Some(to) = plan_entries.iter().find(|e| e.entry_id() == edge.to_entry_id) else {
            continue;
        };
        let reason = match edge.reason.as_str() {
            "Geometry" => DependencyReason::Geometry,
            "Frame" => DependencyReason::Frame,
            "Calibration" => DependencyReason::Calibration,
            "Clock" => DependencyReason::Clock,
            "Query" => DependencyReason::Query,
            _ => return Err(err("Unsupported catalog dependency reason")),
        };
        let obligation = if edge.obligation == "Optional" {
            Obligation::Optional
        } else {
            Obligation::Mandatory
        };
        edges.push(DependencyEdge::bind(from, to, obligation, reason).map_err(err)?);
    }
    let inventory = Inventory::compile(identity, &plan_entries, &edges).map_err(err)?;
    let intents: Vec<SelectionIntent> = plan_entries
        .iter()
        .map(|e| SelectionIntent::scene(e.entry_id(), EntryFamily::TenD, None, None))
        .collect::<Result<_, _>>()
        .map_err(err)?;
    let request = SelectionRequest::scene(&intents).map_err(err)?;
    let principal = PrincipalSelection::new(1, request).map_err(err)?;
    let _schedule = Schedule::compile(&inventory, &[principal], &[]).map_err(err)?;

    let declared_tri: u64 = selected.iter().map(|row| row.triangles).sum();
    let declared_bytes: u64 = selected.iter().map(|row| row.bytes).sum();

    let fetch = FetchRangeService::new(NetworkDisclosureRegistry::new());
    let assets = js_sys::Array::new();
    let mut total_bytes = 0usize;
    let mut asset_receipts = Vec::new();
    let mut material_registry = Vec::new();
    for (n, row) in selected.iter().enumerate() {
        text(
            "native-world-status",
            &format!(
                "Updating this view of the place — {} of {}",
                n + 1,
                selected.len()
            ),
        );
        let url = row.url.as_str();
        fetch.authorize(url).map_err(err)?;
        let limit = 64 * 1024 * 1024;
        let signal = controller.signal();
        let raw = if row.bytes > 0 {
            match fetch
                .get_range(
                    url,
                    0,
                    row.bytes - 1,
                    FallbackPolicy::FORBIDDEN,
                    None,
                    limit,
                    Some(&signal),
                )
                .await
            {
                Ok(body) => body,
                Err(_) => fetch
                    .get_whole_entry(url, limit, None, Some(&signal))
                    .await
                    .map_err(err)?,
            }
        } else {
            fetch
                .get_whole_entry(url, limit, None, Some(&signal))
                .await
                .map_err(err)?
        };
        total_bytes += raw.len();
        if declared_bytes > 0 && total_bytes as u64 > declared_bytes + 64 * 1024 {
            return Err(err("Streamed live window exceeds declared subset"));
        }
        let want = row.sha256.as_str();
        if hash(&raw) != want {
            return Err(err(format!("Asset digest mismatch: {}", row.scene_id)));
        }
        let family = family_from_magic(&raw)?;
        if family != EntryFamily::TenD {
            return Err(err("Family mismatch on admit"));
        }
        let id = row.scene_id.as_str();
        admission::check(id, &raw)?;
        let _ = textures::verify_asset(id, &raw, scene_textures, &mut material_registry)?;
        let colour = colour_of(scene_colours, id);
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"bytes".into(),
            &js_sys::Uint8Array::from(raw.as_slice()),
        )?;
        js_sys::Reflect::set(&obj, &"id".into(), &id.into())?;
        for (name, value) in ["r", "g", "b", "a"].iter().zip(colour) {
            js_sys::Reflect::set(&obj, &(*name).into(), &JsValue::from_f64(value as f64))?;
        }
        assets.push(&obj);
        asset_receipts.push(json!({"id":id,"sha256":want,"streamed":true,"live_window":true}));
    }
    Ok(PreparedWindow {
        assets,
        asset_receipts,
        total_bytes,
        declared_triangles: declared_tri,
    })
}

fn colour_of(scene_colours: &[(String, [f32; 4])], id: &str) -> [f32; 4] {
    scene_colours
        .iter()
        .find(|(name, _)| name == id)
        .map(|(_, c)| *c)
        .unwrap_or(DEFAULT_COLOUR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::catalog::{Aabb, Candidate, Place, SparqlJoin};

    const CAMPER: &str = "vintage-windup-camper--chassis";
    const ORIGIN: [f32; 3] = [0.0, 0.0, 0.0];

    fn place(id: &str, base: &[&str], enhancement: &[&str]) -> Place {
        Place {
            id: id.into(),
            title: id.into(),
            crs: "EPSG:4326".into(),
            bbox_wsen: [0.0; 4],
            elevation_m: [0.0; 2],
            gis_terrain_id: "canberra-reference-terrain".into(),
            authored_ground_id: "habitat-terrain".into(),
            usri: "urn:omni:v1:physical:observable:standard:earth:wgs84".into(),
            realm_class: "physical".into(),
            colocated: Vec::new(),
            base: base.iter().map(|s| (*s).to_string()).collect(),
            enhancement: enhancement.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    fn entry(scene_id: &str, role: &str, place_id: &str) -> CatalogEntry {
        CatalogEntry {
            entry_id: 1,
            scene_id: scene_id.into(),
            place_id: place_id.into(),
            url: format!("/production/{scene_id}.10d"),
            sha256: "00".into(),
            bytes: 1,
            triangles: 1,
            vertices: 1,
            role: role.into(),
            obligation: if role == "Enhancement" {
                "Optional".into()
            } else {
                "Mandatory".into()
            },
            aabb: Some(Aabb {
                min: [-1.0, 0.0, -1.0],
                max: [1.0, 4.0, 1.0],
            }),
            tiles: vec![(0, 0)],
        }
    }

    fn catalog() -> Catalog {
        let first_paint = vec![
            "canberra-reference-terrain".into(),
            "habitat-terrain".into(),
            "prototype-eucalyptus-lush-mid".into(),
            CAMPER.into(),
        ];
        let paint_refs: Vec<&str> = first_paint.iter().map(String::as_str).collect();
        Catalog {
            schema: "webizen.ewm-catalog/v1".into(),
            pack_id: "test".into(),
            inner_ranges: false,
            streaming_default: false,
            places: vec![
                place("field-studio", &paint_refs, &["field-studio-operator-awning"]),
                place(
                    "river",
                    &[
                        "canberra-reference-terrain",
                        "habitat-terrain",
                        "prototype-eucalyptus-lush-mid",
                        "tidal-vessel--bronze",
                    ],
                    &[],
                ),
            ],
            candidate: Candidate {
                descriptor_sha384: "00".repeat(48),
                context_sha384: "00".repeat(48),
                generation: 1,
                entries: 4,
            },
            first_paint: first_paint.clone(),
            entries: vec![
                entry("canberra-reference-terrain", "Base", "field-studio"),
                entry("habitat-terrain", "Base", "field-studio"),
                entry("prototype-eucalyptus-lush-mid", "Base", "field-studio"),
                entry(CAMPER, "Base", "field-studio"),
                entry("tidal-vessel--bronze", "Base", "river"),
                entry(
                    "field-studio-operator-awning",
                    "Enhancement",
                    "field-studio",
                ),
                entry(
                    "car-passat-b7--rubber",
                    "Enhancement",
                    "canberra-public-reference",
                ),
            ],
            edges: Vec::new(),
            declared_bytes: 1,
            declared_triangles: 1,
            declared_vertices: 1,
            join: SparqlJoin {
                engine: "GraphDatabase.sparql".into(),
                gis_place: "canberra-public-reference".into(),
                gis_terrain: "canberra-reference-terrain".into(),
                first_paint,
                sparql_hits: 4,
            },
        }
    }

    #[test]
    fn compose_window_river_omits_camper_that_field_studio_keeps() {
        let catalog = catalog();
        let studio = compose_window(&catalog, "field-studio", ORIGIN, 0.0, 0.0);
        let river = compose_window(&catalog, "river", ORIGIN, 0.0, 0.0);
        assert!(studio.ids.contains(&CAMPER.to_string()));
        assert!(
            !river.ids.contains(&CAMPER.to_string()),
            "river Base does not name the camper, so compose_window must omit it"
        );
        assert!(river.ids.contains(&"tidal-vessel--bronze".to_string()));
        assert_eq!(river.place_id, "river");
        assert!(river.ids.len() <= MAX_WINDOW);
    }

    #[test]
    fn field_studio_boot_window_still_uses_first_paint() {
        let catalog = catalog();
        let boot = live_selection_from_catalog(&catalog, ORIGIN, 0.0, 0.0);
        assert_eq!(boot.place_id, FIELD_STUDIO_PLACE);
        assert_eq!(&boot.ids[..catalog.first_paint.len()], catalog.first_paint.as_slice());
        assert!(boot.ids.contains(&CAMPER.to_string()));
    }

    #[test]
    fn plan_place_change_omits_camper_and_fetches_river_members() {
        let catalog = catalog();
        let studio = compose_window(&catalog, "field-studio", ORIGIN, 0.0, 0.0);
        let change = plan_place_change(&catalog, "river", ORIGIN, 0.0, 0.0, &studio.ids);
        assert!(change.omitted.iter().any(|id| id == CAMPER));
        assert!(change.missing.iter().any(|id| id == "tidal-vessel--bronze"));
        assert_eq!(change.place_id, "river");
        assert_eq!(V1_UNLOAD, "omit-and-rebuild");
    }

    #[test]
    fn first_paint_is_not_an_immortal_window_invariant() {
        let src = include_str!("selection.rs");
        let stays = format!("{} stays in the window", "First-paint");
        let always = format!("{} is always the base", "First-paint");
        let car = format!("if id == \"{}\"", "car");
        assert!(src.contains("pub fn compose_window"));
        assert!(src.contains("FetchRangeService"));
        assert!(!src.contains(&format!("fn bytes_{}", "range")));
        assert!(!src.contains(&stays));
        assert!(!src.contains(&always));
        assert!(!src.contains(&car));
        assert!(src.contains("must keep streaming.default false"));
        assert!(src.contains("must keep inner_ranges false"));
    }

    #[test]
    fn admit_prepared_keeps_loaded_receipt_when_catalog_triangles_differ() {
        let src = include_str!("selection.rs");
        assert!(src.contains("triangle_mismatch"));
        assert!(src.contains("load_scene_assets"));
        assert!(
            src.contains("Native GPU subset triangle total differs from the live window catalog sum"),
            "keep the mismatch phrase on the receipt so Pack-up can still rebind"
        );
        let load = src
            .find("let uploaded = portal.load_scene_assets")
            .expect("admit_prepared loads before it returns");
        let ret_err = src[load..].find("return Err(err(\n            \"Native GPU subset triangle total");
        assert!(
            ret_err.is_none(),
            "do not drop the loaded receipt on a catalog triangle mismatch"
        );
    }
}
