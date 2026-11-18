mod download;
mod reference;
use super::*;

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
    help.set_text_content(Some("Drag to look around; use the wheel or + and - to move closer. Arrow keys change your view. Choose a place to visit it."));
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
        if let Ok(e) = e.dyn_into::<KeyboardEvent>() {
            if super::locomotion::keydown(&mut s.borrow_mut(), &e.key()) {
                e.prevent_default();
                e.stop_immediate_propagation();
            }
        }
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "keyup", move |e| {
        if let Ok(e) = e.dyn_into::<KeyboardEvent>() {
            if super::locomotion::keyup(&mut s.borrow_mut(), &e.key()) {
                e.prevent_default();
                e.stop_immediate_propagation();
            }
        }
    })?;
    Ok(())
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
    event(button.unchecked_ref(), "click", move |event| {
        if let Some(panel) = doc().get_element_by_id("native-world-ui") {
            let show = panel.has_attribute("hidden");
            let _ = if show {
                panel.remove_attribute("hidden")
            } else {
                panel.set_attribute("hidden", "")
            };
            if let Some(button) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                button.set_text_content(Some(if show { "Hide panel" } else { "Show panel" }));
                let _ = button.set_attribute("aria-expanded", if show { "true" } else { "false" });
                let _ = button.set_attribute(
                    "aria-label",
                    if show {
                        "Hide the reading and path panel"
                    } else {
                        "Show the reading and path panel"
                    },
                );
            }
        }
    })
}
