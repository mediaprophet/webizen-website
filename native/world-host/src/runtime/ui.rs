mod download;
mod reference;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TopDrawer {
    Tour,
    FieldGuide,
    Stats,
    Debug,
    WorldPanel,
}

#[derive(Default)]
struct HudUiState {
    clean_view: bool,
    last_heading_text: String,
    last_zone_text: String,
    last_ocs_text: String,
    last_ocs_regime: String,
    last_drive_active: Option<bool>,
    last_speed_text: String,
    last_gear_text: String,
    last_pitch_text: String,
    last_roll_text: String,
    last_traction_text: String,
    last_hitch_attached: Option<bool>,
    last_lights_on: Option<bool>,
    last_cam_text: String,
}

#[derive(Default)]
struct TouchDriveState {
    left: bool,
    right: bool,
    brake: bool,
    accel: bool,
    pointers: [Option<i32>; 4],
}

thread_local! {
    static HUD_UI: RefCell<HudUiState> = RefCell::new(HudUiState::default());
    static TOUCH_DRIVE: RefCell<TouchDriveState> = RefCell::new(TouchDriveState::default());
}

pub(super) fn button(parent: &Element, label: &str) -> Result<Element, JsValue> {
    let b = doc().create_element("button")?;
    b.set_attribute("type", "button")?;
    b.set_text_content(Some(label));
    parent.append_child(&b)?;
    Ok(b)
}
fn routes(state: &Shared, purpose: &str) -> Result<(), JsValue> {
    let order = {
        let mut w = state.borrow_mut();
        let places = w.graph.places.clone();
        let edges = w.graph.edges.clone();
        let result = eval(&mut w, "observer_path", &[places, edges, json!(purpose)])?;
        let ids: Vec<String> = serde_json::from_value(result).map_err(err)?;
        let expected: BTreeSet<String> = w
            .graph
            .places
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p["id"].as_str().map(String::from))
            .collect();
        if ids.len() != expected.len() || ids.iter().cloned().collect::<BTreeSet<_>>() != expected {
            return Err(err(
                "Vibe observer path must preserve every public place exactly once",
            ));
        }
        w.receipt["purpose"] = json!(purpose);
        w.receipt["path"] = json!(ids);
        ids
    };
    let list = doc()
        .get_element_by_id("native-place-list")
        .ok_or(err("No place controls"))?;
    list.set_text_content(None);
    for id in &order {
        let title = state
            .borrow()
            .graph
            .places
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"].as_str() == Some(id))
            .and_then(|p| p["title"].as_str())
            .unwrap_or(id)
            .to_string();
        let b = button(&list, &title)?;
        let id = id.clone();
        let state = state.clone();
        event(b.unchecked_ref(), "click", move |_| {
            if let Err(e) = place(&mut state.borrow_mut(), &id) {
                show_error(e)
            }
        })?;
    }
    Ok(())
}
pub(super) fn set_purpose(state: &Shared, purpose: &str) -> Result<(), JsValue> {
    routes(state, purpose)
}

