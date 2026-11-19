//! Play mode: the visitor shifts from observer to agent.
//!
//! Play is a host stance, not a sequence action (it does not occupy an
//! action slot). Clicking a mesh opens a toolbox of permitted Q42 actions
//! authored `rel:aboutGroup` that group. SPARQL is queried at pick time;
//! QuinIndex `about` is the source of truth when SPARQL returns no rows.
//! Do not mint camp.rs. Do not special-case group ids in this file.

use super::*;
use super::instances::InstanceState64;
use super::instruments::species::{
    is_audio_opt_in_granted, render_species_card_html_with_lens, set_audio_opt_in,
    vocalize_species_call, RhizosphereLensState, SpeciesCatalogEntry as InstrumentSpeciesEntry,
};
use super::locomotion::{current_mode, rhizosphere_xray_active, MovementMode};
use super::ocs_frames::{
    enu_to_ecef, rebase_to_camera_f32, FloatingOrigin, OcsFrameTree, ScaleRegime, WorldPos64,
};
use crate::runtime::vehicle::{
    corridor_height_at, corridor_zone_at, default_corridor_obstacles, CircleObstacle,
    DriveCameraMode, VehicleInput, VehicleRig, CAMPSITE_SPAWN_X, CAMPSITE_SPAWN_YAW_RAD,
    CAMPSITE_SPAWN_Z,
};
use qualia_core_db::mini_parser::hash_token;
use qualia_core_db::render::camera::{orbit_view_projection_with_near, CAMERA_NEAR_PLANE};
use std::collections::BTreeSet;

const CATCHMENT_ORIGIN_LAT_DEG: f64 = -35.29;
const CATCHMENT_ORIGIN_LON_DEG: f64 = 149.07;
const CATCHMENT_ORIGIN_ALT_M: f64 = 541.0;

#[derive(Clone, Debug)]
pub(super) struct CatchmentPlacementRef {
    pub id: String,
    pub slug: String,
    pub pos: [f32; 3],
    pub state64: InstanceState64,
}

#[derive(Clone, Debug)]
pub(super) struct EwmStreamTelemetry {
    pub active_tier: String,
    pub active_place: String,
    pub streamed_species_count: usize,
    pub streamed_species_slugs: Vec<String>,
    pub catchment_placements: usize,
    pub catchment_terrain_active: bool,
    pub catchment_terrain_lod: String,
    pub resident_triangles: u64,
    pub resident_bytes: u64,
    pub budget_max_triangles: u64,
    pub budget_max_bytes: u64,
    pub rhizosphere_layers_mounted: usize,
    pub streaming_phase: String,
}

