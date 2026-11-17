mod admission;
mod boot;
mod frame_telemetry;
mod graphics;
mod input;
mod instances;
mod instruments;
mod textures;
mod ui;
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
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Element, Event, HtmlCanvasElement, HtmlInputElement, HtmlSelectElement, KeyboardEvent,
    PointerEvent, WheelEvent,
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
    benchmarking: bool,
    receipt: Value,
}
#[derive(Deserialize)]
struct Asset {
    id: String,
    url: String,
    colour: [f32; 4],
    sha256: String,
    #[serde(default)]
    placements: Vec<instances::Placement>,
}
#[derive(Deserialize)]
struct Water {
    positions: Vec<f32>,
    indices: Vec<u32>,
}
#[derive(Deserialize)]
struct Scene {
    schema: String,
    assets: Vec<Asset>,
    #[serde(default)]
    textures: Vec<textures::Texture>,
    water: Option<Water>,
    triangle_count: Option<u64>,
    vertex_count: Option<u64>,
    expanded_triangle_count: Option<u64>,
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
    w.portal.tick(canvas, dt)?;
    w.timings.record(now() - submission_start, interval);
    if w.portal.graphics_recovery_requested() {
        return Err(err("The graphics device was interrupted. Reload to restore the complete native scene, or continue with the reading view."));
    }
    w.frames += 1;
    if w.pending_pick {
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
    if stamp - w.last_receipt >= 1000. || w.frames == 2 {
        w.last_receipt = stamp;
        let timing = w.timings.summary();
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