pub(super) fn install_controls(state: &Shared) -> Result<(), JsValue> {
    if state.borrow().receipt["presentation_settings"]["scene_mode"] == "reference-model" {
        return reference::install(state);
    }
    let ui = element("native-world-ui", "aside")?;
    ui.set_attribute("aria-label", "Explore the living world")?;
    ui.set_class_name("native-panel");
    ui.set_attribute("hidden", "")?;
    let h = doc().create_element("h1")?;
    h.set_text_content(Some("A world we share"));
    ui.append_child(&h)?;
    let label = doc().create_element("label")?;
    label.set_text_content(Some("Choose a path "));
    let select = doc().create_element("select")?;
    select.set_id("native-purpose");
    for (value, name) in [
        ("learn", "Understand the ecosystem"),
        ("research", "Explore the evidence"),
        ("create", "Make something"),
        ("participate", "Take part"),
    ] {
        let o = doc().create_element("option")?;
        o.set_attribute("value", value)?;
        o.set_text_content(Some(name));
        select.append_child(&o)?;
    }
    label.append_child(&select)?;
    ui.append_child(&label)?;
    let s = state.clone();
    event(select.unchecked_ref(), "change", move |e| {
        if let Some(select) = e
            .target()
            .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
        {
            if let Err(e) = routes(&s, &select.value()) {
                show_error(e)
            }
        }
    })?;
    let list = doc().create_element("nav")?;
    list.set_id("native-place-list");
    list.set_class_name("native-controls");
    list.set_attribute("aria-label", "Places to explore")?;
    ui.append_child(&list)?;
    let title = doc().create_element("h2")?;
    title.set_id("native-place-title");
    ui.append_child(&title)?;
    let about = doc().create_element("details")?;
    about.set_id("native-place-details");
    let summary = doc().create_element("summary")?;
    summary.set_text_content(Some("About this place"));
    about.append_child(&summary)?;
    let description = doc().create_element("p")?;
    description.set_id("native-place-description");
    about.append_child(&description)?;
    ui.append_child(&about)?;
    let reading = doc().create_element("a")?;
    reading.set_id("native-reading");
    reading.set_class_name("native-reading");
    reading.set_attribute("href", "/publication/reading.html")?;
    reading.set_text_content(Some("Read about this place"));
    ui.append_child(&reading)?;
    download::install(&ui)?;
    let camp_actions: Vec<(String, String, String)> = state
        .borrow()
        .macros
        .as_ref()
        .map(|live| {
            live.actions()
                .iter()
                .map(|a| (a.id.clone(), a.label.clone(), live.panel_label().to_string()))
                .collect()
        })
        .unwrap_or_default();
    if !camp_actions.is_empty() {
        let camp = doc().create_element("nav")?;
        camp.set_id("native-camp-controls");
        camp.set_class_name("native-controls");
        camp.set_attribute("aria-label", &camp_actions[0].2)?;
        for (id, label, _) in camp_actions {
            let b = button(&camp, &label)?;
            let s = state.clone();
            event(b.unchecked_ref(), "click", move |_| {
                if let Err(e) = super::macros::fire(&mut s.borrow_mut(), &id) {
                    show_error(e)
                }
            })?;
        }
        ui.append_child(&camp)?;
    }
    let camera_controls = doc().create_element("nav")?;
    camera_controls.set_id("native-camera-controls");
    camera_controls.set_class_name("native-camera-controls");
    camera_controls.set_attribute("aria-label", "Camera controls")?;
    for (label, glyph, dy) in [
        ("Move closer", "+", -200.),
        ("Move further away", "-", 200.),
    ] {
        let b = button(&camera_controls, glyph)?;
        b.set_attribute("aria-label", label)?;
        b.set_attribute("title", label)?;
        let s = state.clone();
        event(b.unchecked_ref(), "click", move |_| {
            if let Err(e) = camera_input(&mut s.borrow_mut(), "zoom", 0., dy) {
                show_error(e)
            }
        })?;
    }
    ui.append_child(&camera_controls)?;
    let motion = doc().create_element("label")?;
    let check = doc()
        .create_element("input")?
        .dyn_into::<HtmlInputElement>()?;
    check.set_type("checkbox");
    check.set_checked(state.borrow().reduced);
    motion.append_child(&check)?;
    let motion_text = doc().create_element("span")?;
    motion_text.set_text_content(Some(" Reduce motion"));
    motion.append_child(&motion_text)?;
    ui.append_child(&motion)?;
    let s = state.clone();
    event(check.unchecked_ref(), "change", move |e| {
        if let Some(input) = e
            .target()
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        {
            let mut w = s.borrow_mut();
            w.reduced = input.checked();
            let reduced = w.reduced;
            w.portal.set_animation_reduced_motion(reduced);
            w.camera = w.goal.clone();
            if let Err(e) = set_camera(&mut w) {
                show_error(e)
            }
        }
    })?;
    let help = doc().create_element("p")?;
    help.set_text_content(Some("Modes: 1 Observe, 2 Walk, 3 Fly, F Drive. WASD or Arrow keys move/drive; drag to look; V toggles 1st/3rd person; Shift sprints; Space hops, ascends, or handbrakes; C or Ctrl descends; E inspects; R toggles Rhizosphere X-Ray; H toggles Clean View; Escape returns to Observe."));
    about.append_child(&help)?;
    let journey = doc().create_element("details")?;
    let summary = doc().create_element("summary")?;
    summary.set_text_content(Some("Landscape chapters"));
    journey.append_child(&summary)?;
    let chapters = state.borrow().graph.chapters.clone();
    let selector = doc().create_element("select")?;
    selector.set_id("native-chapter");
    selector.set_attribute("aria-label", "Landscape chapter")?;
    let initial = doc().create_element("option")?;
    initial.set_attribute("value", "")?;
    initial.set_text_content(Some("Choose a landscape chapter"));
    selector.append_child(&initial)?;
    for c in chapters.as_array().unwrap() {
        let option = doc().create_element("option")?;
        option.set_attribute("value", c["id"].as_str().unwrap_or(""))?;
        option.set_text_content(c["title"].as_str());
        selector.append_child(&option)?;
    }
    journey.append_child(&selector)?;
    for (id, tag) in [
        ("native-chapter-title", "h3"),
        ("native-chapter-copy", "p"),
        ("native-chapter-months", "p"),
    ] {
        let item = doc().create_element(tag)?;
        item.set_id(id);
        journey.append_child(&item)?;
    }
    let s = state.clone();
    event(selector.unchecked_ref(), "change", move |e| {
        if let Some(select) = e
            .target()
            .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
        {
            if !select.value().is_empty() {
                if let Err(e) = apply_chapter(&mut s.borrow_mut(), &select.value()) {
                    show_error(e)
                }
            }
        }
    })?;
    let method = doc().create_element("p")?;
    method.set_text_content(Some(&state.borrow().graph.chapter_method));
    journey.append_child(&method)?;
    ui.append_child(&journey)?;
    routes(state, "learn")?;
    initialise_selection(state)?;
    let canvas = state.borrow().canvas.clone();
    let s = state.clone();
    event(canvas.unchecked_ref(), "keydown", move |e| {
        if super::play::is_vehicle_active() {
            return;
        }
        if let Ok(e) = e.dyn_into::<KeyboardEvent>() {
            if super::locomotion::keydown_mods(
                &mut s.borrow_mut(),
                &e.key(),
                e.shift_key(),
                e.ctrl_key(),
            ) {
                e.prevent_default();
                e.stop_immediate_propagation();
            }
        }
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "keyup", move |e| {
        if super::play::is_vehicle_active() {
            return;
        }
        if let Ok(e) = e.dyn_into::<KeyboardEvent>() {
            if super::locomotion::keyup(&mut s.borrow_mut(), &e.key()) {
                e.prevent_default();
                e.stop_immediate_propagation();
            }
        }
    })?;
    install_compass_scale()?;
    install_mode_bar(state)?;
    install_touch_joystick()?;
    install_vehicle_cluster(state)?;
    install_drive_touch()?;
    install_explore_hud(state)?;
    install_stats_drawer()?;
    install_debug_drawer()?;
    install_window_debug_api()?;
    install_hud_listeners(state)?;
    sync_mode_ui(
        super::locomotion::current_mode(),
        super::locomotion::rhizosphere_xray_active(),
    );
    sync_dynamic_hud(state);
    refresh_streaming_status_pill(&state.borrow());
    start_hud_sync_loop(state)?;
    Ok(())
}

fn is_driving_now() -> bool {
    super::locomotion::current_mode() == super::locomotion::MovementMode::DriveVehicle
        || read_vehicle_active_js()
}

fn set_clean_view(active: bool) {
    HUD_UI.with(|h| {
        h.borrow_mut().clean_view = active;
    });
    super::play::html_class(active, "clean-view");
    if active {
        set_active_top_drawer(None, false);
    }
    if let Some(btn) = doc().get_element_by_id("native-clean-view-toggle") {
        btn.set_text_content(Some(if active { "Show HUD" } else { "Clean View" }));
        let _ = btn.set_attribute("aria-pressed", if active { "true" } else { "false" });
        let _ = btn.set_attribute(
            "aria-label",
            if active {
                "Restore HUD chrome (key H)"
            } else {
                "Hide HUD chrome for unobstructed sightseeing (key H)"
            },
        );
    }
    text(
        "native-world-status",
        if active {
            "Clean View active: press H or Show HUD to restore controls."
        } else {
            "HUD controls restored."
        },
    );
    emit_custom_event("webizen:clean-view-toggle", &json!({ "clean_view": active }));
}

fn install_compass_scale() -> Result<(), JsValue> {
    let pill = element("native-compass-scale", "aside")?;
    pill.set_attribute("aria-label", "Heading compass, corridor zone, and OCS scale frame")?;
    pill.set_attribute("data-ocs-regime", "LocalEnu")?;
    pill.set_text_content(None);

    let heading = doc().create_element("span")?;
    heading.set_id("native-compass-heading");
    heading.set_class_name("compass-chip");
    heading.set_text_content(Some("N · 000°"));
    pill.append_child(&heading)?;

    let sep1 = doc().create_element("span")?;
    sep1.set_class_name("compass-sep");
    sep1.set_attribute("aria-hidden", "true")?;
    sep1.set_text_content(Some("·"));
    pill.append_child(&sep1)?;

    let zone = doc().create_element("span")?;
    zone.set_id("native-compass-zone");
    zone.set_class_name("compass-chip");
    zone.set_text_content(Some("Forest Campsite"));
    pill.append_child(&zone)?;

    let sep2 = doc().create_element("span")?;
    sep2.set_class_name("compass-sep");
    sep2.set_attribute("aria-hidden", "true")?;
    sep2.set_text_content(Some("·"));
    pill.append_child(&sep2)?;

    let ocs = doc().create_element("span")?;
    ocs.set_id("native-compass-ocs");
    ocs.set_class_name("compass-chip compass-ocs");
    ocs.set_text_content(Some("Local ENU · 1.7 m"));
    pill.append_child(&ocs)?;

    Ok(())
}

fn install_mode_bar(state: &Shared) -> Result<(), JsValue> {
    use super::locomotion::MovementMode;

    let bar = element("native-mode-bar", "nav")?;
    bar.set_attribute("role", "toolbar")?;
    bar.set_attribute("aria-label", "Exploration modes, camera perspective, and X-Ray lenses")?;
    bar.set_text_content(None);

    let modes = [
        (
            "native-mode-observe",
            MovementMode::ObserveOrbit,
            "Observe",
            "Observe orbit mode (key 1)",
            "Observe mode: orbit, chapters, and waypoints.",
        ),
        (
            "native-mode-walk",
            MovementMode::WalkExplore,
            "Walk",
            "Walk explore mode (key 2)",
            "Walk mode: WASD or thumbstick to walk, V 1st/3rd, Shift sprint, Space hop, E inspect, R Rhizosphere X-Ray.",
        ),
    ];
    for (id, mode, label, aria, status_copy) in modes {
        let b = button(&bar, label)?;
        b.set_id(id);
        b.set_class_name("native-pill");
        b.set_attribute("aria-label", aria)?;
        b.set_attribute(
            "aria-pressed",
            if mode == MovementMode::ObserveOrbit { "true" } else { "false" },
        )?;
        let s = state.clone();
        event(b.unchecked_ref(), "click", move |_| {
            let was_driving = is_driving_now();
            if was_driving {
                clear_drive_touch_inputs();
                emit_custom_event(
                    "webizen:drive-toggle",
                    &json!({ "active": false, "source": "mode-bar", "target_mode": mode.as_str() }),
                );
            }
            {
                let mut w = s.borrow_mut();
                if let Err(e) = super::locomotion::set_mode(&mut w, mode) {
                    show_error(e);
                    return;
                }
                text("native-world-status", status_copy);
            }
            sync_dynamic_hud(&s);
        })?;
    }

    let drive_btn = button(&bar, "Drive")?;
    drive_btn.set_id("native-mode-drive");
    drive_btn.set_class_name("native-pill");
    drive_btn.set_attribute("aria-pressed", "false")?;
    drive_btn.set_attribute("aria-label", "Drive VW Passat B7 and Jayco camper (key F)")?;
    let s = state.clone();
    event(drive_btn.unchecked_ref(), "click", move |_| {
        let next_active = !is_driving_now();
        if !next_active {
            clear_drive_touch_inputs();
        }
        emit_custom_event(
            "webizen:drive-toggle",
            &json!({ "active": next_active, "source": "mode-bar" }),
        );
        sync_dynamic_hud(&s);
    })?;

    let fly_btn = button(&bar, "Fly")?;
    fly_btn.set_id("native-mode-fly");
    fly_btn.set_class_name("native-pill");
    fly_btn.set_attribute("aria-pressed", "false")?;
    fly_btn.set_attribute("aria-label", "Fly glide mode (key 3)")?;
    let s = state.clone();
    event(fly_btn.unchecked_ref(), "click", move |_| {
        let was_driving = is_driving_now();
        if was_driving {
            clear_drive_touch_inputs();
            emit_custom_event(
                "webizen:drive-toggle",
                &json!({ "active": false, "source": "mode-bar", "target_mode": "fly" }),
            );
        }
        {
            let mut w = s.borrow_mut();
            if let Err(e) = super::locomotion::set_mode(&mut w, MovementMode::FlyGlide) {
                show_error(e);
                return;
            }
            text(
                "native-world-status",
                "Fly mode: Space ascend, C or Ctrl descend, climb from creek to orbit.",
            );
        }
        sync_dynamic_hud(&s);
    })?;

    let view_btn = button(&bar, "1st/3rd")?;
    view_btn.set_id("native-view-toggle");
    view_btn.set_class_name("native-pill");
    view_btn.set_attribute("aria-pressed", "false")?;
    view_btn.set_attribute("title", "Toggle 1st / 3rd person perspective (V)")?;
    view_btn.set_attribute("aria-label", "Toggle 1st or 3rd person camera perspective (key V)")?;
    let s = state.clone();
    event(view_btn.unchecked_ref(), "click", move |_| {
        let first_person = {
            let mut w = s.borrow_mut();
            !super::locomotion::toggle_perspective(&mut w)
        };
        emit_custom_event(
            "webizen:camera-perspective-toggle",
            &json!({
                "first_person": first_person,
                "perspective": if first_person { "first-person" } else { "third-person-chase" }
            }),
        );
        sync_dynamic_hud(&s);
    })?;

    let rhizo = button(&bar, "Rhizosphere")?;
    rhizo.set_id("native-rhizosphere-toggle");
    rhizo.set_class_name("native-pill");
    rhizo.set_attribute("aria-pressed", "false")?;
    rhizo.set_attribute("title", "Toggle Rhizosphere & Mycelium X-Ray Lens (R)")?;
    rhizo.set_attribute("aria-label", "Toggle Rhizosphere X-Ray Lens (key R)")?;
    let s = state.clone();
    event(rhizo.unchecked_ref(), "click", move |_| {
        let active = {
            let mut w = s.borrow_mut();
            super::locomotion::toggle_rhizosphere_xray(&mut w)
        };
        emit_custom_event("webizen:rhizosphere-toggle", &json!({ "active": active }));
        sync_dynamic_hud(&s);
    })?;

    let anatomy = button(&bar, "Anatomy")?;
    anatomy.set_id("native-anatomy-toggle");
    anatomy.set_class_name("native-pill");
    anatomy.set_attribute("aria-pressed", "false")?;
    anatomy.set_attribute("title", "Toggle Human Anatomy X-Ray Lens")?;
    anatomy.set_attribute("aria-label", "Toggle Human Anatomy X-Ray Lens")?;
    let s = state.clone();
    event(anatomy.unchecked_ref(), "click", move |_| {
        let active = {
            let mut w = s.borrow_mut();
            super::locomotion::toggle_anatomy_xray(&mut w)
        };
        super::play::html_class(active, "anatomy-xray");
        emit_custom_event("webizen:anatomy-xray-toggle", &json!({ "active": active }));
        sync_dynamic_hud(&s);
    })?;

    let inspect = button(&bar, "Inspect")?;
    inspect.set_id("native-inspect-action");
    inspect.set_class_name("native-pill");
    inspect.set_attribute("title", "Inspect nearest species or exhibit (E)")?;
    inspect.set_attribute("aria-label", "Inspect nearest species or exhibit (key E)")?;
    let s = state.clone();
    event(inspect.unchecked_ref(), "click", move |_| {
        if let Err(e) = super::play::interact_nearest(&mut s.borrow_mut()) {
            show_error(e);
        }
    })?;

    let fly_nav = element("native-fly-controls", "nav")?;
    fly_nav.set_attribute("aria-label", "Vertical flight controls")?;
    fly_nav.set_attribute("hidden", "")?;
    fly_nav.set_text_content(None);

    let up_btn = button(&fly_nav, "Ascend")?;
    up_btn.set_id("native-fly-up");
    up_btn.set_class_name("native-pill");
    up_btn.set_attribute("aria-label", "Ascend in flight")?;
    event(up_btn.unchecked_ref(), "pointerdown", move |e| {
        e.stop_propagation();
        super::locomotion::set_vertical_input(Some(true), None);
    })?;
    for ev_name in ["pointerup", "pointercancel", "pointerleave"] {
        event(up_btn.unchecked_ref(), ev_name, move |_| {
            super::locomotion::set_vertical_input(Some(false), None);
        })?;
    }

    let down_btn = button(&fly_nav, "Descend")?;
    down_btn.set_id("native-fly-down");
    down_btn.set_class_name("native-pill");
    down_btn.set_attribute("aria-label", "Descend in flight")?;
    event(down_btn.unchecked_ref(), "pointerdown", move |e| {
        e.stop_propagation();
        super::locomotion::set_vertical_input(None, Some(true));
    })?;
    for ev_name in ["pointerup", "pointercancel", "pointerleave"] {
        event(down_btn.unchecked_ref(), ev_name, move |_| {
            super::locomotion::set_vertical_input(None, Some(false));
        })?;
    }

    Ok(())
}

fn install_vehicle_cluster(state: &Shared) -> Result<(), JsValue> {
    let cluster = element("native-vehicle-cluster", "aside")?;
    cluster.set_attribute("role", "region")?;
    cluster.set_attribute("aria-label", "VW Passat B7 and Jayco camper instrument cluster")?;
    cluster.set_attribute("hidden", "")?;
    cluster.set_text_content(None);

    let row = doc().create_element("div")?;
    row.set_class_name("veh-telemetry-row");

    let speed_box = doc().create_element("div")?;
    speed_box.set_class_name("veh-metric veh-speed-box");
    let speed_val = doc().create_element("span")?;
    speed_val.set_id("native-veh-speed");
    speed_val.set_class_name("veh-speed-val");
    speed_val.set_text_content(Some("0"));
    let speed_unit = doc().create_element("span")?;
    speed_unit.set_class_name("veh-speed-unit");
    speed_unit.set_text_content(Some("km/h"));
    speed_box.append_child(&speed_val)?;
    speed_box.append_child(&speed_unit)?;
    row.append_child(&speed_box)?;

    let gear_box = doc().create_element("div")?;
    gear_box.set_class_name("veh-metric veh-gear-box");
    let gear_lbl = doc().create_element("span")?;
    gear_lbl.set_class_name("veh-label");
    gear_lbl.set_text_content(Some("GEAR"));
    let gear_val = doc().create_element("span")?;
    gear_val.set_id("native-veh-gear");
    gear_val.set_class_name("veh-gear-val");
    gear_val.set_text_content(Some("P"));
    gear_box.append_child(&gear_lbl)?;
    gear_box.append_child(&gear_val)?;
    row.append_child(&gear_box)?;

    let incline_box = doc().create_element("div")?;
    incline_box.set_class_name("veh-metric veh-incline-box");
    let inc_lbl = doc().create_element("span")?;
    inc_lbl.set_class_name("veh-label");
    inc_lbl.set_text_content(Some("PITCH / ROLL"));
    let inc_vals = doc().create_element("span")?;
    inc_vals.set_class_name("veh-incline-vals");
    let pitch_el = doc().create_element("span")?;
    pitch_el.set_id("native-veh-pitch");
    pitch_el.set_text_content(Some("+0.0°"));
    let slash = doc().create_element("span")?;
    slash.set_text_content(Some(" / "));
    let roll_el = doc().create_element("span")?;
    roll_el.set_id("native-veh-roll");
    roll_el.set_text_content(Some("+0.0°"));
    inc_vals.append_child(&pitch_el)?;
    inc_vals.append_child(&slash)?;
    inc_vals.append_child(&roll_el)?;
    incline_box.append_child(&inc_lbl)?;
    incline_box.append_child(&inc_vals)?;
    row.append_child(&incline_box)?;

    let traction_box = doc().create_element("div")?;
    traction_box.set_class_name("veh-metric veh-traction-box");
    let tr_lbl = doc().create_element("span")?;
    tr_lbl.set_class_name("veh-label");
    tr_lbl.set_text_content(Some("SURFACE"));
    let tr_val = doc().create_element("span")?;
    tr_val.set_id("native-veh-traction");
    tr_val.set_class_name("veh-traction-val");
    tr_val.set_attribute("data-surface", "Bitumen")?;
    tr_val.set_text_content(Some("Bitumen"));
    traction_box.append_child(&tr_lbl)?;
    traction_box.append_child(&tr_val)?;
    row.append_child(&traction_box)?;

    cluster.append_child(&row)?;

    let actions = doc().create_element("div")?;
    actions.set_class_name("veh-actions-row");
    actions.set_attribute("role", "group")?;
    actions.set_attribute("aria-label", "Vehicle and tow-hitch quick actions")?;

    let hitch_btn = button(&actions, "Hitch Camper")?;
    hitch_btn.set_id("native-veh-hitch");
    hitch_btn.set_class_name("native-pill veh-action-pill");
    hitch_btn.set_attribute("aria-pressed", "false")?;
    hitch_btn.set_attribute("aria-label", "Hitch or unhitch Jayco camper trailer")?;
    let s = state.clone();
    event(hitch_btn.unchecked_ref(), "click", move |_| {
        emit_custom_event("webizen:hitch-toggle", &json!({ "source": "vehicle-cluster" }));
        sync_dynamic_hud(&s);
    })?;

    let lights_btn = button(&actions, "Lights (L)")?;
    lights_btn.set_id("native-veh-lights");
    lights_btn.set_class_name("native-pill veh-action-pill");
    lights_btn.set_attribute("aria-pressed", "false")?;
    lights_btn.set_attribute("aria-label", "Toggle Passat headlights (key L)")?;
    let s = state.clone();
    event(lights_btn.unchecked_ref(), "click", move |_| {
        emit_custom_event(
            "webizen:headlights-toggle",
            &json!({ "source": "vehicle-cluster" }),
        );
        sync_dynamic_hud(&s);
    })?;

    let cam_btn = button(&actions, "Cam: Chase")?;
    cam_btn.set_id("native-veh-cam");
    cam_btn.set_class_name("native-pill veh-action-pill");
    cam_btn.set_attribute("aria-label", "Cycle driving camera: Chase, Bonnet, or Cockpit (key C)")?;
    let s = state.clone();
    event(cam_btn.unchecked_ref(), "click", move |_| {
        emit_custom_event(
            "webizen:drive-camera-toggle",
            &json!({ "source": "vehicle-cluster" }),
        );
        sync_dynamic_hud(&s);
    })?;

    let exit_btn = button(&actions, "Exit Car (F)")?;
    exit_btn.set_id("native-veh-exit");
    exit_btn.set_class_name("native-pill veh-action-pill veh-exit-pill");
    exit_btn.set_attribute("aria-label", "Exit VW Passat B7 (key F)")?;
    let s = state.clone();
    event(exit_btn.unchecked_ref(), "click", move |_| {
        clear_drive_touch_inputs();
        emit_custom_event(
            "webizen:drive-toggle",
            &json!({ "active": false, "source": "vehicle-cluster" }),
        );
        sync_dynamic_hud(&s);
    })?;

    cluster.append_child(&actions)?;
    Ok(())
}

fn emit_touch_pedal_state() {
    let (throttle, brake, steer, l, r, b, a) = TOUCH_DRIVE.with(|t| {
        let st = t.borrow();
        let steer = (if st.right { 1.0_f64 } else { 0.0_f64 })
            - (if st.left { 1.0_f64 } else { 0.0_f64 });
        let throttle = if st.accel { 1.0_f64 } else { 0.0_f64 };
        let brake = if st.brake { 1.0_f64 } else { 0.0_f64 };
        (throttle, brake, steer, st.left, st.right, st.brake, st.accel)
    });
    let d = doc();
    for (id, pressed) in [
        ("native-drive-steer-left", l),
        ("native-drive-steer-right", r),
        ("native-drive-brake", b),
        ("native-drive-accel", a),
    ] {
        if let Some(el) = d.get_element_by_id(id) {
            let _ = el.set_attribute("aria-pressed", if pressed { "true" } else { "false" });
        }
    }
    emit_custom_event(
        "webizen:pedal-input",
        &json!({ "throttle": throttle, "brake": brake, "steer": steer }),
    );
}

fn clear_drive_touch_inputs() {
    let had_any = TOUCH_DRIVE.with(|t| {
        let mut st = t.borrow_mut();
        let any = st.left || st.right || st.brake || st.accel;
        *st = TouchDriveState::default();
        any
    });
    if had_any {
        emit_touch_pedal_state();
    }
}

fn bind_drive_touch_button(btn: &Element, slot: usize) -> Result<(), JsValue> {
    let btn_down = btn.clone();
    event(btn.unchecked_ref(), "pointerdown", move |e| {
        e.prevent_default();
        e.stop_propagation();
        if let Ok(p) = e.dyn_into::<PointerEvent>() {
            let _ = btn_down.set_pointer_capture(p.pointer_id());
            TOUCH_DRIVE.with(|t| {
                let mut st = t.borrow_mut();
                st.pointers[slot] = Some(p.pointer_id());
                match slot {
                    0 => st.left = true,
                    1 => st.right = true,
                    2 => st.brake = true,
                    _ => st.accel = true,
                }
            });
            emit_touch_pedal_state();
        }
    })?;
    for ev_name in ["pointerup", "pointercancel", "lostpointercapture"] {
        let btn_up = btn.clone();
        event(btn.unchecked_ref(), ev_name, move |e| {
            e.stop_propagation();
            if let Ok(p) = e.dyn_into::<PointerEvent>() {
                let changed = TOUCH_DRIVE.with(|t| {
                    let mut st = t.borrow_mut();
                    if st.pointers[slot] == Some(p.pointer_id()) || st.pointers[slot].is_none() {
                        let _ = btn_up.release_pointer_capture(p.pointer_id());
                        st.pointers[slot] = None;
                        match slot {
                            0 => st.left = false,
                            1 => st.right = false,
                            2 => st.brake = false,
                            _ => st.accel = false,
                        }
                        true
                    } else {
                        false
                    }
                });
                if changed {
                    emit_touch_pedal_state();
                }
            }
        })?;
    }
    Ok(())
}

fn install_drive_touch() -> Result<(), JsValue> {
    let root = element("native-drive-touch", "div")?;
    root.set_attribute("role", "group")?;
    root.set_attribute("aria-label", "Touch driving steering and pedals")?;
    root.set_attribute("hidden", "")?;
    root.set_text_content(None);

    let steer_group = doc().create_element("div")?;
    steer_group.set_class_name("drive-touch-steer");
    let left_btn = button(&steer_group, "◀ Left")?;
    left_btn.set_id("native-drive-steer-left");
    left_btn.set_class_name("native-pill drive-touch-btn");
    left_btn.set_attribute("aria-pressed", "false")?;
    left_btn.set_attribute("aria-label", "Steer left")?;
    bind_drive_touch_button(&left_btn, 0)?;

    let right_btn = button(&steer_group, "Right ▶")?;
    right_btn.set_id("native-drive-steer-right");
    right_btn.set_class_name("native-pill drive-touch-btn");
    right_btn.set_attribute("aria-pressed", "false")?;
    right_btn.set_attribute("aria-label", "Steer right")?;
    bind_drive_touch_button(&right_btn, 1)?;
    root.append_child(&steer_group)?;

    let pedal_group = doc().create_element("div")?;
    pedal_group.set_class_name("drive-touch-pedals");
    let brake_btn = button(&pedal_group, "Brake")?;
    brake_btn.set_id("native-drive-brake");
    brake_btn.set_class_name("native-pill drive-touch-btn drive-pedal-brake");
    brake_btn.set_attribute("aria-pressed", "false")?;
    brake_btn.set_attribute("aria-label", "Brake or reverse")?;
    bind_drive_touch_button(&brake_btn, 2)?;

    let accel_btn = button(&pedal_group, "Accel")?;
    accel_btn.set_id("native-drive-accel");
    accel_btn.set_class_name("native-pill drive-touch-btn drive-pedal-accel");
    accel_btn.set_attribute("aria-pressed", "false")?;
    accel_btn.set_attribute("aria-label", "Accelerate")?;
    bind_drive_touch_button(&accel_btn, 3)?;
    root.append_child(&pedal_group)?;

    Ok(())
}

fn install_touch_joystick() -> Result<(), JsValue> {
    let joy = element("native-touch-joystick", "div")?;
    joy.set_attribute("role", "application")?;
    joy.set_attribute("aria-label", "Virtual movement thumbstick")?;
    joy.set_attribute("hidden", "")?;
    joy.set_text_content(None);
    let knob = doc().create_element("span")?;
    knob.set_id("native-touch-knob");
    knob.set_attribute("aria-hidden", "true")?;
    joy.append_child(&knob)?;

    let active_pointer: std::rc::Rc<std::cell::RefCell<Option<(i32, f64, f64)>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));

    let joy_down = joy.clone();
    let knob_down = knob.clone();
    let ptr_down = active_pointer.clone();
    event(joy.unchecked_ref(), "pointerdown", move |e| {
        e.stop_propagation();
        e.prevent_default();
        if let Ok(p) = e.dyn_into::<PointerEvent>() {
            let _ = joy_down.set_pointer_capture(p.pointer_id());
            let rect = joy_down.get_bounding_client_rect();
            let cx = rect.left() + rect.width() * 0.5;
            let cy = rect.top() + rect.height() * 0.5;
            *ptr_down.borrow_mut() = Some((p.pointer_id(), cx, cy));
            update_joystick_knob(&knob_down, p.client_x() as f64 - cx, p.client_y() as f64 - cy);
        }
    })?;

    let knob_move = knob.clone();
    let ptr_move = active_pointer.clone();
    event(joy.unchecked_ref(), "pointermove", move |e| {
        e.stop_propagation();
        if let Ok(p) = e.dyn_into::<PointerEvent>() {
            if let Some((id, cx, cy)) = *ptr_move.borrow() {
                if id == p.pointer_id() {
                    update_joystick_knob(
                        &knob_move,
                        p.client_x() as f64 - cx,
                        p.client_y() as f64 - cy,
                    );
                }
            }
        }
    })?;

    for ev_name in ["pointerup", "pointercancel"] {
        let joy_end = joy.clone();
        let knob_end = knob.clone();
        let ptr_end = active_pointer.clone();
        event(joy.unchecked_ref(), ev_name, move |e| {
            e.stop_propagation();
            if let Ok(p) = e.dyn_into::<PointerEvent>() {
                if let Some((id, _, _)) = *ptr_end.borrow() {
                    if id == p.pointer_id() {
                        let _ = joy_end.release_pointer_capture(id);
                        *ptr_end.borrow_mut() = None;
                        let _ = knob_end.set_attribute("style", "transform:translate(0px,0px)");
                        super::locomotion::clear_joystick_input();
                    }
                }
            }
        })?;
    }

    Ok(())
}

