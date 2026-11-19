mod admission;
mod boot;
mod catalog;
pub mod character;
mod frame_telemetry;
mod graphics;
mod gis_ingest;
mod gis_join;
mod ground;
mod input;
mod instances;
mod instruments;
mod knowledge;
mod locomotion;
mod macros;
mod play;
mod ocs_frames;
mod scene_q42;
mod stream;
mod textures;
mod ui;
mod understorey;
mod resident_ground;
mod surface_field;
mod woody;
pub mod vehicle;
mod consent;
use crate::{
    graph,
    policy::{self, Camera},
};
pub use boot::boot;
use qualia_core_db::{
    render::portal::{portal_abort_webgpu, portal_init_webgpu, probe_portal_graphics},
    vibe_host::PoetSnapshot,
    QualiaPortal,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    AbortController, Element, Event, HtmlCanvasElement, HtmlInputElement, HtmlSelectElement,
    KeyboardEvent, MouseEvent, PointerEvent, WheelEvent,
};
type Shared = Rc<RefCell<World>>;
struct World {
    portal: QualiaPortal,
    canvas: HtmlCanvasElement,
    host: PoetSnapshot,
    source: String,
    graph: graph::Graph,
    camera: Camera,
    goal: Camera,
    reduced: bool,
    drag: Option<(i32, f32, f32, f32, f32)>,
    last_input: f64,
    last_frame: f64,
    last_receipt: f64,
    frames: u64,
    timings: frame_telemetry::FrameTelemetry,
    pending_pick: bool,
    pending_pick_age: u8,
    benchmarking: bool,
    receipt: Value,
    scene: Scene,
    ewm: Option<EwmLive>,
    macros: Option<macros::Live>,
    upload: Value,
    play_mode: bool,
    play_observer: Option<Camera>,
    play_purpose: String,
    play_focus_group: Option<String>,
    tufts: Option<understorey::ResidentTufts>,
    tuft_shift: f32,
    wood: Option<woody::ResidentWood>,
    woody_shift: f32,
}
struct EwmLive {
    catalog: catalog::Catalog,
    selection: stream::selection::LiveSelection,
    colours: Vec<(String, [f32; 4])>,
    textures: Vec<textures::Texture>,
    enhancement_pending: bool,
    place_volume_pending: bool,
    busy: Rc<AtomicBool>,
    abort: Rc<RefCell<Option<Rc<AbortController>>>>,
}
struct EwmTileJob {
    catalog: catalog::Catalog,
    ids: Vec<String>,
    place_id: String,
    omitted: Vec<String>,
    admit_place_volume: bool,
    colours: Vec<(String, [f32; 4])>,
    textures: Vec<textures::Texture>,
    tile: (i32, i32),
    busy: Rc<AtomicBool>,
    controller: Rc<AbortController>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Asset {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) colour: [f32; 4],
    pub(crate) sha256: String,
    #[serde(default)]
    pub(crate) placements: Vec<instances::Placement>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Water {
    pub(crate) positions: Vec<f32>,
    pub(crate) indices: Vec<u32>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Scene {
    pub(crate) schema: String,
    pub(crate) assets: Vec<Asset>,
    #[serde(default)]
    pub(crate) textures: Vec<textures::Texture>,
    pub(crate) water: Option<Water>,
    pub(crate) triangle_count: Option<u64>,
    pub(crate) vertex_count: Option<u64>,
    pub(crate) expanded_triangle_count: Option<u64>,
}
fn err(v: impl ToString) -> JsValue {
    JsValue::from_str(&v.to_string())
}
fn doc() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}
fn now() -> f64 {
    web_sys::window().unwrap().performance().unwrap().now()
}
fn element(id: &str, tag: &str) -> Result<Element, JsValue> {
    let d = doc();
    if let Some(e) = d.get_element_by_id(id) {
        return Ok(e);
    }
    let e = d.create_element(tag)?;
    e.set_id(id);
    d.body().ok_or(err("No document body"))?.append_child(&e)?;
    Ok(e)
}
fn text(id: &str, value: &str) {
    if let Some(e) = doc().get_element_by_id(id) {
        e.set_text_content(Some(value));
    }
}
fn same_origin(path: &str) -> Result<(), JsValue> {
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.contains("..")
        || path.contains('\\')
    {
        Err(err("Invalid same-origin artifact path"))
    } else {
        Ok(())
    }
}
async fn bytes(path: &str, limit: usize) -> Result<Vec<u8>, JsValue> {
    same_origin(path)?;
    let response: web_sys::Response =
        JsFuture::from(web_sys::window().unwrap().fetch_with_str(path))
            .await?
            .dyn_into()?;
    if !response.ok() {
        return Err(err(format!("Could not load {path}: {}", response.status())));
    }
    let raw = js_sys::Uint8Array::new(&JsFuture::from(response.array_buffer()?).await?);
    if raw.length() as usize > limit {
        return Err(err("Artifact exceeds native host load budget"));
    }
    Ok(raw.to_vec())
}
async fn json_file(path: &str, limit: usize) -> Result<Value, JsValue> {
    serde_json::from_slice(&bytes(path, limit).await?).map_err(err)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn set_camera(w: &mut World) -> Result<(), JsValue> {
    let c = &w.camera;
    w.portal.set_camera_target(
        c.yaw,
        c.pitch,
        c.zoom,
        c.target[0],
        c.target[1],
        c.target[2],
    )
}
fn eval(w: &mut World, name: &str, args: &[Value]) -> Result<Value, JsValue> {
    policy::evaluate(&mut w.host, &w.source, name, args).map_err(err)
}
fn place(w: &mut World, id: &str) -> Result<(), JsValue> {
    let places = w.graph.places.clone();
    let chosen = eval(w, "select_place", &[places, json!(id)])?;
    let camera = Camera::place(&chosen).map_err(err)?;
    let rect = w.canvas.get_bounding_client_rect();
    let viewport = json!({"width":rect.width().max(1.),"height":rect.height().max(1.)});
    let aspect = rect.width().max(1.) / rect.height().max(1.);
    let fitted = eval(
        w,
        "fit_view",
        &[serde_json::to_value(camera).map_err(err)?, json!(aspect)],
    )?;
    let camera: Camera = serde_json::from_value(fitted).map_err(err)?;
    camera.validate().map_err(err)?;
    w.receipt["framing_viewport"] = viewport;
    w.goal = camera;
    if w.reduced {
        w.camera = w.goal.clone();
        set_camera(w)?;
    }
    text(
        "native-place-title",
        chosen["title"].as_str().unwrap_or("Explore"),
    );
    text(
        "native-place-description",
        chosen["description"].as_str().unwrap_or(""),
    );
    if let Some(a) = doc().get_element_by_id("native-reading") {
        let route = chosen["reading_route"]
            .as_str()
            .unwrap_or("/publication/reading.html");
        same_origin(route)?;
        a.set_attribute("href", route)?;
    }
    w.receipt["selected_place"] = json!(id);
    if let Some(ewm) = w.ewm.as_mut() {
        let prev = ewm.abort.borrow_mut().take();
        stream::selection::abort_in_flight(prev.as_deref());
        ewm.selection.place_id = id.to_string();
        ewm.enhancement_pending = true;
        ewm.place_volume_pending = true;
        w.receipt["ewm_live_place"] = json!(id);
        w.receipt["ewm_live_place_base"] = json!(ewm.catalog.live_place_base(id));
        w.receipt["ewm_unload"] = json!(stream::selection::V1_UNLOAD);
    }
    if let Some(pos) = chosen["position"].as_array() {
        if pos.len() >= 3 {
            let x = pos[0].as_f64().unwrap_or(0.) as f32;
            let z = pos[2].as_f64().unwrap_or(0.) as f32;
            macros::travel_with_place(w, x, z)?;
        }
    }
    Ok(())
}
fn camera_input(w: &mut World, kind: &str, dx: f32, dy: f32) -> Result<(), JsValue> {
    let c = serde_json::to_value(&w.camera).map_err(err)?;
    let result = eval(
        w,
        "input_camera",
        &[c, json!({"kind":kind,"dx":dx,"dy":dy})],
    )?;
    let next: Camera = serde_json::from_value(result).map_err(err)?;
    next.validate().map_err(err)?;
    w.camera = next;
    w.goal = w.camera.clone();
    set_camera(w)
}
fn event(
    target: &web_sys::EventTarget,
    name: &str,
    f: impl FnMut(Event) + 'static,
) -> Result<(), JsValue> {
    let closure = Closure::<dyn FnMut(Event)>::new(f);
    target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}
fn show_error(e: JsValue) {
    web_sys::console::error_1(&e);
    text(
        "native-world-status",
        "That action could not be completed. The reading view remains available.",
    );
}
fn take_ewm_tile_job(w: &mut World) -> Result<Option<EwmTileJob>, JsValue> {
    let target = w.camera.target;
    let yaw = w.camera.yaw;
    let pitch = w.camera.pitch;
    let ewm = match w.ewm.as_mut() {
        Some(ewm) => ewm,
        None => return Ok(None),
    };
    let place_id = if ewm.selection.place_id.is_empty() {
        "field-studio".to_string()
    } else {
        ewm.selection.place_id.clone()
    };
    let change = stream::selection::plan_place_change(
        &ewm.catalog,
        &place_id,
        target,
        yaw,
        pitch,
        &ewm.selection.ids,
    );
    let pending = ewm.enhancement_pending;
    let tile_moved = stream::selection::tile_changed(ewm.selection.tile, change.window.tile);
    if !pending && !tile_moved && change.omitted.is_empty() && change.missing.is_empty() {
        return Ok(None);
    }
    if let Some(prev) = ewm.abort.borrow_mut().take() {
        stream::selection::abort_in_flight(Some(prev.as_ref()));
    }
    if ewm.busy.swap(true, Ordering::SeqCst) {
        ewm.enhancement_pending = true;
        return Ok(None);
    }
    ewm.enhancement_pending = false;
    let admit_place_volume = ewm.place_volume_pending;
    ewm.place_volume_pending = false;
    let controller = match AbortController::new() {
        Ok(controller) => Rc::new(controller),
        Err(e) => {
            ewm.busy.store(false, Ordering::SeqCst);
            ewm.enhancement_pending = true;
            ewm.place_volume_pending = admit_place_volume;
            return Err(e);
        }
    };
    *ewm.abort.borrow_mut() = Some(controller.clone());
    let job = EwmTileJob {
        catalog: ewm.catalog.clone(),
        ids: change.window.ids,
        place_id: change.place_id,
        omitted: change.omitted,
        admit_place_volume,
        colours: ewm.colours.clone(),
        textures: ewm.textures.clone(),
        tile: change.window.tile,
        busy: ewm.busy.clone(),
        controller,
    };
    ewm.selection.tile = job.tile;
    Ok(Some(job))
}
async fn yield_macrotask() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
    });
    let _ = JsFuture::from(promise).await;
}
async fn run_ewm_tile_job(state: Shared, job: EwmTileJob) {
    let mut place_volume = if job.admit_place_volume {
        if let Some(url) = boot::place_scene_volume_if_differs(&job.place_id) {
            bytes(&url, 16 * 1024 * 1024).await.ok().map(|raw| (url, raw))
        } else {
            None
        }
    } else {
        None
    };
    let prepared = match stream::selection::prepare_window(
        &job.catalog,
        &job.ids,
        job.controller.as_ref(),
        &job.colours,
        &job.textures,
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(e) => {
            job.busy.store(false, Ordering::SeqCst);
            show_error(e);
            return;
        }
    };
    if prepared.assets.length() == 0 {
        job.busy.store(false, Ordering::SeqCst);
        return;
    }
    let mut prepared = Some(prepared);
    for _ in 0..16 {
        match state.try_borrow_mut() {
            Ok(mut w) => {
                if let Some((url, raw)) = place_volume.take() {
                    let name = url.rsplit('/').next().unwrap_or("place.q42");
                    match knowledge::admit(&mut w.host, &raw, "place", name).map_err(err) {
                        Ok(_) => {
                            w.receipt["ewm_place_volume"] = json!(url);
                        }
                        Err(e) => show_error(e),
                    }
                }
                let prepared = prepared.take().expect("prepared window still owned");
                match stream::selection::admit_prepared(&mut w.portal, prepared) {
                    Ok(live) => {
                        let scene = w.scene.clone();
                        let overlay = w.macros.clone();
                        // GPU mesh already matches live.upload_receipt parts.
                        // Pin that receipt before bind so Pack-up rebind cannot
                        // partition first-paint spans onto an enhanced mesh.
                        w.upload = live.upload_receipt.clone();
                        w.receipt["upload"] = live.upload_receipt.clone();
                        match instances::bind_resident(
                            &mut w.portal,
                            &scene,
                            &live.upload_receipt,
                            overlay.as_ref(),
                        )
                        {
                            Ok(instance_receipt) => {
                                w.receipt["ewm_overlapping"] = json!(job.ids);
                                w.receipt["assets"] = json!(live.asset_receipts);
                                w.receipt["instances"] = instance_receipt;
                                w.receipt["ewm_live_tile"] =
                                    json!({"x": job.tile.0, "z": job.tile.1});
                                w.receipt["ewm_live_bytes"] = json!(live.total_bytes);
                                w.receipt["ewm_live_triangles"] = json!(live.declared_triangles);
                                w.receipt["ewm_triangle_mismatch"] =
                                    live.upload_receipt["triangle_mismatch"].clone();
                                let base: BTreeSet<String> = job
                                    .catalog
                                    .live_place_base(&job.place_id)
                                    .into_iter()
                                    .collect();
                                w.receipt["ewm_window"] = json!(job.ids.len());
                                w.receipt["ewm_enhancement"] = json!(job
                                    .ids
                                    .iter()
                                    .filter(|id| !base.contains(*id))
                                    .count());
                                w.receipt["ewm_live_place"] = json!(job.place_id);
                                w.receipt["ewm_live_place_base"] =
                                    json!(job.catalog.live_place_base(&job.place_id));
                                w.receipt["ewm_omitted"] = json!(job.omitted);
                                w.receipt["ewm_unload"] = json!(stream::selection::V1_UNLOAD);
                                w.receipt["asset_bytes"] = json!(live.total_bytes);
                                text(
                                    "native-world-status",
                                    "This view of the place now includes nearby enhancement.",
                                );
                                if let Some(ewm) = w.ewm.as_mut() {
                                    ewm.selection.ids = job.ids.clone();
                                    ewm.selection.tile = job.tile;
                                    ewm.selection.place_id = job.place_id.clone();
                                }
                            }
                            Err(e) => show_error(e),
                        }
                    }
                    Err(e) => show_error(e),
                }
                job.busy.store(false, Ordering::SeqCst);
                return;
            }
            Err(_) => yield_macrotask().await,
        }
    }
    job.busy.store(false, Ordering::SeqCst);
}
/// Pose grass blades into the buffers allocated at boot, then upload that span.
/// Frozen base vertices are copied from the rest pose. No per-frame heap allocation.
fn sway_tufts(w: &mut World, t_s: f64) {
    let Some(mut tufts) = w.tufts.take() else {
        return;
    };
    let wind = if w.reduced { 0.0 } else { 8.5 };
    tufts.pose(t_s, wind);
    w.tuft_shift = tufts.tip_shift_m;
    for span in tufts.spans() {
        let start = span.scratch_at;
        let end = start + span.vertex_count * 3;
        w.portal
            .write_part_vertices(span.vertex_start, &tufts.scratch()[start..end]);
    }
    w.tufts = Some(tufts);
}
/// Pose eucalypt crowns into the buffers allocated at boot. Root vertices stay
/// at the soil fit written once. No per-frame heap allocation.
fn sway_wood(w: &mut World, t_s: f64) {
    let Some(mut wood) = w.wood.take() else {
        return;
    };
    let wind = if w.reduced { 0.0 } else { 8.5 };
    wood.pose(&mut w.portal, t_s, wind);
    w.woody_shift = wood.tip_shift_m;
    w.wood = Some(wood);
}