impl Default for EwmStreamTelemetry {
    fn default() -> Self {
        Self {
            active_tier: "tier-a".into(),
            active_place: "field-studio".into(),
            streamed_species_count: 0,
            streamed_species_slugs: Vec::new(),
            catchment_placements: 0,
            catchment_terrain_active: false,
            catchment_terrain_lod: "legacy".into(),
            resident_triangles: 0,
            resident_bytes: 0,
            budget_max_triangles: 650_000,
            budget_max_bytes: 384 * 1024 * 1024,
            rhizosphere_layers_mounted: 0,
            streaming_phase: "first-paint".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct VehicleKeyState {
    throttle: bool,
    brake: bool,
    left: bool,
    right: bool,
    handbrake: bool,
}

struct LivePlayBridge {
    ocs_tree: OcsFrameTree,
    floating_origin: FloatingOrigin,
    active_ocs_frame: String,
    rhizosphere_lens: RhizosphereLensState,
    last_rhizosphere_active: bool,
    last_mode: MovementMode,
    species_entries: Vec<InstrumentSpeciesEntry>,
    placements: Vec<CatchmentPlacementRef>,
    telemetry: EwmStreamTelemetry,
    last_publish_ms: f64,
    last_inspected_slug: Option<String>,
    last_inspected_placement: Option<String>,
    vehicle: VehicleRig,
    vehicle_keys: VehicleKeyState,
    vehicle_pedal: VehicleInput,
    vehicle_obstacles: Vec<CircleObstacle>,
    last_vehicle_stamp_ms: f64,
    last_vehicle_active: bool,
}

impl LivePlayBridge {
    fn new() -> Self {
        let ocs_tree = OcsFrameTree::default_webizen_cosmos();
        let origin_ecef = enu_to_ecef(
            [-22.0, 28.0, 4.49],
            CATCHMENT_ORIGIN_LAT_DEG,
            CATCHMENT_ORIGIN_LON_DEG,
            CATCHMENT_ORIGIN_ALT_M,
        );
        Self {
            ocs_tree,
            floating_origin: FloatingOrigin::new(origin_ecef, ScaleRegime::LocalEnu),
            active_ocs_frame: "canberra-public-reference".into(),
            rhizosphere_lens: RhizosphereLensState::default(),
            last_rhizosphere_active: false,
            last_mode: MovementMode::ObserveOrbit,
            species_entries: Vec::new(),
            placements: Vec::new(),
            telemetry: EwmStreamTelemetry::default(),
            last_publish_ms: 0.0,
            last_inspected_slug: None,
            last_inspected_placement: None,
            vehicle: VehicleRig::spawn_campsite(),
            vehicle_keys: VehicleKeyState::default(),
            vehicle_pedal: VehicleInput::default(),
            vehicle_obstacles: default_corridor_obstacles(),
            last_vehicle_stamp_ms: 0.0,
            last_vehicle_active: false,
        }
    }
}

thread_local! {
    static PLAY_BRIDGE: RefCell<LivePlayBridge> = RefCell::new(LivePlayBridge::new());
}

pub(super) fn resolve_active_ocs_frame(tree: &OcsFrameTree, regime: ScaleRegime) -> String {
    let preferred_id = match regime {
        ScaleRegime::LocalEnu => "canberra-public-reference",
        ScaleRegime::PlanetaryWgs84 => "did:q42:cosmos:earth",
        ScaleRegime::HeliocentricIcrf => "did:q42:cosmos:sun",
        ScaleRegime::GalacticCosmos => "did:q42:cosmos:milky-way",
    };
    if let Some(node) = tree.get(preferred_id) {
        return node.id.clone();
    }
    tree.nodes_in_regime(regime)
        .first()
        .map(|n| n.id.clone())
        .unwrap_or_else(|| preferred_id.to_string())
}

pub(super) fn find_nearest_catchment_placement(
    origin: [f32; 3],
    placements: &[CatchmentPlacementRef],
    max_dist_m: f32,
) -> Option<(f32, usize)> {
    let mut best: Option<(f32, usize)> = None;
    for (idx, p) in placements.iter().enumerate() {
        let dx = p.pos[0] - origin[0];
        let dy = p.pos[1] - origin[1];
        let dz = p.pos[2] - origin[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist <= max_dist_m && best.map(|(d, _)| dist < d).unwrap_or(true) {
            best = Some((dist, idx));
        }
    }
    best
}

pub(super) fn init_ewm_bridge(
    state: &Shared,
    tier_id: &str,
    max_triangles: u64,
    max_bytes: u64,
) {
    PLAY_BRIDGE.with(|b| {
        let mut br = b.borrow_mut();
        br.telemetry.active_tier = tier_id.to_string();
        br.telemetry.budget_max_triangles = max_triangles;
        br.telemetry.budget_max_bytes = max_bytes;
    });
    if let Ok(mut w) = state.try_borrow_mut() {
        publish_ewm_state(&mut w);
    }
}

pub(super) fn register_catchment_biosphere(
    entries: Vec<InstrumentSpeciesEntry>,
    placements: Vec<CatchmentPlacementRef>,
) {
    PLAY_BRIDGE.with(|b| {
        let mut br = b.borrow_mut();
        br.telemetry.catchment_placements = placements.len();
        br.species_entries = entries;
        br.placements = placements;
    });
}

pub(super) fn update_ewm_stream_telemetry(w: &mut World, telemetry: EwmStreamTelemetry) {
    PLAY_BRIDGE.with(|b| {
        b.borrow_mut().telemetry = telemetry;
    });
    publish_ewm_state(w);
}

#[allow(dead_code)]
pub(super) fn current_rhizosphere_lens() -> RhizosphereLensState {
    PLAY_BRIDGE.with(|b| b.borrow().rhizosphere_lens)
}

#[allow(dead_code)]
pub(super) fn current_ocs_frame_id() -> String {
    PLAY_BRIDGE.with(|b| b.borrow().active_ocs_frame.clone())
}

pub(super) fn publish_ewm_state(w: &mut World) {
    let mode = current_mode();
    let rhizo_active = rhizosphere_xray_active();
    let altitude_m = w
        .receipt
        .get("locomotion")
        .and_then(|l| l.get("altitude_m"))
        .and_then(Value::as_f64)
        .unwrap_or_else(|| (w.camera.target[1] as f64).max(0.0));
    let active_place = w
        .ewm
        .as_ref()
        .map(|e| {
            if e.selection.place_id.is_empty() {
                "field-studio".to_string()
            } else {
                e.selection.place_id.clone()
            }
        })
        .unwrap_or_else(|| "field-studio".to_string());

    let cam_target = w.camera.target;
    let payload = PLAY_BRIDGE.with(|b| {
        let mut br = b.borrow_mut();
        if br.rhizosphere_lens.active != rhizo_active {
            br.rhizosphere_lens.set_active(rhizo_active, None);
        }
        br.last_rhizosphere_active = rhizo_active;
        br.last_mode = mode;

        // Rebase camera into WGS84 ECEF / ICRF double-precision FloatingOrigin.
        let camera_ecef = enu_to_ecef(
            [cam_target[0] as f64, -cam_target[2] as f64, altitude_m],
            CATCHMENT_ORIGIN_LAT_DEG,
            CATCHMENT_ORIGIN_LON_DEG,
            CATCHMENT_ORIGIN_ALT_M,
        );
        let regime = br.floating_origin.update_altitude(camera_ecef, altitude_m);
        let frame_id = resolve_active_ocs_frame(&br.ocs_tree, regime);
        br.active_ocs_frame = frame_id.clone();

        let rebased_local = rebase_to_camera_f32(
            WorldPos64::new(camera_ecef.x, camera_ecef.y, camera_ecef.z),
            &br.floating_origin,
        );
        let rebased_err = (rebased_local[0] * rebased_local[0]
            + rebased_local[1] * rebased_local[1]
            + rebased_local[2] * rebased_local[2])
            .sqrt();

        let vehicle_active = br.vehicle.active;
        br.last_vehicle_active = vehicle_active;
        let t = &br.telemetry;
        let place_out = if t.active_place.is_empty() {
            active_place.clone()
        } else {
            t.active_place.clone()
        };

        let corridor_zone = if vehicle_active {
            br.vehicle.corridor_zone.as_str()
        } else {
            corridor_zone_at(cam_target[0] as f64, cam_target[2] as f64).as_str()
        };
        let movement_mode = if vehicle_active {
            "drive"
        } else {
            mode.as_str()
        };

        json!({
            "active_tier": t.active_tier,
            "active_place": place_out,
            "streamed_species_count": t.streamed_species_count,
            "streamed_species_slugs": t.streamed_species_slugs,
            "catchment_placements": t.catchment_placements,
            "catchment_terrain_active": t.catchment_terrain_active,
            "catchment_terrain_lod": t.catchment_terrain_lod,
            "resident_triangles": t.resident_triangles,
            "resident_bytes": t.resident_bytes,
            "budget_max_triangles": t.budget_max_triangles,
            "budget_max_bytes": t.budget_max_bytes,
            "rhizosphere_active": br.rhizosphere_lens.active,
            "rhizosphere_ground_opacity": br.rhizosphere_lens.ground_opacity,
            "rhizosphere_layers_mounted": t.rhizosphere_layers_mounted,
            "movement_mode": movement_mode,
            "vehicle_active": vehicle_active,
            "corridor_zone": corridor_zone,
            "active_ocs_frame": frame_id,
            "ocs_regime": regime.as_str(),
            "altitude_m": altitude_m,
            "floating_origin_error_m": rebased_err,
            "audio_opt_in": is_audio_opt_in_granted(),
            "streaming_phase": t.streaming_phase
        })
    });

    w.receipt["ocs_floating_origin"] = json!({
        "active_ocs_frame": payload["active_ocs_frame"],
        "ocs_regime": payload["ocs_regime"],
        "altitude_m": payload["altitude_m"],
        "floating_origin_error_m": payload["floating_origin_error_m"]
    });
    w.receipt["rhizosphere_lens"] = json!({
        "active": payload["rhizosphere_active"],
        "ground_opacity": payload["rhizosphere_ground_opacity"],
        "terrain_cutaway": payload["rhizosphere_active"],
        "layers_mounted": payload["rhizosphere_layers_mounted"]
    });

    if let Some(win) = web_sys::window() {
        if let Ok(parsed) = js_sys::JSON::parse(&payload.to_string()) {
            let _ = js_sys::Reflect::set(&win, &"__webizenEwmState".into(), &parsed);
        }
    }
}

pub(super) fn is_vehicle_active() -> bool {
    PLAY_BRIDGE.with(|b| b.borrow().vehicle.active)
}

fn combined_vehicle_input(keys: VehicleKeyState, pedal: VehicleInput) -> VehicleInput {
    let key_throttle: f64 = if keys.throttle { 1.0 } else { 0.0 };
    let key_brake: f64 = if keys.brake { 1.0 } else { 0.0 };
    let key_steer: f64 = (if keys.right { 1.0 } else { 0.0 }) - (if keys.left { 1.0 } else { 0.0 });
    VehicleInput {
        throttle: key_throttle.max(pedal.throttle.clamp(0.0, 1.0)),
        brake: key_brake.max(pedal.brake.clamp(0.0, 1.0)),
        steer: if key_steer.abs() > 0.01 {
            key_steer
        } else {
            pedal.steer.clamp(-1.0, 1.0)
        },
        handbrake: keys.handbrake || pedal.handbrake,
    }
}

fn publish_vehicle_state(rig: &VehicleRig) {
    let t = rig.telemetry();
    let payload = json!({
        "active": t.active,
        "speed_kmh": t.speed_kmh,
        "gear": t.gear,
        "pitch_deg": t.pitch_deg,
        "roll_deg": t.roll_deg,
        "steer_deg": t.steer_deg,
        "surface": t.surface,
        "traction": t.traction,
        "hitch_attached": t.hitch_attached,
        "trailer_yaw_deg": t.trailer_yaw_deg,
        "headlights": t.headlights,
        "brake_lights": t.brake_lights,
        "reverse_lights": t.reverse_lights,
        "camera_mode": t.camera_mode,
        "x": t.x,
        "y": t.y,
        "z": t.z,
        "corridor_zone": t.corridor_zone
    });
    if let Some(win) = web_sys::window() {
        if let Ok(parsed) = js_sys::JSON::parse(&payload.to_string()) {
            let _ = js_sys::Reflect::set(&win, &"__webizenVehicleState".into(), &parsed);
        }
    }
}

fn apply_vehicle_camera(w: &mut World, rig: &VehicleRig) {
    let pose = rig.compute_drive_camera();
    w.camera.yaw = pose.yaw;
    w.camera.pitch = pose.pitch;
    w.camera.zoom = pose.zoom;
    w.camera.target = pose.target;
    w.goal = w.camera.clone();
    let _ = set_camera(w);
}

fn is_editable_target(target: Option<web_sys::EventTarget>) -> bool {
    let Some(el) = target.and_then(|t| t.dyn_into::<Element>().ok()) else {
        return false;
    };
    let tag = el.tag_name();
    tag.eq_ignore_ascii_case("INPUT")
        || tag.eq_ignore_ascii_case("SELECT")
        || tag.eq_ignore_ascii_case("TEXTAREA")
        || el.has_attribute("contenteditable")
}

fn event_detail(e: &Event) -> Option<JsValue> {
    let val = js_sys::Reflect::get(e.as_ref(), &"detail".into()).ok()?;
    if val.is_undefined() || val.is_null() {
        None
    } else {
        Some(val)
    }
}

fn detail_bool(detail: &JsValue, key: &str) -> Option<bool> {
    if let Some(b) = detail.as_bool() {
        return Some(b);
    }
    js_sys::Reflect::get(detail, &key.into())
        .ok()
        .and_then(|v| v.as_bool())
}

fn detail_f64(detail: &JsValue, key: &str) -> Option<f64> {
    js_sys::Reflect::get(detail, &key.into())
        .ok()
        .and_then(|v| v.as_f64())
        .filter(|v| v.is_finite())
}

fn detail_str(detail: &JsValue, key: &str) -> Option<String> {
    if let Some(s) = detail.as_string() {
        return Some(s);
    }
    js_sys::Reflect::get(detail, &key.into())
        .ok()
        .and_then(|v| v.as_string())
}

fn set_vehicle_active(state: &Shared, desired: Option<bool>) {
    let (now_active, rig_snapshot) = PLAY_BRIDGE.with(|b| {
        let mut br = b.borrow_mut();
        match desired {
            Some(true) => br.vehicle.enter(),
            Some(false) => {
                br.vehicle.exit();
                br.vehicle_keys = VehicleKeyState::default();
                br.vehicle_pedal = VehicleInput::default();
            }
            None => {
                let active = br.vehicle.toggle_active();
                if !active {
                    br.vehicle_keys = VehicleKeyState::default();
                    br.vehicle_pedal = VehicleInput::default();
                }
            }
        }
        (br.vehicle.active, br.vehicle.clone())
    });

    if let Ok(mut w) = state.try_borrow_mut() {
        if now_active {
            if super::locomotion::current_mode() != MovementMode::ObserveOrbit {
                let _ = super::locomotion::set_mode(&mut w, MovementMode::ObserveOrbit);
            }
            apply_vehicle_camera(&mut w, &rig_snapshot);
            text(
                "native-world-status",
                "Driving VW Passat B7. WASD/Arrows drive, Space handbrake, H/T hitch Jayco camper, L lights, C camera, F exit.",
            );
        } else {
            text(
                "native-world-status",
                "Exited VW Passat B7. Press F to re-enter or 2 to walk.",
            );
        }
        publish_vehicle_state(&rig_snapshot);
        publish_ewm_state(&mut w);
    } else {
        publish_vehicle_state(&rig_snapshot);
    }
}

fn match_drive_pedal_key(code: &str, key: &str) -> Option<&'static str> {
    match code {
        "KeyW" | "ArrowUp" => return Some("throttle"),
        "KeyS" | "ArrowDown" => return Some("brake"),
        "KeyA" | "ArrowLeft" => return Some("left"),
        "KeyD" | "ArrowRight" => return Some("right"),
        "Space" => return Some("handbrake"),
        _ => {}
    }
    match key {
        "w" | "W" | "ArrowUp" => Some("throttle"),
        "s" | "S" | "ArrowDown" => Some("brake"),
        "a" | "A" | "ArrowLeft" => Some("left"),
        "d" | "D" | "ArrowRight" => Some("right"),
        " " => Some("handbrake"),
        _ => None,
    }
}

fn set_drive_pedal_flag(action: &str, pressed: bool) {
    PLAY_BRIDGE.with(|b| {
        let keys = &mut b.borrow_mut().vehicle_keys;
        match action {
            "throttle" => keys.throttle = pressed,
            "brake" => keys.brake = pressed,
            "left" => keys.left = pressed,
            "right" => keys.right = pressed,
            "handbrake" => keys.handbrake = pressed,
            _ => {}
        }
    });
}

fn install_vehicle_controls(state: &Shared) -> Result<(), JsValue> {
    let Some(win) = web_sys::window() else {
        return Ok(());
    };

    // Publish initial parked Passat + Jayco state immediately on boot
    PLAY_BRIDGE.with(|b| publish_vehicle_state(&b.borrow().vehicle));

    // Capture-phase keydown on `window` so drive inputs & shortcuts are received BEFORE
    // any canvas/locomotion bubble-phase listener can stop propagation.
    let s = state.clone();
    let win_keydown = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
        let Ok(k) = e.dyn_into::<KeyboardEvent>() else {
            return;
        };
        if is_editable_target(k.target()) || k.ctrl_key() || k.meta_key() || k.alt_key() {
            return;
        }
        let code = k.code();
        let key = k.key();

        if code == "KeyF" || matches!(key.as_str(), "f" | "F") {
            k.prevent_default();
            set_vehicle_active(&s, None);
            return;
        }
        if code == "KeyT"
            || matches!(key.as_str(), "t" | "T")
            || (is_vehicle_active() && (code == "KeyH" || matches!(key.as_str(), "h" | "H")))
        {
            k.prevent_default();
            let (attached, rig) = PLAY_BRIDGE.with(|b| {
                let mut br = b.borrow_mut();
                let attached = br.vehicle.toggle_hitch();
                (attached, br.vehicle.clone())
            });
            text(
                "native-world-status",
                if attached {
                    "Jayco camper hitched to VW Passat B7."
                } else {
                    "Jayco camper unhitched."
                },
            );
            publish_vehicle_state(&rig);
            return;
        }

        if !is_vehicle_active() {
            return;
        }

        if code == "KeyL" || matches!(key.as_str(), "l" | "L") {
            k.prevent_default();
            let (on, rig) = PLAY_BRIDGE.with(|b| {
                let mut br = b.borrow_mut();
                let on = br.vehicle.toggle_headlights();
                (on, br.vehicle.clone())
            });
            text(
                "native-world-status",
                if on {
                    "Passat headlights ON."
                } else {
                    "Passat headlights OFF."
                },
            );
            publish_vehicle_state(&rig);
            return;
        }
        if code == "KeyC" || matches!(key.as_str(), "c" | "C") {
            k.prevent_default();
            let (mode, rig) = PLAY_BRIDGE.with(|b| {
                let mut br = b.borrow_mut();
                let mode = br.vehicle.cycle_camera_mode();
                (mode, br.vehicle.clone())
            });
            text("native-world-status", &format!("Drive camera: {}.", mode.as_str()));
            if let Ok(mut w) = s.try_borrow_mut() {
                apply_vehicle_camera(&mut w, &rig);
            }
            publish_vehicle_state(&rig);
            return;
        }

        if let Some(action) = match_drive_pedal_key(&code, &key) {
            k.prevent_default();
            set_drive_pedal_flag(action, true);
        }
    });
    let win_target: &web_sys::EventTarget = win.unchecked_ref();
    win_target.add_event_listener_with_callback_and_bool(
        "keydown",
        win_keydown.as_ref().unchecked_ref(),
        true,
    )?;
    win_keydown.forget();

    let win_keyup = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
        let Ok(k) = e.dyn_into::<KeyboardEvent>() else {
            return;
        };
        if let Some(action) = match_drive_pedal_key(&k.code(), &k.key()) {
            set_drive_pedal_flag(action, false);
        }
    });
    win_target.add_event_listener_with_callback_and_bool(
        "keyup",
        win_keyup.as_ref().unchecked_ref(),
        true,
    )?;
    win_keyup.forget();

    // Also mirror idempotent pedal keydown/keyup on `document` (capture phase) for
    // synthetic dispatchers that target `document` directly.
    let doc_el = doc();
    let doc_target: &web_sys::EventTarget = doc_el.unchecked_ref();
    let doc_keydown = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
        let Ok(k) = e.dyn_into::<KeyboardEvent>() else {
            return;
        };
        if !is_vehicle_active()
            || is_editable_target(k.target())
            || k.ctrl_key()
            || k.meta_key()
            || k.alt_key()
        {
            return;
        }
        if let Some(action) = match_drive_pedal_key(&k.code(), &k.key()) {
            k.prevent_default();
            set_drive_pedal_flag(action, true);
        }
    });
    doc_target.add_event_listener_with_callback_and_bool(
        "keydown",
        doc_keydown.as_ref().unchecked_ref(),
        true,
    )?;
    doc_keydown.forget();

    let doc_keyup = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
        let Ok(k) = e.dyn_into::<KeyboardEvent>() else {
            return;
        };
        if let Some(action) = match_drive_pedal_key(&k.code(), &k.key()) {
            set_drive_pedal_flag(action, false);
        }
    });
    doc_target.add_event_listener_with_callback_and_bool(
        "keyup",
        doc_keyup.as_ref().unchecked_ref(),
        true,
    )?;
    doc_keyup.forget();

    event(win.unchecked_ref(), "blur", move |_| {
        PLAY_BRIDGE.with(|b| {
            let mut br = b.borrow_mut();
            br.vehicle_keys = VehicleKeyState::default();
            br.vehicle_pedal = VehicleInput::default();
        });
    })?;

    // CustomEvent: webizen:drive-toggle
    let s = state.clone();
    event(win.unchecked_ref(), "webizen:drive-toggle", move |e| {
        let desired = event_detail(&e).and_then(|d| detail_bool(&d, "active"));
        set_vehicle_active(&s, desired);
    })?;

    // CustomEvent: webizen:hitch-toggle
    event(win.unchecked_ref(), "webizen:hitch-toggle", move |e| {
        let desired = event_detail(&e).and_then(|d| detail_bool(&d, "attached"));
        let (attached, rig) = PLAY_BRIDGE.with(|b| {
            let mut br = b.borrow_mut();
            let attached = match desired {
                Some(v) => br.vehicle.set_hitch_attached(v),
                None => br.vehicle.toggle_hitch(),
            };
            (attached, br.vehicle.clone())
        });
        text(
            "native-world-status",
            if attached {
                "Jayco camper hitched to VW Passat B7."
            } else {
                "Jayco camper unhitched."
            },
        );
        publish_vehicle_state(&rig);
    })?;

    // CustomEvent: webizen:headlights-toggle
    event(win.unchecked_ref(), "webizen:headlights-toggle", move |e| {
        let desired = event_detail(&e).and_then(|d| detail_bool(&d, "on"));
        let (on, rig) = PLAY_BRIDGE.with(|b| {
            let mut br = b.borrow_mut();
            let on = match desired {
                Some(v) => {
                    br.vehicle.headlights_on = v;
                    v
                }
                None => br.vehicle.toggle_headlights(),
            };
            (on, br.vehicle.clone())
        });
        text(
            "native-world-status",
            if on {
                "Passat headlights ON."
            } else {
                "Passat headlights OFF."
            },
        );
        publish_vehicle_state(&rig);
    })?;

    // CustomEvent: webizen:drive-camera-toggle
    let s = state.clone();
    event(win.unchecked_ref(), "webizen:drive-camera-toggle", move |e| {
        let explicit = event_detail(&e)
            .and_then(|d| detail_str(&d, "mode"))
            .and_then(|m| match m.trim().to_ascii_lowercase().as_str() {
                "chase" | "chasefollow" => Some(DriveCameraMode::ChaseFollow),
                "bonnet" | "hood" | "bonnethood" => Some(DriveCameraMode::BonnetHood),
                "cockpit" | "driver" | "cockpitdriver" => Some(DriveCameraMode::CockpitDriver),
                _ => None,
            });
        let (mode, rig) = PLAY_BRIDGE.with(|b| {
            let mut br = b.borrow_mut();
            let mode = match explicit {
                Some(m) => {
                    br.vehicle.camera_mode = m;
                    m
                }
                None => br.vehicle.cycle_camera_mode(),
            };
            (mode, br.vehicle.clone())
        });
        text("native-world-status", &format!("Drive camera: {}.", mode.as_str()));
        if rig.active {
            if let Ok(mut w) = s.try_borrow_mut() {
                apply_vehicle_camera(&mut w, &rig);
            }
        }
        publish_vehicle_state(&rig);
    })?;

    // CustomEvent: webizen:tour-stop
    let s = state.clone();
    event(win.unchecked_ref(), "webizen:tour-stop", move |e| {
        let Some(detail) = event_detail(&e) else {
            return;
        };
        let stop_coords = if let (Some(x), Some(z)) = (detail_f64(&detail, "x"), detail_f64(&detail, "z")) {
            let yaw = detail_f64(&detail, "yaw").unwrap_or(CAMPSITE_SPAWN_YAW_RAD);
            Some((x, z, yaw, "Tour stop"))
        } else {
            let key = detail_str(&detail, "stop")
                .or_else(|| detail_str(&detail, "id"))
                .or_else(|| detail_str(&detail, "zone"))
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            match key.as_str() {
                "campsite" | "forest" | "forestcampsite" | "forest-campsite" => {
                    Some((CAMPSITE_SPAWN_X, CAMPSITE_SPAWN_Z, CAMPSITE_SPAWN_YAW_RAD, "Forest Campsite"))
                }
                "road" | "corridor" | "roadcorridor" | "road-corridor" => {
                    Some((180.0, 50.0, CAMPSITE_SPAWN_YAW_RAD, "Road Corridor"))
                }
                "community" | "community-grounds" | "town" | "townoutskirtscommunitygrounds" => {
                    Some((330.0, 54.0, CAMPSITE_SPAWN_YAW_RAD, "Town-Outskirts Community Grounds"))
                }
                _ => None,
            }
        };
        let Some((x, z, yaw, label)) = stop_coords else {
            return;
        };
        let rig = PLAY_BRIDGE.with(|b| {
            let mut br = b.borrow_mut();
            br.vehicle.teleport_to(x, z, yaw, corridor_height_at);
            br.vehicle.clone()
        });
        if let Ok(mut w) = s.try_borrow_mut() {
            if rig.active {
                apply_vehicle_camera(&mut w, &rig);
            } else {
                w.camera.target = [x as f32, corridor_height_at(x, z) as f32 + 1.5, z as f32];
                w.goal.target = w.camera.target;
                let _ = set_camera(&mut w);
            }
            publish_ewm_state(&mut w);
        }
        publish_vehicle_state(&rig);
        text("native-world-status", &format!("Moved to {label}."));
    })?;

    // CustomEvent: webizen:pedal-input (mobile touch pedals & steering buttons)
    event(win.unchecked_ref(), "webizen:pedal-input", move |e| {
        let Some(detail) = event_detail(&e) else {
            return;
        };
        PLAY_BRIDGE.with(|b| {
            let mut br = b.borrow_mut();
            if let Some(t) = detail_f64(&detail, "throttle") {
                br.vehicle_pedal.throttle = t.clamp(0.0, 1.0);
            }
            if let Some(bk) = detail_f64(&detail, "brake") {
                br.vehicle_pedal.brake = bk.clamp(0.0, 1.0);
            }
            if let Some(st) = detail_f64(&detail, "steer") {
                br.vehicle_pedal.steer = st.clamp(-1.0, 1.0);
            }
            if let Some(hb) = detail_bool(&detail, "handbrake") {
                br.vehicle_pedal.handbrake = hb;
            }
        });
    })?;

    Ok(())
}

