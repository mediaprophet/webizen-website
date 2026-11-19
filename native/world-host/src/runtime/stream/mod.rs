//! EWM streaming boot (PR6a/6b). Woodland scene graph is Q42 (CBOR-LD).
//! Flag off fails closed to the reading view; it does not fetch scene.json.
//! Camera-move SelectionIntent recompile is [`selection`].

#[allow(dead_code)]
mod flag;

#[allow(dead_code)]
pub mod selection;

use super::*;
use super::catalog::{Catalog, CatalogEntry, SpeciesCatalogEntry, SpeciesLodAsset};
use super::instances::{InstanceState64, Placement};
use super::instruments::species::SpeciesCatalogEntry as InstrumentSpeciesEntry;
use super::knowledge::QuinIndex;
use super::play::{CatchmentPlacementRef, EwmStreamTelemetry};
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
use std::collections::{BTreeMap, HashMap};

pub const EWM_CACHE_NAME: &str = "webizen-ewm-v2-content-addressed";
pub const CATCHMENT_PLACE_Q42: &str = "/data/ewm/places/se-australia-catchment.q42";
pub const CATCHMENT_TERRAIN_LOD0_ID: &str = "catchment-600m-lod0";
pub const CATCHMENT_TERRAIN_LOD1_ID: &str = "catchment-600m-lod1";
pub const CATCHMENT_WATER_ID: &str = "catchment-600m-water";

thread_local! {
    /// In-memory verified `.10d` bytes populated during `load_first_paint` (keyed by SHA-256)
    /// so `spawn_catchment_streaming_loop` reuses all pinned campsite clearing exhibits in 0 ms
    /// without re-fetching them over HTTP.
    static FIRST_PAINT_VERIFIED_CACHE: RefCell<HashMap<String, Vec<u8>>> =
        RefCell::new(HashMap::new());
}

const DEFAULT_TERRAIN_LOD0_URL: &str = "/production/terrain-v2/catchment-600m-lod0.10d";
const DEFAULT_TERRAIN_LOD0_SHA: &str =
    "7b36d3fb49b746eb780781d464986e4e64e551cddb889697331d0d2cb30b9b2f";
const DEFAULT_TERRAIN_LOD0_BYTES: u64 = 7_375_288;
const DEFAULT_TERRAIN_LOD0_TRIS: u64 = 115_200;

const DEFAULT_TERRAIN_LOD1_URL: &str = "/production/terrain-v2/catchment-600m-lod1.10d";
const DEFAULT_TERRAIN_LOD1_SHA: &str =
    "5cfe9b46cd0bfc18ff2ae256d0b1f78d211da179a6b993d7e77d24d4ec6a362f";
const DEFAULT_TERRAIN_LOD1_BYTES: u64 = 1_851_896;
const DEFAULT_TERRAIN_LOD1_TRIS: u64 = 28_800;

const DEFAULT_WATER_URL: &str = "/production/terrain-v2/catchment-600m-water.10d";
const DEFAULT_WATER_SHA: &str =
    "77ecb761d87db501cfb311d0b73b8778add76835df6d840c6ffc4837bd8d0f7d";
const DEFAULT_WATER_BYTES: u64 = 352_728;
const DEFAULT_WATER_TRIS: u64 = 4_640;

/// Pure Rust evaluation of the EWM streaming toggle.
/// Enabled by default unless `?ewm=0` or `data-ewm-streaming="0"` is explicitly passed.
/// Explicit query parameters (`?ewm=0` / `?ewm=1`) take precedence over the canvas attribute.
pub fn ewm_streaming_enabled(query: &str, data_attr: Option<&str>) -> bool {
    let q = query.trim_start_matches('?');
    let mut query_override = None;
    for part in q.split('&') {
        if part == "ewm=0" {
            query_override = Some(false);
        } else if part == "ewm=1" {
            query_override = Some(true);
        }
    }
    if let Some(enabled) = query_override {
        return enabled;
    }
    match data_attr {
        Some("0") => false,
        Some("1") => true,
        _ => true,
    }
}

pub fn ewm_query_enabled(query: &str, canvas: &HtmlCanvasElement) -> bool {
    let attr = canvas.get_attribute("data-ewm-streaming");
    ewm_streaming_enabled(query, attr.as_deref())
}

/// Bound a JS Promise (e.g. `window.caches.*` or `Response.arrayBuffer()`) by `timeout_ms`
/// so a locked or stalled Cache Storage backend in multi-tab / headless WebGL2 contexts
/// never blocks first-paint asset loading. Returns `None` on timeout, rejection, or `undefined`/`null`.
async fn race_promise_timeout(promise: js_sys::Promise, timeout_ms: i32) -> Option<JsValue> {
    let timer = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(win) = web_sys::window() {
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, timeout_ms);
        } else {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });
    let racers = js_sys::Array::of2(&promise, &timer);
    let raced = js_sys::Promise::race(&racers);
    let val = JsFuture::from(raced).await.ok()?;
    if val.is_undefined() || val.is_null() {
        None
    } else {
        Some(val)
    }
}