fn update_joystick_knob(knob: &Element, dx: f64, dy: f64) {
    let max_r = super::locomotion::JOYSTICK_MAX_RADIUS_PX;
    let len = (dx * dx + dy * dy).sqrt();
    let (kx, ky) = if len > max_r && len > 1e-6 {
        (dx * max_r / len, dy * max_r / len)
    } else {
        (dx, dy)
    };
    let _ = knob.set_attribute(
        "style",
        &format!("transform:translate({kx:.1}px,{ky:.1}px)"),
    );
    super::locomotion::set_joystick_input(dx, dy, max_r);
}

fn read_vehicle_active_js() -> bool {
    let veh = read_window_global("__webizenVehicleState");
    read_js_bool(&veh, "active")
        .or_else(|| read_js_bool(&veh, "driving"))
        .unwrap_or(false)
}

pub(super) fn sync_mode_ui(mode: super::locomotion::MovementMode, rhizosphere: bool) {
    use super::locomotion::MovementMode;
    use super::play::html_class;

    let is_driving = mode == MovementMode::DriveVehicle || read_vehicle_active_js();
    let anatomy = super::locomotion::anatomy_xray_active();
    let first_person = !super::locomotion::third_person_active();

    html_class(mode == MovementMode::ObserveOrbit && !is_driving, "mode-observe");
    html_class(mode == MovementMode::WalkExplore && !is_driving, "mode-walk");
    html_class(mode == MovementMode::FlyGlide && !is_driving, "mode-fly");
    html_class(is_driving, "drive-mode");
    html_class(rhizosphere, "rhizosphere-xray");
    html_class(anatomy, "anatomy-xray");

    let d = doc();
    for (id, active) in [
        ("native-mode-observe", mode == MovementMode::ObserveOrbit && !is_driving),
        ("native-mode-walk", mode == MovementMode::WalkExplore && !is_driving),
        ("native-mode-drive", is_driving),
        ("native-mode-fly", mode == MovementMode::FlyGlide && !is_driving),
        ("native-view-toggle", first_person),
        ("native-rhizosphere-toggle", rhizosphere),
        ("native-anatomy-toggle", anatomy),
    ] {
        if let Some(el) = d.get_element_by_id(id) {
            let _ = el.set_attribute("aria-pressed", if active { "true" } else { "false" });
        }
    }
    if let Some(view_btn) = d.get_element_by_id("native-view-toggle") {
        let _ = view_btn.set_attribute(
            "aria-label",
            if first_person {
                "Camera view: 1st-person explorer (press V for 3rd-person)"
            } else {
                "Camera view: 3rd-person chase (press V for 1st-person)"
            },
        );
    }
    if let Some(joy) = d.get_element_by_id("native-touch-joystick") {
        if is_driving || mode == MovementMode::ObserveOrbit {
            let _ = joy.set_attribute("hidden", "");
            super::locomotion::clear_joystick_input();
        } else {
            let _ = joy.remove_attribute("hidden");
        }
    }
    if let Some(fly_nav) = d.get_element_by_id("native-fly-controls") {
        if !is_driving && mode == MovementMode::FlyGlide {
            let _ = fly_nav.remove_attribute("hidden");
        } else {
            let _ = fly_nav.set_attribute("hidden", "");
            super::locomotion::set_vertical_input(Some(false), Some(false));
        }
    }
    if let Some(cluster) = d.get_element_by_id("native-vehicle-cluster") {
        if is_driving {
            let _ = cluster.remove_attribute("hidden");
        } else {
            let _ = cluster.set_attribute("hidden", "");
        }
    }
    if let Some(drive_touch) = d.get_element_by_id("native-drive-touch") {
        if is_driving {
            let _ = drive_touch.remove_attribute("hidden");
        } else {
            let _ = drive_touch.set_attribute("hidden", "");
            clear_drive_touch_inputs();
        }
    }
}

fn apply_chapter(w: &mut World, id: &str) -> Result<(), JsValue> {
    let chapters = w.graph.chapters.clone();
    let chosen = eval(w, "chapter_settings", &[chapters, json!(id)])?;
    if chosen["id"].as_str() != Some(id) {
        return Err(err("Vibe chapter selection did not resolve"));
    }
    text(
        "native-chapter-title",
        chosen["title"].as_str().unwrap_or(""),
    );
    text(
        "native-chapter-copy",
        chosen["landscape_copy"].as_str().unwrap_or(""),
    );
    if let Some(months) = chosen["month_bands"].as_array() {
        text(
            "native-chapter-months",
            &months
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    if !chosen["sky_colour"].is_null() {
        let colour: [f64; 4] = serde_json::from_value(chosen["sky_colour"].clone()).map_err(err)?;
        if !colour
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        {
            return Err(err("Invalid chapter sky colour"));
        }
        w.portal
            .set_clear_color(colour[0], colour[1], colour[2], colour[3]);
    }
    if !chosen["sun"].is_null() {
        let sun: [f32; 5] = serde_json::from_value(chosen["sun"].clone()).map_err(err)?;
        if !sun.iter().all(|v| v.is_finite()) {
            return Err(err("Invalid chapter lighting"));
        }
        w.portal
            .set_lighting(sun[0], sun[1], sun[2], sun[3], sun[4]);
    }
    w.receipt["chapter"] = chosen;
    Ok(())
}

fn initialise_selection(state: &Shared) -> Result<(), JsValue> {
    let location = js_sys::Reflect::get(&web_sys::window().unwrap(), &"location".into())?;
    let hash = js_sys::Reflect::get(&location, &"hash".into())?
        .as_string()
        .unwrap_or_default();
    let requested = hash.trim_start_matches('#');
    let selected = {
        let w = state.borrow();
        if w.graph
            .places
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"].as_str() == Some(requested))
        {
            requested.to_string()
        } else {
            "field-studio".into()
        }
    };
    place(&mut state.borrow_mut(), &selected)?;
    let query = js_sys::Reflect::get(&location, &"search".into())?
        .as_string()
        .unwrap_or_default();
    if let Some(chapter) = query
        .trim_start_matches('?')
        .split('&')
        .filter_map(|v| v.split_once('='))
        .find_map(|(key, value)| if key == "chapter" { Some(value) } else { None })
    {
        let valid = state
            .borrow()
            .graph
            .chapters
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"].as_str() == Some(chapter));
        if valid {
            if let Some(selector) = doc()
                .get_element_by_id("native-chapter")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
            {
                selector.set_value(chapter);
            }
            apply_chapter(&mut state.borrow_mut(), chapter)?;
        }
    }
    Ok(())
}

fn is_top_drawer_open(drawer: TopDrawer) -> bool {
    let id = match drawer {
        TopDrawer::Tour => "native-tour-drawer",
        TopDrawer::FieldGuide => "native-field-guide-drawer",
        TopDrawer::Stats => "native-stats-drawer",
        TopDrawer::Debug => "native-debug-drawer",
        TopDrawer::WorldPanel => "native-world-ui",
    };
    doc()
        .get_element_by_id(id)
        .map(|el| !el.has_attribute("hidden"))
        .unwrap_or(false)
}

fn active_top_drawer() -> Option<TopDrawer> {
    for drawer in [
        TopDrawer::FieldGuide,
        TopDrawer::Stats,
        TopDrawer::Debug,
        TopDrawer::Tour,
        TopDrawer::WorldPanel,
    ] {
        if is_top_drawer_open(drawer) {
            return Some(drawer);
        }
    }
    None
}

fn set_active_top_drawer(target: Option<TopDrawer>, restore_focus: bool) {
    let d = doc();
    let was_fg_open = is_top_drawer_open(TopDrawer::FieldGuide);
    let was_stats_open = is_top_drawer_open(TopDrawer::Stats);
    let was_debug_open = is_top_drawer_open(TopDrawer::Debug);
    let fg_open = target == Some(TopDrawer::FieldGuide);
    let tour_open = target == Some(TopDrawer::Tour);
    let stats_open = target == Some(TopDrawer::Stats);
    let debug_open = target == Some(TopDrawer::Debug);
    let panel_open = target == Some(TopDrawer::WorldPanel);

    if target.is_some() {
        if let Some(lab) = d.get_element_by_id("native-proof-lab") {
            let _ = lab.remove_attribute("open");
        }
    }

    if let Some(tour_drawer) = d.get_element_by_id("native-tour-drawer") {
        let _ = if tour_open {
            tour_drawer.remove_attribute("hidden")
        } else {
            tour_drawer.set_attribute("hidden", "")
        };
    }
    if let Some(tour_btn) = d.get_element_by_id("native-tour-toggle") {
        let _ = tour_btn.set_attribute("aria-expanded", if tour_open { "true" } else { "false" });
    }

    if let Some(fg_drawer) = d.get_element_by_id("native-field-guide-drawer") {
        let _ = if fg_open {
            fg_drawer.remove_attribute("hidden")
        } else {
            fg_drawer.set_attribute("hidden", "")
        };
    }
    if let Some(fg_btn) = d.get_element_by_id("native-field-guide-toggle") {
        let _ = fg_btn.set_attribute("aria-expanded", if fg_open { "true" } else { "false" });
        if was_fg_open && !fg_open && restore_focus {
            if let Ok(el) = fg_btn.dyn_into::<web_sys::HtmlElement>() {
                let _ = el.focus();
            }
        }
    }
    if fg_open {
        super::instruments::species::sync_field_guide_ui();
        if let Some(close_btn) = d
            .get_element_by_id("native-field-guide-close")
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = close_btn.focus();
        }
    }
    if was_fg_open != fg_open {
        emit_custom_event("webizen:field-guide-toggle", &json!({ "open": fg_open }));
    }

    if let Some(stats_drawer) = d.get_element_by_id("native-stats-drawer") {
        let _ = if stats_open {
            stats_drawer.remove_attribute("hidden")
        } else {
            stats_drawer.set_attribute("hidden", "")
        };
    }
    if let Some(stats_btn) = d.get_element_by_id("native-stats-toggle") {
        let _ = stats_btn.set_attribute("aria-expanded", if stats_open { "true" } else { "false" });
        if was_stats_open && !stats_open && restore_focus {
            if let Ok(el) = stats_btn.dyn_into::<web_sys::HtmlElement>() {
                let _ = el.focus();
            }
        }
    }
    if stats_open {
        sync_stats_drawer();
    }

    if let Some(debug_drawer) = d.get_element_by_id("native-debug-drawer") {
        let _ = if debug_open {
            debug_drawer.remove_attribute("hidden")
        } else {
            debug_drawer.set_attribute("hidden", "")
        };
    }
    if let Some(debug_btn) = d.get_element_by_id("native-debug-toggle") {
        let _ = debug_btn.set_attribute("aria-expanded", if debug_open { "true" } else { "false" });
        if was_debug_open && !debug_open && restore_focus {
            if let Ok(el) = debug_btn.dyn_into::<web_sys::HtmlElement>() {
                let _ = el.focus();
            }
        }
    }
    if debug_open && !was_debug_open {
        let _ = log_diagnostics_to_console();
    }

    if let Some(panel) = d.get_element_by_id("native-world-ui") {
        let _ = if panel_open {
            panel.remove_attribute("hidden")
        } else {
            panel.set_attribute("hidden", "")
        };
    }
    if let Some(panel_btn) = d.get_element_by_id("native-panel-toggle") {
        panel_btn.set_text_content(Some(if panel_open {
            "Hide panel"
        } else {
            "Show panel"
        }));
        let _ = panel_btn.set_attribute("aria-expanded", if panel_open { "true" } else { "false" });
        let _ = panel_btn.set_attribute(
            "aria-label",
            if panel_open {
                "Hide the reading and path panel"
            } else {
                "Show the reading and path panel"
            },
        );
    }

    let active_name = match target {
        Some(TopDrawer::Tour) => "tour",
        Some(TopDrawer::FieldGuide) => "field-guide",
        Some(TopDrawer::Stats) => "stats",
        Some(TopDrawer::Debug) => "debug",
        Some(TopDrawer::WorldPanel) => "world-panel",
        None => "none",
    };
    emit_custom_event("webizen:drawer-change", &json!({ "active": active_name }));
}

fn toggle_top_drawer(drawer: TopDrawer) -> bool {
    let currently_open = is_top_drawer_open(drawer);
    let next_open = !currently_open;
    set_active_top_drawer(if next_open { Some(drawer) } else { None }, currently_open);
    next_open
}

pub(super) fn install_panel_toggle() -> Result<(), JsValue> {
    let button = element("native-panel-toggle", "button")?;
    button.set_attribute("type", "button")?;
    button.set_attribute("aria-controls", "native-world-ui")?;
    button.set_attribute("aria-expanded", "false")?;
    button.set_attribute("aria-label", "Show the reading and path panel")?;
    button.set_text_content(Some("Show panel"));
    if let Some(panel) = doc().get_element_by_id("native-world-ui") {
        panel.set_attribute("hidden", "")?;
    }
    event(button.unchecked_ref(), "click", move |_| {
        toggle_top_drawer(TopDrawer::WorldPanel);
    })
}

pub(crate) fn emit_custom_event(name: &str, detail: &Value) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(detail_js) = js_sys::JSON::parse(&detail.to_string()) else {
        return;
    };
    let Ok(ctor_val) = js_sys::Reflect::get(&window, &"CustomEvent".into()) else {
        return;
    };
    let Ok(ctor) = ctor_val.dyn_into::<js_sys::Function>() else {
        return;
    };
    let init = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&init, &"detail".into(), &detail_js);
    let _ = js_sys::Reflect::set(&init, &"bubbles".into(), &JsValue::TRUE);
    let _ = js_sys::Reflect::set(&init, &"composed".into(), &JsValue::TRUE);
    let args = js_sys::Array::of2(&JsValue::from_str(name), &init);
    if let Ok(ev_val) = js_sys::Reflect::construct(&ctor, &args) {
        if let Ok(ev) = ev_val.dyn_into::<web_sys::Event>() {
            let _ = doc().dispatch_event(&ev);
        }
    }
}

