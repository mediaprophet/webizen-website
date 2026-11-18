//! EWM streaming boot (PR6a/6b). Woodland scene graph is Q42 (CBOR-LD).
//! Flag off fails closed to the reading view; it does not fetch scene.json.
//! Camera-move SelectionIntent recompile is [`selection`].

#[allow(dead_code)]
mod flag;

#[allow(dead_code)]
pub mod selection;

use super::*;
use super::catalog::{Catalog, CatalogEntry};
use qualia_core_db::bundle::stream::descriptor::CandidateIdentity;
use qualia_core_db::bundle::stream::http::FallbackPolicy;
use qualia_core_db::bundle::stream::plan::{
    DependencyEdge, DependencyReason, EntryFamily, Inventory, Obligation, PlanEntry, PlanEntrySpec,
    PrincipalSelection, RepresentationClass, ResourceEstimate, Schedule, SelectionIntent,
    SelectionRequest,
};
use qualia_core_db::container_10d::MAGIC_10D;
use qualia_core_db::net::disclosure::NetworkDisclosureRegistry;
use qualia_core_db::net::fetch_range_wasm::FetchRangeService;
use qualia_core_db::net::qdnf::types::StrongDigest;

pub fn ewm_query_enabled(query: &str, canvas: &HtmlCanvasElement) -> bool {
    let q = query.trim_start_matches('?');
    let from_query = q.split('&').any(|p| p == "ewm=1");
    let from_attr = canvas
        .get_attribute("data-ewm-streaming")
        .as_deref()
        == Some("1");
    from_query || from_attr
}

pub(super) fn parse_hex48(hex: &str) -> Result<StrongDigest, JsValue> {
    if hex.len() != 96 {
        return Err(err("EWM candidate digest must be 96 hex characters"));
    }
    let mut out = [0u8; 48];
    for i in 0..48 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(err)?;
    }
    StrongDigest::from_bytes(&out).map_err(err)
}

pub(super) fn family_from_magic(bytes: &[u8]) -> Result<EntryFamily, JsValue> {
    if bytes.len() >= 4 && bytes[..4] == MAGIC_10D {
        Ok(EntryFamily::TenD)
    } else {
        Err(err("EWM family re-check failed: expected 10d magic"))
    }
}

pub struct FirstPaint {
    pub uploaded: JsValue,
    pub upload_receipt: Value,
    pub asset_receipts: Vec<Value>,
    pub texture_bindings: usize,
    pub total_bytes: usize,
    pub pointer_sha256: String,
    pub overlapping: Vec<String>,
    pub infosphere: bool,
    pub catalog: Catalog,
    pub colours: Vec<(String, [f32; 4])>,
    pub selection: selection::LiveSelection,
}