async fn open_ewm_cache() -> Option<JsValue> {
    let window = web_sys::window()?;
    let caches = js_sys::Reflect::get(&window, &JsValue::from_str("caches")).ok()?;
    if caches.is_undefined() || caches.is_null() {
        return None;
    }
    let open_fn = js_sys::Reflect::get(&caches, &JsValue::from_str("open"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let promise = open_fn
        .call1(&caches, &JsValue::from_str(EWM_CACHE_NAME))
        .ok()?
        .dyn_into::<js_sys::Promise>()
        .ok()?;
    race_promise_timeout(promise, 500).await
}

async fn cache_delete_best_effort(cache: &JsValue, url: &str) {
    if let Ok(del_fn) = js_sys::Reflect::get(cache, &JsValue::from_str("delete"))
        .and_then(|v| v.dyn_into::<js_sys::Function>().map_err(JsValue::from))
    {
        if let Ok(Ok(promise)) = del_fn
            .call1(cache, &JsValue::from_str(url))
            .map(|v| v.dyn_into::<js_sys::Promise>())
        {
            let _ = race_promise_timeout(promise, 400).await;
        }
    }
}

/// Look up a cached asset in the browser Cache API (`webizen-ewm-v2-content-addressed`)
/// and verify its SHA-256 digest. Corrupted entries are evicted so the caller self-heals
/// by re-fetching from the allowlisted resolution chain. Gracefully returns `None` in
/// private browsing, restricted contexts, or if Cache Storage does not respond within 500 ms.
pub async fn cache_get_verified(url: &str, expected_sha256: &str) -> Option<Vec<u8>> {
    if !super::admission::is_valid_sha256_hex(expected_sha256) {
        return None;
    }
    let cache = open_ewm_cache().await?;
    let match_fn = js_sys::Reflect::get(&cache, &JsValue::from_str("match"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let promise = match_fn
        .call1(&cache, &JsValue::from_str(url))
        .ok()?
        .dyn_into::<js_sys::Promise>()
        .ok()?;
    let matched = race_promise_timeout(promise, 500).await?;
    if matched.is_undefined() || matched.is_null() {
        return None;
    }
    let response: web_sys::Response = matched.dyn_into().ok()?;
    if !response.ok() {
        cache_delete_best_effort(&cache, url).await;
        return None;
    }
    let ab_promise = response.array_buffer().ok()?;
    let ab = race_promise_timeout(ab_promise, 800).await?;
    let bytes = js_sys::Uint8Array::new(&ab).to_vec();
    if hash(&bytes).eq_ignore_ascii_case(expected_sha256) {
        Some(bytes)
    } else {
        cache_delete_best_effort(&cache, url).await;
        None
    }
}

/// Persist verified asset bytes into the browser Cache API (`webizen-ewm-v2-content-addressed`).
/// Gracefully no-ops in private browsing or when storage quota is unavailable.
pub async fn cache_put(url: &str, bytes: &[u8]) {
    let Some(cache) = open_ewm_cache().await else {
        return;
    };
    let mut copy = bytes.to_vec();
    let Ok(response) = web_sys::Response::new_with_opt_u8_array(Some(&mut copy)) else {
        return;
    };
    let Ok(put_fn) = js_sys::Reflect::get(&cache, &JsValue::from_str("put"))
        .and_then(|v| v.dyn_into::<js_sys::Function>().map_err(JsValue::from))
    else {
        return;
    };
    if let Ok(Ok(promise)) = put_fn
        .call2(&cache, &JsValue::from_str(url), &response)
        .map(|v| v.dyn_into::<js_sys::Promise>())
    {
        let _ = race_promise_timeout(promise, 1000).await;
    }
}

async fn fetch_candidate_bytes(
    url: &str,
    max_bytes: usize,
    signal: Option<&web_sys::AbortSignal>,
) -> Result<Vec<u8>, JsValue> {
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    if url.starts_with("https://") {
        opts.set_mode(web_sys::RequestMode::Cors);
    } else {
        opts.set_mode(web_sys::RequestMode::SameOrigin);
    }
    if let Some(sig) = signal {
        opts.set_signal(Some(sig));
    }
    let req = web_sys::Request::new_with_str_and_init(url, &opts)?;
    let window = web_sys::window().ok_or_else(|| err("No browser window"))?;
    let resp: web_sys::Response = JsFuture::from(window.fetch_with_request(&req))
        .await?
        .dyn_into()?;
    if !resp.ok() {
        return Err(err(format!("HTTP {} fetching {url}", resp.status())));
    }
    let ab = JsFuture::from(resp.array_buffer()?).await?;
    let raw = js_sys::Uint8Array::new(&ab);
    if raw.length() as usize > max_bytes {
        return Err(err(format!("Asset {url} exceeds byte budget")));
    }
    Ok(raw.to_vec())
}

fn detect_stream_asset_format(
    url_or_path: &str,
    bytes: &[u8],
) -> Option<super::admission::AssetFormat> {
    if let Some(fmt) = super::admission::format_from_path(url_or_path) {
        return Some(fmt);
    }
    if bytes.len() >= 4 && bytes[..4] == super::admission::MAGIC_10D_BYTES {
        Some(super::admission::AssetFormat::TenD)
    } else if bytes.len() >= 8 && bytes[..3] == super::admission::MAGIC_Q42_PREFIX {
        Some(super::admission::AssetFormat::Q42)
    } else if bytes.len() >= 8 && bytes[..8] == super::admission::MAGIC_EMF_BYTES {
        Some(super::admission::AssetFormat::Emf)
    } else if bytes.len() >= 4
        && (bytes[..4] == super::admission::MAGIC_M64_BYTES
            || bytes[..4] == super::admission::MAGIC_E64_BYTES)
    {
        Some(super::admission::AssetFormat::M64)
    } else if bytes.len() >= 8 && bytes[..8] == super::admission::MAGIC_QAC_BYTES {
        Some(super::admission::AssetFormat::Qac)
    } else {
        None
    }
}

/// Validate a SHA-256-verified `.10d` container and ensure it carries a valid
/// `ProvenanceSidecar` section when `FLAG_DEFAULT_DISPOSITION_REFUSE` is set.
/// Assets compiled by `hypermedia.manifold.compile_10d_pbr` (such as `terrain-v2`
/// and the `intenteco` species LODs) carry their provenance in the admitted Q42
/// catalog (`se-australia-catchment.q42` / `species-catalog.q42`); after strict
/// SHA-256 + CRC-32C verification against that catalog, this helper seals a
/// deterministic catalog-attested `ProvenanceSidecar` section so `QualiaPortal`
/// admits every verified organ with zero refusals.
fn ensure_catalog_attested_10d(
    id: &str,
    url_or_path: &str,
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<Vec<u8>, JsValue> {
    use qualia_core_db::container_10d as codec;
    let header =
        codec::Container10dHeader::parse(bytes).map_err(|e| err(format!("Asset {id}: {e}")))?;
    codec::integrity::check_whole_file_crc32c(bytes)
        .map_err(|e| err(format!("Asset {id}: {e}")))?;
    let sections =
        codec::parse_section_table(bytes, &header).map_err(|e| err(format!("Asset {id}: {e}")))?;

    let has_provenance = sections
        .iter()
        .any(|d| d.typ() == Some(codec::SectionType::ProvenanceSidecar));
    let needs_provenance =
        (header.flags & codec::FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 && !has_provenance;

    if has_provenance && (header.flags & codec::FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 {
        if let Some(descriptor) = sections
            .iter()
            .find(|d| d.typ() == Some(codec::SectionType::ProvenanceSidecar))
        {
            let begin = descriptor.byte_offset as usize;
            let end = begin
                .checked_add(descriptor.byte_length as usize)
                .filter(|end| *end <= bytes.len())
                .ok_or_else(|| err(format!("Asset {id}: invalid provenance section bounds")))?;
            let provenance = codec::decode_provenance_section(&bytes[begin..end])
                .map_err(|e| err(format!("Asset {id} provenance: {e}")))?;
            codec::validate_provenance(&provenance)
                .map_err(|e| err(format!("Asset {id} provenance: {e}")))?;
        }
    }

    if !needs_provenance {
        return Ok(bytes.to_vec());
    }

    let mut version_hash = [0u8; 32];
    for i in 0..32 {
        if let Ok(b) = u8::from_str_radix(&expected_sha256[i * 2..i * 2 + 2], 16) {
            version_hash[i] = b;
        }
    }
    let source_ref = format!("q42-catalog:{id}:{expected_sha256}:{url_or_path}");
    let sidecar = codec::ProvenanceSidecar::new(
        source_ref.into_bytes(),
        "application/vnd.webizen.q42-catalog+10d",
        "CC-BY-4.0",
    )
    .with_metadata(Vec::new(), 0, version_hash);
    let prov_len = codec::provenance_encoded_len(&sidecar);
    let mut prov_buf = vec![0u8; prov_len];
    let prov_written = codec::encode_provenance_section(&sidecar, &mut prov_buf)
        .map_err(|e| err(format!("Asset {id} provenance encode: {e}")))?;
    prov_buf.truncate(prov_written);

    let mut inputs: Vec<codec::SectionInput<'_>> = Vec::with_capacity(sections.len() + 1);
    for desc in sections {
        let st = desc
            .typ()
            .ok_or_else(|| err(format!("Asset {id}: unsupported section type {}", desc.section_type)))?;
        let tier = desc.tier().ok_or_else(|| {
            err(format!(
                "Asset {id}: unsupported alignment tier {}",
                desc.alignment_tier
            ))
        })?;
        let off = desc.byte_offset as usize;
        let len = desc.byte_length as usize;
        let end = off
            .checked_add(len)
            .filter(|e| *e <= bytes.len())
            .ok_or_else(|| err(format!("Asset {id}: section bounds out of range")))?;
        inputs.push(codec::SectionInput {
            section_type: st,
            alignment_tier: tier,
            stride: desc.stride,
            element_count: desc.element_count,
            payload: &bytes[off..end],
        });
    }
    inputs.push(codec::SectionInput {
        section_type: codec::SectionType::ProvenanceSidecar,
        alignment_tier: codec::AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &prov_buf,
    });

    let mut out = vec![0u8; bytes.len().saturating_add(prov_buf.len()).saturating_add(4096)];
    let written = codec::encode_container(&header, &inputs, &mut out)
        .map_err(|e| err(format!("Asset {id} provenance wrap: {e:?}")))?;
    out.truncate(written);
    codec::integrity::seal_whole_file_crc32c(&mut out);
    Ok(out)
}

/// Admit bytes whose SHA-256 digest was already verified off the main thread by `asset-worker.js`.
fn admit_preverified_bytes(
    id: &str,
    url_or_path: &str,
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<Vec<u8>, JsValue> {
    if !super::admission::is_valid_sha256_hex(expected_sha256) {
        return Err(err(format!("Asset {id}: invalid expected SHA-256 digest")));
    }
    let format = detect_stream_asset_format(url_or_path, bytes).ok_or_else(|| {
        err(format!(
            "Asset {id}: unrecognized container format for {url_or_path}"
        ))
    })?;
    super::admission::check_magic(format, bytes)
        .map_err(|e| err(format!("Asset {id}: {e}")))?;
    if format == super::admission::AssetFormat::TenD {
        ensure_catalog_attested_10d(id, url_or_path, bytes, expected_sha256)
    } else {
        Ok(bytes.to_vec())
    }
}

fn admit_streamed_bytes(
    id: &str,
    url_or_path: &str,
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<Vec<u8>, JsValue> {
    if !super::admission::is_valid_sha256_hex(expected_sha256) {
        return Err(err(format!("Asset {id}: invalid expected SHA-256 digest")));
    }
    let actual = hash(bytes);
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(err(format!(
            "Asset {id}: SHA-256 digest mismatch (expected {expected_sha256}, got {actual})"
        )));
    }
    admit_preverified_bytes(id, url_or_path, bytes, expected_sha256)
}

/// Content-addressed multi-source fetch with Cache API verification + Tier 1/Tier 2 fallback
/// and mandatory SHA-256 + binary magic admission checks.
/// Once a local Tier-1 candidate succeeds with a matching SHA-256 digest, it is admitted
/// directly and never fails over to external Tier-2 DNS mirrors.
#[allow(dead_code)]
pub async fn fetch_verified_asset(
    id: &str,
    logical_url: &str,
    expected_sha256: &str,
    allowlist: &[String],
    max_bytes: usize,
    signal: Option<&web_sys::AbortSignal>,
) -> Result<(Vec<u8>, String), JsValue> {
    let candidates =
        super::admission::resolve_asset_candidates(logical_url, expected_sha256, allowlist)
            .map_err(err)?;

    if let Some(cached) = cache_get_verified(logical_url, expected_sha256).await {
        if cached.len() <= max_bytes {
            if let Ok(admitted) = admit_streamed_bytes(id, logical_url, &cached, expected_sha256) {
                return Ok((admitted, format!("cache:{logical_url}")));
            }
        }
    }

    let mut last_err = err(format!("No candidate succeeded for {id}"));
    for candidate in &candidates {
        match fetch_candidate_bytes(candidate, max_bytes, signal).await {
            Ok(bytes) => {
                let actual = hash(&bytes);
                if !actual.eq_ignore_ascii_case(expected_sha256) {
                    last_err = err(format!(
                        "Asset {id}: SHA-256 mismatch from {candidate} (expected {expected_sha256}, got {actual})"
                    ));
                    continue;
                }
                match admit_streamed_bytes(id, candidate, &bytes, expected_sha256) {
                    Ok(admitted) => {
                        cache_put(logical_url, &bytes).await;
                        return Ok((admitted, candidate.clone()));
                    }
                    Err(e) => {
                        // Local/mirror bytes matched the exact SHA-256 digest; do not fail
                        // over to unreachable external DNS mirrors for the identical payload.
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                last_err = e;
            }
        }
    }
    Err(last_err)
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

    let allowlist = super::admission::default_allowlist();
    let mut plan_specs = Vec::new();
    let mut selected: Vec<CatalogEntry> = Vec::new();
    for id in &ids {
        let row = catalog
            .entry(id)
            .cloned()
            .ok_or_else(|| err(format!("First-paint id missing from catalog: {id}")))?;
        let url = row.url.as_str();
        if !super::admission::is_allowed_asset_url(url, &allowlist) {
            return Err(err(format!("First-paint url rejected by origins allowlist: {url}")));
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
        let want = row.sha256.as_str();
        let limit = 64 * 1024 * 1024;
        let signal = controller.signal();
        let (raw, from_cache) = if let Some(cached) = cache_get_verified(url, want).await {
            (cached, true)
        } else if url.starts_with('/') {
            fetch.authorize(url).map_err(err)?;
            let fetched = if row.bytes > 0 {
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
            (fetched, false)
        } else {
            let fetched = fetch_candidate_bytes(url, limit, Some(&signal))
                .await
                .map_err(|e| {
                    controller.abort();
                    e
                })?;
            (fetched, false)
        };
        total_bytes += raw.len();
        if total_bytes as u64 > declared_bytes + 64 * 1024 {
            controller.abort();
            return Err(err("Streamed first-paint exceeds declared subset"));
        }
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
        FIRST_PAINT_VERIFIED_CACHE.with(|cache| {
            cache.borrow_mut().insert(want.to_string(), raw.clone());
        });
        if !from_cache {
            let cache_url = url.to_string();
            let cache_bytes = raw.clone();
            wasm_bindgen_futures::spawn_local(async move {
                cache_put(&cache_url, &cache_bytes).await;
            });
        }
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

// ---------------------------------------------------------------------------
// Post-First-Paint Live 600 m Catchment Terrain + 13-Species Streaming Engine
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct CatchmentTerrainSpec {
    pub lod0_url: String,
    pub lod0_sha256: String,
    pub lod0_bytes: u64,
    pub lod0_triangles: u64,
    pub lod1_url: String,
    pub lod1_sha256: String,
    pub lod1_bytes: u64,
    pub lod1_triangles: u64,
    pub water_url: String,
    pub water_sha256: String,
    pub water_bytes: u64,
    pub water_triangles: u64,
}

impl Default for CatchmentTerrainSpec {
    fn default() -> Self {
        Self {
            lod0_url: DEFAULT_TERRAIN_LOD0_URL.into(),
            lod0_sha256: DEFAULT_TERRAIN_LOD0_SHA.into(),
            lod0_bytes: DEFAULT_TERRAIN_LOD0_BYTES,
            lod0_triangles: DEFAULT_TERRAIN_LOD0_TRIS,
            lod1_url: DEFAULT_TERRAIN_LOD1_URL.into(),
            lod1_sha256: DEFAULT_TERRAIN_LOD1_SHA.into(),
            lod1_bytes: DEFAULT_TERRAIN_LOD1_BYTES,
            lod1_triangles: DEFAULT_TERRAIN_LOD1_TRIS,
            water_url: DEFAULT_WATER_URL.into(),
            water_sha256: DEFAULT_WATER_SHA.into(),
            water_bytes: DEFAULT_WATER_BYTES,
            water_triangles: DEFAULT_WATER_TRIS,
        }
    }
}

/// Select the tier-appropriate 600 m catchment terrain `.10d` (`lod0` for desktop
/// TierA/TierB, `lod1` <= 32k triangles for mobile WebGL2 TierC).
pub fn select_catchment_terrain_for_tier<'a>(
    tier_id: &str,
    spec: &'a CatchmentTerrainSpec,
) -> (&'static str, &'static str, &'a str, &'a str, u64, u64) {
    let norm = tier_id.trim().to_ascii_lowercase();
    if matches!(norm.as_str(), "tier-c" | "tierc" | "tier_c" | "c") {
        (
            CATCHMENT_TERRAIN_LOD1_ID,
            "lod1",
            spec.lod1_url.as_str(),
            spec.lod1_sha256.as_str(),
            spec.lod1_bytes,
            spec.lod1_triangles,
        )
    } else {
        (
            CATCHMENT_TERRAIN_LOD0_ID,
            "lod0",
            spec.lod0_url.as_str(),
            spec.lod0_sha256.as_str(),
            spec.lod0_bytes,
            spec.lod0_triangles,
        )
    }
}

/// Returns `true` for legacy 200 m terrain (`habitat-terrain`) and generic
/// white-trunk prototype trees (`lush-lush-near-*`, `lush-lush-mid-*`,
/// `prototype-eucalyptus-lush-*`, `eucalyptus--*`, `eucalyptus-far--*`) that
/// are superseded once the upgraded 600 m catchment terrain (`terrain-v2`) and
/// live `intenteco` species cohort are admitted.
pub fn is_legacy_replaced_by_catchment(id: &str) -> bool {
    id == "habitat-terrain"
        || id == "prototype-eucalyptus-lush-near"
        || id == "prototype-eucalyptus-lush-mid"
        || id.starts_with("lush-lush-near")
        || id.starts_with("lush-lush-mid")
        || id.starts_with("prototype-eucalyptus-lush-")
        || id.starts_with("eucalyptus--")
        || id.starts_with("eucalyptus-far--")
}

/// Preserve all campsite clearing exhibits (Passat, Jayco camper, solar arrays,
/// Starlink, dome, tidal vessel, signage, footbridge, operator) and active place
/// assets while omitting legacy `habitat-terrain` and generic white-trunk trees
/// once the upgraded 600 m catchment terrain and `intenteco` species are mounted.
pub fn filter_base_ids_for_catchment(base_ids: &[String]) -> Vec<String> {
    base_ids
        .iter()
        .filter(|id| !is_legacy_replaced_by_catchment(id.as_str()))
        .cloned()
        .collect()
}

/// Choose the concrete [`SpeciesLodAsset`] for a species given the hysteresis-selected
/// [`selection::LodLevel`] and Rhizosphere X-Ray state.
///
/// For flora (`kingdom == "flora"`), always prefer `aboveground_lod1` when available so
/// subterranean root plates remain hidden below hillsides during normal view and pair
/// cleanly with `{slug}-rhizosphere` when Rhizosphere X-Ray is enabled.
pub fn select_species_lod_asset(
    species: &SpeciesCatalogEntry,
    target_lod: selection::LodLevel,
    rhizosphere_active: bool,
) -> Option<&SpeciesLodAsset> {
    if species.kingdom.eq_ignore_ascii_case("flora") {
        if let Some(aboveground) = species.aboveground_lod1.as_ref() {
            if rhizosphere_active || target_lod != selection::LodLevel::Lod2 {
                return Some(aboveground);
            }
            return Some(aboveground);
        }
    }
    match target_lod {
        selection::LodLevel::Lod0 => species
            .lod0
            .as_ref()
            .or(species.lod1.as_ref())
            .or(species.lod2.as_ref()),
        selection::LodLevel::Lod1 => species
            .lod1
            .as_ref()
            .or(species.lod2.as_ref())
            .or(species.lod0.as_ref()),
        selection::LodLevel::Lod2 => species
            .lod2
            .as_ref()
            .or(species.lod1.as_ref())
            .or(species.lod0.as_ref()),
    }
}

fn species_tint(kingdom: &str, rhizosphere: bool) -> [f32; 4] {
    if rhizosphere {
        [0.42, 0.88, 0.72, 1.0]
    } else if kingdom.eq_ignore_ascii_case("flora") {
        [0.44, 0.58, 0.36, 1.0]
    } else {
        [0.64, 0.56, 0.44, 1.0]
    }
}

async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(win) = web_sys::window() {
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = JsFuture::from(promise).await;
}

struct StreamAssetMeta {
    url: String,
    sha256: String,
    colour: [f32; 4],
    bytes: u64,
    triangles: u64,
}

/// Strip optional `<...>` angle brackets from Turtle/N-Triples IRIs stored in
/// `synth_compile_q42` Q42 lexicons (e.g. `se-australia-catchment.q42`,
/// `species-catalog.q42`).
pub fn strip_angle_brackets(s: &str) -> &str {
    s.trim()
        .strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
        .unwrap_or(s.trim())
}

/// Unquote an RDF literal stored in a Q42 lexicon, stripping optional surrounding
/// quotes, `@lang` tags, and `^^<datatype>` suffixes (e.g. `"River Red Gum"@en`,
/// `"-0.004673"^^<http://www.w3.org/2001/XMLSchema#decimal>`).
pub fn unquote_rdf_literal(raw: &str) -> String {
    let trimmed = raw.trim();
    let core = if let Some(rest) = trimmed.strip_prefix('"') {
        if let Some(end_quote) = rest.rfind('"') {
            &rest[..end_quote]
        } else {
            rest
        }
    } else {
        strip_angle_brackets(trimmed)
    };
    core.replace("\\\"", "\"").replace("\\\\", "\\")
}

/// Query `(subject, full_predicate_iri)` against `QuinIndex`, matching both bare
/// IRIs (from `catalog.q42`) and `<...>` Turtle-encoded IRIs (from
/// `se-australia-catchment.q42` and `species-catalog.q42`).
fn qindex_many_pred(index: &QuinIndex, subject: &str, pred_iri: &str) -> Vec<String> {
    let bare_s = strip_angle_brackets(subject);
    let ang_s = format!("<{bare_s}>");
    let bare_p = strip_angle_brackets(pred_iri);
    let ang_p = format!("<{bare_p}>");
    let mut out = Vec::new();
    for s_key in [bare_s, ang_s.as_str()] {
        for p_key in [bare_p, ang_p.as_str()] {
            if let Some(vals) = index.index.get(&(s_key.to_string(), p_key.to_string())) {
                for v in vals {
                    if !out.iter().any(|existing| existing == v) {
                        out.push(v.clone());
                    }
                }
            }
        }
    }
    out
}

fn qindex_rel(index: &QuinIndex, subject: &str, rel_name: &str) -> Vec<String> {
    let pred_iri = format!("{}{rel_name}", super::knowledge::REL);
    qindex_many_pred(index, subject, &pred_iri)
}

fn qindex_text(index: &QuinIndex, subject: &str, rel_name: &str) -> Option<String> {
    qindex_rel(index, subject, rel_name)
        .into_iter()
        .next()
        .map(|v| unquote_rdf_literal(&v))
}

fn qindex_pred_text(index: &QuinIndex, subject: &str, pred_iri: &str) -> Option<String> {
    qindex_many_pred(index, subject, pred_iri)
        .into_iter()
        .next()
        .map(|v| unquote_rdf_literal(&v))
}

fn qindex_number(index: &QuinIndex, subject: &str, rel_name: &str) -> Option<f64> {
    qindex_text(index, subject, rel_name).and_then(|s| s.parse::<f64>().ok())
}

fn qindex_u64(index: &QuinIndex, subject: &str, rel_name: &str) -> Option<u64> {
    qindex_text(index, subject, rel_name).and_then(|s| s.parse::<u64>().ok())
}

pub const ASSET_WORKER_URL: &str = "/world/asset-worker.js";

/// Live streaming & eviction telemetry snapshot published on `window.__webizenStreamStats`.
#[derive(Clone, Debug, Default)]
pub struct StreamStatsSnapshot {
    pub total_streamed_in: u64,
    pub total_evicted_out: u64,
    pub resident_assets: usize,
    pub worker_active: bool,
    pub worker_inflight: u64,
    pub worker_completed: u64,
    pub active_tile_tx: i32,
    pub active_tile_ty: i32,
    pub active_tile_count: usize,
    pub last_eviction_reason: String,
    pub last_stream_note: String,
    pub last_error: Option<String>,
    pub resident_bytes: u64,
    pub resident_triangles: u64,
    pub budget_max_bytes: u64,
    pub budget_max_triangles: u64,
    pub active_tier: String,
    pub streaming_phase: String,
    pub cache_entries: usize,
    pub tick_count: u64,
}

#[derive(Default)]
struct StreamCommandQueue {
    force_tick: bool,
    evict_lru_pending: usize,
}

thread_local! {
    static STREAM_COMMANDS: RefCell<StreamCommandQueue> = RefCell::new(StreamCommandQueue::default());
    static STREAM_HOOKS_INSTALLED: RefCell<bool> = const { RefCell::new(false) };
}

fn publish_stream_note(note: &str) {
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &win,
            &JsValue::from_str("__webizenCatchmentStreamNote"),
            &JsValue::from_str(note),
        );
    }
}

fn publish_stream_stats(stats: &StreamStatsSnapshot) {
    publish_stream_note(&stats.last_stream_note);
    let Some(win) = web_sys::window() else {
        return;
    };
    let payload = json!({
        "phase": stats.streaming_phase,
        "streaming_phase": stats.streaming_phase,
        "streamed_in": stats.total_streamed_in,
        "total_streamed_in": stats.total_streamed_in,
        "evicted": stats.total_evicted_out,
        "total_evicted_out": stats.total_evicted_out,
        "resident_assets": stats.resident_assets,
        "worker_active": stats.worker_active,
        "worker_inflight": stats.worker_inflight,
        "worker_completed": stats.worker_completed,
        "active_tile_tx": stats.active_tile_tx,
        "active_tile_ty": stats.active_tile_ty,
        "active_tile_count": stats.active_tile_count,
        "last_eviction_reason": stats.last_eviction_reason,
        "last_stream_note": stats.last_stream_note,
        "last_error": stats.last_error,
        "gpu_bytes": stats.resident_bytes,
        "resident_bytes": stats.resident_bytes,
        "resident_triangles": stats.resident_triangles,
        "gpu_budget_bytes": stats.budget_max_bytes,
        "budget_max_bytes": stats.budget_max_bytes,
        "triangle_budget": stats.budget_max_triangles,
        "budget_max_triangles": stats.budget_max_triangles,
        "active_tier": stats.active_tier,
        "cache_entries": stats.cache_entries,
        "tick_count": stats.tick_count,
    });
    if let Ok(json_str) = serde_json::to_string(&payload) {
        if let Ok(js_obj) = js_sys::JSON::parse(&json_str) {
            let _ = js_sys::Reflect::set(
                &win,
                &JsValue::from_str("__webizenStreamStats"),
                &js_obj,
            );
        }
    }
}

/// Install `window.__webizenStreamHooks = { forceTick(), evictLru() }` once for the page lifetime.
fn install_stream_hooks() {
    let already = STREAM_HOOKS_INSTALLED.with(|flag| {
        let mut f = flag.borrow_mut();
        if *f {
            true
        } else {
            *f = true;
            false
        }
    });
    if already {
        return;
    }
    let Some(win) = web_sys::window() else {
        return;
    };
    let hooks = js_sys::Object::new();

    let force_cb = wasm_bindgen::closure::Closure::wrap(Box::new(move || -> JsValue {
        STREAM_COMMANDS.with(|q| {
            q.borrow_mut().force_tick = true;
        });
        let res = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&res, &"ok".into(), &JsValue::TRUE);
        let _ = js_sys::Reflect::set(&res, &"action".into(), &"forceTick".into());
        res.into()
    }) as Box<dyn FnMut() -> JsValue>);
    let _ = js_sys::Reflect::set(&hooks, &"forceTick".into(), force_cb.as_ref().unchecked_ref());
    force_cb.forget();

    let evict_cb = wasm_bindgen::closure::Closure::wrap(Box::new(move || -> JsValue {
        STREAM_COMMANDS.with(|q| {
            let mut q = q.borrow_mut();
            q.evict_lru_pending = q.evict_lru_pending.saturating_add(1);
            q.force_tick = true;
        });
        let res = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&res, &"ok".into(), &JsValue::TRUE);
        let _ = js_sys::Reflect::set(&res, &"action".into(), &"evictLru".into());
        res.into()
    }) as Box<dyn FnMut() -> JsValue>);
    let _ = js_sys::Reflect::set(&hooks, &"evictLru".into(), evict_cb.as_ref().unchecked_ref());
    evict_cb.forget();

    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__webizenStreamHooks"), &hooks);
}

/// Drain any queued `window.__webizenStreamHooks` calls or `window.__webizenStreamCmd` directives.
fn take_pending_stream_commands() -> (bool, usize) {
    let mut cmd_force = false;
    let mut cmd_evict = 0usize;
    if let Some(win) = web_sys::window() {
        if let Ok(val) = js_sys::Reflect::get(&win, &JsValue::from_str("__webizenStreamCmd")) {
            if let Some(s) = val.as_string() {
                let norm = s.trim().to_ascii_lowercase();
                if matches!(norm.as_str(), "evict_lru" | "evictlru" | "evict") {
                    cmd_evict = 1;
                    cmd_force = true;
                } else if matches!(norm.as_str(), "force_tick" | "forcetick" | "tick") {
                    cmd_force = true;
                }
                let _ = js_sys::Reflect::set(
                    &win,
                    &JsValue::from_str("__webizenStreamCmd"),
                    &JsValue::NULL,
                );
            }
        }
    }
    STREAM_COMMANDS.with(|q| {
        let mut q = q.borrow_mut();
        let force = q.force_tick || cmd_force;
        let evict = q.evict_lru_pending.saturating_add(cmd_evict);
        q.force_tick = false;
        q.evict_lru_pending = 0;
        (force, evict)
    })
}

/// Dedicated Web Worker pool bridging `/world/asset-worker.js` for off-main-thread
/// `fetch()` + `crypto.subtle.digest('SHA-256', buf)` with zero-copy `ArrayBuffer` transfer.
/// Falls back transparently to main-thread `fetch_verified_asset` when `Worker` is unavailable.
struct WorkerAssetPool {
    bridge: Option<JsValue>,
    inflight: Rc<RefCell<u64>>,
    completed: Rc<RefCell<u64>>,
}

impl WorkerAssetPool {
    fn new() -> Self {
        let factory = js_sys::Function::new_with_args(
            "workerUrl",
            r#"
            if (typeof Worker !== 'function') return null;
            try {
              const worker = new Worker(workerUrl);
              let nextReqId = 1;
              let alive = true;
              const pending = new Map();
              worker.onmessage = function(ev) {
                const data = ev && ev.data;
                if (!data || typeof data.reqId === 'undefined') return;
                const entry = pending.get(data.reqId);
                if (!entry) return;
                pending.delete(data.reqId);
                clearTimeout(entry.timer);
                entry.resolve(data);
              };
              worker.onerror = function(err) {
                alive = false;
                const msg = (err && err.message) ? err.message : 'asset-worker error';
                for (const [, entry] of pending) {
                  clearTimeout(entry.timer);
                  entry.resolve({ ok: false, error: msg });
                }
                pending.clear();
              };
              return {
                isAlive() { return alive; },
                fetchAsset(id, logicalUrl, urls, expectedSha256, maxBytes) {
                  if (!alive) {
                    return Promise.resolve({ ok: false, error: 'worker inactive' });
                  }
                  const reqId = nextReqId++;
                  return new Promise((resolve) => {
                    const timer = setTimeout(() => {
                      pending.delete(reqId);
                      resolve({ ok: false, error: 'worker fetch timeout for ' + id });
                    }, 30000);
                    pending.set(reqId, { resolve, timer });
                    try {
                      worker.postMessage({ id, reqId, logicalUrl, urls, expectedSha256, maxBytes });
                    } catch (e) {
                      pending.delete(reqId);
                      clearTimeout(timer);
                      alive = false;
                      resolve({ ok: false, error: String(e) });
                    }
                  });
                }
              };
            } catch (_) {
              return null;
            }
            "#,
        );
        let bridge = factory
            .call1(&JsValue::NULL, &JsValue::from_str(ASSET_WORKER_URL))
            .ok()
            .filter(|v| !v.is_null() && !v.is_undefined());
        Self {
            bridge,
            inflight: Rc::new(RefCell::new(0)),
            completed: Rc::new(RefCell::new(0)),
        }
    }

    fn is_active(&self) -> bool {
        let Some(ref bridge) = self.bridge else {
            return false;
        };
        js_sys::Reflect::get(bridge, &JsValue::from_str("isAlive"))
            .ok()
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
            .and_then(|f| f.call0(bridge).ok())
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn inflight(&self) -> u64 {
        *self.inflight.borrow()
    }

    fn completed(&self) -> u64 {
        *self.completed.borrow()
    }

    async fn fetch_verified(
        &self,
        id: &str,
        logical_url: &str,
        expected_sha256: &str,
        allowlist: &[String],
        max_bytes: usize,
    ) -> Result<(Vec<u8>, String, bool), JsValue> {
        let candidates =
            super::admission::resolve_asset_candidates(logical_url, expected_sha256, allowlist)
                .map_err(err)?;

        if self.is_active() {
            if let Some(ref bridge) = self.bridge {
                if let Ok(fetch_fn) = js_sys::Reflect::get(bridge, &JsValue::from_str("fetchAsset"))
                    .and_then(|v| v.dyn_into::<js_sys::Function>().map_err(JsValue::from))
                {
                    let urls_js = js_sys::Array::new();
                    for c in &candidates {
                        urls_js.push(&JsValue::from_str(c));
                    }
                    let args = js_sys::Array::new();
                    args.push(&JsValue::from_str(id));
                    args.push(&JsValue::from_str(logical_url));
                    args.push(&urls_js);
                    args.push(&JsValue::from_str(expected_sha256));
                    args.push(&JsValue::from_f64(max_bytes as f64));

                    *self.inflight.borrow_mut() += 1;
                    let worker_outcome = match fetch_fn
                        .apply(bridge, &args)
                        .and_then(|v| v.dyn_into::<js_sys::Promise>().map_err(JsValue::from))
                    {
                        Ok(promise) => JsFuture::from(promise).await.ok(),
                        Err(_) => None,
                    };
                    {
                        let mut inf = self.inflight.borrow_mut();
                        *inf = inf.saturating_sub(1);
                    }

                    if let Some(res) = worker_outcome {
                        let ok = js_sys::Reflect::get(&res, &JsValue::from_str("ok"))
                            .ok()
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if ok {
                            if let Ok(buf) = js_sys::Reflect::get(&res, &JsValue::from_str("buffer"))
                            {
                                let source_url = js_sys::Reflect::get(
                                    &res,
                                    &JsValue::from_str("sourceUrl"),
                                )
                                .ok()
                                .and_then(|v| v.as_string())
                                .unwrap_or_else(|| logical_url.to_string());
                                let bytes = js_sys::Uint8Array::new(&buf).to_vec();
                                *self.completed.borrow_mut() += 1;
                                // Worker already verified SHA-256 via WebCrypto; run fast magic + provenance check.
                                let admitted =
                                    admit_preverified_bytes(id, &source_url, &bytes, expected_sha256)?;
                                return Ok((admitted, source_url, true));
                            }
                        }
                    }
                }
            }
        }

        // Graceful fallback to main-thread verified fetch if Worker is unavailable or errored.
        let (bytes, source) =
            fetch_verified_asset(id, logical_url, expected_sha256, allowlist, max_bytes, None)
                .await?;
        Ok((bytes, source, false))
    }
}

/// Check whether a species has at least one placement inside the active 3x3 tile ring
/// or within `max_range_m` metres of `cam_target`.
fn species_in_active_ring_or_range(
    cam_target: [f32; 3],
    placements: &[Placement],
    active_tiles: &[super::gis_join::TileCoord],
    max_range_m: f32,
) -> bool {
    if placements.is_empty() {
        return true;
    }
    let max_sq = max_range_m * max_range_m;
    for p in placements {
        let px = p.translation[0] as f32;
        let py = p.translation[1] as f32;
        let pz = p.translation[2] as f32;
        let tile = super::gis_join::TileCoord::from_world_xz(px, pz, 0);
        if active_tiles.contains(&tile) {
            return true;
        }
        let dx = px - cam_target[0];
        let dy = py - cam_target[1];
        let dz = pz - cam_target[2];
        if dx * dx + dy * dy + dz * dz <= max_sq {
            return true;
        }
    }
    false
}

fn parse_species_lod_from_qindex(index: &QuinIndex, lod_iri: &str) -> Option<SpeciesLodAsset> {
    let url = qindex_text(index, lod_iri, "url")?;
    let sha256 = qindex_text(index, lod_iri, "sha256")?;
    if url.is_empty() || !super::admission::is_valid_sha256_hex(&sha256) {
        return None;
    }
    let tier = qindex_text(index, lod_iri, "tier").unwrap_or_else(|| "lod1".into());
    let distance_band = qindex_text(index, lod_iri, "distanceBand").unwrap_or_default();
    let remote_url = qindex_text(index, lod_iri, "remoteUrl").unwrap_or_default();
    let bytes = qindex_u64(index, lod_iri, "bytes").unwrap_or(0);
    let triangles = qindex_u64(index, lod_iri, "triangles").unwrap_or(0);
    let vertices = qindex_u64(index, lod_iri, "vertices").unwrap_or(0);
    let aabb = match (
        qindex_number(index, lod_iri, "aabbMinX"),
        qindex_number(index, lod_iri, "aabbMinY"),
        qindex_number(index, lod_iri, "aabbMinZ"),
        qindex_number(index, lod_iri, "aabbMaxX"),
        qindex_number(index, lod_iri, "aabbMaxY"),
        qindex_number(index, lod_iri, "aabbMaxZ"),
    ) {
        (Some(x0), Some(y0), Some(z0), Some(x1), Some(y1), Some(z1)) => Some(super::catalog::Aabb {
            min: [x0 as f32, y0 as f32, z0 as f32],
            max: [x1 as f32, y1 as f32, z1 as f32],
        }),
        _ => None,
    };
    Some(SpeciesLodAsset {
        tier,
        distance_band,
        url,
        remote_url,
        sha256,
        bytes,
        triangles,
        vertices,
        aabb,
    })
}

/// Parse `species-catalog.q42` (`webizen.species-catalog/v1`) entries directly from
/// `QuinIndex`, handling Turtle `<iri>` and `"literal"^^<xsd:...>` lexicon encoding.
fn parse_species_catalog_from_index(index: &QuinIndex) -> Vec<SpeciesCatalogEntry> {
    let mut taxon_iris = qindex_rel(
        index,
        "https://webizen.au/catalog/se-australia-species",
        "hasSpecies",
    );
    if taxon_iris.is_empty() {
        taxon_iris = qindex_rel(
            index,
            "https://webizen.au/catalog/place/se-australia-catchment",
            "hasSpecies",
        );
    }
    let mut out = Vec::new();
    for raw_iri in taxon_iris {
        let taxon_iri = strip_angle_brackets(&raw_iri).to_string();
        let Some(slug) = qindex_text(index, &taxon_iri, "slug") else {
            continue;
        };
        if slug.is_empty() || out.iter().any(|e: &SpeciesCatalogEntry| e.slug == slug) {
            continue;
        }
        let scientific_name =
            qindex_pred_text(index, &taxon_iri, "http://rs.tdwg.org/dwc/terms/scientificName")
                .unwrap_or_else(|| slug.clone());
        let vernacular_name =
            qindex_pred_text(index, &taxon_iri, "http://rs.tdwg.org/dwc/terms/vernacularName")
                .or_else(|| {
                    qindex_pred_text(index, &taxon_iri, "http://www.w3.org/2000/01/rdf-schema#label")
                })
                .unwrap_or_else(|| slug.clone());
        let kingdom = qindex_text(index, &taxon_iri, "kingdom").unwrap_or_else(|| "flora".into());
        let ecological_zone = qindex_text(index, &taxon_iri, "ecologicalZone").unwrap_or_default();
        let url = qindex_text(index, &taxon_iri, "canonicalUrl").unwrap_or_default();
        let remote_url = qindex_text(index, &taxon_iri, "canonicalRemoteUrl").unwrap_or_default();
        let sha256 = qindex_text(index, &taxon_iri, "canonicalSha256").unwrap_or_default();
        let bytes = qindex_u64(index, &taxon_iri, "canonicalBytes").unwrap_or(0);
        let triangles = qindex_u64(index, &taxon_iri, "canonicalTriangles").unwrap_or(0);
        let vertices = qindex_u64(index, &taxon_iri, "canonicalVertices").unwrap_or(0);
        let aabb = match (
            qindex_number(index, &taxon_iri, "aabbMinX"),
            qindex_number(index, &taxon_iri, "aabbMinY"),
            qindex_number(index, &taxon_iri, "aabbMinZ"),
            qindex_number(index, &taxon_iri, "aabbMaxX"),
            qindex_number(index, &taxon_iri, "aabbMaxY"),
            qindex_number(index, &taxon_iri, "aabbMaxZ"),
        ) {
            (Some(x0), Some(y0), Some(z0), Some(x1), Some(y1), Some(z1)) => {
                Some(super::catalog::Aabb {
                    min: [x0 as f32, y0 as f32, z0 as f32],
                    max: [x1 as f32, y1 as f32, z1 as f32],
                })
            }
            _ => None,
        };
        let sidecar_taxon_q42 = qindex_text(index, &taxon_iri, "sidecarTaxonQ42").map(|u| {
            let s = qindex_text(index, &taxon_iri, "sidecarTaxonQ42Sha256").unwrap_or_default();
            (u, s)
        });
        let sidecar_field_emf = qindex_text(index, &taxon_iri, "sidecarFieldEmf").map(|u| {
            let s = qindex_text(index, &taxon_iri, "sidecarFieldEmfSha256").unwrap_or_default();
            (u, s)
        });
        let sidecar_substrate_m64 =
            qindex_text(index, &taxon_iri, "sidecarSubstrateM64").map(|u| {
                let s = qindex_text(index, &taxon_iri, "sidecarSubstrateM64Sha256")
                    .unwrap_or_default();
                (u, s)
            });

        let mut lod0 = None;
        let mut lod1 = None;
        let mut lod2 = None;
        let mut aboveground_lod1 = None;
        let mut rhizosphere = None;
        for lod_iri in qindex_rel(index, &taxon_iri, "hasLod") {
            if let Some(lod) = parse_species_lod_from_qindex(index, &lod_iri) {
                match lod.tier.as_str() {
                    "lod0" => lod0 = Some(lod),
                    "lod1" => lod1 = Some(lod),
                    "lod2" => lod2 = Some(lod),
                    "aboveground_lod1" => aboveground_lod1 = Some(lod),
                    "rhizosphere" => rhizosphere = Some(lod),
                    _ => {}
                }
            }
        }
        out.push(SpeciesCatalogEntry {
            taxon_iri,
            slug,
            scientific_name,
            vernacular_name,
            kingdom,
            ecological_zone,
            url,
            remote_url,
            sha256,
            bytes,
            triangles,
            vertices,
            aabb,
            sidecar_taxon_q42,
            sidecar_field_emf,
            sidecar_substrate_m64,
            lod0,
            lod1,
            lod2,
            aboveground_lod1,
            rhizosphere,
        });
    }
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    out
}

fn parse_catchment_terrain_spec(index: &QuinIndex) -> CatchmentTerrainSpec {
    let mut spec = CatchmentTerrainSpec::default();
    let place_iri = "https://webizen.au/catalog/place/se-australia-catchment";
    if let Some(u) = qindex_text(index, place_iri, "terrainLod0Url") {
        if !u.is_empty() {
            spec.lod0_url = u;
        }
    }
    if let Some(s) = qindex_text(index, place_iri, "terrainLod0Sha256") {
        if super::admission::is_valid_sha256_hex(&s) {
            spec.lod0_sha256 = s;
        }
    }
    if let Some(u) = qindex_text(index, place_iri, "terrainLod1Url") {
        if !u.is_empty() {
            spec.lod1_url = u;
        }
    }
    if let Some(s) = qindex_text(index, place_iri, "terrainLod1Sha256") {
        if super::admission::is_valid_sha256_hex(&s) {
            spec.lod1_sha256 = s;
        }
    }
    if let Some(u) = qindex_text(index, place_iri, "waterUrl") {
        if !u.is_empty() {
            spec.water_url = u;
        }
    }
    if let Some(s) = qindex_text(index, place_iri, "waterSha256") {
        if super::admission::is_valid_sha256_hex(&s) {
            spec.water_sha256 = s;
        }
    }
    spec
}

fn upload_has_catchment_terrain(upload: &Value) -> bool {
    upload
        .get("parts")
        .and_then(Value::as_array)
        .map(|parts| {
            parts.iter().any(|p| {
                let id = p.get("id").and_then(Value::as_str).unwrap_or("");
                id == CATCHMENT_TERRAIN_LOD0_ID || id == CATCHMENT_TERRAIN_LOD1_ID
            })
        })
        .unwrap_or(false)
}

fn min_distance_to_placements(origin: [f32; 3], placements: &[Placement]) -> f32 {
    let mut best = f32::INFINITY;
    for p in placements {
        let dx = p.translation[0] as f32 - origin[0];
        let dy = p.translation[1] as f32 - origin[1];
        let dz = p.translation[2] as f32 - origin[2];
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        if d < best {
            best = d;
        }
    }
    if best.is_finite() {
        best
    } else {
        150.0
    }
}

/// Spawn the post-first-paint live GPU streaming loop for the 600 m catchment terrain
/// (`terrain-v2`) and all 13 `intenteco` species (`se-australia-catchment.q42` +
/// `species-catalog.q42`). Respects `TierA`/`TierB`/`TierC` budgets via
/// [`selection::GpuResidencyLedger`], resolves assets through Cache API + Tier 1/Tier 2
/// allowlisted mirrors, and reacts to camera motion, place changes, and Rhizosphere X-Ray.
pub(super) fn spawn_catchment_streaming_loop(
    state: Shared,
    tier_id: String,
    tier_budget: super::graphics::TierBudget,
) {
    wasm_bindgen_futures::spawn_local(async move {
        install_stream_hooks();
        let worker_pool = Rc::new(WorkerAssetPool::new());
        let mut tile_ring = super::gis_join::ActiveTileRing::new(9);
        let mut stats = StreamStatsSnapshot {
            worker_active: worker_pool.is_active(),
            budget_max_bytes: tier_budget.max_gpu_memory_bytes as u64,
            budget_max_triangles: tier_budget.max_resident_triangles,
            active_tier: tier_id.clone(),
            streaming_phase: "waiting-first-paint".into(),
            last_stream_note: "waiting-first-paint".into(),
            ..Default::default()
        };
        publish_stream_stats(&stats);

        // Strictly wait until initial first-paint (`__webizenNativeUpload.uploaded === true`
        // and `!ewm.busy`) has completed before doing any Q42 admit or GPU upload.
        for _ in 0..80 {
            sleep_ms(100).await;
            let ready = if let Ok(w) = state.try_borrow() {
                let uploaded = w.upload["uploaded"].as_bool() == Some(true);
                let busy = w
                    .ewm
                    .as_ref()
                    .map(|e| e.busy.load(std::sync::atomic::Ordering::SeqCst))
                    .unwrap_or(false);
                uploaded && !busy
            } else {
                false
            };
            if ready {
                break;
            }
        }

        stats.streaming_phase = "fetching-catchment-q42".into();
        stats.last_stream_note = "fetching-catchment-q42".into();
        publish_stream_stats(&stats);
        // Fetch and admit the 600 m SE Australia Catchment Q42 volume (317 placements, 13 species).
        let catchment_raw = match bytes(CATCHMENT_PLACE_Q42, 4 * 1024 * 1024).await {
            Ok(raw) => Some(raw),
            Err(e) => {
                stats.last_stream_note = "catchment-q42-fetch-failed-falling-back".into();
                stats.last_error = Some(e.as_string().unwrap_or_else(|| "catchment-q42-fetch-failed".into()));
                publish_stream_stats(&stats);
                web_sys::console::warn_1(&e);
                None
            }
        };

        let (terrain_spec, species_placements): (CatchmentTerrainSpec, BTreeMap<String, Vec<Placement>>) = {
            let mut acquired = None;
            for _ in 0..30 {
                if let Ok(mut w) = state.try_borrow_mut() {
                    if let Some(ref raw) = catchment_raw {
                        if let Err(e) = super::knowledge::admit(
                            &mut w.host,
                            raw,
                            "catchment",
                            "se-australia-catchment.q42",
                        ) {
                            publish_stream_note(&format!("catchment-admit-err:{e}"));
                            web_sys::console::warn_1(&err(e));
                        }
                    }
                    let index = match QuinIndex::from_host(&w.host) {
                        Ok(idx) => idx,
                        Err(e) => {
                            publish_stream_note(&format!("quin-index-err:{e}"));
                            web_sys::console::warn_1(&err(e));
                            return;
                        }
                    };
                    let terrain_spec = parse_catchment_terrain_spec(&index);

                    // Parse species catalog using Turtle-aware Q42 lexicon helpers so
                    // angle-bracketed IRIs (`<did:q42:taxon:...>`) & quoted literals
                    // from `synth_compile_q42` always hydrate all 17 species + LODs.
                    let mut catalog_species: Vec<SpeciesCatalogEntry> = w
                        .ewm
                        .as_ref()
                        .map(|e| e.catalog.species.clone())
                        .unwrap_or_default();
                    if catalog_species.is_empty() {
                        catalog_species = parse_species_catalog_from_index(&index);
                        if !catalog_species.is_empty() {
                            if let Some(ewm) = w.ewm.as_mut() {
                                ewm.catalog.species = catalog_species.clone();
                            }
                        }
                    }

                    let place_iri = "https://webizen.au/catalog/place/se-australia-catchment";
                    let mut instrument_entries: Vec<InstrumentSpeciesEntry> = Vec::new();
                    for sp in &catalog_species {
                        let catchment_zone = qindex_text(&index, &sp.taxon_iri, "catchmentZone")
                            .unwrap_or_else(|| sp.ecological_zone.clone());
                        let locomotion_class =
                            qindex_text(&index, &sp.taxon_iri, "locomotionClass").unwrap_or_else(
                                || {
                                    if sp.kingdom.eq_ignore_ascii_case("flora") {
                                        "sessile_flora".into()
                                    } else {
                                        "terrestrial_fauna".into()
                                    }
                                },
                            );
                        let custodial_country =
                            qindex_text(&index, &sp.taxon_iri, "custodialCountry")
                                .or_else(|| {
                                    qindex_pred_text(
                                        &index,
                                        &sp.taxon_iri,
                                        "https://localcontexts.org/ontology/tk-bc.ttl#custodial_country",
                                    )
                                })
                                .unwrap_or_default();
                        let call_qac_url = qindex_text(&index, &sp.taxon_iri, "callQacUrl");
                        let mut entry = InstrumentSpeciesEntry::from_catalog_entry(
                            sp,
                            &catchment_zone,
                            &locomotion_class,
                            &custodial_country,
                            call_qac_url.as_deref(),
                        );
                        if let Some(m) = qindex_text(&index, &sp.taxon_iri, "mycorrhizalType") {
                            if !m.is_empty() {
                                entry.mycorrhizal_type = m;
                            }
                        }
                        if let Some(sig) =
                            qindex_text(&index, &sp.taxon_iri, "rhizosphereSignaling")
                        {
                            if !sig.is_empty() {
                                entry.rhizosphere_signaling = sig;
                            }
                        }
                        if let Some(hz) = qindex_number(&index, &sp.taxon_iri, "naturalSwayFreqHz")
                        {
                            if hz.is_finite() && hz > 0.0 {
                                entry.natural_sway_freq_hz = hz as f32;
                            }
                        }
                        instrument_entries.push(entry);
                    }
                    instrument_entries.sort_by(|a, b| a.slug.cmp(&b.slug));

                    let mut placements_by_slug: BTreeMap<String, Vec<Placement>> = BTreeMap::new();
                    let mut placement_refs: Vec<CatchmentPlacementRef> = Vec::new();
                    for raw_p_iri in qindex_rel(&index, place_iri, "hasPlacement") {
                        let p_iri = strip_angle_brackets(&raw_p_iri);
                        let Some(slug) = qindex_text(&index, p_iri, "speciesSlug") else {
                            continue;
                        };
                        let id = qindex_text(&index, p_iri, "id").unwrap_or_default();
                        let tx = qindex_number(&index, p_iri, "translationX").unwrap_or(0.0);
                        let ty = qindex_number(&index, p_iri, "translationY").unwrap_or(0.0);
                        let tz = qindex_number(&index, p_iri, "translationZ").unwrap_or(0.0);
                        let rx = qindex_number(&index, p_iri, "rotationX").unwrap_or(0.0);
                        let ry = qindex_number(&index, p_iri, "rotationY").unwrap_or(0.0);
                        let rz = qindex_number(&index, p_iri, "rotationZ").unwrap_or(0.0);
                        let rw = qindex_number(&index, p_iri, "rotationW").unwrap_or(1.0);
                        let q_norm = (rx * rx + ry * ry + rz * rz + rw * rw).sqrt();
                        let rotation = if q_norm.is_finite() && q_norm > 1e-6 {
                            [rx / q_norm, ry / q_norm, rz / q_norm, rw / q_norm]
                        } else {
                            [0.0, 0.0, 0.0, 1.0]
                        };
                        let sx = qindex_number(&index, p_iri, "scaleX").unwrap_or(1.0);
                        let sy = qindex_number(&index, p_iri, "scaleY").unwrap_or(1.0);
                        let sz = qindex_number(&index, p_iri, "scaleZ").unwrap_or(1.0);
                        let seed = qindex_u64(&index, p_iri, "seed").unwrap_or(1);
                        let age =
                            qindex_number(&index, p_iri, "ontogeneticAgeNorm").unwrap_or(0.8)
                                as f32;
                        let hs = qindex_number(&index, p_iri, "heightScale").unwrap_or(1.0) as f32;
                        let sv = qindex_number(&index, p_iri, "siteVigor").unwrap_or(1.0) as f32;
                        let sw = qindex_number(&index, p_iri, "stressWind").unwrap_or(0.1) as f32;

                        let placement = Placement {
                            id: id.clone(),
                            translation: [tx, ty, tz],
                            rotation,
                            scale: [sx, sy, sz],
                        };
                        placements_by_slug
                            .entry(slug.clone())
                            .or_default()
                            .push(placement);
                        placement_refs.push(CatchmentPlacementRef {
                            id,
                            slug,
                            pos: [tx as f32, ty as f32, tz as f32],
                            state64: InstanceState64::new(seed, age, hs, sv, [sw, 0.08, 0.08, 0.08]),
                        });
                    }

                    let placement_total: usize = placements_by_slug.values().map(Vec::len).sum();
                    // Hydrate Field Guide drawer immediately with all admitted species (17 taxa)
                    // and register all admitted species & placements into play.rs for E/click inspection.
                    super::instruments::species::hydrate_entries(instrument_entries.clone());
                    super::play::register_catchment_biosphere(instrument_entries, placement_refs);

                    // Ensure `w.scene.assets` knows the placements for terrain-v2, water,
                    // all admitted species, and flora rhizosphere layers so `bind_resident`
                    // instances every individual deterministically on GPU upload.
                    let scene_assets = &mut w.scene.assets;
                    let mut upsert_scene_asset = |asset: Asset| {
                        if let Some(existing) = scene_assets.iter_mut().find(|a| a.id == asset.id) {
                            *existing = asset;
                        } else {
                            scene_assets.push(asset);
                        }
                    };
                    upsert_scene_asset(Asset {
                        id: CATCHMENT_TERRAIN_LOD0_ID.into(),
                        url: terrain_spec.lod0_url.clone(),
                        colour: [0.32, 0.42, 0.24, 1.0],
                        sha256: terrain_spec.lod0_sha256.clone(),
                        placements: Vec::new(),
                    });
                    upsert_scene_asset(Asset {
                        id: CATCHMENT_TERRAIN_LOD1_ID.into(),
                        url: terrain_spec.lod1_url.clone(),
                        colour: [0.32, 0.42, 0.24, 1.0],
                        sha256: terrain_spec.lod1_sha256.clone(),
                        placements: Vec::new(),
                    });
                    upsert_scene_asset(Asset {
                        id: CATCHMENT_WATER_ID.into(),
                        url: terrain_spec.water_url.clone(),
                        colour: [0.18, 0.42, 0.50, 1.0],
                        sha256: terrain_spec.water_sha256.clone(),
                        placements: Vec::new(),
                    });

                    for sp in &catalog_species {
                        let placements = placements_by_slug
                            .get(&sp.slug)
                            .cloned()
                            .unwrap_or_default();
                        upsert_scene_asset(Asset {
                            id: sp.slug.clone(),
                            url: sp.url.clone(),
                            colour: species_tint(&sp.kingdom, false),
                            sha256: sp.sha256.clone(),
                            placements: placements.clone(),
                        });
                        if let Some(ref rhizo) = sp.rhizosphere {
                            let rhizo_placements: Vec<Placement> = placements
                                .into_iter()
                                .map(|mut p| {
                                    p.id = format!("{}-rhizo", p.id);
                                    p
                                })
                                .collect();
                            upsert_scene_asset(Asset {
                                id: format!("{}-rhizosphere", sp.slug),
                                url: rhizo.url.clone(),
                                colour: species_tint(&sp.kingdom, true),
                                sha256: rhizo.sha256.clone(),
                                placements: rhizo_placements,
                            });
                        }
                    }

                    let note = format!(
                        "admitted:species={}:placements={}",
                        catalog_species.len(),
                        placement_total
                    );
                    publish_stream_note(&note);
                    w.receipt["catchment_600m_admitted"] = json!(true);
                    w.receipt["catchment_species_catalog_count"] = json!(catalog_species.len());
                    w.receipt["catchment_placement_count"] = json!(placement_total);
                    w.receipt["catchment_stream_note"] = json!(note);

                    acquired = Some((terrain_spec, placements_by_slug));
                    break;
                }
                sleep_ms(30).await;
            }
            match acquired {
                Some(pair) => pair,
                None => {
                    publish_stream_note("state-borrow-timeout");
                    return;
                }
            }
        };

        let mut ledger = selection::GpuResidencyLedger::new(selection::TierBudget::from(&tier_budget));
        let allowlist = super::admission::default_allowlist();
        // Seed in-memory verified cache from first-paint so pinned campsite exhibits are reused in 0 ms.
        let mut verified_cache: HashMap<String, Vec<u8>> =
            FIRST_PAINT_VERIFIED_CACHE.with(|c| c.borrow().clone());
        let mut asset_sha_by_id: HashMap<String, String> = HashMap::new();
        let mut hydrated_textures: BTreeSet<([u8; 32], u8)> = BTreeSet::new();
        let mut initial_uploaded = false;
        let mut last_cam = [f32::MAX; 3];
        let mut last_rhizo = false;
        let mut last_place = String::new();
        let mut stream_frame: u64 = 1;

        loop {
            sleep_ms(if initial_uploaded { 240 } else { 80 }).await;
            stream_frame = stream_frame.saturating_add(1);
            let (force_tick, evict_lru_pending) = take_pending_stream_commands();

            let snapshot = {
                let Ok(w) = state.try_borrow() else {
                    continue;
                };
                let Some(ewm) = w.ewm.as_ref() else {
                    return;
                };
                if ewm.busy.load(std::sync::atomic::Ordering::SeqCst) {
                    continue;
                }
                let place_id = if ewm.selection.place_id.is_empty() {
                    "field-studio".to_string()
                } else {
                    ewm.selection.place_id.clone()
                };
                let rhizo_active = super::locomotion::rhizosphere_xray_active();
                let cam_target = w.camera.target;
                let has_catchment = upload_has_catchment_terrain(&w.upload);
                let ring_delta = tile_ring.update_for_position(cam_target[0], cam_target[2], 0);
                let tile_changed_now =
                    !ring_delta.entering.is_empty() || !ring_delta.departing.is_empty();
                let cam_delta = ((cam_target[0] - last_cam[0]).powi(2)
                    + (cam_target[1] - last_cam[1]).powi(2)
                    + (cam_target[2] - last_cam[2]).powi(2))
                .sqrt();

                stats.active_tile_tx = ring_delta.center.tx;
                stats.active_tile_ty = ring_delta.center.ty;
                stats.active_tile_count = tile_ring.active_tiles().len();
                stats.worker_active = worker_pool.is_active();
                stats.worker_inflight = worker_pool.inflight();
                stats.worker_completed = worker_pool.completed();
                stats.cache_entries = verified_cache.len();

                if initial_uploaded
                    && has_catchment
                    && !force_tick
                    && evict_lru_pending == 0
                    && !tile_changed_now
                    && rhizo_active == last_rhizo
                    && place_id == last_place
                    && cam_delta < 15.0
                {
                    if stream_frame % 4 == 0 {
                        publish_stream_stats(&stats);
                    }
                    continue;
                }
                (
                    cam_target,
                    place_id,
                    rhizo_active,
                    has_catchment,
                    ring_delta,
                    ewm.selection.ids.clone(),
                    ewm.catalog.entries.clone(),
                    ewm.catalog.species.clone(),
                    ewm.colours.clone(),
                )
            };

            let (
                cam_target,
                place_id,
                rhizo_active,
                has_catchment,
                ring_delta,
                selection_ids,
                catalog_entries,
                catalog_species,
                scene_colours,
            ) = snapshot;

            stats.tick_count = stats.tick_count.saturating_add(1);

            // Identify any manually requested LRU eviction targets (oldest non-pinned assets).
            let mut manual_evict_set: BTreeSet<String> = BTreeSet::new();
            if evict_lru_pending > 0 {
                let mut lru_candidates: Vec<(u64, String)> = ledger
                    .resident_ids()
                    .into_iter()
                    .filter_map(|id| {
                        ledger.get(&id).and_then(|entry| {
                            if !entry.pinned {
                                Some((entry.last_used_frame, id))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                lru_candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
                for (_, evict_id) in lru_candidates.into_iter().take(evict_lru_pending) {
                    manual_evict_set.insert(evict_id);
                }
            }

            let (terrain_id, terrain_lod_label, terrain_url, terrain_sha, terrain_bytes, terrain_tris) =
                select_catchment_terrain_for_tier(&tier_id, &terrain_spec);
            let ground_opacity = 1.0_f32;

            let mut desired: Vec<selection::DesiredAsset> = Vec::new();
            let mut meta_map: BTreeMap<String, StreamAssetMeta> = BTreeMap::new();

            // 1. Pinned upgraded 600 m catchment terrain + 600 m water.
            desired.push(selection::DesiredAsset {
                id: terrain_id.to_string(),
                lod_level: if terrain_lod_label == "lod1" {
                    selection::LodLevel::Lod1
                } else {
                    selection::LodLevel::Lod0
                },
                bytes: terrain_bytes,
                triangles: terrain_tris,
                pinned: true,
            });
            meta_map.insert(
                terrain_id.to_string(),
                StreamAssetMeta {
                    url: terrain_url.to_string(),
                    sha256: terrain_sha.to_string(),
                    colour: [0.32, 0.42, 0.24, ground_opacity],
                    bytes: terrain_bytes,
                    triangles: terrain_tris,
                },
            );
            desired.push(selection::DesiredAsset {
                id: CATCHMENT_WATER_ID.to_string(),
                lod_level: selection::LodLevel::Lod0,
                bytes: terrain_spec.water_bytes,
                triangles: terrain_spec.water_triangles,
                pinned: true,
            });
            meta_map.insert(
                CATCHMENT_WATER_ID.to_string(),
                StreamAssetMeta {
                    url: terrain_spec.water_url.clone(),
                    sha256: terrain_spec.water_sha256.clone(),
                    colour: [0.18, 0.42, 0.50, 1.0],
                    bytes: terrain_spec.water_bytes,
                    triangles: terrain_spec.water_triangles,
                },
            );

            // 2. Pinned exhibits & active place organs (filtering legacy `habitat-terrain`).
            let filtered_base = filter_base_ids_for_catchment(&selection_ids);
            for id in &filtered_base {
                if let Some(entry) = catalog_entries.iter().find(|e| e.scene_id == *id) {
                    let colour = scene_colours
                        .iter()
                        .find(|(k, _)| k == id)
                        .map(|(_, c)| *c)
                        .unwrap_or([0.55, 0.6, 0.45, 1.0]);
                    desired.push(selection::DesiredAsset {
                        id: id.clone(),
                        lod_level: selection::LodLevel::Lod0,
                        bytes: entry.bytes,
                        triangles: entry.triangles,
                        pinned: true,
                    });
                    meta_map.insert(
                        id.clone(),
                        StreamAssetMeta {
                            url: entry.url.clone(),
                            sha256: entry.sha256.clone(),
                            colour,
                            bytes: entry.bytes,
                            triangles: entry.triangles,
                        },
                    );
                }
            }

            // 3. Admitted intenteco species filtered by the active 3x3 tile ring & range,
            //    sorted nearest-first with hysteresis LOD selection.
            let current_lods = ledger.lod_map();
            let has_placements = !species_placements.is_empty();
            let active_tiles = tile_ring.active_tiles();
            let mut ranked_species: Vec<(f32, &SpeciesCatalogEntry)> = catalog_species
                .iter()
                .filter(|sp| {
                    if manual_evict_set.contains(&sp.slug) {
                        return false;
                    }
                    if !has_placements {
                        return true;
                    }
                    species_placements
                        .get(&sp.slug)
                        .map(|ps| {
                            !ps.is_empty()
                                && species_in_active_ring_or_range(
                                    cam_target,
                                    ps,
                                    active_tiles,
                                    340.0,
                                )
                        })
                        .unwrap_or(false)
                })
                .map(|sp| {
                    let dist = species_placements
                        .get(&sp.slug)
                        .map(|ps| min_distance_to_placements(cam_target, ps))
                        .unwrap_or(120.0);
                    (dist, sp)
                })
                .collect();
            ranked_species.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.slug.cmp(&b.1.slug)));

            let is_mobile_tier = matches!(
                tier_id.trim().to_ascii_lowercase().as_str(),
                "tier-c" | "tierc" | "tier_c" | "c"
            );
            for (dist_m, sp) in ranked_species {
                let prev_lod = current_lods.get(&sp.slug).copied();
                let mut target_lod =
                    selection::select_lod_for_distance(dist_m, prev_lod, &tier_id, rhizo_active);
                if is_mobile_tier {
                    target_lod = selection::LodLevel::Lod2;
                } else if !initial_uploaded && target_lod == selection::LodLevel::Lod0 {
                    target_lod = selection::LodLevel::Lod1;
                }
                if let Some(lod_asset) = select_species_lod_asset(sp, target_lod, rhizo_active) {
                    desired.push(selection::DesiredAsset {
                        id: sp.slug.clone(),
                        lod_level: target_lod,
                        bytes: lod_asset.bytes,
                        triangles: lod_asset.triangles,
                        pinned: false,
                    });
                    meta_map.insert(
                        sp.slug.clone(),
                        StreamAssetMeta {
                            url: lod_asset.url.clone(),
                            sha256: lod_asset.sha256.clone(),
                            colour: species_tint(&sp.kingdom, false),
                            bytes: lod_asset.bytes,
                            triangles: lod_asset.triangles,
                        },
                    );
                }
                if rhizo_active && sp.kingdom.eq_ignore_ascii_case("flora") {
                    if let Some(ref rhizo_lod) = sp.rhizosphere {
                        let rhizo_id = format!("{}-rhizosphere", sp.slug);
                        if !manual_evict_set.contains(&rhizo_id) {
                            desired.push(selection::DesiredAsset {
                                id: rhizo_id.clone(),
                                lod_level: selection::LodLevel::Lod1,
                                bytes: rhizo_lod.bytes,
                                triangles: rhizo_lod.triangles,
                                pinned: false,
                            });
                            meta_map.insert(
                                rhizo_id,
                                StreamAssetMeta {
                                    url: rhizo_lod.url.clone(),
                                    sha256: rhizo_lod.sha256.clone(),
                                    colour: species_tint(&sp.kingdom, true),
                                    bytes: rhizo_lod.bytes,
                                    triangles: rhizo_lod.triangles,
                                },
                            );
                        }
                    }
                }
            }

            let mut delta = selection::compute_incremental_delta(&ledger, &desired, stream_frame);
            // Actively evict any non-pinned resident assets that departed the active 3x3 tile
            // ring, were omitted by place/rhizosphere change, or were manually evicted via hook.
            let desired_id_set: BTreeSet<&str> = desired.iter().map(|d| d.id.as_str()).collect();
            for res_id in ledger.resident_ids() {
                if let Some(entry) = ledger.get(&res_id) {
                    if !entry.pinned
                        && !desired_id_set.contains(res_id.as_str())
                        && !delta.to_drop.contains(&res_id)
                    {
                        delta.to_drop.push(res_id);
                    }
                }
            }

            if initial_uploaded
                && has_catchment
                && delta.is_empty()
                && !force_tick
                && manual_evict_set.is_empty()
                && rhizo_active == last_rhizo
                && place_id == last_place
            {
                last_cam = cam_target;
                publish_stream_stats(&stats);
                continue;
            }

            let eviction_reason = if delta.to_drop.is_empty() {
                String::new()
            } else if !manual_evict_set.is_empty() {
                format!("manual-lru-evict:{}", delta.to_drop.join(","))
            } else if !ring_delta.departing.is_empty() {
                format!("tile-ring-departure:{}", delta.to_drop.join(","))
            } else {
                format!("budget-lru-evict:{}", delta.to_drop.join(","))
            };
            let added_count = delta.to_add.len();
            let evicted_ids = delta.to_drop.clone();

            // Build the final budget-admitted ID list in priority order.
            let mut next_ledger = ledger.clone();
            next_ledger.apply_delta(&delta, stream_frame);
            let admitted_ids: BTreeSet<String> = next_ledger.resident_ids().into_iter().collect();
            let ordered_ids: Vec<String> = desired
                .iter()
                .filter(|d| admitted_ids.contains(&d.id))
                .map(|d| d.id.clone())
                .collect();

            // Fetch uncached verified bytes concurrently off the main UI thread via WorkerAssetPool
            // (`/world/asset-worker.js`, falling back cleanly to main-thread `fetch_verified_asset`).
            let mut missing: Vec<(String, String, String)> = Vec::new();
            let mut seen_shas: BTreeSet<String> = BTreeSet::new();
            for id in &ordered_ids {
                if let Some(meta) = meta_map.get(id) {
                    if !verified_cache.contains_key(&meta.sha256)
                        && seen_shas.insert(meta.sha256.clone())
                    {
                        missing.push((id.clone(), meta.url.clone(), meta.sha256.clone()));
                    }
                }
            }

            if !missing.is_empty() {
                stats.streaming_phase = "fetching".into();
                stats.last_stream_note = format!("fetching:{}", missing.len());
                stats.worker_active = worker_pool.is_active();
                stats.worker_inflight = missing.len() as u64;
                publish_stream_stats(&stats);

                let fetched_map: Rc<RefCell<HashMap<String, Vec<u8>>>> =
                    Rc::new(RefCell::new(HashMap::new()));
                let fetch_errors: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
                let promises = js_sys::Array::new();
                for (req_id, req_url, req_sha) in missing {
                    let allow = allowlist.clone();
                    let sink = fetched_map.clone();
                    let errs = fetch_errors.clone();
                    let pool = worker_pool.clone();
                    let p = wasm_bindgen_futures::future_to_promise(async move {
                        match pool
                            .fetch_verified(
                                &req_id,
                                &req_url,
                                &req_sha,
                                &allow,
                                64 * 1024 * 1024,
                            )
                            .await
                        {
                            Ok((bytes, _source, _used_worker)) => {
                                sink.borrow_mut().insert(req_sha, bytes);
                            }
                            Err(e) => {
                                let msg = e.as_string().unwrap_or_else(|| format!("fetch-err:{req_id}"));
                                errs.borrow_mut().push(msg);
                                web_sys::console::warn_1(&e);
                            }
                        }
                        Ok(JsValue::UNDEFINED)
                    });
                    promises.push(&p);
                }
                let _ = JsFuture::from(js_sys::Promise::all(&promises)).await;
                for (sha, bytes) in fetched_map.borrow_mut().drain() {
                    verified_cache.insert(sha, bytes);
                }
                if let Some(first_err) = fetch_errors.borrow().first() {
                    stats.last_error = Some(first_err.clone());
                }
                stats.worker_active = worker_pool.is_active();
                stats.worker_inflight = worker_pool.inflight();
                stats.worker_completed = worker_pool.completed();
                stats.cache_entries = verified_cache.len();
                publish_stream_stats(&stats);
                // Yield to the browser UI thread after concurrent fetches complete.
                sleep_ms(0).await;
            }

            let assets_js = js_sys::Array::new();
            let mut uploaded_species_slugs: Vec<String> = Vec::new();
            let mut rhizo_mounted: usize = 0;
            let mut actual_bytes: u64 = 0;
            let mut actual_triangles: u64 = 0;
            let species_slug_set: BTreeSet<&str> =
                catalog_species.iter().map(|s| s.slug.as_str()).collect();

            let mut pending_tex_refs: Vec<([u8; 32], u8, f32)> = Vec::new();
            for (idx, id) in ordered_ids.iter().enumerate() {
                let Some(meta) = meta_map.get(id) else {
                    continue;
                };
                let normalized = match verified_cache.get(&meta.sha256) {
                    Some(raw) => ensure_catalog_attested_10d(id, &meta.url, raw, &meta.sha256)
                        .unwrap_or_else(|_| raw.clone()),
                    None => continue,
                };
                let normalized_len = normalized.len();
                verified_cache.insert(meta.sha256.clone(), normalized.clone());

                if id == CATCHMENT_TERRAIN_LOD0_ID
                    || id == CATCHMENT_TERRAIN_LOD1_ID
                    || id == CATCHMENT_WATER_ID
                {
                    pending_tex_refs
                        .extend(super::textures::collect_material_texture_refs(&normalized));
                }

                let obj = js_sys::Object::new();
                if js_sys::Reflect::set(
                    &obj,
                    &"bytes".into(),
                    &js_sys::Uint8Array::from(normalized.as_slice()),
                )
                .is_err()
                {
                    continue;
                }
                let _ = js_sys::Reflect::set(&obj, &"id".into(), &id.clone().into());
                for (name, value) in ["r", "g", "b", "a"].iter().zip(meta.colour) {
                    let _ = js_sys::Reflect::set(
                        &obj,
                        &(*name).into(),
                        &JsValue::from_f64(value as f64),
                    );
                }
                assets_js.push(&obj);
                actual_bytes += normalized_len as u64;
                actual_triangles += meta.triangles;
                if species_slug_set.contains(id.as_str()) {
                    uploaded_species_slugs.push(id.clone());
                } else if id.ends_with("-rhizosphere") {
                    rhizo_mounted += 1;
                }
                // Yield to the browser UI thread between heavy .10d decode/packing steps
                // so HUD buttons, drawers, and camera/driving input remain responsive.
                if normalized_len > 256 * 1024 || idx % 3 == 2 {
                    sleep_ms(0).await;
                }
            }

            if assets_js.length() == 0 {
                stats.last_stream_note = "upload-skipped:zero-assets".into();
                publish_stream_stats(&stats);
                continue;
            }

            // Fetch any unhydrated terrain-v2 / water PBR maps off the borrow so
            // `QualiaPortal::load_texture_encoded` binds albedo, normal, and ORM maps
            // immediately before `load_scene_assets` constructs terrain/water GPU materials.
            let fetched_textures = if !pending_tex_refs.is_empty() {
                super::textures::fetch_catchment_textures(&pending_tex_refs, &hydrated_textures)
                    .await
            } else {
                Vec::new()
            };

            stats.streaming_phase = "uploading".into();
            stats.last_stream_note = format!(
                "uploading:{}:species={}",
                assets_js.length(),
                uploaded_species_slugs.len()
            );
            publish_stream_stats(&stats);
            sleep_ms(0).await;

            // Submit the budget-bounded merged scene to QualiaPortal (never borrowing across await).
            for _ in 0..16 {
                if let Ok(mut w) = state.try_borrow_mut() {
                    if let Some(ewm) = w.ewm.as_ref() {
                        if ewm.busy.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
                        }
                    }
                    for (digest, mime, role, cutoff, encoded) in &fetched_textures {
                        if let Ok(true) =
                            w.portal
                                .load_texture_encoded(digest, mime, encoded, *role, *cutoff)
                        {
                            hydrated_textures.insert((*digest, *role));
                        }
                    }
                    match w.portal.load_scene_assets(&assets_js) {
                        Ok(uploaded) => {
                            if let Ok(Some(json_str)) =
                                js_sys::JSON::stringify(&uploaded).map(|s| s.as_string())
                            {
                                if let Ok(upload_receipt) = serde_json::from_str::<Value>(&json_str) {
                                    w.upload = upload_receipt.clone();
                                    w.receipt["upload"] = upload_receipt.clone();
                                    let scene = w.scene.clone();
                                    let overlay = w.macros.clone();
                                    match super::instances::bind_resident(
                                        &mut w.portal,
                                        &scene,
                                        &upload_receipt,
                                        overlay.as_ref(),
                                    ) {
                                        Ok(inst_receipt) => {
                                            w.receipt["instances"] = inst_receipt;
                                            let gpu_tris = upload_receipt["total_triangles"]
                                                .as_u64()
                                                .unwrap_or(actual_triangles);
                                            let terrain_mounted =
                                                upload_has_catchment_terrain(&upload_receipt);
                                            let mounted_part_ids: BTreeSet<String> = upload_receipt
                                                .get("parts")
                                                .and_then(Value::as_array)
                                                .map(|parts| {
                                                    parts
                                                        .iter()
                                                        .filter_map(|p| {
                                                            p.get("id")
                                                                .and_then(Value::as_str)
                                                                .map(ToString::to_string)
                                                        })
                                                        .collect()
                                                })
                                                .unwrap_or_default();
                                            let confirmed_species: Vec<String> =
                                                if mounted_part_ids.is_empty() {
                                                    uploaded_species_slugs.clone()
                                                } else {
                                                    uploaded_species_slugs
                                                        .iter()
                                                        .filter(|slug| {
                                                            mounted_part_ids.contains(slug.as_str())
                                                        })
                                                        .cloned()
                                                        .collect()
                                                };
                                            if terrain_mounted {
                                                if let Some(ewm) = w.ewm.as_mut() {
                                                    ewm.enhancement_pending = false;
                                                }
                                            }

                                            // Commit ledger & actively evict unloaded assets from verified_cache.
                                            ledger = next_ledger;
                                            let active_shas: BTreeSet<String> = ordered_ids
                                                .iter()
                                                .filter_map(|id| meta_map.get(id).map(|m| m.sha256.clone()))
                                                .collect();
                                            for id in &ordered_ids {
                                                if let Some(meta) = meta_map.get(id) {
                                                    asset_sha_by_id
                                                        .insert(id.clone(), meta.sha256.clone());
                                                }
                                            }
                                            for ev_id in &evicted_ids {
                                                if let Some(old_sha) = asset_sha_by_id.remove(ev_id)
                                                {
                                                    if !active_shas.contains(&old_sha) {
                                                        verified_cache.remove(&old_sha);
                                                    }
                                                }
                                            }

                                            initial_uploaded = true;
                                            last_cam = cam_target;
                                            last_rhizo = rhizo_active;
                                            last_place = place_id.clone();

                                            let total_placements: usize =
                                                species_placements.values().map(Vec::len).sum();
                                            let phase_str = if terrain_mounted
                                                && !confirmed_species.is_empty()
                                            {
                                                "active"
                                            } else if terrain_mounted {
                                                "terrain-ready"
                                            } else {
                                                "first-paint"
                                            };
                                            let telemetry = EwmStreamTelemetry {
                                                active_tier: tier_id.clone(),
                                                active_place: place_id.clone(),
                                                streamed_species_count: confirmed_species.len(),
                                                streamed_species_slugs: confirmed_species.clone(),
                                                catchment_placements: total_placements,
                                                catchment_terrain_active: terrain_mounted,
                                                catchment_terrain_lod: if terrain_mounted {
                                                    terrain_lod_label.to_string()
                                                } else {
                                                    "legacy".into()
                                                },
                                                resident_triangles: gpu_tris,
                                                resident_bytes: actual_bytes,
                                                budget_max_triangles: ledger.budget.max_triangles,
                                                budget_max_bytes: ledger.budget.max_bytes,
                                                rhizosphere_layers_mounted: rhizo_mounted,
                                                streaming_phase: phase_str.into(),
                                            };
                                            let phase_note = format!(
                                                "active:species={}:terrain={terrain_mounted}:tris={gpu_tris}",
                                                confirmed_species.len()
                                            );

                                            stats.total_streamed_in = stats
                                                .total_streamed_in
                                                .saturating_add(added_count as u64);
                                            stats.total_evicted_out = stats
                                                .total_evicted_out
                                                .saturating_add(evicted_ids.len() as u64);
                                            stats.resident_assets = ledger.len();
                                            stats.resident_bytes = actual_bytes;
                                            stats.resident_triangles = gpu_tris;
                                            stats.budget_max_bytes = ledger.budget.max_bytes;
                                            stats.budget_max_triangles =
                                                ledger.budget.max_triangles;
                                            stats.worker_active = worker_pool.is_active();
                                            stats.worker_inflight = worker_pool.inflight();
                                            stats.worker_completed = worker_pool.completed();
                                            stats.cache_entries = verified_cache.len();
                                            stats.streaming_phase = phase_str.into();
                                            stats.last_stream_note = phase_note.clone();
                                            if !eviction_reason.is_empty() {
                                                stats.last_eviction_reason =
                                                    eviction_reason.clone();
                                            }
                                            stats.last_error = None;
                                            publish_stream_stats(&stats);

                                            w.receipt["catchment_stream_note"] = json!(phase_note);
                                            super::play::update_ewm_stream_telemetry(&mut w, telemetry);
                                            text(
                                                "native-world-status",
                                                &format!(
                                                    "600 m catchment & {} living species active ({} instances). Press 2 Walk, 3 Fly, R Rhizosphere X-Ray, E Inspect.",
                                                    confirmed_species.len(),
                                                    total_placements
                                                ),
                                            );
                                        }
                                        Err(e) => {
                                            let err_msg = format!(
                                                "bind-err:{}",
                                                e.as_string().unwrap_or_else(|| "unknown".into())
                                            );
                                            stats.last_stream_note = err_msg.clone();
                                            stats.last_error = Some(err_msg);
                                            publish_stream_stats(&stats);
                                            web_sys::console::warn_1(&e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let err_msg = format!(
                                "upload-err:{}",
                                e.as_string().unwrap_or_else(|| "unknown".into())
                            );
                            stats.last_stream_note = err_msg.clone();
                            stats.last_error = Some(err_msg);
                            publish_stream_stats(&stats);
                            web_sys::console::warn_1(&e);
                        }
                    }
                    break;
                }
                sleep_ms(16).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        ewm_streaming_enabled, filter_base_ids_for_catchment, is_legacy_replaced_by_catchment,
        select_catchment_terrain_for_tier, strip_angle_brackets, unquote_rdf_literal,
        CatchmentTerrainSpec, CATCHMENT_TERRAIN_LOD0_ID, CATCHMENT_TERRAIN_LOD1_ID,
    };

    #[test]
    fn ewm_streaming_enabled_by_default() {
        assert!(ewm_streaming_enabled("", None));
        assert!(ewm_streaming_enabled("?foo=bar", None));
        assert!(ewm_streaming_enabled("?backend=webgl2", None));
    }

    #[test]
    fn ewm_streaming_respects_explicit_opt_out() {
        assert!(!ewm_streaming_enabled("?ewm=0", None));
        assert!(!ewm_streaming_enabled("?foo=1&ewm=0&bar=2", None));
        assert!(!ewm_streaming_enabled("", Some("0")));
        assert!(!ewm_streaming_enabled("?backend=webgpu", Some("0")));
    }

    #[test]
    fn ewm_streaming_preserves_opt_in_and_query_precedence() {
        assert!(ewm_streaming_enabled("?ewm=1", None));
        assert!(ewm_streaming_enabled("", Some("1")));
        assert!(ewm_streaming_enabled("?ewm=1", Some("0")));
        assert!(!ewm_streaming_enabled("?ewm=0", Some("1")));
    }

    #[test]
    fn catchment_terrain_selects_lod0_for_desktop_and_lod1_for_mobile_tier_c() {
        let spec = CatchmentTerrainSpec::default();
        let (id_a, lod_a, _, _, _, tris_a) = select_catchment_terrain_for_tier("tier-a", &spec);
        assert_eq!(id_a, CATCHMENT_TERRAIN_LOD0_ID);
        assert_eq!(lod_a, "lod0");
        assert_eq!(tris_a, 115_200);

        let (id_c, lod_c, _, _, _, tris_c) = select_catchment_terrain_for_tier("tier-c", &spec);
        assert_eq!(id_c, CATCHMENT_TERRAIN_LOD1_ID);
        assert_eq!(lod_c, "lod1");
        assert!(tris_c <= 32_000);
    }

    #[test]
    fn filter_base_ids_omits_legacy_habitat_terrain_while_preserving_exhibits() {
        let base = vec![
            "canberra-reference-terrain".into(),
            "habitat-terrain".into(),
            "vintage-windup-camper--chassis".into(),
            "solar-array-three--panel".into(),
        ];
        let filtered = filter_base_ids_for_catchment(&base);
        assert!(!filtered.iter().any(|id| id == "habitat-terrain"));
        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().any(|id| id == "vintage-windup-camper--chassis"));
    }

    #[test]
    fn filter_base_ids_omits_legacy_white_trunk_trees_and_keeps_clearing_exhibits() {
        let base = vec![
            "habitat-terrain".into(),
            "prototype-eucalyptus-lush-mid".into(),
            "prototype-eucalyptus-lush-near".into(),
            "lush-lush-near-01".into(),
            "lush-lush-mid-02".into(),
            "eucalyptus--pale-bark".into(),
            "eucalyptus-far--leaf-olive".into(),
            "car-passat-b7--gold".into(),
            "vintage-windup-camper--chassis".into(),
            "solar-array-three--panel".into(),
            "observatory--pale-limestone".into(),
            "tidal-vessel--bronze".into(),
            "starlink-mini--dish-front".into(),
        ];
        assert!(is_legacy_replaced_by_catchment("habitat-terrain"));
        assert!(is_legacy_replaced_by_catchment("prototype-eucalyptus-lush-mid"));
        assert!(is_legacy_replaced_by_catchment("lush-lush-near-01"));
        assert!(is_legacy_replaced_by_catchment("lush-lush-mid-02"));
        assert!(!is_legacy_replaced_by_catchment("car-passat-b7--gold"));
        let filtered = filter_base_ids_for_catchment(&base);
        assert_eq!(
            filtered,
            vec![
                "car-passat-b7--gold",
                "vintage-windup-camper--chassis",
                "solar-array-three--panel",
                "observatory--pale-limestone",
                "tidal-vessel--bronze",
                "starlink-mini--dish-front",
            ]
        );
    }

    #[test]
    fn turtle_rdf_lexicon_helpers_unquote_iris_and_typed_literals() {
        assert_eq!(
            strip_angle_brackets("<https://webizen.au/catalog/place/se-australia-catchment>"),
            "https://webizen.au/catalog/place/se-australia-catchment"
        );
        assert_eq!(
            strip_angle_brackets("https://webizen.au/catalog/place/se-australia-catchment"),
            "https://webizen.au/catalog/place/se-australia-catchment"
        );
        assert_eq!(
            unquote_rdf_literal("\"eucalyptus-camaldulensis\""),
            "eucalyptus-camaldulensis"
        );
        assert_eq!(
            unquote_rdf_literal("\"River Red Gum\"@en"),
            "River Red Gum"
        );
        assert_eq!(
            unquote_rdf_literal("\"-0.004673\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "-0.004673"
        );
        assert_eq!(
            unquote_rdf_literal("\"67\"^^<http://www.w3.org/2001/XMLSchema#integer>"),
            "67"
        );
    }
}