fn start_ewm_state_loop(state: &Shared) -> Result<(), JsValue> {
    let closure = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
    let keep = closure.clone();
    let s = state.clone();
    *closure.borrow_mut() = Some(Closure::new(move |stamp: f64| {
        // Step vehicle dynamics & publish window.__webizenVehicleState at frame rate
        let (vehicle_snapshot, vehicle_active) = PLAY_BRIDGE.with(|b| {
            let mut br = b.borrow_mut();
            let dt_s = if br.last_vehicle_stamp_ms > 0.0 {
                ((stamp - br.last_vehicle_stamp_ms) * 0.001).clamp(0.0, 0.05)
            } else {
                0.0
            };
            br.last_vehicle_stamp_ms = stamp;
            if dt_s > 0.0 && (br.vehicle.active || br.vehicle.speed_mps.abs() > 0.02) {
                let input = if br.vehicle.active {
                    combined_vehicle_input(br.vehicle_keys, br.vehicle_pedal)
                } else {
                    VehicleInput::default()
                };
                let obstacles = br.vehicle_obstacles.clone();
                br.vehicle.step_with_terrain(
                    input,
                    dt_s,
                    corridor_height_at,
                    None,
                    &obstacles,
                );
            }
            (br.vehicle.clone(), br.vehicle.active)
        });

        publish_vehicle_state(&vehicle_snapshot);

        let mode = current_mode();
        let rhizo = rhizosphere_xray_active();
        let should_publish = PLAY_BRIDGE.with(|b| {
            let br = b.borrow();
            mode != br.last_mode
                || rhizo != br.last_rhizosphere_active
                || vehicle_active != br.last_vehicle_active
                || (stamp - br.last_publish_ms) >= 250.0
        });

        if vehicle_active || should_publish {
            if let Ok(mut w) = s.try_borrow_mut() {
                if vehicle_active {
                    apply_vehicle_camera(&mut w, &vehicle_snapshot);
                }
                if should_publish {
                    PLAY_BRIDGE.with(|b| {
                        b.borrow_mut().last_publish_ms = stamp;
                    });
                    publish_ewm_state(&mut w);
                }
            }
        }
        if let Some(cb) = keep.borrow().as_ref() {
            if let Some(win) = web_sys::window() {
                let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
            }
        }
    }));
    if let Some(win) = web_sys::window() {
        if let Some(cb) = closure.borrow().as_ref() {
            win.request_animation_frame(cb.as_ref().unchecked_ref())?;
        }
    }
    Ok(())
}