fn frame(state: &Shared, stamp: f64) -> Result<(), JsValue> {
    let mut w = state.borrow_mut();
    let interval = stamp - w.last_frame;
    let dt = interval.clamp(0., 50.) as f32;
    w.last_frame = stamp;
    if w.benchmarking {
        return Ok(());
    }
    let c = w.goal.clone();
    let blend = if w.reduced {
        1.
    } else {
        1. - (-dt * 0.004).exp()
    };
    let old = w.camera.clone();
    w.camera.yaw += (c.yaw - w.camera.yaw) * blend;
    w.camera.pitch += (c.pitch - w.camera.pitch) * blend;
    w.camera.zoom += (c.zoom - w.camera.zoom) * blend;
    for i in 0..3 {
        w.camera.target[i] += (c.target[i] - w.camera.target[i]) * blend;
    }
    if (old.yaw - w.camera.yaw).abs()
        + (old.pitch - w.camera.pitch).abs()
        + (old.zoom - w.camera.zoom).abs()
        + old
            .target
            .iter()
            .zip(w.camera.target)
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
        > 0.00001
    {
        set_camera(&mut w)?;
    }
    let canvas = w.canvas.clone();
    let submission_start = now();
    locomotion::tick(&mut w, dt)?;
    macros::tick(&mut w, dt)?;
    if w.play_mode {
        play::follow_camera(&mut w)?;
    }
    sway_tufts(&mut w, stamp * 0.001);
    sway_wood(&mut w, stamp * 0.001);
    w.portal.tick(canvas, dt)?;
    w.timings.record(now() - submission_start, interval);
    if w.portal.graphics_recovery_requested() {
        return Err(err("The graphics device was interrupted. Reload to restore the complete native scene, or continue with the reading view."));
    }
    w.frames += 1;
    if w.pending_pick {
        if w.play_mode {
            if play::consume_pick(&mut w)? {
                w.pending_pick = false;
                w.pending_pick_age = 0;
            } else {
                w.pending_pick_age = w.pending_pick_age.saturating_add(1);
                if w.pending_pick_age >= 12 {
                    w.pending_pick = false;
                    w.pending_pick_age = 0;
                }
            }
        } else {
        let index = w.portal.selected_node_index();
        if index >= 0 {
            let selected = w
                .graph
                .places
                .as_array()
                .unwrap()
                .get(index as usize)
                .and_then(|p| p["id"].as_str())
                .map(String::from);
            if let Some(id) = selected {
                place(&mut w, &id)?;
                w.receipt["picked_place"] = json!(id);
                w.pending_pick = false;
            }
        }
        let id = w.portal.selected_semantic_id();
        if !id.is_empty() {
            w.receipt["picked_semantic_id"] = json!(id);
            let selected = w
                .graph
                .places
                .as_array()
                .unwrap()
                .iter()
                .find(|p| {
                    format!(
                        "{:016x}",
                        qualia_core_db::mini_parser::hash_token(p["uri"].as_str().unwrap_or(""))
                    ) == id
                })
                .and_then(|p| p["id"].as_str())
                .map(String::from);
            if let Some(id) = selected {
                place(&mut w, &id)?;
            }
            w.pending_pick = false;
        }
        }
    }
    if stamp - w.last_receipt >= 1000. || w.frames == 2 {
        w.last_receipt = stamp;
        let timing = w.timings.summary();
        w.receipt["understorey_tip_m"] = json!(w.tuft_shift);
        w.receipt["woody_tip_m"] = json!(w.woody_shift);
        w.receipt["frame_timing"] = json!({"sample_count":timing.sample_count,"total_frames":timing.total_frames,"window_capacity":64,"quantile_method":"nearest rank","tick_cpu_submission_ms":{"p50":timing.tick_p50_ms,"p95":timing.tick_p95_ms},"raf_interval_ms":{"p50":timing.interval_p50_ms,"p95":timing.interval_p95_ms},"gpu_duration_measured":false,"scope":"Elapsed successful portal.tick CPU/submission call, not GPU execution duration; raw RAF intervals exclude benchmark-paused frames"});
        w.receipt["gpu_pass_timing"] = match w.portal.gpu_pass_timings() {
            Ok(snapshot) => js_sys::JSON::stringify(&snapshot)
                .ok().and_then(|s| s.as_string())
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| json!({"available":false,"gpu_duration_measured":false,"reason":"Invalid native GPU timing snapshot"})),
            Err(e) => json!({"available":false,"gpu_duration_measured":false,"reason":e.as_string().unwrap_or("GPU timing snapshot unavailable".into())}),
        };
        let receipt = w.portal.scene_render_receipt()?;
        let report = js_sys::JSON::stringify(&receipt)?
            .as_string()
            .unwrap_or_default();
        w.receipt["scene"] = serde_json::from_str(&report).unwrap_or(Value::Null);
        w.receipt["frames"] = json!(w.frames);
        w.receipt["camera"] = serde_json::to_value(&w.camera).map_err(err)?;
        let debug = w.portal.debug_scene_snapshot()?;
        let debug_report = js_sys::JSON::stringify(&debug)?
            .as_string()
            .unwrap_or_default();
        w.receipt["scene_debug"] = serde_json::from_str(&debug_report).unwrap_or(Value::Null);
        let parsed = js_sys::JSON::parse(&w.receipt.to_string())?;
        js_sys::Reflect::set(
            &web_sys::window().unwrap(),
            &"__webizenNativeWorld".into(),
            &parsed,
        )?;
        js_sys::Reflect::set(
            &web_sys::window().unwrap(),
            &"__webizenNativeDebug".into(),
            &debug,
        )?;
    }
    let job = take_ewm_tile_job(&mut w)?;
    drop(w);
    if let Some(job) = job {
        let state = state.clone();
        spawn_local(async move {
            run_ewm_tile_job(state, job).await;
        });
    }
    Ok(())
}
fn start_frames(state: Shared) -> Result<(), JsValue> {
    let closure = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
    let keep = closure.clone();
    *closure.borrow_mut() = Some(Closure::new(move |stamp: f64| {
        if let Err(e) = frame(&state, stamp) {
            fail(&e);
            return;
        }
        if let Some(cb) = keep.borrow().as_ref() {
            let _ = web_sys::window()
                .unwrap()
                .request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }));
    web_sys::window()
        .unwrap()
        .request_animation_frame(closure.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}
pub fn fail(error: &JsValue) {
    web_sys::console::error_1(error);
    let message = error
        .as_string()
        .unwrap_or("The native world could not start".into());
    if let Ok(e) = element("native-world-status", "p") {
        e.set_text_content(Some(&message));
    }
    if let Ok(a) = element("native-fallback", "a") {
        let _ = a.set_attribute("href", "/publication/reading.html");
        a.set_text_content(Some("Continue with the accessible reading view"));
    }
}