pub(crate) fn apply_atmosphere_spec(
    w: &mut World,
    spec: super::instruments::species::AtmosphereLightingSpec,
) {
    w.portal.set_sky_preset(spec.sky_preset);
    w.portal.set_clear_color(
        spec.clear_color[0],
        spec.clear_color[1],
        spec.clear_color[2],
        spec.clear_color[3],
    );
    w.portal
        .set_lighting(spec.sun[0], spec.sun[1], spec.sun[2], spec.sun[3], spec.sun[4]);
    w.portal.set_exposure_compensation(spec.exposure_ev);
    super::play::html_class(spec.starfield_active, "orbital-starfield");
    w.receipt["time_of_day"] = json!({
        "id": spec.id,
        "label": spec.label,
        "sky_preset": spec.sky_preset,
        "starfield_active": spec.starfield_active,
        "sun": spec.sun,
        "clear_color": spec.clear_color,
    });
}

fn read_window_global(key: &str) -> JsValue {
    web_sys::window()
        .and_then(|win| js_sys::Reflect::get(&win, &key.into()).ok())
        .unwrap_or(JsValue::UNDEFINED)
}

fn read_js_string(obj: &JsValue, key: &str) -> Option<String> {
    if obj.is_null() || obj.is_undefined() || !obj.is_object() {
        return None;
    }
    js_sys::Reflect::get(obj, &key.into())
        .ok()
        .and_then(|v| v.as_string())
        .filter(|s| !s.trim().is_empty())
}

fn read_js_bool(obj: &JsValue, key: &str) -> Option<bool> {
    if obj.is_null() || obj.is_undefined() || !obj.is_object() {
        return None;
    }
    js_sys::Reflect::get(obj, &key.into())
        .ok()
        .and_then(|v| v.as_bool())
}

fn read_js_usize(obj: &JsValue, key: &str) -> Option<usize> {
    if obj.is_null() || obj.is_undefined() || !obj.is_object() {
        return None;
    }
    js_sys::Reflect::get(obj, &key.into())
        .ok()
        .and_then(|v| v.as_f64())
        .filter(|n| n.is_finite() && *n >= 0.0)
        .map(|n| n as usize)
}

fn read_js_f64(obj: &JsValue, key: &str) -> Option<f64> {
    if obj.is_null() || obj.is_undefined() || !obj.is_object() {
        return None;
    }
    js_sys::Reflect::get(obj, &key.into())
        .ok()
        .and_then(|v| v.as_f64())
        .filter(|n| n.is_finite())
}

pub(crate) fn refresh_streaming_status_pill(w: &World) {
    let Some(el) = doc().get_element_by_id("native-stream-status") else {
        return;
    };
    let window = web_sys::window();
    let init_obj = window
        .as_ref()
        .and_then(|win| js_sys::Reflect::get(win, &"__webizenNativeInit".into()).ok())
        .unwrap_or(JsValue::UNDEFINED);
    let ewm_obj = window
        .as_ref()
        .and_then(|win| js_sys::Reflect::get(win, &"__webizenEwmState".into()).ok())
        .unwrap_or(JsValue::UNDEFINED);

    let backend = read_js_string(&init_obj, "selected_backend")
        .or_else(|| w.receipt.get("renderer").and_then(Value::as_str).map(str::to_string));
    let tier = read_js_string(&init_obj, "device_tier")
        .or_else(|| w.receipt.get("device_tier").and_then(Value::as_str).map(str::to_string));
    let is_mobile = read_js_bool(&init_obj, "is_mobile")
        .or_else(|| w.receipt.get("is_mobile").and_then(Value::as_bool))
        .unwrap_or(false);

    let loaded_species = super::instruments::species::species_count_loaded();
    let ewm_species = read_js_usize(&ewm_obj, "species_count")
        .or_else(|| {
            w.receipt
                .get("catchment_species_count")
                .and_then(Value::as_u64)
                .map(|n| n as usize)
        })
        .unwrap_or(0);
    let species_count = loaded_species.max(ewm_species);

    let ewm_phase = read_js_string(&ewm_obj, "phase")
        .or_else(|| read_js_string(&ewm_obj, "status"))
        .or_else(|| read_js_string(&ewm_obj, "state"));

    let label = super::instruments::species::format_streaming_status_pill(
        backend.as_deref(),
        tier.as_deref(),
        is_mobile,
        species_count,
        ewm_phase.as_deref(),
    );
    el.set_text_content(Some(&label));
}

fn current_altitude_m(w: &World) -> f64 {
    if super::locomotion::current_mode() == super::locomotion::MovementMode::ObserveOrbit {
        let ground =
            super::locomotion::terrain_datum_at(w.camera.target[0] as f64, w.camera.target[2] as f64);
        return (w.camera.target[1] as f64 - ground).max(0.0);
    }
    w.receipt
        .get("locomotion")
        .and_then(|l| l.get("altitude_m"))
        .and_then(Value::as_f64)
        .unwrap_or_else(|| (w.camera.target[1] as f64).max(0.0))
}

fn apply_time_of_day(w: &mut World, preset: super::instruments::species::TimeOfDayPreset) {
    use super::instruments::species::{
        resolve_atmosphere_for_altitude, set_current_time_of_day, TimeOfDayPreset,
    };

    set_current_time_of_day(preset);
    let alt_m = current_altitude_m(w);
    let spec = resolve_atmosphere_for_altitude(preset, alt_m);
    apply_atmosphere_spec(w, spec);

    for p in TimeOfDayPreset::all() {
        let btn_id = format!("native-time-{}", p.as_str());
        if let Some(btn) = doc().get_element_by_id(&btn_id) {
            let _ = btn.set_attribute("aria-pressed", if *p == preset { "true" } else { "false" });
        }
    }

    text("native-world-status", spec.description_en_au);
    emit_custom_event(
        "webizen:time-of-day",
        &json!({
            "id": spec.id,
            "label": spec.label,
            "sky_preset": spec.sky_preset,
            "sun": spec.sun,
            "clear_color": spec.clear_color,
            "starfield": spec.starfield_active,
        }),
    );
}

fn apply_tour_stop(
    w: &mut World,
    stop: &super::instruments::species::EcologicalTourStop,
) -> Result<(), JsValue> {
    use super::instruments::species::{
        current_time_of_day, mark_species_discovered, resolve_atmosphere_for_altitude,
        ECOLOGICAL_TOUR_STOPS,
    };
    use super::locomotion::{set_mode, set_rhizosphere_xray, MovementMode};

    let mode = MovementMode::from_str(stop.movement_mode).unwrap_or(MovementMode::ObserveOrbit);
    let _ = set_mode(w, mode)?;

    w.goal.target = stop.target;
    w.goal.yaw = stop.yaw;
    w.goal.pitch = stop.pitch;
    w.goal.zoom = stop.zoom;
    if w.reduced {
        w.camera = w.goal.clone();
        set_camera(w)?;
    }

    let _ = set_rhizosphere_xray(w, stop.activate_rhizosphere);
    if let Some(slug) = stop.focus_species_slug {
        mark_species_discovered(slug);
    }

    let alt_m = stop.altitude_m.unwrap_or_else(|| {
        let ground = super::locomotion::terrain_datum_at(
            stop.target[0] as f64,
            stop.target[2] as f64,
        );
        (stop.target[1] as f64 - ground).max(0.0)
    });
    let atmo = resolve_atmosphere_for_altitude(current_time_of_day(), alt_m);
    apply_atmosphere_spec(w, atmo);

    let ocs_tree = super::ocs_frames::OcsFrameTree::default_webizen_cosmos();
    let earth_node = ocs_tree.get("did:q42:cosmos:earth");
    let sun_node = ocs_tree.get("did:q42:cosmos:sun");
    w.receipt["tour_stop"] = json!({
        "id": stop.id,
        "title": stop.title,
        "zone_label": stop.zone_label,
        "movement_mode": stop.movement_mode,
        "rhizosphere_xray": stop.activate_rhizosphere,
        "ocs_regime": stop.ocs_regime,
        "altitude_m": alt_m,
        "focus_species": stop.focus_species_slug,
    });
    w.receipt["ocs_frame_context"] = json!({
        "epoch_tdb": ocs_tree.epoch_tdb,
        "active_regime": stop.ocs_regime,
        "earth_id": earth_node.map(|n| n.id.as_str()),
        "sun_id": sun_node.map(|n| n.id.as_str()),
    });

    for candidate in ECOLOGICAL_TOUR_STOPS {
        let btn_id = format!("native-tour-stop-{}", candidate.id);
        if let Some(btn) = doc().get_element_by_id(&btn_id) {
            let _ = btn.set_attribute(
                "aria-pressed",
                if candidate.id == stop.id { "true" } else { "false" },
            );
        }
    }

    text("native-world-status", stop.summary_en_au);
    emit_custom_event(
        "webizen:tour-stop",
        &json!({
            "id": stop.id,
            "title": stop.title,
            "ocs_regime": stop.ocs_regime,
            "movement_mode": stop.movement_mode,
            "rhizosphere_xray": stop.activate_rhizosphere,
            "target": stop.target,
            "altitude_m": alt_m,
            "focus_species": stop.focus_species_slug,
        }),
    );
    Ok(())
}

fn set_field_guide_open(open: bool, restore_focus: bool) {
    set_active_top_drawer(
        if open {
            Some(TopDrawer::FieldGuide)
        } else {
            None
        },
        restore_focus,
    );
}