const PLAY_ZOOM: f32 = 16.0;
const PLAY_PITCH_FLOOR: f32 = 0.35;
const PLAY_STATUS: &str = "You are in the world. WASD moves you with the travelling party. Click something to see what you can do with it. Escape returns you to observing.";
const OBSERVE_STATUS: &str = "Explore the world. Your chosen path changes the presentation, never your rights.";
const EMPTY_COPY: &str = "Nothing to do with this yet.";
const PICK_SLOP_FRAC: f32 = 0.12;
const PICK_SLOP_MIN: f32 = 48.0;
const PICK_SLOP_MAX: f32 = 160.0;

pub(super) fn install(state: &Shared) -> Result<(), JsValue> {
    install_toggle(state)?;
    install_toolbox(state)?;
    install_escape(state)?;
    install_vehicle_controls(state)?;
    start_ewm_state_loop(state)?;
    Ok(())
}

pub(super) fn consume_pick(w: &mut World) -> Result<bool, JsValue> {
    let hex = w.portal.selected_semantic_id();
    if hex.is_empty() {
        return Ok(false);
    }
    w.receipt["picked_semantic_id"] = json!(hex);
    let Some((placement_id, asset_id)) = placement_for_semantic(w, &hex) else {
        return Ok(true);
    };
    fill_toolbox(w, &placement_id, &asset_id)?;
    w.receipt["play_pick"] = json!("gpu");
    Ok(true)
}