pub async fn load_first_paint(
    portal: &mut QualiaPortal,
    scene: &Scene,
    camera: &crate::policy::Camera,
    catalog: Catalog,
) -> Result<FirstPaint, JsValue> {
    text("native-world-status", "Opening the streamed landscape…");
    if catalog.schema != "webizen.ewm-catalog/v1" {
        return Err(err("Unsupported EWM catalog"));
    }
    if catalog.streaming_default {
        return Err(err("EWM catalog must keep streaming.default false"));
    }
    let place = catalog.place()?;
    let _stack = super::ocs_frames::colocated_stack(place)?;
    let infosphere = super::ocs_frames::infosphere_layer(place).is_some();
    let identity = CandidateIdentity::from_parts(
        parse_hex48(&catalog.candidate.descriptor_sha384)?,
        parse_hex48(&catalog.candidate.context_sha384)?,
        catalog.candidate.generation,
        catalog.candidate.entries,
    )
    .map_err(err)?;

    let ids = catalog.first_paint.clone();
    let declared_bytes = catalog.declared_bytes;
    let declared_tri = catalog.declared_triangles;
    let declared_vtx = catalog.declared_vertices;
    if declared_bytes == 0 || declared_bytes > 17 * 1024 * 1024 {
        return Err(err("EWM first-paint byte budget refused"));
    }

    // First paint fetches the SPARQL firstPaint window. Enhancement after
    // base is selection.rs (compose_window, tile debounce, window ≤64).
    let pose = super::gis_join::camera_pose(camera.target, camera.yaw, camera.pitch);
    let overlapping: Vec<String> = super::gis_join::overlapping_entries(
        &catalog.entries,
        &pose,
        super::gis_join::DEFAULT_RADIUS_M,
    )
    .into_iter()
    .map(|e| e.scene_id.clone())
    .collect();
    let selection = selection::live_selection_from_catalog(
        &catalog,
        camera.target,
        camera.yaw,
        camera.pitch,
    );
    let colours: Vec<(String, [f32; 4])> = scene
        .assets
        .iter()
        .map(|a| (a.id.clone(), a.colour))
        .collect();
    let pointer_sha256 = catalog.join.engine.clone();

    let mut plan_specs = Vec::new();
    let mut selected: Vec<CatalogEntry> = Vec::new();
    for id in &ids {
        let row = catalog
            .entry(id)
            .cloned()
            .ok_or_else(|| err(format!("First-paint id missing from catalog: {id}")))?;
        let url = row.url.as_str();
        if !(url.starts_with("/packages/") || url.starts_with("/production/")) {
            return Err(err("First-paint url must be same-origin /packages/ or /production/"));
        }
        plan_specs.push(PlanEntrySpec {
            candidate: identity,
            entry_id: row.entry_id,
            family: EntryFamily::TenD,
            role: RepresentationClass::Base,
            interval: None,
            timebase: None,
            estimate: ResourceEstimate::new(row.bytes, 0, 0),
            obligation: Obligation::Mandatory,
        });
        selected.push(row);
    }
    if plan_specs.len() > 64 {
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

    let fetch = FetchRangeService::new(NetworkDisclosureRegistry::new());
    let controller = web_sys::AbortController::new()?;
    let assets = js_sys::Array::new();
    let mut total_bytes = 0usize;
    let mut texture_bindings = 0usize;
    let mut material_registry = Vec::new();
    let mut asset_receipts = Vec::new();
    for (n, row) in selected.iter().enumerate() {
        text(
            "native-world-status",
            &format!(
                "Opening the streamed landscape — {} of {}",
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
                    .map_err(|e| {
                        controller.abort();
                        err(e)
                    })?,
            }
        } else {
            fetch
                .get_whole_entry(url, limit, None, Some(&signal))
                .await
                .map_err(|e| {
                    controller.abort();
                    err(e)
                })?
        };
        total_bytes += raw.len();
        if total_bytes as u64 > declared_bytes + 64 * 1024 {
            controller.abort();
            return Err(err("Streamed first-paint exceeds declared subset"));
        }
        let want = row.sha256.as_str();
        if hash(&raw) != want {
            controller.abort();
            return Err(err(format!("Asset digest mismatch: {}", row.scene_id)));
        }
        let family = family_from_magic(&raw)?;
        if family != EntryFamily::TenD {
            controller.abort();
            return Err(err("Family mismatch on admit"));
        }
        let id = row.scene_id.as_str();
        super::admission::check(id, &raw)?;
        texture_bindings +=
            super::textures::verify_asset(id, &raw, &scene.textures, &mut material_registry)?;
        let colour = scene
            .assets
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.colour)
            .unwrap_or([0.55, 0.6, 0.45, 1.0]);
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
        asset_receipts.push(json!({"id":id,"sha256":want,"streamed":true}));
    }
    let uploaded = portal.load_scene_assets(&assets)?;
    let upload_receipt: Value = serde_json::from_str(
        &js_sys::JSON::stringify(&uploaded)?
            .as_string()
            .ok_or(err("Invalid native upload receipt"))?,
    )
    .map_err(err)?;
    let authored_triangles = js_sys::Reflect::get(&uploaded, &"total_triangles".into())?
        .as_f64()
        .unwrap_or(0.) as u64;
    let organs_loaded = js_sys::Reflect::get(&uploaded, &"organs_loaded".into())
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.) as u64;
    let organs_refused = js_sys::Reflect::get(&uploaded, &"organs_refused".into())
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.) as u64;
    if authored_triangles != declared_tri {
        return Err(err(format!(
            "Native GPU subset triangle total differs from declared first-paint ({authored_triangles} vs {declared_tri}; organs_loaded={organs_loaded} organs_refused={organs_refused})"
        )));
    }
    let got_vtx = js_sys::Reflect::get(&uploaded, &"total_vertices".into())
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.) as u64;
    if got_vtx != 0 && declared_vtx != 0 && got_vtx != declared_vtx {
        return Err(err(format!(
            "Native GPU subset vertex total differs from declared first-paint ({got_vtx} vs {declared_vtx})"
        )));
    }
    Ok(FirstPaint {
        uploaded,
        upload_receipt,
        asset_receipts,
        texture_bindings,
        total_bytes,
        pointer_sha256,
        overlapping,
        infosphere,
        catalog,
        colours,
        selection,
    })
}