fn install_explore_hud(state: &Shared) -> Result<(), JsValue> {
    use super::instruments::species::{
        discovery_badge_text, is_audio_opt_in_granted, set_bush_soundscape,
        set_field_guide_filter, sync_field_guide_ui, TimeOfDayPreset, ECOLOGICAL_TOUR_STOPS,
    };

    let status_pill = element("native-stream-status", "div")?;
    status_pill.set_attribute("role", "status")?;
    status_pill.set_attribute("aria-live", "polite")?;
    status_pill.set_attribute("aria-label", "Graphics tier and progressive streaming status")?;

    let dock = element("native-explore-dock", "nav")?;
    dock.set_attribute(
        "aria-label",
        "Places, field guide, soundscape, technical stats, diagnostics, and clean view",
    )?;
    dock.set_text_content(None);

    let tour_toggle = button(&dock, "Places")?;
    tour_toggle.set_id("native-tour-toggle");
    tour_toggle.set_class_name("native-pill");
    tour_toggle.set_attribute("aria-controls", "native-tour-drawer")?;
    tour_toggle.set_attribute("aria-expanded", "false")?;
    tour_toggle.set_attribute("title", "Guided ecological tour stops & time of day")?;
    tour_toggle.set_attribute("aria-label", "Toggle guided ecological tour stops and lighting")?;

    let fg_toggle = button(&dock, &discovery_badge_text())?;
    fg_toggle.set_id("native-field-guide-toggle");
    fg_toggle.set_class_name("native-pill");
    fg_toggle.set_attribute("aria-controls", "native-field-guide-drawer")?;
    fg_toggle.set_attribute("aria-expanded", "false")?;
    fg_toggle.set_attribute("title", "SE Australian Species Field Guide")?;
    fg_toggle.set_attribute("aria-label", "Open SE Australian species field guide")?;

    let sound_btn = button(
        &dock,
        if is_audio_opt_in_granted() {
            "Sound: On"
        } else {
            "Sound"
        },
    )?;
    sound_btn.set_id("native-sound-toggle");
    sound_btn.set_class_name("native-pill");
    sound_btn.set_attribute(
        "aria-pressed",
        if is_audio_opt_in_granted() { "true" } else { "false" },
    )?;
    sound_btn.set_attribute("title", "Toggle bush soundscape & spatial .qac fauna calls")?;
    sound_btn.set_attribute(
        "aria-label",
        "Toggle opt-in bush soundscape and spatial fauna vocalisations (muted by default)",
    )?;

    let stats_toggle = button(&dock, "Stats")?;
    stats_toggle.set_id("native-stats-toggle");
    stats_toggle.set_class_name("native-pill");
    stats_toggle.set_attribute("aria-controls", "native-stats-drawer")?;
    stats_toggle.set_attribute("aria-expanded", "false")?;
    stats_toggle.set_attribute("title", "Technical Statistics HUD (F3 or `)")?;
    stats_toggle.set_attribute("aria-label", "Toggle live technical statistics panel (F3 or backtick)")?;
    event(stats_toggle.unchecked_ref(), "click", move |_| {
        toggle_top_drawer(TopDrawer::Stats);
    })?;

    let debug_toggle = button(&dock, "Debug")?;
    debug_toggle.set_id("native-debug-toggle");
    debug_toggle.set_class_name("native-pill");
    debug_toggle.set_attribute("aria-controls", "native-debug-drawer")?;
    debug_toggle.set_attribute("aria-expanded", "false")?;
    debug_toggle.set_attribute("title", "Diagnostics drawer & console dump")?;
    debug_toggle.set_attribute("aria-label", "Open diagnostics drawer and log structured report to console")?;
    event(debug_toggle.unchecked_ref(), "click", move |_| {
        toggle_top_drawer(TopDrawer::Debug);
    })?;

    let clean_btn = button(&dock, "Clean View")?;
    clean_btn.set_id("native-clean-view-toggle");
    clean_btn.set_class_name("native-pill");
    clean_btn.set_attribute("aria-pressed", "false")?;
    clean_btn.set_attribute("title", "Toggle Clean View (H)")?;
    clean_btn.set_attribute(
        "aria-label",
        "Hide HUD chrome for unobstructed sightseeing (key H)",
    )?;
    event(clean_btn.unchecked_ref(), "click", move |_| {
        let next = HUD_UI.with(|h| !h.borrow().clean_view);
        set_clean_view(next);
    })?;

    let tour_drawer = element("native-tour-drawer", "nav")?;
    tour_drawer.set_attribute("aria-label", "Guided ecological tour stops and lighting")?;
    tour_drawer.set_attribute("hidden", "")?;
    tour_drawer.set_text_content(None);

    let time_bar = doc().create_element("div")?;
    time_bar.set_id("native-time-bar");
    time_bar.set_attribute("role", "group")?;
    time_bar.set_attribute("aria-label", "SE Australian time of day lighting")?;
    for preset in TimeOfDayPreset::all() {
        let btn = button(&time_bar, preset.label())?;
        btn.set_id(&format!("native-time-{}", preset.as_str()));
        btn.set_class_name("native-pill native-time-pill");
        let active = *preset == TimeOfDayPreset::Midday;
        btn.set_attribute("aria-pressed", if active { "true" } else { "false" })?;
        btn.set_attribute(
            "aria-label",
            &format!("Set lighting to {} in SE Australia", preset.label()),
        )?;
        let s = state.clone();
        let chosen = *preset;
        event(btn.unchecked_ref(), "click", move |_| {
            apply_time_of_day(&mut s.borrow_mut(), chosen);
        })?;
    }
    tour_drawer.append_child(&time_bar)?;

    let stops_grid = doc().create_element("div")?;
    stops_grid.set_class_name("tour-stops-grid");
    for stop in ECOLOGICAL_TOUR_STOPS {
        let btn = doc().create_element("button")?;
        btn.set_attribute("type", "button")?;
        btn.set_id(&format!("native-tour-stop-{}", stop.id));
        btn.set_class_name("native-pill native-tour-stop");
        btn.set_attribute("aria-pressed", "false")?;
        btn.set_attribute(
            "aria-label",
            &format!("Go to {} ({})", stop.title, stop.zone_label),
        )?;
        let title_span = doc().create_element("span")?;
        title_span.set_class_name("tour-stop-title");
        title_span.set_text_content(Some(stop.title));
        let zone_small = doc().create_element("small")?;
        zone_small.set_class_name("tour-stop-zone");
        zone_small.set_text_content(Some(stop.zone_label));
        btn.append_child(&title_span)?;
        btn.append_child(&zone_small)?;
        stops_grid.append_child(&btn)?;

        let s = state.clone();
        event(btn.unchecked_ref(), "click", move |_| {
            if let Err(e) = apply_tour_stop(&mut s.borrow_mut(), stop) {
                show_error(e);
            }
        })?;
    }
    tour_drawer.append_child(&stops_grid)?;

    event(tour_toggle.unchecked_ref(), "click", move |_| {
        toggle_top_drawer(TopDrawer::Tour);
    })?;

    let s = state.clone();
    event(sound_btn.unchecked_ref(), "click", move |_| {
        let next = !is_audio_opt_in_granted();
        let reduced = s.borrow().reduced;
        let enabled = set_bush_soundscape(next, reduced);
        if let Some(btn) = doc().get_element_by_id("native-sound-toggle") {
            btn.set_text_content(Some(if enabled { "Sound: On" } else { "Sound" }));
            let _ = btn.set_attribute("aria-pressed", if enabled { "true" } else { "false" });
        }
        s.borrow_mut().receipt["audio_soundscape"] = json!({
            "enabled": enabled,
            "muted": !enabled,
            "reduced_motion": reduced,
        });
        text(
            "native-world-status",
            if enabled {
                "Bush soundscape enabled: gentle creek, canopy wind, and spatial .qac fauna calls active."
            } else {
                "Bush soundscape muted."
            },
        );
        emit_custom_event(
            "webizen:audio-toggle",
            &json!({
                "enabled": enabled,
                "muted": !enabled,
                "opt_in_granted": enabled,
                "reduced_motion": reduced,
            }),
        );
    })?;

    // Expandable Species Field Guide Drawer
    let drawer = element("native-field-guide-drawer", "aside")?;
    drawer.set_attribute("role", "dialog")?;
    drawer.set_attribute("aria-modal", "false")?;
    drawer.set_attribute("aria-label", "SE Australian Species Field Guide")?;
    drawer.set_attribute("hidden", "")?;
    drawer.set_text_content(None);

    let header = doc().create_element("div")?;
    header.set_class_name("fg-drawer-header");
    let head_title = doc().create_element("div")?;
    let h2 = doc().create_element("h2")?;
    h2.set_text_content(Some("SE Australia Field Guide"));
    let count = doc().create_element("p")?;
    count.set_id("native-field-guide-count");
    count.set_text_content(Some(&discovery_badge_text()));
    head_title.append_child(&h2)?;
    head_title.append_child(&count)?;
    header.append_child(&head_title)?;

    let close_btn = button(&header, "Close")?;
    close_btn.set_id("native-field-guide-close");
    close_btn.set_class_name("native-pill");
    close_btn.set_attribute("aria-label", "Close species field guide (Escape)")?;
    drawer.append_child(&header)?;

    let filters = doc().create_element("div")?;
    filters.set_id("native-fg-filters");
    filters.set_attribute("role", "group")?;
    filters.set_attribute("aria-label", "Filter species by kingdom")?;
    for (id, label, filter_val, active) in [
        ("native-fg-filter-all", "All (17)", None, true),
        ("native-fg-filter-flora", "Flora (11)", Some("flora"), false),
        ("native-fg-filter-fauna", "Fauna (6)", Some("fauna"), false),
    ] {
        let fb = button(&filters, label)?;
        fb.set_id(id);
        fb.set_class_name("native-pill");
        fb.set_attribute("aria-pressed", if active { "true" } else { "false" })?;
        event(fb.unchecked_ref(), "click", move |_| {
            set_field_guide_filter(filter_val);
        })?;
    }
    drawer.append_child(&filters)?;

    let list = doc().create_element("div")?;
    list.set_id("native-field-guide-list");
    drawer.append_child(&list)?;
    sync_field_guide_ui();

    event(fg_toggle.unchecked_ref(), "click", move |_| {
        let is_open = doc()
            .get_element_by_id("native-field-guide-drawer")
            .map(|el| !el.has_attribute("hidden"))
            .unwrap_or(false);
        set_field_guide_open(!is_open, is_open);
    })?;

    event(close_btn.unchecked_ref(), "click", move |_| {
        set_field_guide_open(false, true);
    })?;

    let s = state.clone();
    event(list.unchecked_ref(), "click", move |e| {
        let Some(target) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let mut cur = Some(target);
        while let Some(el) = cur {
            if let Some(slug) = el.get_attribute("data-focus-species") {
                if !slug.is_empty() {
                    super::instruments::species::focus_species_in_world(&mut s.borrow_mut(), &slug);
                    return;
                }
            }
            if let Some(qac_url) = el.get_attribute("data-vocalize-qac") {
                if !qac_url.is_empty() {
                    let slug = el.get_attribute("data-species-slug").unwrap_or_default();
                    if !slug.is_empty() {
                        super::instruments::species::mark_species_discovered(&slug);
                    }
                    let reduced = s.borrow().reduced;
                    if !is_audio_opt_in_granted() {
                        let enabled = set_bush_soundscape(true, reduced);
                        if enabled {
                            if let Some(sb) = doc().get_element_by_id("native-sound-toggle") {
                                sb.set_text_content(Some("Sound: On"));
                                let _ = sb.set_attribute("aria-pressed", "true");
                            }
                            emit_custom_event(
                                "webizen:audio-toggle",
                                &json!({
                                    "enabled": true,
                                    "muted": false,
                                    "opt_in_granted": true,
                                    "reduced_motion": reduced,
                                }),
                            );
                        }
                    }
                    let dispatch = super::instruments::species::vocalize_species_call(&qac_url, 6.0);
                    text("native-world-status", &dispatch.status);
                    return;
                }
            }
            cur = el.parent_element();
        }
    })?;

    // Capture-phase Escape handler so closing any HUD drawer never exits Walk/Fly mode unexpectedly.
    if let Some(win) = web_sys::window() {
        let closure = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
            let Ok(k) = e.dyn_into::<KeyboardEvent>() else {
                return;
            };
            if k.key() != "Escape" {
                return;
            }
            if active_top_drawer().is_some() {
                k.prevent_default();
                k.stop_propagation();
                k.stop_immediate_propagation();
                set_active_top_drawer(None, true);
            }
        });
        let target: &web_sys::EventTarget = win.unchecked_ref();
        target.add_event_listener_with_callback_and_bool(
            "keydown",
            closure.as_ref().unchecked_ref(),
            true,
        )?;
        closure.forget();
    }

    Ok(())
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

fn install_hud_listeners(state: &Shared) -> Result<(), JsValue> {
    if let Some(win) = web_sys::window() {
        event(win.unchecked_ref(), "keydown", move |e| {
            let Ok(k) = e.dyn_into::<KeyboardEvent>() else {
                return;
            };
            if is_editable_target(k.target()) || k.ctrl_key() || k.meta_key() || k.alt_key() {
                return;
            }
            match k.key().as_str() {
                "h" | "H" => {
                    k.prevent_default();
                    let next = HUD_UI.with(|h| !h.borrow().clean_view);
                    set_clean_view(next);
                }
                "F3" | "`" => {
                    k.prevent_default();
                    if HUD_UI.with(|h| h.borrow().clean_view) {
                        set_clean_view(false);
                    }
                    let _ = toggle_top_drawer(TopDrawer::Stats);
                }
                _ => {}
            }
        })?;

        event(win.unchecked_ref(), "blur", move |_| {
            clear_drive_touch_inputs();
        })?;

        for ev_name in [
            "webizen:vehicle-state",
            "webizen:drive-toggle",
            "webizen:hitch-toggle",
            "webizen:headlights-toggle",
            "webizen:drive-camera-toggle",
            "webizen:ewm-state",
            "webizen:tour-stop",
        ] {
            let s = state.clone();
            event(win.unchecked_ref(), ev_name, move |_| {
                sync_dynamic_hud(&s);
            })?;
        }
    }

    let d = doc();
    event(d.unchecked_ref(), "click", move |e| {
        let Some(target) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let mut cur = Some(target);
        while let Some(el) = cur {
            if el.tag_name().eq_ignore_ascii_case("SUMMARY") {
                if let Some(parent) = el.parent_element() {
                    if parent.id() == "native-proof-lab" && !parent.has_attribute("open") {
                        set_active_top_drawer(None, false);
                    }
                }
                return;
            }
            cur = el.parent_element();
        }
    })?;

    Ok(())
}