/// Click-to-toolbox: CPU screen pick opens actions immediately; GPU pick may refine.
pub(super) fn pick_at(w: &mut World, x: f32, y: f32, width: u32, height: u32) -> Result<(), JsValue> {
    if let Err(e) = w.portal.select_node_at(x, y, width, height) {
        show_error(e);
    }
    w.pending_pick = true;
    w.pending_pick_age = 0;
    if let Some((placement_id, asset_id)) = nearest_asset(w, x, y, width, height) {
        fill_toolbox(w, &placement_id, &asset_id)?;
        w.receipt["play_pick"] = json!("cpu");
        w.receipt["picked_pixel"] = json!([x, y]);
        return Ok(());
    }
    if super::locomotion::current_mode() == super::locomotion::MovementMode::WalkExplore
        && super::locomotion::tap_terrain_at(w, x, y, width, height)
    {
        w.receipt["play_pick"] = json!("tap-walk");
        w.receipt["picked_pixel"] = json!([x, y]);
        text("native-world-status", "Walking toward tapped point. Tap an object or press E to inspect.");
        return Ok(());
    }
    show_empty_toolbox("this");
    w.play_focus_group = None;
    w.receipt["play_pick"] = json!("cpu-miss");
    w.receipt["picked_pixel"] = json!([x, y]);
    w.receipt["play_toolbox"] = json!({
        "group": Value::Null,
        "placement": Value::Null,
        "asset": Value::Null,
        "sparql_hits": 0,
        "actions": []
    });
    Ok(())
}

