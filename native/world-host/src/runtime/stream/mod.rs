//! EWM streaming boot (PR6a/6b). Dual-boot: flag off keeps whole-file scene fetch.

#[allow(dead_code)]
mod flag;

use super::*;
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

fn parse_hex48(hex: &str) -> Result<StrongDigest, JsValue> {
    if hex.len() != 96 {
        return Err(err("EWM candidate digest must be 96 hex characters"));
    }
    let mut out = [0u8; 48];
    for i in 0..48 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(err)?;
    }
    StrongDigest::from_bytes(&out).map_err(err)
}

fn family_from_magic(bytes: &[u8]) -> Result<EntryFamily, JsValue> {
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
}

pub async fn load_first_paint(
    portal: &mut QualiaPortal,
    scene: &Scene,
    camera: &crate::policy::Camera,
) -> Result<FirstPaint, JsValue> {
    text("native-world-status", "Opening the streamed landscape…");
    let pointer_bytes = bytes("/packages/current.json", 65536).await?;
    let pointer_sha256 = hash(&pointer_bytes);
    let pointer: Value = serde_json::from_slice(&pointer_bytes).map_err(err)?;
    if pointer["streaming"]["default"].as_bool() == Some(true) {
        return Err(err("EWM catalog pointer must keep streaming.default false"));
    }
    let catalog_url = pointer["catalog"]
        .as_str()
        .ok_or(err("EWM catalog pointer missing catalog url"))?;
    same_origin(catalog_url)?;
    let catalog: Value = json_file(catalog_url, 2 * 1024 * 1024).await?;
    if catalog["schema"] != "webizen.ewm-catalog/v1" {
        return Err(err("Unsupported EWM catalog"));
    }
    if catalog["streaming"]["default"].as_bool() == Some(true) {
        return Err(err("EWM catalog must keep streaming.default false"));
    }
    let place = super::gis_join::place_from_catalog(&catalog)?;
    let _stack = super::ocs_frames::colocated_stack(place)?;
    let infosphere = super::ocs_frames::infosphere_layer(place).is_some();
    let cand = &catalog["candidate"];
    let identity = CandidateIdentity::from_parts(
        parse_hex48(cand["descriptor_sha384"].as_str().ok_or(err("No descriptor"))?)?,
        parse_hex48(cand["context_sha384"].as_str().ok_or(err("No context"))?)?,
        cand["generation"].as_u64().unwrap_or(1),
        cand["entries"].as_u64().unwrap_or(0) as u32,
    )
    .map_err(err)?;

    let ids: Vec<String> = catalog["first_paint"]
        .as_array()
        .ok_or(err("No first_paint list"))?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let entries = catalog["entries"].as_array().ok_or(err("No catalog entries"))?;
    let declared = catalog["declared_subset"].clone();
    let declared_bytes = declared["bytes"].as_u64().unwrap_or(0);
    let declared_tri = declared["triangles"].as_u64().unwrap_or(0);
    let declared_vtx = declared["vertices"].as_u64().unwrap_or(0);
    if declared_bytes == 0 || declared_bytes > 15 * 1024 * 1024 {
        return Err(err("EWM first-paint byte budget refused"));
    }

    // First paint fetches the sealed first_paint window. overlapping_entries
    // records the camera-in-place AABB/tile join on the receipt. A later
    // camera-move SelectionIntent recompile is out of v1 first paint.
    let pose = super::gis_join::camera_pose(camera.target, camera.yaw, camera.pitch);
    let overlapping: Vec<String> = super::gis_join::overlapping_entries(
        entries,
        &pose,
        super::gis_join::DEFAULT_RADIUS_M,
    )
    .into_iter()
    .filter_map(|e| e["scene_id"].as_str().map(String::from))
    .collect();

    let mut plan_specs = Vec::new();
    let mut selected = Vec::new();
    for id in &ids {
        let row = entries
            .iter()
            .find(|e| e["scene_id"].as_str() == Some(id.as_str()))
            .ok_or_else(|| err(format!("First-paint id missing from catalog: {id}")))?;
        let url = row["url"].as_str().ok_or(err("catalog url"))?;
        if !(url.starts_with("/packages/") || url.starts_with("/production/")) {
            return Err(err("First-paint url must be same-origin /packages/ or /production/"));
        }
        let entry_id = row["entry_id"].as_u64().ok_or(err("entry_id"))? as u32;
        plan_specs.push(PlanEntrySpec {
            candidate: identity,
            entry_id,
            family: EntryFamily::TenD,
            role: RepresentationClass::Base,
            interval: None,
            timebase: None,
            estimate: ResourceEstimate::new(row["bytes"].as_u64().unwrap_or(0), 0, 0),
            obligation: Obligation::Mandatory,
        });
        selected.push(row.clone());
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
    if let Some(declared_edges) = catalog["edges"].as_array() {
        for edge in declared_edges {
            let from_id = edge["from_entry_id"].as_u64().ok_or(err("edge from_entry_id"))? as u32;
            let to_id = edge["to_entry_id"].as_u64().ok_or(err("edge to_entry_id"))? as u32;
            let Some(from) = plan_entries.iter().find(|e| e.entry_id() == from_id) else {
                continue;
            };
            let Some(to) = plan_entries.iter().find(|e| e.entry_id() == to_id) else {
                continue;
            };
            let reason = match edge["reason"].as_str() {
                Some("Geometry") => DependencyReason::Geometry,
                Some("Frame") => DependencyReason::Frame,
                Some("Calibration") => DependencyReason::Calibration,
                Some("Clock") => DependencyReason::Clock,
                Some("Query") => DependencyReason::Query,
                _ => return Err(err("Unsupported catalog dependency reason")),
            };
            let obligation = match edge["obligation"].as_str() {
                Some("Optional") => Obligation::Optional,
                _ => Obligation::Mandatory,
            };
            edges.push(DependencyEdge::bind(from, to, obligation, reason).map_err(err)?);
        }
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
        let url = row["url"].as_str().ok_or(err("catalog url"))?;
        fetch.authorize(url).map_err(err)?;
        let limit = 64 * 1024 * 1024;
        let signal = controller.signal();
        let raw = match row["bytes"].as_u64() {
            Some(size) if size > 0 => {
                match fetch
                    .get_range(
                        url,
                        0,
                        size - 1,
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
            }
            _ => fetch
                .get_whole_entry(url, limit, None, Some(&signal))
                .await
                .map_err(|e| {
                    controller.abort();
                    err(e)
                })?,
        };
        total_bytes += raw.len();
        if total_bytes as u64 > declared_bytes + 64 * 1024 {
            controller.abort();
            return Err(err("Streamed first-paint exceeds declared subset"));
        }
        let want = row["sha256"].as_str().ok_or(err("catalog sha256"))?;
        if hash(&raw) != want {
            controller.abort();
            return Err(err(format!(
                "Asset digest mismatch: {}",
                row["scene_id"].as_str().unwrap_or("?")
            )));
        }
        let family = family_from_magic(&raw)?;
        if family != EntryFamily::TenD {
            controller.abort();
            return Err(err("Family mismatch on admit"));
        }
        let id = row["scene_id"].as_str().ok_or(err("scene_id"))?;
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
    if authored_triangles != declared_tri {
        return Err(err("Native GPU subset triangle total differs from declared first-paint"));
    }
    let got_vtx = js_sys::Reflect::get(&uploaded, &"total_vertices".into())
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.) as u64;
    if got_vtx != 0 && declared_vtx != 0 && got_vtx != declared_vtx {
        return Err(err("Native GPU subset vertex total differs from declared first-paint"));
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
    })
}