fn sync_dynamic_hud(state: &Shared) {
    let ewm = read_window_global("__webizenEwmState");
    let veh = read_window_global("__webizenVehicleState");

    let (yaw_rad, pos_x, pos_z, alt_fallback, regime_fallback) = match state.try_borrow() {
        Ok(w) => (
            w.camera.yaw,
            w.camera.target[0] as f64,
            w.camera.target[2] as f64,
            current_altitude_m(&w),
            w.receipt
                .get("ocs_regime")
                .and_then(Value::as_str)
                .map(str::to_string),
        ),
        Err(_) => (0.0, -22.0, 28.0, 1.7, None),
    };

    let mode = super::locomotion::current_mode();
    let rhizo = super::locomotion::rhizosphere_xray_active();
    let is_driving = mode == super::locomotion::MovementMode::DriveVehicle
        || read_js_bool(&veh, "active")
            .or_else(|| read_js_bool(&veh, "driving"))
            .unwrap_or(false);

    sync_mode_ui(mode, rhizo);

    let bearing_deg = if is_driving {
        read_js_f64(&veh, "heading_deg")
            .or_else(|| read_js_f64(&veh, "bearing_deg"))
            .unwrap_or_else(|| bearing_deg_from_yaw(yaw_rad))
    } else {
        bearing_deg_from_yaw(yaw_rad)
    };
    let heading_text = format_compass_heading(bearing_deg);

    let veh_x = read_js_f64(&veh, "x").unwrap_or(pos_x);
    let veh_z = read_js_f64(&veh, "z").unwrap_or(pos_z);
    let raw_zone = if is_driving {
        read_js_string(&veh, "corridor_zone")
            .or_else(|| read_js_string(&veh, "zone_label"))
            .or_else(|| read_js_string(&ewm, "corridor_zone"))
    } else {
        read_js_string(&ewm, "corridor_zone")
            .or_else(|| read_js_string(&ewm, "zone_label"))
            .or_else(|| read_js_string(&veh, "corridor_zone"))
    };
    let zone_text = format_corridor_zone_label(
        raw_zone.as_deref(),
        if is_driving { veh_x } else { pos_x },
        if is_driving { veh_z } else { pos_z },
    )
    .to_string();

    let ocs_regime = read_js_string(&ewm, "ocs_regime").or(regime_fallback);
    let alt_m = read_js_f64(&ewm, "altitude_m").unwrap_or(alt_fallback);
    let ocs_text = format_ocs_scale_label(ocs_regime.as_deref(), alt_m);
    let regime_attr = normalize_ocs_regime_attr(ocs_regime.as_deref(), alt_m).to_string();

    HUD_UI.with(|h| {
        let mut st = h.borrow_mut();
        if st.last_heading_text != heading_text {
            text("native-compass-heading", &heading_text);
            st.last_heading_text = heading_text;
        }
        if st.last_zone_text != zone_text {
            text("native-compass-zone", &zone_text);
            st.last_zone_text = zone_text;
        }
        if st.last_ocs_text != ocs_text {
            text("native-compass-ocs", &ocs_text);
            st.last_ocs_text = ocs_text;
        }
        if st.last_ocs_regime != regime_attr {
            if let Some(pill) = doc().get_element_by_id("native-compass-scale") {
                let _ = pill.set_attribute("data-ocs-regime", &regime_attr);
            }
            st.last_ocs_regime = regime_attr;
        }
        st.last_drive_active = Some(is_driving);

        if !is_driving {
            return;
        }

        let speed_kmh = read_js_f64(&veh, "speed_kmh")
            .or_else(|| read_js_f64(&veh, "speed_mps").map(|v| v * 3.6))
            .unwrap_or(0.0);
        let speed_text = format!("{:.0}", speed_kmh.abs().round());
        if st.last_speed_text != speed_text {
            text("native-veh-speed", &speed_text);
            st.last_speed_text = speed_text;
        }

        let gear_text = read_js_string(&veh, "gear").unwrap_or_else(|| "P".to_string());
        if st.last_gear_text != gear_text {
            text("native-veh-gear", &gear_text);
            st.last_gear_text = gear_text;
        }

        let pitch_deg = read_js_f64(&veh, "pitch_deg").unwrap_or(0.0);
        let pitch_text = format!("{pitch_deg:+.1}°");
        if st.last_pitch_text != pitch_text {
            text("native-veh-pitch", &pitch_text);
            st.last_pitch_text = pitch_text;
        }

        let roll_deg = read_js_f64(&veh, "roll_deg").unwrap_or(0.0);
        let roll_text = format!("{roll_deg:+.1}°");
        if st.last_roll_text != roll_text {
            text("native-veh-roll", &roll_text);
            st.last_roll_text = roll_text;
        }

        let raw_surface = read_js_string(&veh, "surface")
            .or_else(|| read_js_string(&veh, "surface_traction"));
        let traction_text = format_surface_traction(raw_surface.as_deref()).to_string();
        if st.last_traction_text != traction_text {
            if let Some(el) = doc().get_element_by_id("native-veh-traction") {
                el.set_text_content(Some(&traction_text));
                let _ = el.set_attribute("data-surface", &traction_text);
            }
            st.last_traction_text = traction_text;
        }

        let hitched = read_js_bool(&veh, "hitch_attached")
            .or_else(|| read_js_bool(&veh, "hitched"))
            .unwrap_or(false);
        if st.last_hitch_attached != Some(hitched) {
            if let Some(btn) = doc().get_element_by_id("native-veh-hitch") {
                btn.set_text_content(Some(if hitched { "Unhitch" } else { "Hitch Camper" }));
                let _ = btn.set_attribute("aria-pressed", if hitched { "true" } else { "false" });
            }
            st.last_hitch_attached = Some(hitched);
        }

        let lights = read_js_bool(&veh, "headlights")
            .or_else(|| read_js_bool(&veh, "headlights_on"))
            .unwrap_or(false);
        if st.last_lights_on != Some(lights) {
            if let Some(btn) = doc().get_element_by_id("native-veh-lights") {
                btn.set_text_content(Some(if lights {
                    "Lights: On (L)"
                } else {
                    "Lights (L)"
                }));
                let _ = btn.set_attribute("aria-pressed", if lights { "true" } else { "false" });
            }
            st.last_lights_on = Some(lights);
        }

        let raw_cam = read_js_string(&veh, "camera_mode")
            .or_else(|| read_js_string(&veh, "drive_camera"));
        let cam_text = format_drive_camera_label(raw_cam.as_deref()).to_string();
        if st.last_cam_text != cam_text {
            text("native-veh-cam", &cam_text);
            st.last_cam_text = cam_text;
        }
    });
}