/// Inspect nearest species, camp rig group, or exhibit within interaction range (`E` / Inspect pill).
pub(super) fn interact_nearest(w: &mut World) -> Result<bool, JsValue> {
    let origin = w.camera.target;
    let uploaded = uploaded_ids(w);
    let mut best_priority: Option<(f32, String, String)> = None;
    let mut best_any: Option<(f32, String, String)> = None;

    for asset in &w.scene.assets {
        if asset.id == "gpu-suffix"
            || asset.id == "habitat-terrain"
            || asset.id.starts_with("catchment-600m-")
        {
            continue;
        }
        if let Some(ids) = uploaded.as_ref() {
            if !ids.contains(asset.id.as_str()) {
                continue;
            }
        }
        let has_placement = !asset.placements.is_empty();
        let extra = w
            .macros
            .as_ref()
            .map(|live| live.overlay(&asset.id, has_placement))
            .unwrap_or([0.; 3]);
        let is_priority = w
            .macros
            .as_ref()
            .and_then(|live| live.group_id_for_asset(&asset.id))
            .is_some()
            || species_for_asset(w, &asset.id).is_some();

        let check_pos = |pos: [f32; 3],
                         placement_id: String,
                         best_priority: &mut Option<(f32, String, String)>,
                         best_any: &mut Option<(f32, String, String)>| {
            let dx = pos[0] - origin[0];
            let dy = pos[1] - origin[1];
            let dz = pos[2] - origin[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist > 28.0 {
                return;
            }
            let slot = if is_priority { best_priority } else { best_any };
            if slot.as_ref().map(|(d, _, _)| dist < *d).unwrap_or(true) {
                *slot = Some((dist, placement_id, asset.id.clone()));
            }
        };

        if has_placement {
            for placement in &asset.placements {
                check_pos(
                    [
                        (placement.translation[0] + extra[0]) as f32,
                        (placement.translation[1] + extra[1]) as f32,
                        (placement.translation[2] + extra[2]) as f32,
                    ],
                    placement.id.clone(),
                    &mut best_priority,
                    &mut best_any,
                );
            }
        } else {
            check_pos(
                [extra[0] as f32, extra[1] as f32, extra[2] as f32],
                asset.id.clone(),
                &mut best_priority,
                &mut best_any,
            );
        }
    }

    if let Some((_, placement_id, asset_id)) = best_priority.or(best_any) {
        fill_toolbox(w, &placement_id, &asset_id)?;
        return Ok(true);
    }

    // Search the 317 SE Australia catchment species placements in PLAY_BRIDGE.
    let nearest_catchment = PLAY_BRIDGE.with(|b| {
        let br = b.borrow();
        find_nearest_catchment_placement(origin, &br.placements, 35.0)
            .or_else(|| find_nearest_catchment_placement(origin, &br.placements, 120.0))
            .map(|(dist, idx)| (dist, br.placements[idx].clone()))
    });
    if let Some((_, placement_ref)) = nearest_catchment {
        fill_toolbox(w, &placement_ref.id, &placement_ref.slug)?;
        return Ok(true);
    }

    if let Some(places) = w.graph.places.as_array() {
        let mut best_place: Option<(f32, &Value)> = None;
        for p in places {
            let Some(pos) = p.get("position").and_then(Value::as_array) else {
                continue;
            };
            if pos.len() < 3 {
                continue;
            }
            let px = pos[0].as_f64().unwrap_or(0.0) as f32;
            let py = pos[1].as_f64().unwrap_or(0.0) as f32;
            let pz = pos[2].as_f64().unwrap_or(0.0) as f32;
            let dist = ((px - origin[0]).powi(2) + (py - origin[1]).powi(2) + (pz - origin[2]).powi(2)).sqrt();
            if dist <= 40.0 && best_place.as_ref().map(|(d, _)| dist < *d).unwrap_or(true) {
                best_place = Some((dist, p));
            }
        }
        if let Some((_, place_val)) = best_place {
            let id = place_val["id"].as_str().unwrap_or("exhibit");
            let title = place_val["title"].as_str().unwrap_or(id);
            let desc = place_val["description"].as_str().unwrap_or("");
            if let Some(box_el) = doc().get_element_by_id("native-play-toolbox") {
                box_el.remove_attribute("hidden")?;
                text("native-play-toolbox-title", title);
                text("native-play-toolbox-copy", desc);
                text("native-world-status", &format!("Inspecting {title}."));
                if let Some(nav) = doc().get_element_by_id("native-play-toolbox-actions") {
                    nav.set_text_content(None);
                }
            }
            w.receipt["inspected_exhibit"] = json!({"id": id, "title": title});
            return Ok(true);
        }
    }

    text(
        "native-world-status",
        "No species or exhibit within reach. Walk or fly closer, then press E or tap Inspect.",
    );
    Ok(false)
}

pub(super) fn follow_camera(w: &mut World) -> Result<(), JsValue> {
    if is_vehicle_active()
        || !w.play_mode
        || super::locomotion::current_mode() != super::locomotion::MovementMode::ObserveOrbit
    {
        return Ok(());
    }
    let Some(live) = w.macros.as_ref() else {
        return Ok(());
    };
    let target = live.focus_world();
    if !target.iter().all(|v| v.is_finite()) {
        return Ok(());
    }
    if w.camera.target == target && w.goal.target == target {
        return Ok(());
    }
    w.camera.target = target;
    w.goal.target = target;
    set_camera(w)
}

pub(super) fn refresh_toolbox(w: &mut World) -> Result<(), JsValue> {
    let Some(group_id) = w.play_focus_group.clone() else {
        return Ok(());
    };
    let asset = w
        .receipt
        .get("play_toolbox")
        .and_then(|v| v.get("asset"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let placement = w
        .receipt
        .get("play_toolbox")
        .and_then(|v| v.get("placement"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if asset.is_empty() {
        render_toolbox(&group_id, w.macros.as_ref().and_then(|l| l.group_label_for(&group_id)).unwrap_or(group_id.as_str()), &[], 0)?;
        return Ok(());
    }
    let _ = group_id;
    fill_toolbox(w, &placement, &asset)
}

fn install_toggle(state: &Shared) -> Result<(), JsValue> {
    let button = element("native-play-toggle", "button")?;
    button.set_attribute("type", "button")?;
    button.set_attribute("aria-pressed", "false")?;
    button.set_attribute("aria-label", "Enter play mode and act in the world")?;
    button.set_text_content(Some("Play"));
    let s = state.clone();
    event(button.unchecked_ref(), "click", move |_| {
        if let Err(e) = toggle_play(&s) {
            show_error(e);
        }
    })?;
    Ok(())
}

fn install_toolbox(state: &Shared) -> Result<(), JsValue> {
    let box_el = element("native-play-toolbox", "aside")?;
    box_el.set_attribute("role", "dialog")?;
    box_el.set_attribute("aria-label", "Actions for the selected thing")?;
    box_el.set_attribute("hidden", "")?;
    let title = doc().create_element("h2")?;
    title.set_id("native-play-toolbox-title");
    title.set_text_content(Some("Toolbox"));
    box_el.append_child(&title)?;
    let copy = doc().create_element("p")?;
    copy.set_id("native-play-toolbox-copy");
    box_el.append_child(&copy)?;
    let actions = doc().create_element("nav")?;
    actions.set_id("native-play-toolbox-actions");
    actions.set_class_name("native-controls");
    actions.set_attribute("aria-label", "Actions")?;
    box_el.append_child(&actions)?;
    let close = super::ui::button(&box_el, "Close")?;
    close.set_id("native-play-toolbox-close");
    let s = state.clone();
    event(close.unchecked_ref(), "click", move |_| {
        close_toolbox();
        s.borrow_mut().play_focus_group = None;
    })?;
    let s = state.clone();
    event(box_el.unchecked_ref(), "click", move |e| {
        let Some(start) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let mut node = Some(start);
        while let Some(el) = node {
            if let Some(action) = el.get_attribute("data-action") {
                if action == "toggle-rhizosphere-xray" {
                    let mut w = s.borrow_mut();
                    let active = super::locomotion::toggle_rhizosphere_xray(&mut w);
                    PLAY_BRIDGE.with(|b| {
                        b.borrow_mut().rhizosphere_lens.set_active(active, None);
                    });
                    publish_ewm_state(&mut w);
                    let re_inspect = PLAY_BRIDGE.with(|b| {
                        let br = b.borrow();
                        match (&br.last_inspected_placement, &br.last_inspected_slug) {
                            (Some(p), Some(sl)) => Some((p.clone(), sl.clone())),
                            _ => None,
                        }
                    });
                    if let Some((placement_id, slug)) = re_inspect {
                        let _ = fill_toolbox(&mut w, &placement_id, &slug);
                    }
                    return;
                }
                if action == "vocalize-species-call" {
                    let qac_url = el.get_attribute("data-qac-url").unwrap_or_default();
                    let dist = el
                        .get_attribute("data-distance-m")
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(12.0);
                    set_audio_opt_in(true);
                    let dispatch = vocalize_species_call(&qac_url, dist);
                    let mut w = s.borrow_mut();
                    w.receipt["spatial_audio_dispatch"] = json!({
                        "qac_url": dispatch.qac_url,
                        "distance_m": dispatch.distance_m,
                        "gain": dispatch.gain,
                        "attenuation_db": dispatch.attenuation_db,
                        "lowpass_hz": dispatch.lowpass_hz,
                        "audible": dispatch.audible,
                        "status": dispatch.status,
                    });
                    text("native-world-status", &dispatch.status);
                    publish_ewm_state(&mut w);
                    return;
                }
            }
            node = el.parent_element();
        }
    })?;
    let s = state.clone();
    event(actions.unchecked_ref(), "click", move |e| {
        let Some(start) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let mut node = Some(start);
        while let Some(el) = node {
            if let Some(action) = el.get_attribute("data-action") {
                if !action.is_empty() {
                    if let Err(err) = macros::fire(&mut s.borrow_mut(), &action) {
                        show_error(err);
                        return;
                    }
                    if let Err(err) = refresh_toolbox(&mut s.borrow_mut()) {
                        show_error(err);
                    }
                    return;
                }
            }
            node = el.parent_element();
        }
    })?;
    Ok(())
}

fn install_escape(state: &Shared) -> Result<(), JsValue> {
    let s = state.clone();
    event(
        web_sys::window().unwrap().unchecked_ref(),
        "keydown",
        move |e| {
            let Ok(k) = e.dyn_into::<KeyboardEvent>() else {
                return;
            };
            if k.key() != "Escape" {
                return;
            }
            let toolbox_open = doc()
                .get_element_by_id("native-play-toolbox")
                .map(|el| !el.has_attribute("hidden"))
                .unwrap_or(false);
            if toolbox_open {
                k.prevent_default();
                close_toolbox();
                s.borrow_mut().play_focus_group = None;
                return;
            }
            if is_vehicle_active() {
                k.prevent_default();
                set_vehicle_active(&s, Some(false));
                return;
            }
            let not_observe =
                super::locomotion::current_mode() != super::locomotion::MovementMode::ObserveOrbit;
            if not_observe {
                k.prevent_default();
                let mut w = s.borrow_mut();
                if let Err(err) = super::locomotion::set_mode(
                    &mut w,
                    super::locomotion::MovementMode::ObserveOrbit,
                ) {
                    show_error(err);
                }
                drop(w);
                if s.borrow().play_mode {
                    if let Err(err) = leave_play(&s) {
                        show_error(err);
                    }
                } else {
                    text("native-world-status", OBSERVE_STATUS);
                }
                return;
            }
            if s.borrow().play_mode {
                k.prevent_default();
                if let Err(err) = leave_play(&s) {
                    show_error(err);
                }
            }
        },
    )?;
    Ok(())
}

fn toggle_play(state: &Shared) -> Result<(), JsValue> {
    if state.borrow().play_mode {
        leave_play(state)
    } else {
        enter_play(state)
    }
}

fn enter_play(state: &Shared) -> Result<(), JsValue> {
    {
        let mut w = state.borrow_mut();
        if w.play_mode {
            return Ok(());
        }
        w.play_observer = Some(w.camera.clone());
        w.play_purpose = w
            .receipt
            .get("purpose")
            .and_then(Value::as_str)
            .unwrap_or("learn")
            .to_string();
        w.play_mode = true;
        if w.goal.zoom > 18.0 {
            w.goal.zoom = PLAY_ZOOM;
        }
        if w.goal.pitch < PLAY_PITCH_FLOOR {
            w.goal.pitch = PLAY_PITCH_FLOOR;
        }
        w.camera.zoom = w.goal.zoom;
        w.camera.pitch = w.goal.pitch;
        if let Some(live) = w.macros.as_ref() {
            let target = live.focus_world();
            if target.iter().all(|v| v.is_finite()) {
                w.camera.target = target;
                w.goal.target = target;
            }
        }
        set_camera(&mut w)?;
        w.receipt["play_mode"] = json!(true);
        w.canvas.set_attribute(
            "aria-label",
            "Play mode. WASD moves you. Click something to see what you can do. Escape returns you to observing.",
        )?;
        let canvas_el: web_sys::HtmlElement = w.canvas.clone().unchecked_into();
        let _ = canvas_el.focus();
    }
    html_class(true, "play-mode");
    if let Some(button) = doc().get_element_by_id("native-play-toggle") {
        button.set_text_content(Some("Observe"));
        button.set_attribute("aria-pressed", "true")?;
        button.set_attribute("aria-label", "Leave play mode and observe the world")?;
    }
    if let Some(select) = doc()
        .get_element_by_id("native-purpose")
        .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
    {
        select.set_value("participate");
    }
    super::ui::set_purpose(state, "participate")?;
    if super::locomotion::current_mode() == super::locomotion::MovementMode::ObserveOrbit {
        if let Err(err) = super::locomotion::set_mode(
            &mut state.borrow_mut(),
            super::locomotion::MovementMode::WalkExplore,
        ) {
            show_error(err);
        }
    }
    text("native-world-status", PLAY_STATUS);
    Ok(())
}

fn leave_play(state: &Shared) -> Result<(), JsValue> {
    if super::locomotion::current_mode() != super::locomotion::MovementMode::ObserveOrbit {
        let _ = super::locomotion::set_mode(
            &mut state.borrow_mut(),
            super::locomotion::MovementMode::ObserveOrbit,
        );
    }
    let purpose = {
        let mut w = state.borrow_mut();
        if !w.play_mode {
            return Ok(());
        }
        w.play_mode = false;
        w.play_focus_group = None;
        if let Some(prev) = w.play_observer.take() {
            w.goal.yaw = prev.yaw;
            w.goal.pitch = prev.pitch;
            w.goal.zoom = prev.zoom;
            w.camera.yaw = prev.yaw;
            w.camera.pitch = prev.pitch;
            w.camera.zoom = prev.zoom;
        }
        set_camera(&mut w)?;
        w.receipt["play_mode"] = json!(false);
        w.receipt["play_toolbox"] = Value::Null;
        w.canvas.set_attribute(
            "aria-label",
            "Interactive native world. Use arrow keys to look around, plus and minus to zoom.",
        )?;
        w.play_purpose.clone()
    };
    html_class(false, "play-mode");
    close_toolbox();
    if let Some(button) = doc().get_element_by_id("native-play-toggle") {
        button.set_text_content(Some("Play"));
        button.set_attribute("aria-pressed", "false")?;
        button.set_attribute("aria-label", "Enter play mode and act in the world")?;
    }
    let restore = if purpose.is_empty() {
        "learn".to_string()
    } else {
        purpose
    };
    if let Some(select) = doc()
        .get_element_by_id("native-purpose")
        .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
    {
        select.set_value(&restore);
    }
    super::ui::set_purpose(state, &restore)?;
    text("native-world-status", OBSERVE_STATUS);
    Ok(())
}

fn species_for_asset<'a>(w: &'a World, asset_id: &str) -> Option<&'a super::catalog::SpeciesCatalogEntry> {
    let ewm = w.ewm.as_ref()?;
    let norm = asset_id.strip_suffix("-rhizosphere").unwrap_or(asset_id);
    ewm.catalog
        .species
        .iter()
        .find(|s| norm == s.slug || norm.starts_with(&format!("{}-", s.slug)) || norm.contains(&s.slug))
}

fn instrument_species_for_asset(
    w: &World,
    placement_id: &str,
    asset_id: &str,
) -> Option<(InstrumentSpeciesEntry, InstanceState64, f32)> {
    let norm = asset_id.strip_suffix("-rhizosphere").unwrap_or(asset_id);
    let cam_target = w.camera.target;

    let from_bridge = PLAY_BRIDGE.with(|b| {
        let br = b.borrow();
        let entry = br
            .species_entries
            .iter()
            .find(|s| norm == s.slug || norm.starts_with(&format!("{}-", s.slug)) || norm.contains(&s.slug))
            .cloned()?;
        let matched_placement = br
            .placements
            .iter()
            .find(|p| p.id == placement_id && p.slug == entry.slug)
            .or_else(|| {
                br.placements
                    .iter()
                    .filter(|p| p.slug == entry.slug)
                    .min_by(|a, b| {
                        let da = (a.pos[0] - cam_target[0]).powi(2)
                            + (a.pos[1] - cam_target[1]).powi(2)
                            + (a.pos[2] - cam_target[2]).powi(2);
                        let db = (b.pos[0] - cam_target[0]).powi(2)
                            + (b.pos[1] - cam_target[1]).powi(2)
                            + (b.pos[2] - cam_target[2]).powi(2);
                        da.total_cmp(&db)
                    })
            });
        let (state64, dist_m) = if let Some(p) = matched_placement {
            let dx = p.pos[0] - cam_target[0];
            let dy = p.pos[1] - cam_target[1];
            let dz = p.pos[2] - cam_target[2];
            (p.state64, (dx * dx + dy * dy + dz * dz).sqrt())
        } else {
            (InstanceState64::new(1, 0.85, 1.0, 1.0, [0.1; 4]), 12.0)
        };
        Some((entry, state64, dist_m))
    });
    if from_bridge.is_some() {
        return from_bridge;
    }
    let cat_entry = species_for_asset(w, asset_id)?;
    let fallback = InstrumentSpeciesEntry::from_catalog_entry(
        cat_entry,
        &cat_entry.ecological_zone,
        if cat_entry.kingdom.eq_ignore_ascii_case("flora") {
            "sessile_flora"
        } else {
            "saltatorial"
        },
        "SE Australian Country (custodial attribution in taxon.q42)",
        None,
    );
    Some((
        fallback,
        InstanceState64::new(1, 0.85, 1.0, 1.0, [0.1; 4]),
        12.0,
    ))
}

fn fill_toolbox(w: &mut World, placement_id: &str, asset_id: &str) -> Result<(), JsValue> {
    let group_id = w
        .macros
        .as_ref()
        .and_then(|live| live.group_id_for_asset(asset_id))
        .map(str::to_string);
    let Some(group_id) = group_id else {
        w.play_focus_group = None;
        if let Some((species, state64, dist_m)) =
            instrument_species_for_asset(w, placement_id, asset_id)
        {
            let title = if species.vernacular_name.is_empty() {
                species.scientific_name.clone()
            } else {
                format!("{} ({})", species.vernacular_name, species.scientific_name)
            };
            let rhizo_active = rhizosphere_xray_active();
            let lens = PLAY_BRIDGE.with(|b| {
                let mut br = b.borrow_mut();
                if br.rhizosphere_lens.active != rhizo_active {
                    br.rhizosphere_lens.set_active(rhizo_active, None);
                }
                br.last_inspected_slug = Some(species.slug.clone());
                br.last_inspected_placement = Some(placement_id.to_string());
                br.rhizosphere_lens
            });
            let card_html =
                render_species_card_html_with_lens(&species, &state64, &lens, dist_m);
            if let Some(box_el) = doc().get_element_by_id("native-play-toolbox") {
                box_el.remove_attribute("hidden")?;
                text("native-play-toolbox-title", &title);
                if let Some(copy_el) = doc().get_element_by_id("native-play-toolbox-copy") {
                    copy_el.set_inner_html(&card_html);
                    if let Some(call_btn) = copy_el
                        .query_selector("[data-action='vocalize-species-call']")
                        .ok()
                        .flatten()
                    {
                        let _ =
                            call_btn.set_attribute("data-distance-m", &format!("{dist_m:.2}"));
                    }
                }
                text("native-world-status", &format!("Inspecting {title}."));
                if let Some(nav) = doc().get_element_by_id("native-play-toolbox-actions") {
                    nav.set_text_content(None);
                }
            }
            if is_audio_opt_in_granted() {
                if let Some(ref qac_url) = species.call_qac_url {
                    let dispatch = vocalize_species_call(qac_url, dist_m);
                    w.receipt["spatial_audio_dispatch"] = json!({
                        "qac_url": dispatch.qac_url,
                        "distance_m": dispatch.distance_m,
                        "gain": dispatch.gain,
                        "attenuation_db": dispatch.attenuation_db,
                        "lowpass_hz": dispatch.lowpass_hz,
                        "audible": dispatch.audible,
                        "status": dispatch.status,
                    });
                }
            }
            w.receipt["inspected_species"] = json!({
                "taxon_iri": species.taxon_iri,
                "slug": species.slug,
                "scientific_name": species.scientific_name,
                "vernacular_name": species.vernacular_name,
                "kingdom": species.kingdom,
                "ecological_zone": species.ecological_zone,
                "catchment_zone": species.catchment_zone,
                "distance_m": dist_m,
                "has_rhizosphere": species.rhizosphere_url.is_some(),
                "has_field_emf": !species.field_emf_url.is_empty(),
                "has_call_qac": species.call_qac_url.is_some(),
                "placement": placement_id,
                "asset": asset_id
            });
            publish_ewm_state(w);
            return Ok(());
        }
        show_empty_toolbox(asset_id);
        w.receipt["play_toolbox"] = json!({
            "group": Value::Null,
            "placement": placement_id,
            "asset": asset_id,
            "sparql_hits": 0,
            "actions": []
        });
        return Ok(());
    };
    let label = w
        .macros
        .as_ref()
        .and_then(|live| live.group_label_for(&group_id))
        .unwrap_or(group_id.as_str())
        .to_string();
    let query = knowledge::actions_about_group_query(&group_id);
    let sparql_ids: Vec<String> = match query.as_ref() {
        Some(q) => match knowledge::sparql(&mut w.host, q) {
            Ok(result) => knowledge::sparql_iris(&w.host, &result, 0)
                .into_iter()
                .filter_map(|iri| iri.rsplit('/').next().map(str::to_string))
                .collect(),
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    let sparql_hits = sparql_ids.len();
    let permitted: Vec<(String, String)> = w
        .macros
        .as_ref()
        .map(|live| {
            live.permitted_actions_about(&group_id)
                .into_iter()
                .map(|a| (a.id.clone(), a.label.clone()))
                .collect()
        })
        .unwrap_or_default();
    let actions: Vec<(String, String)> = if sparql_hits == 0 {
        permitted
    } else {
        permitted
            .into_iter()
            .filter(|(id, _)| sparql_ids.iter().any(|s| s == id))
            .collect()
    };
    w.play_focus_group = Some(group_id.clone());
    w.receipt["play_toolbox"] = json!({
        "group": group_id,
        "placement": placement_id,
        "asset": asset_id,
        "sparql_hits": sparql_hits,
        "actions": actions.iter().map(|(id, _)| id).cloned().collect::<Vec<_>>()
    });
    render_toolbox(&group_id, &label, &actions, sparql_hits)?;
    Ok(())
}

fn render_toolbox(
    group_id: &str,
    label: &str,
    actions: &[(String, String)],
    _sparql_hits: usize,
) -> Result<(), JsValue> {
    let Some(box_el) = doc().get_element_by_id("native-play-toolbox") else {
        return Ok(());
    };
    box_el.remove_attribute("hidden")?;
    text("native-play-toolbox-title", label);
    let copy = if actions.is_empty() {
        EMPTY_COPY.to_string()
    } else {
        format!("Things you can do with the {label}.")
    };
    text("native-play-toolbox-copy", &copy);
    text("native-world-status", &copy);
    let Some(nav) = doc().get_element_by_id("native-play-toolbox-actions") else {
        return Ok(());
    };
    nav.set_text_content(None);
    nav.set_attribute("data-group", group_id)?;
    for (id, label) in actions {
        let b = super::ui::button(&nav, label)?;
        b.set_attribute("data-action", id)?;
    }
    if actions.is_empty() {
        text("native-world-status", EMPTY_COPY);
    }
    Ok(())
}

fn show_empty_toolbox(what: &str) {
    let _ = what;
    let _ = render_toolbox("", "This", &[], 0);
}

fn close_toolbox() {
    if let Some(el) = doc().get_element_by_id("native-play-toolbox") {
        let _ = el.set_attribute("hidden", "");
    }
    if let Some(nav) = doc().get_element_by_id("native-play-toolbox-actions") {
        nav.set_text_content(None);
    }
}

fn placement_for_semantic(w: &World, hex: &str) -> Option<(String, String)> {
    for asset in &w.scene.assets {
        if asset.placements.is_empty() {
            let token = format!("urn:webizen:placement:{}", asset.id);
            if format!("{:016x}", hash_token(&token)) == hex {
                return Some((asset.id.clone(), asset.id.clone()));
            }
            continue;
        }
        for placement in &asset.placements {
            let token = format!("urn:webizen:placement:{}", placement.id);
            if format!("{:016x}", hash_token(&token)) == hex {
                return Some((placement.id.clone(), asset.id.clone()));
            }
        }
    }
    None
}

fn nearest_asset(w: &World, x: f32, y: f32, width: u32, height: u32) -> Option<(String, String)> {
    if width == 0 || height == 0 {
        return None;
    }
    let aspect = width as f32 / height as f32;
    let vp = orbit_view_projection_with_near(
        w.camera.yaw,
        w.camera.pitch,
        w.camera.zoom,
        w.camera.target,
        aspect,
        CAMERA_NEAR_PLANE,
    );
    let slop = (PICK_SLOP_FRAC * (width.min(height) as f32)).clamp(PICK_SLOP_MIN, PICK_SLOP_MAX);
    let uploaded = uploaded_ids(w);
    let mut best_group: Option<(f32, f32, String, String)> = None;
    let mut best_any: Option<(f32, f32, String, String)> = None;
    for asset in &w.scene.assets {
        if asset.id == "gpu-suffix" || asset.id.starts_with("catchment-600m-") {
            continue;
        }
        if let Some(ids) = uploaded.as_ref() {
            if !ids.contains(asset.id.as_str()) {
                continue;
            }
        }
        let has_placement = !asset.placements.is_empty();
        let extra = w
            .macros
            .as_ref()
            .map(|live| live.overlay(&asset.id, has_placement))
            .unwrap_or([0.; 3]);
        let grouped = w
            .macros
            .as_ref()
            .and_then(|live| live.group_id_for_asset(&asset.id))
            .is_some();
        if has_placement {
            for placement in &asset.placements {
                consider_point(
                    &vp,
                    [
                        (placement.translation[0] + extra[0]) as f32,
                        (placement.translation[1] + extra[1]) as f32,
                        (placement.translation[2] + extra[2]) as f32,
                    ],
                    x,
                    y,
                    width as f32,
                    height as f32,
                    slop,
                    grouped,
                    placement.id.clone(),
                    asset.id.clone(),
                    &mut best_group,
                    &mut best_any,
                );
            }
        } else {
            consider_point(
                &vp,
                [extra[0] as f32, extra[1] as f32, extra[2] as f32],
                x,
                y,
                width as f32,
                height as f32,
                slop,
                grouped,
                asset.id.clone(),
                asset.id.clone(),
                &mut best_group,
                &mut best_any,
            );
        }
    }
    best_group
        .or(best_any)
        .map(|(_, _, placement, asset)| (placement, asset))
}

fn uploaded_ids(w: &World) -> Option<BTreeSet<&str>> {
    let parts = w.upload.get("parts")?.as_array()?;
    if parts.is_empty() {
        return None;
    }
    Some(
        parts
            .iter()
            .filter_map(|part| part.get("id").and_then(Value::as_str))
            .collect(),
    )
}

fn consider_point(
    vp: &[[f32; 4]; 4],
    world: [f32; 3],
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    slop: f32,
    grouped: bool,
    placement_id: String,
    asset_id: String,
    best_group: &mut Option<(f32, f32, String, String)>,
    best_any: &mut Option<(f32, f32, String, String)>,
) {
    let Some((sx, sy, depth)) = screen_from_world(vp, world, width, height) else {
        return;
    };
    let dist = (sx - x).hypot(sy - y);
    if dist > slop {
        return;
    }
    let candidate = (dist, depth, placement_id, asset_id);
    if grouped {
        if better_hit(&candidate, best_group.as_ref()) {
            *best_group = Some(candidate);
        }
    } else if better_hit(&candidate, best_any.as_ref()) {
        *best_any = Some(candidate);
    }
}

fn better_hit(candidate: &(f32, f32, String, String), current: Option<&(f32, f32, String, String)>) -> bool {
    let Some(current) = current else {
        return true;
    };
    candidate.0 + candidate.1 * 0.01 < current.0 + current.1 * 0.01
}

fn screen_from_world(
    vp: &[[f32; 4]; 4],
    p: [f32; 3],
    width: f32,
    height: f32,
) -> Option<(f32, f32, f32)> {
    let clip_x = vp[0][0] * p[0] + vp[1][0] * p[1] + vp[2][0] * p[2] + vp[3][0];
    let clip_y = vp[0][1] * p[0] + vp[1][1] * p[1] + vp[2][1] * p[2] + vp[3][1];
    let clip_z = vp[0][2] * p[0] + vp[1][2] * p[1] + vp[2][2] * p[2] + vp[3][2];
    let clip_w = vp[0][3] * p[0] + vp[1][3] * p[1] + vp[2][3] * p[2] + vp[3][3];
    if !clip_w.is_finite() || clip_w.abs() < 1e-6 {
        return None;
    }
    let ndc_x = clip_x / clip_w;
    let ndc_y = clip_y / clip_w;
    let ndc_z = clip_z / clip_w;
    if !ndc_x.is_finite() || !ndc_y.is_finite() || !ndc_z.is_finite() {
        return None;
    }
    if ndc_z < 0.0 || ndc_z > 1.0 {
        return None;
    }
    Some((
        (ndc_x * 0.5 + 0.5) * width,
        (0.5 - ndc_y * 0.5) * height,
        ndc_z,
    ))
}

pub(super) fn html_class(add: bool, name: &str) {
    let Some(el) = doc().document_element() else {
        return;
    };
    let mut classes: Vec<String> = el
        .class_name()
        .split_whitespace()
        .filter(|c| *c != name)
        .map(str::to_string)
        .collect();
    if add {
        classes.push(name.to_string());
    }
    el.set_class_name(&classes.join(" "));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_from_world_maps_origin_to_canvas_centre() {
        let vp = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.5, 1.0],
        ];
        let (sx, sy, depth) = screen_from_world(&vp, [0.0, 0.0, 0.0], 1280.0, 800.0).unwrap();
        assert!((sx - 640.0).abs() < 0.01);
        assert!((sy - 400.0).abs() < 0.01);
        assert!((depth - 0.5).abs() < 0.01);
    }

    #[test]
    fn screen_from_world_rejects_behind_camera() {
        let vp = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, -0.5, 1.0],
        ];
        assert!(screen_from_world(&vp, [0.0, 0.0, 0.0], 1280.0, 800.0).is_none());
    }

    #[test]
    fn better_hit_prefers_nearer_pixel() {
        let a = (10.0, 0.2, "a".into(), "car".into());
        let b = (40.0, 0.1, "b".into(), "tree".into());
        assert!(better_hit(&a, Some(&b)));
        assert!(!better_hit(&b, Some(&a)));
    }
}