fn start_hud_sync_loop(state: &Shared) -> Result<(), JsValue> {
    let closure: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let keep = closure.clone();
    let s = state.clone();
    let last_sync = Rc::new(RefCell::new(0.0f64));
    let last_stats_sync = Rc::new(RefCell::new(0.0f64));
    *closure.borrow_mut() = Some(Closure::new(move |stamp: f64| {
        let drive_changed = HUD_UI.with(|h| h.borrow().last_drive_active != Some(is_driving_now()));
        let due = {
            let mut prev = last_sync.borrow_mut();
            if drive_changed || (stamp - *prev) >= 80.0 {
                *prev = stamp;
                true
            } else {
                false
            }
        };
        if due {
            sync_dynamic_hud(&s);
        }
        if is_top_drawer_open(TopDrawer::Stats) {
            let stats_due = {
                let mut prev = last_stats_sync.borrow_mut();
                if (stamp - *prev) >= 250.0 {
                    *prev = stamp;
                    true
                } else {
                    false
                }
            };
            if stats_due {
                sync_stats_drawer();
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

fn read_js_prop(obj: &JsValue, key: &str) -> JsValue {
    if obj.is_null() || obj.is_undefined() || !obj.is_object() {
        return JsValue::UNDEFINED;
    }
    js_sys::Reflect::get(obj, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

fn read_js_u64(obj: &JsValue, key: &str) -> Option<u64> {
    read_js_f64(obj, key)
        .filter(|n| *n >= 0.0)
        .map(|n| n.round() as u64)
}

fn add_stat_row(parent: &Element, label: &str, value_id: &str, initial: &str) -> Result<Element, JsValue> {
    let row = doc().create_element("div")?;
    row.set_class_name("stats-row");
    let dt = doc().create_element("span")?;
    dt.set_class_name("stats-k");
    dt.set_text_content(Some(label));
    let dd = doc().create_element("span")?;
    dd.set_id(value_id);
    dd.set_class_name("stats-v");
    dd.set_text_content(Some(initial));
    row.append_child(&dt)?;
    row.append_child(&dd)?;
    parent.append_child(&row)?;
    Ok(dd)
}

fn add_stat_bar(parent: &Element, label: &str, value_id: &str, bar_id: &str, initial: &str) -> Result<(), JsValue> {
    let wrap = doc().create_element("div")?;
    wrap.set_class_name("stats-bar-wrap");
    let head = doc().create_element("div")?;
    head.set_class_name("stats-row");
    let dt = doc().create_element("span")?;
    dt.set_class_name("stats-k");
    dt.set_text_content(Some(label));
    let dd = doc().create_element("span")?;
    dd.set_id(value_id);
    dd.set_class_name("stats-v");
    dd.set_text_content(Some(initial));
    head.append_child(&dt)?;
    head.append_child(&dd)?;
    let track = doc().create_element("div")?;
    track.set_class_name("stats-bar-track");
    let fill = doc().create_element("div")?;
    fill.set_id(bar_id);
    fill.set_class_name("stats-bar-fill");
    fill.set_attribute("style", "width:0%")?;
    track.append_child(&fill)?;
    wrap.append_child(&head)?;
    wrap.append_child(&track)?;
    parent.append_child(&wrap)?;
    Ok(())
}

fn install_stats_drawer() -> Result<(), JsValue> {
    let drawer = element("native-stats-drawer", "aside")?;
    drawer.set_attribute("role", "region")?;
    drawer.set_attribute("aria-label", "Technical Statistics Panel")?;
    drawer.set_attribute("hidden", "")?;
    drawer.set_text_content(None);

    let head = doc().create_element("div")?;
    head.set_class_name("hud-drawer-head");
    let title_wrap = doc().create_element("div")?;
    let h2 = doc().create_element("h2")?;
    h2.set_text_content(Some("Technical Statistics"));
    let sub = doc().create_element("span")?;
    sub.set_id("native-stat-fps-badge");
    sub.set_class_name("stats-fps-badge");
    sub.set_text_content(Some("-- FPS"));
    title_wrap.append_child(&h2)?;
    title_wrap.append_child(&sub)?;
    head.append_child(&title_wrap)?;

    let close_btn = button(&head, "Close")?;
    close_btn.set_id("native-stats-close");
    close_btn.set_class_name("native-pill hud-drawer-close");
    close_btn.set_attribute("aria-label", "Close technical statistics panel (Escape or F3)")?;
    event(close_btn.unchecked_ref(), "click", move |_| {
        set_active_top_drawer(None, true);
    })?;
    drawer.append_child(&head)?;

    let body = doc().create_element("div")?;
    body.set_class_name("stats-sections");

    // 1. Graphics & Timing
    let s1 = doc().create_element("section")?;
    s1.set_class_name("stats-section");
    let h_s1 = doc().create_element("h3")?;
    h_s1.set_text_content(Some("1. Graphics & Timing"));
    s1.append_child(&h_s1)?;
    add_stat_row(&s1, "Backend", "native-stat-backend", "--")?;
    add_stat_row(&s1, "Device Tier", "native-stat-tier", "--")?;
    add_stat_row(&s1, "Live FPS", "native-stat-fps", "--")?;
    add_stat_row(&s1, "Frame avg / p95", "native-stat-framems", "--")?;
    add_stat_row(&s1, "Dropped (>33.3ms)", "native-stat-dropped", "0")?;
    add_stat_row(&s1, "DPR / Canvas", "native-stat-resolution", "--")?;
    body.append_child(&s1)?;

    // 2. Streaming & Residency (Add / Unload)
    let s2 = doc().create_element("section")?;
    s2.set_class_name("stats-section");
    let h_s2 = doc().create_element("h3")?;
    h_s2.set_text_content(Some("2. Streaming & Residency"));
    s2.append_child(&h_s2)?;
    add_stat_row(&s2, "Phase", "native-stat-phase", "first-paint")?;
    add_stat_row(&s2, "Resident Assets", "native-stat-resident", "--")?;
    add_stat_row(&s2, "Streamed In / Evicted", "native-stat-io", "+0 / -0")?;
    add_stat_bar(&s2, "Triangles", "native-stat-tris", "native-stat-tris-bar", "--")?;
    add_stat_bar(&s2, "GPU Memory", "native-stat-gpu", "native-stat-gpu-bar", "--")?;
    add_stat_row(&s2, "Web Worker", "native-stat-worker", "idle")?;
    add_stat_row(&s2, "3×3 Tile / Zone", "native-stat-tile", "(0, 0) · Forest Campsite")?;
    body.append_child(&s2)?;

    // 3. Biosphere & Simulation
    let s3 = doc().create_element("section")?;
    s3.set_class_name("stats-section");
    let h_s3 = doc().create_element("h3")?;
    h_s3.set_text_content(Some("3. Biosphere & Simulation"));
    s3.append_child(&h_s3)?;
    add_stat_row(&s3, "Streamed Species", "native-stat-species", "0 / 17")?;
    add_stat_row(&s3, "Placements", "native-stat-placements", "317")?;
    add_stat_row(&s3, "Rhizosphere X-Ray", "native-stat-rhizo", "Off")?;
    add_stat_row(&s3, "Anatomy X-Ray", "native-stat-anatomy", "Off")?;
    add_stat_row(&s3, "Movement Mode", "native-stat-mode", "Observe")?;
    add_stat_row(&s3, "Vehicle", "native-stat-vehicle", "Parked")?;
    add_stat_row(&s3, "OCS Frame / Origin Err", "native-stat-ocs", "Local ENU · 0.00 m")?;
    body.append_child(&s3)?;

    // 4. Admission & Health
    let s4 = doc().create_element("section")?;
    s4.set_class_name("stats-section");
    let h_s4 = doc().create_element("h3")?;
    h_s4.set_text_content(Some("4. Admission & Health"));
    s4.append_child(&h_s4)?;
    add_stat_row(&s4, "Last Stream Note", "native-stat-note", "Ready")?;
    add_stat_row(&s4, "Refused Count", "native-stat-refused", "0")?;
    let err_el = add_stat_row(&s4, "Health / Error", "native-stat-error", "OK")?;
    err_el.set_attribute("data-health", "ok")?;
    body.append_child(&s4)?;

    drawer.append_child(&body)?;
    Ok(())
}

fn format_tier_label(raw: Option<&str>) -> String {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("tier-a") | Some("tiera") | Some("a") => "TierA".to_string(),
        Some("tier-b") | Some("tierb") | Some("b") => "TierB".to_string(),
        Some("tier-c") | Some("tierc") | Some("c") => "TierC".to_string(),
        Some(other) if !other.is_empty() => other.to_string(),
        _ => "TierB".to_string(),
    }
}

fn format_backend_label(raw: Option<&str>) -> String {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("webgpu") => "WebGPU".to_string(),
        Some("webgl2") => "WebGL2".to_string(),
        Some(other) if !other.is_empty() => other.to_string(),
        _ => "Pending".to_string(),
    }
}

fn set_bar_percent(bar_id: &str, used: f64, budget: f64) {
    let pct = if budget > 0.0 && used.is_finite() {
        ((used / budget) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    if let Some(el) = doc().get_element_by_id(bar_id) {
        let _ = el.set_attribute("style", &format!("width:{pct:.1}%"));
        let tone = if pct >= 90.0 {
            "high"
        } else if pct >= 70.0 {
            "mid"
        } else {
            "ok"
        };
        let _ = el.set_attribute("data-level", tone);
    }
}

fn sync_stats_drawer() {
    let init = read_window_global("__webizenNativeInit");
    let upload = read_window_global("__webizenNativeUpload");
    let frame = read_window_global("__webizenFrameStats");
    let ewm = read_window_global("__webizenEwmState");
    let stream = read_window_global("__webizenStreamStats");
    let veh = read_window_global("__webizenVehicleState");
    let note_global = read_window_global("__webizenCatchmentStreamNote");

    // 1. Graphics & Timing
    let backend = format_backend_label(
        read_js_string(&init, "selected_backend")
            .or_else(|| read_js_string(&init, "backend"))
            .as_deref(),
    );
    let tier = format_tier_label(
        read_js_string(&init, "device_tier")
            .or_else(|| read_js_string(&init, "tier"))
            .as_deref(),
    );
    let fps = read_js_f64(&frame, "fps");
    let avg_ms = read_js_f64(&frame, "avg_ms");
    let p95_ms = read_js_f64(&frame, "p95_ms");
    let dropped = read_js_u64(&frame, "dropped").unwrap_or(0);
    let total_frames = read_js_u64(&frame, "total_frames").unwrap_or(0);

    text("native-stat-backend", &backend);
    text("native-stat-tier", &tier);
    if let Some(f) = fps {
        text("native-stat-fps", &format!("{f:.1} fps ({total_frames} f)"));
        text("native-stat-fps-badge", &format!("{f:.0} FPS"));
    } else {
        text("native-stat-fps", "--");
        text("native-stat-fps-badge", "-- FPS");
    }
    match (avg_ms, p95_ms) {
        (Some(a), Some(p)) => text("native-stat-framems", &format!("{a:.1} / {p:.1} ms")),
        (Some(a), None) => text("native-stat-framems", &format!("{a:.1} ms")),
        _ => text("native-stat-framems", "--"),
    }
    text("native-stat-dropped", &format!("{dropped}"));

    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or(1.0);
    let (cw, ch) = doc()
        .get_element_by_id("native-world")
        .and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok())
        .map(|c| (c.width(), c.height()))
        .unwrap_or((0, 0));
    text("native-stat-resolution", &format!("{dpr:.2}× · {cw}×{ch}"));

    // 2. Streaming & Residency (Add / Unload)
    let phase = read_js_string(&stream, "phase")
        .or_else(|| read_js_string(&ewm, "phase"))
        .or_else(|| read_js_string(&ewm, "status"))
        .unwrap_or_else(|| "first-paint".to_string());
    let resident_assets = read_js_u64(&stream, "resident_assets")
        .or_else(|| read_js_u64(&stream, "resident_count"))
        .or_else(|| read_js_u64(&upload, "resident_assets"))
        .or_else(|| read_js_u64(&upload, "asset_count"))
        .or_else(|| read_js_u64(&ewm, "resident_count"))
        .unwrap_or(0);
    let streamed_in = read_js_u64(&stream, "streamed_in")
        .or_else(|| read_js_u64(&stream, "added_count"))
        .or_else(|| read_js_u64(&stream, "loaded_count"))
        .unwrap_or(0);
    let evicted = read_js_u64(&stream, "evicted")
        .or_else(|| read_js_u64(&stream, "evicted_count"))
        .or_else(|| read_js_u64(&stream, "unloaded_count"))
        .unwrap_or(0);

    let tier_budget = read_js_prop(&init, "tier_budget");
    let res_tris = read_js_u64(&stream, "resident_triangles")
        .or_else(|| read_js_u64(&stream, "triangles"))
        .or_else(|| read_js_u64(&upload, "resident_triangles"))
        .or_else(|| read_js_u64(&upload, "triangles"))
        .unwrap_or(0);
    let max_tris = read_js_u64(&stream, "triangle_budget")
        .or_else(|| read_js_u64(&stream, "max_resident_triangles"))
        .or_else(|| read_js_u64(&tier_budget, "max_resident_triangles"))
        .unwrap_or(320_000);

    let gpu_bytes = read_js_f64(&stream, "gpu_bytes")
        .or_else(|| read_js_f64(&stream, "resident_bytes"))
        .or_else(|| read_js_f64(&upload, "gpu_bytes"))
        .or_else(|| read_js_f64(&upload, "bytes"))
        .unwrap_or(0.0);
    let max_gpu_bytes = read_js_f64(&stream, "gpu_budget_bytes")
        .or_else(|| read_js_f64(&stream, "max_gpu_memory_bytes"))
        .or_else(|| read_js_f64(&tier_budget, "max_gpu_memory_bytes"))
        .unwrap_or(192.0 * 1024.0 * 1024.0);

    let worker_obj = read_js_prop(&stream, "worker");
    let worker_active = read_js_bool(&stream, "worker_active")
        .or_else(|| read_js_bool(&worker_obj, "active"))
        .unwrap_or(false);
    let worker_inflight = read_js_u64(&stream, "worker_inflight")
        .or_else(|| read_js_u64(&worker_obj, "inflight"))
        .unwrap_or(0);
    let worker_completed = read_js_u64(&stream, "worker_completed")
        .or_else(|| read_js_u64(&worker_obj, "completed"))
        .unwrap_or(0);

    let tile_x = read_js_f64(&stream, "tile_x")
        .or_else(|| read_js_f64(&ewm, "tile_x"))
        .map(|v| v.round() as i32)
        .unwrap_or(0);
    let tile_y = read_js_f64(&stream, "tile_y")
        .or_else(|| read_js_f64(&ewm, "tile_y"))
        .map(|v| v.round() as i32)
        .unwrap_or(0);
    let zone = HUD_UI.with(|h| {
        let z = h.borrow().last_zone_text.clone();
        if z.is_empty() {
            "Forest Campsite".to_string()
        } else {
            z
        }
    });

    text("native-stat-phase", &phase);
    text("native-stat-resident", &format!("{resident_assets}"));
    text("native-stat-io", &format!("+{streamed_in} / -{evicted}"));
    text(
        "native-stat-tris",
        &format!("{}k / {}k", res_tris / 1000, max_tris / 1000),
    );
    set_bar_percent("native-stat-tris-bar", res_tris as f64, max_tris as f64);
    text(
        "native-stat-gpu",
        &format!(
            "{:.1} / {:.0} MiB",
            gpu_bytes / (1024.0 * 1024.0),
            max_gpu_bytes / (1024.0 * 1024.0)
        ),
    );
    set_bar_percent("native-stat-gpu-bar", gpu_bytes, max_gpu_bytes);
    text(
        "native-stat-worker",
        &format!(
            "{} · {} in / {} done",
            if worker_active { "active" } else { "idle" },
            worker_inflight,
            worker_completed
        ),
    );
    text("native-stat-tile", &format!("({tile_x}, {tile_y}) · {zone}"));

    // 3. Biosphere & Simulation
    let loaded_species = super::instruments::species::species_count_loaded();
    let ewm_species = read_js_usize(&ewm, "species_count")
        .or_else(|| read_js_usize(&stream, "species_count"))
        .unwrap_or(0);
    let species_count = loaded_species.max(ewm_species);
    let placements = read_js_u64(&ewm, "placements")
        .or_else(|| read_js_u64(&stream, "placements"))
        .unwrap_or(317);
    let rhizo = super::locomotion::rhizosphere_xray_active();
    let anatomy = super::locomotion::anatomy_xray_active();
    let mode_label = if is_driving_now() {
        "Drive"
    } else {
        match super::locomotion::current_mode() {
            super::locomotion::MovementMode::ObserveOrbit => "Observe",
            super::locomotion::MovementMode::WalkExplore => "Walk",
            super::locomotion::MovementMode::DriveVehicle => "Drive",
            super::locomotion::MovementMode::FlyGlide => "Fly",
        }
    };
    let veh_summary = if is_driving_now() {
        let kmh = read_js_f64(&veh, "speed_kmh")
            .or_else(|| read_js_f64(&veh, "speed_mps").map(|v| v * 3.6))
            .unwrap_or(0.0);
        let gear = read_js_string(&veh, "gear").unwrap_or_else(|| "D".to_string());
        let pitch = read_js_f64(&veh, "pitch_deg").unwrap_or(0.0);
        let roll = read_js_f64(&veh, "roll_deg").unwrap_or(0.0);
        let hitched = read_js_bool(&veh, "hitch_attached")
            .or_else(|| read_js_bool(&veh, "hitched"))
            .unwrap_or(false);
        format!(
            "{:.0} km/h · {} · {:+.1}°/{:+.1}° · {}",
            kmh.abs(),
            gear,
            pitch,
            roll,
            if hitched { "Hitched" } else { "Solo" }
        )
    } else {
        "Parked (press F)".to_string()
    };
    let ocs_text = HUD_UI.with(|h| {
        let s = h.borrow().last_ocs_text.clone();
        if s.is_empty() {
            "Local ENU".to_string()
        } else {
            s
        }
    });
    let origin_err = read_js_f64(&ewm, "floating_origin_error_m")
        .or_else(|| read_js_f64(&stream, "floating_origin_error_m"))
        .unwrap_or(0.0);

    text("native-stat-species", &format!("{species_count} / 17"));
    text("native-stat-placements", &format!("{placements}"));
    text("native-stat-rhizo", if rhizo { "Active" } else { "Off" });
    text("native-stat-anatomy", if anatomy { "Active" } else { "Off" });
    text("native-stat-mode", mode_label);
    text("native-stat-vehicle", &veh_summary);
    text("native-stat-ocs", &format!("{ocs_text} · err {origin_err:.2} m"));

    // 4. Admission & Health
    let last_note = note_global
        .as_string()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| read_js_string(&stream, "last_note"))
        .or_else(|| read_js_string(&ewm, "last_note"))
        .unwrap_or_else(|| "Ready".to_string());
    let refused = read_js_u64(&stream, "refused_count")
        .or_else(|| read_js_u64(&stream, "refused"))
        .or_else(|| read_js_u64(&ewm, "refused_count"))
        .unwrap_or(0);
    let last_err = read_js_string(&stream, "last_error")
        .or_else(|| read_js_string(&ewm, "last_error"))
        .or_else(|| read_js_string(&init, "last_error"));

    text("native-stat-note", &last_note);
    text("native-stat-refused", &format!("{refused}"));
    if let Some(el) = doc().get_element_by_id("native-stat-error") {
        if let Some(msg) = last_err {
            el.set_text_content(Some(&msg));
            let _ = el.set_attribute("data-health", "error");
        } else if refused > 0 {
            el.set_text_content(Some(&format!("Warn ({refused} refused)")));
            let _ = el.set_attribute("data-health", "warn");
        } else {
            el.set_text_content(Some("OK"));
            let _ = el.set_attribute("data-health", "ok");
        }
    }
}

fn build_diagnostics_snapshot() -> js_sys::Object {
    let snap = js_sys::Object::new();
    let set = |obj: &js_sys::Object, key: &str, val: &JsValue| {
        let _ = js_sys::Reflect::set(obj, &JsValue::from_str(key), val);
    };

    let ts = js_sys::Date::new_0().to_iso_string();
    set(&snap, "timestamp", &ts.into());

    let vp = js_sys::Object::new();
    if let Some(win) = web_sys::window() {
        let iw = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ih = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        set(&vp, "width", &JsValue::from_f64(iw));
        set(&vp, "height", &JsValue::from_f64(ih));
        set(&vp, "dpr", &JsValue::from_f64(win.device_pixel_ratio()));
        let nav = read_js_prop(&win.into(), "navigator");
        let ua = read_js_string(&nav, "userAgent").unwrap_or_default();
        set(&snap, "userAgent", &JsValue::from_str(&ua));
    }
    if let Some(canvas) = doc()
        .get_element_by_id("native-world")
        .and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok())
    {
        set(&vp, "canvas_width", &JsValue::from_f64(canvas.width() as f64));
        set(&vp, "canvas_height", &JsValue::from_f64(canvas.height() as f64));
    }
    set(&snap, "viewport", &vp.into());

    let hud = js_sys::Object::new();
    set(&hud, "mode", &JsValue::from_str(super::locomotion::current_mode().as_str()));
    set(&hud, "driving", &JsValue::from_bool(is_driving_now()));
    set(&hud, "clean_view", &JsValue::from_bool(HUD_UI.with(|h| h.borrow().clean_view)));
    set(&hud, "rhizosphere_xray", &JsValue::from_bool(super::locomotion::rhizosphere_xray_active()));
    set(&hud, "anatomy_xray", &JsValue::from_bool(super::locomotion::anatomy_xray_active()));
    let active_drawer = match active_top_drawer() {
        Some(TopDrawer::Tour) => "tour",
        Some(TopDrawer::FieldGuide) => "field-guide",
        Some(TopDrawer::Stats) => "stats",
        Some(TopDrawer::Debug) => "debug",
        Some(TopDrawer::WorldPanel) => "world-panel",
        None => "none",
    };
    set(&hud, "active_drawer", &JsValue::from_str(active_drawer));
    set(&snap, "hud", &hud.into());

    for (field, global_key) in [
        ("nativeInit", "__webizenNativeInit"),
        ("nativeUpload", "__webizenNativeUpload"),
        ("frameStats", "__webizenFrameStats"),
        ("ewmState", "__webizenEwmState"),
        ("streamStats", "__webizenStreamStats"),
        ("vehicleState", "__webizenVehicleState"),
        ("catchmentStreamNote", "__webizenCatchmentStreamNote"),
        ("nativeWorld", "__webizenNativeWorld"),
    ] {
        let val = read_window_global(global_key);
        let null_val = JsValue::NULL;
        set(&snap, field, if val.is_undefined() { &null_val } else { &val });
    }

    snap
}

fn call_console_method(name: &str, arg: Option<&JsValue>) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let console = read_js_prop(&win.into(), "console");
    let func_val = read_js_prop(&console, name);
    if let Ok(func) = func_val.dyn_into::<js_sys::Function>() {
        let _ = match arg {
            Some(a) => func.call1(&console, a),
            None => func.call0(&console),
        };
    }
}

fn log_diagnostics_to_console() -> JsValue {
    let snap = build_diagnostics_snapshot();
    let snap_js: JsValue = snap.clone().into();

    call_console_method(
        "groupCollapsed",
        Some(&JsValue::from_str("[Webizen Diagnostics]")),
    );
    call_console_method("log", Some(&snap_js));

    let table = js_sys::Object::new();
    let put_row = |section: &str, val: JsValue| {
        let _ = js_sys::Reflect::set(&table, &JsValue::from_str(section), &val);
    };
    let frame = read_window_global("__webizenFrameStats");
    let stream = read_window_global("__webizenStreamStats");
    let init = read_window_global("__webizenNativeInit");
    let ewm = read_window_global("__webizenEwmState");
    let make_row = |a: &str, b: &str| -> JsValue {
        let o = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str("value"), &JsValue::from_str(a));
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str("detail"), &JsValue::from_str(b));
        o.into()
    };
    put_row(
        "graphics",
        make_row(
            &format_backend_label(read_js_string(&init, "selected_backend").as_deref()),
            &format_tier_label(read_js_string(&init, "device_tier").as_deref()),
        ),
    );
    put_row(
        "frame_timing",
        make_row(
            &format!("{:.1} fps", read_js_f64(&frame, "fps").unwrap_or(0.0)),
            &format!(
                "avg {:.1}ms / p95 {:.1}ms / dropped {}",
                read_js_f64(&frame, "avg_ms").unwrap_or(0.0),
                read_js_f64(&frame, "p95_ms").unwrap_or(0.0),
                read_js_u64(&frame, "dropped").unwrap_or(0)
            ),
        ),
    );
    put_row(
        "streaming",
        make_row(
            &read_js_string(&stream, "phase")
                .or_else(|| read_js_string(&ewm, "phase"))
                .unwrap_or_else(|| "first-paint".to_string()),
            &format!(
                "resident {} (+{} / -{})",
                read_js_u64(&stream, "resident_assets").unwrap_or(0),
                read_js_u64(&stream, "streamed_in").unwrap_or(0),
                read_js_u64(&stream, "evicted").unwrap_or(0)
            ),
        ),
    );
    put_row(
        "biosphere",
        make_row(
            &format!("{} / 17 species", super::instruments::species::species_count_loaded()),
            super::locomotion::current_mode().as_str(),
        ),
    );
    call_console_method("table", Some(&table.into()));
    call_console_method("groupEnd", None);

    let summary_preview = format!(
        "Logged [Webizen Diagnostics] to console.\nBackend: {} ({}) | FPS: {:.1} | Mode: {}",
        format_backend_label(read_js_string(&init, "selected_backend").as_deref()),
        format_tier_label(read_js_string(&init, "device_tier").as_deref()),
        read_js_f64(&frame, "fps").unwrap_or(0.0),
        super::locomotion::current_mode().as_str(),
    );
    text("native-debug-output", &summary_preview);
    snap_js
}

fn stringify_snapshot_pretty() -> String {
    let snap = build_diagnostics_snapshot();
    js_sys::JSON::stringify_with_replacer_and_space(
        &snap.into(),
        &JsValue::NULL,
        &JsValue::from_f64(2.0),
    )
    .ok()
    .map(String::from)
    .unwrap_or_else(|| "{}".to_string())
}

fn copy_diagnostics_json() {
    let json_text = stringify_snapshot_pretty();
    let Some(win) = web_sys::window() else {
        return;
    };
    let nav = read_js_prop(&win.clone().into(), "navigator");
    let clip = read_js_prop(&nav, "clipboard");
    let write_fn = read_js_prop(&clip, "writeText");
    if let Ok(func) = write_fn.dyn_into::<js_sys::Function>() {
        let _ = func.call1(&clip, &JsValue::from_str(&json_text));
        if let Some(btn) = doc().get_element_by_id("native-debug-copy") {
            btn.set_text_content(Some("Copied!"));
            let reset = Closure::<dyn FnMut()>::new(move || {
                if let Some(b) = doc().get_element_by_id("native-debug-copy") {
                    b.set_text_content(Some("Copy JSON"));
                }
            });
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                reset.as_ref().unchecked_ref(),
                1800,
            );
            reset.forget();
        }
        text(
            "native-debug-output",
            &format!("Copied {} bytes of diagnostics JSON to clipboard.", json_text.len()),
        );
    } else {
        text(
            "native-debug-output",
            "Clipboard API unavailable in this browser context; use Download JSON or Log to Console.",
        );
    }
}

fn download_diagnostics_json() -> Result<(), JsValue> {
    let json_text = stringify_snapshot_pretty();
    let raw_ts = String::from(js_sys::Date::new_0().to_iso_string());
    let safe_ts = raw_ts.replace(':', "-").replace('.', "-");
    let filename = format!("webizen-diagnostics-{safe_ts}.json");

    let array = js_sys::Uint8Array::from(json_text.as_bytes());
    let parts = js_sys::Array::of1(&array);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/json");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
    let href = web_sys::Url::create_object_url_with_blob(&blob)?;
    let a = doc()
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    a.set_href(&href);
    a.set_download(&filename);
    a.click();
    let _ = web_sys::Url::revoke_object_url(&href);
    text(
        "native-debug-output",
        &format!("Downloaded {filename} ({} bytes).", json_text.len()),
    );
    Ok(())
}

fn call_stream_hook(method_name: &str) -> JsValue {
    let hooks = read_window_global("__webizenStreamHooks");
    let func_val = read_js_prop(&hooks, method_name);
    if let Ok(func) = func_val.dyn_into::<js_sys::Function>() {
        match func.call0(&hooks) {
            Ok(res) => {
                let res_str = js_sys::JSON::stringify(&res)
                    .ok()
                    .map(String::from)
                    .unwrap_or_else(|| "ok".to_string());
                text(
                    "native-debug-output",
                    &format!("__webizenStreamHooks.{method_name}() -> {res_str}"),
                );
                if is_top_drawer_open(TopDrawer::Stats) {
                    sync_stats_drawer();
                }
                res
            }
            Err(e) => {
                let msg = e.as_string().unwrap_or_else(|| "hook error".to_string());
                text(
                    "native-debug-output",
                    &format!("__webizenStreamHooks.{method_name}() failed: {msg}"),
                );
                let out = js_sys::Object::new();
                let _ = js_sys::Reflect::set(&out, &JsValue::from_str("ok"), &JsValue::FALSE);
                let _ = js_sys::Reflect::set(&out, &JsValue::from_str("error"), &JsValue::from_str(&msg));
                out.into()
            }
        }
    } else {
        let msg = format!("window.__webizenStreamHooks.{method_name} is not registered yet.");
        text("native-debug-output", &msg);
        let out = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&out, &JsValue::from_str("available"), &JsValue::FALSE);
        let _ = js_sys::Reflect::set(&out, &JsValue::from_str("reason"), &JsValue::from_str(&msg));
        out.into()
    }
}

fn install_debug_drawer() -> Result<(), JsValue> {
    let drawer = element("native-debug-drawer", "aside")?;
    drawer.set_attribute("role", "region")?;
    drawer.set_attribute("aria-label", "Diagnostics and Debug Tools")?;
    drawer.set_attribute("hidden", "")?;
    drawer.set_text_content(None);

    let head = doc().create_element("div")?;
    head.set_class_name("hud-drawer-head");
    let h2 = doc().create_element("h2")?;
    h2.set_text_content(Some("Diagnostics & Debug"));
    head.append_child(&h2)?;

    let close_btn = button(&head, "Close")?;
    close_btn.set_id("native-debug-close");
    close_btn.set_class_name("native-pill hud-drawer-close");
    close_btn.set_attribute("aria-label", "Close diagnostics drawer (Escape)")?;
    event(close_btn.unchecked_ref(), "click", move |_| {
        set_active_top_drawer(None, true);
    })?;
    drawer.append_child(&head)?;

    let actions = doc().create_element("div")?;
    actions.set_class_name("debug-actions-grid");
    actions.set_attribute("role", "group")?;
    actions.set_attribute("aria-label", "Diagnostics actions")?;

    let log_btn = button(&actions, "Log to Console")?;
    log_btn.set_id("native-debug-log");
    log_btn.set_class_name("native-pill debug-action-btn");
    event(log_btn.unchecked_ref(), "click", move |_| {
        let _ = log_diagnostics_to_console();
    })?;

    let copy_btn = button(&actions, "Copy JSON")?;
    copy_btn.set_id("native-debug-copy");
    copy_btn.set_class_name("native-pill debug-action-btn");
    event(copy_btn.unchecked_ref(), "click", move |_| {
        copy_diagnostics_json();
    })?;

    let dl_btn = button(&actions, "Download JSON")?;
    dl_btn.set_id("native-debug-download");
    dl_btn.set_class_name("native-pill debug-action-btn");
    event(dl_btn.unchecked_ref(), "click", move |_| {
        if let Err(e) = download_diagnostics_json() {
            show_error(e);
        }
    })?;

    let tick_btn = button(&actions, "Force Stream Tick")?;
    tick_btn.set_id("native-debug-stream-tick");
    tick_btn.set_class_name("native-pill debug-action-btn");
    event(tick_btn.unchecked_ref(), "click", move |_| {
        let _ = call_stream_hook("forceTick");
    })?;

    let evict_btn = button(&actions, "Test LRU Evict")?;
    evict_btn.set_id("native-debug-evict-lru");
    evict_btn.set_class_name("native-pill debug-action-btn");
    event(evict_btn.unchecked_ref(), "click", move |_| {
        let _ = call_stream_hook("evictLru");
    })?;

    drawer.append_child(&actions)?;

    let out = doc().create_element("pre")?;
    out.set_id("native-debug-output");
    out.set_text_content(Some("window.__webizenDebug ready: snapshot(), log(), forceStreamTick(), evictLru(), toggleStats(), toggleDebug()."));
    drawer.append_child(&out)?;

    Ok(())
}

fn install_window_debug_api() -> Result<(), JsValue> {
    let Some(win) = web_sys::window() else {
        return Ok(());
    };
    let api = js_sys::Object::new();

    let attach_fn = |name: &str, closure: Closure<dyn FnMut() -> JsValue>| -> Result<(), JsValue> {
        js_sys::Reflect::set(&api, &JsValue::from_str(name), closure.as_ref().unchecked_ref())?;
        closure.forget();
        Ok(())
    };

    attach_fn(
        "snapshot",
        Closure::new(move || build_diagnostics_snapshot().into()),
    )?;
    attach_fn("log", Closure::new(move || log_diagnostics_to_console()))?;
    attach_fn(
        "forceStreamTick",
        Closure::new(move || call_stream_hook("forceTick")),
    )?;
    attach_fn(
        "evictLru",
        Closure::new(move || call_stream_hook("evictLru")),
    )?;
    attach_fn(
        "toggleStats",
        Closure::new(move || JsValue::from_bool(toggle_top_drawer(TopDrawer::Stats))),
    )?;
    attach_fn(
        "toggleDebug",
        Closure::new(move || JsValue::from_bool(toggle_top_drawer(TopDrawer::Debug))),
    )?;

    js_sys::Reflect::set(&win, &JsValue::from_str("__webizenDebug"), &api)?;
    Ok(())
}

fn bearing_deg_from_yaw(yaw_rad: f32) -> f64 {
    if !yaw_rad.is_finite() {
        return 0.0;
    }
    (-yaw_rad as f64).to_degrees().rem_euclid(360.0)
}

fn cardinal_from_bearing_deg(deg: f64) -> (&'static str, i32) {
    const DIRS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let norm = if deg.is_finite() {
        deg.rem_euclid(360.0)
    } else {
        0.0
    };
    let rounded = (norm.round() as i32).rem_euclid(360);
    let idx = (((norm + 22.5) / 45.0).floor() as usize) % 8;
    (DIRS[idx], rounded)
}

fn format_compass_heading(deg: f64) -> String {
    let (cardinal, rounded) = cardinal_from_bearing_deg(deg);
    format!("{cardinal} · {rounded:03}°")
}

fn format_corridor_zone_label(raw: Option<&str>, x: f64, z: f64) -> &'static str {
    if let Some(s) = raw {
        let lower = s.trim().to_ascii_lowercase();
        if lower.contains("community") || lower.contains("town") {
            return "Community Grounds";
        }
        if lower.contains("road") {
            return "Country Road";
        }
        if lower.contains("campsite") || lower.contains("forest") {
            return "Forest Campsite";
        }
        if lower.contains("riparian") || lower.contains("creek") {
            return "Riparian Creek";
        }
        if lower.contains("montane") || lower.contains("slope") {
            return "Montane Slope";
        }
    }
    if x >= 300.0 {
        "Community Grounds"
    } else if x >= 100.0 {
        "Country Road"
    } else if z >= 100.0 && x <= 100.0 {
        "Riparian Creek"
    } else {
        "Forest Campsite"
    }
}

fn normalize_ocs_regime_attr(regime: Option<&str>, alt_m: f64) -> &'static str {
    if let Some(r) = regime {
        let lower = r.trim().to_ascii_lowercase();
        if lower.contains("galactic") {
            return "GalacticCosmos";
        }
        if lower.contains("helio") || lower.contains("icrf") || lower.contains("solar") {
            return "HeliocentricIcrf";
        }
        if lower.contains("planet") || lower.contains("wgs84") || lower.contains("earth") {
            return "PlanetaryWgs84";
        }
        if lower.contains("local") || lower.contains("enu") {
            return "LocalEnu";
        }
    }
    if alt_m >= 1.0e12 {
        "GalacticCosmos"
    } else if alt_m >= 1.0e7 {
        "HeliocentricIcrf"
    } else if alt_m >= 1.0e4 {
        "PlanetaryWgs84"
    } else {
        "LocalEnu"
    }
}

fn format_ocs_scale_label(regime: Option<&str>, alt_m: f64) -> String {
    let frame = match normalize_ocs_regime_attr(regime, alt_m) {
        "GalacticCosmos" => "Galactic",
        "HeliocentricIcrf" => "Solar ICRF",
        "PlanetaryWgs84" => "Earth WGS84",
        _ => "Local ENU",
    };
    let alt = if alt_m.is_finite() && alt_m >= 0.0 {
        alt_m
    } else {
        0.0
    };
    let alt_text = if alt < 100.0 {
        format!("{alt:.1} m")
    } else if alt < 1_000.0 {
        format!("{alt:.0} m")
    } else if alt < 1_000_000.0 {
        format!("{:.1} km", alt / 1_000.0)
    } else if alt < 1.0e10 {
        format!("{:.2} Mm", alt / 1_000_000.0)
    } else {
        format!("{:.2} AU", alt / 1.495_978_707e11)
    };
    format!("{frame} · {alt_text}")
}

fn format_surface_traction(raw: Option<&str>) -> &'static str {
    let Some(s) = raw else {
        return "Bitumen";
    };
    let lower = s.trim().to_ascii_lowercase();
    if lower.contains("gravel") {
        "Gravel"
    } else if lower.contains("grass") {
        "Grass"
    } else if lower.contains("silt") || lower.contains("creek") {
        "Silt"
    } else {
        "Bitumen"
    }
}

fn format_drive_camera_label(raw: Option<&str>) -> &'static str {
    let Some(s) = raw else {
        return "Cam: Chase";
    };
    let lower = s.trim().to_ascii_lowercase();
    if lower.contains("bonnet") || lower.contains("hood") {
        "Cam: Bonnet"
    } else if lower.contains("cockpit") || lower.contains("driver") {
        "Cam: Cockpit"
    } else {
        "Cam: Chase"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compass_bearing_maps_orbit_yaw_to_cardinal_directions() {
        assert_eq!(format_compass_heading(bearing_deg_from_yaw(0.0)), "N · 000°");
        assert_eq!(
            format_compass_heading(bearing_deg_from_yaw(-std::f32::consts::FRAC_PI_2)),
            "E · 090°"
        );
        assert_eq!(
            format_compass_heading(bearing_deg_from_yaw(std::f32::consts::PI)),
            "S · 180°"
        );
        assert_eq!(
            format_compass_heading(bearing_deg_from_yaw(std::f32::consts::FRAC_PI_2)),
            "W · 270°"
        );
        assert_eq!(format_compass_heading(359.8), "N · 000°");
    }

    #[test]
    fn corridor_zone_and_ocs_scale_format_cleanly() {
        assert_eq!(
            format_corridor_zone_label(Some("ForestCampsite"), 0.0, 0.0),
            "Forest Campsite"
        );
        assert_eq!(
            format_corridor_zone_label(Some("RoadCorridor"), 150.0, 50.0),
            "Country Road"
        );
        assert_eq!(
            format_corridor_zone_label(Some("TownOutskirtsCommunityGrounds"), 340.0, 56.0),
            "Community Grounds"
        );
        assert_eq!(
            format_corridor_zone_label(None, 180.0, 50.0),
            "Country Road"
        );

        assert_eq!(
            format_ocs_scale_label(Some("LocalEnu"), 1.70),
            "Local ENU · 1.7 m"
        );
        assert_eq!(
            format_ocs_scale_label(Some("PlanetaryWgs84"), 420_000.0),
            "Earth WGS84 · 420.0 km"
        );
        assert_eq!(
            format_ocs_scale_label(Some("HeliocentricIcrf"), 1.495_978_707e11),
            "Solar ICRF · 1.00 AU"
        );
    }

    #[test]
    fn vehicle_surface_and_camera_labels_normalize_variants() {
        assert_eq!(format_surface_traction(Some("BitumenRoad")), "Bitumen");
        assert_eq!(format_surface_traction(Some("GravelTrack")), "Gravel");
        assert_eq!(format_surface_traction(Some("GrassClearing")), "Grass");
        assert_eq!(format_surface_traction(Some("AlluvialSilt")), "Silt");
        assert_eq!(format_surface_traction(Some("CreekBank")), "Silt");

        assert_eq!(format_drive_camera_label(Some("ChaseFollow")), "Cam: Chase");
        assert_eq!(format_drive_camera_label(Some("BonnetHood")), "Cam: Bonnet");
        assert_eq!(
            format_drive_camera_label(Some("CockpitDriver")),
            "Cam: Cockpit"
        );
    }
}
