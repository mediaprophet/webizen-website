use super::*;
pub(super) fn install(state: &Shared) -> Result<(), JsValue> {
    let settings = state.borrow().receipt["presentation_settings"].clone();
    let ui = element("native-world-ui", "aside")?;
    ui.set_class_name("native-panel");
    ui.set_attribute("aria-label", "Explore the reference model")?;
    let h = doc().create_element("h1")?;
    h.set_text_content(Some(
        settings["scene_title"]
            .as_str()
            .unwrap_or("Explore the reference model"),
    ));
    ui.append_child(&h)?;
    let p = doc().create_element("p")?;
    p.set_text_content(Some(settings["scene_description"].as_str().unwrap_or("This model is separate from the authored living-world landscape. Inspect its source in the proof laboratory.")));
    ui.append_child(&p)?;
    let controls = doc().create_element("nav")?;
    controls.set_id("native-camera-controls");
    controls.set_class_name("native-camera-controls");
    controls.set_attribute("aria-label", "Camera controls")?;
    for (label, glyph, dy) in [
        ("Move closer", "+", -200.),
        ("Move further away", "-", 200.),
    ] {
        let b = button(&controls, glyph)?;
        b.set_attribute("aria-label", label)?;
        let s = state.clone();
        event(b.unchecked_ref(), "click", move |_| {
            if let Err(e) = camera_input(&mut s.borrow_mut(), "zoom", 0., dy) {
                show_error(e)
            }
        })?;
    }
    ui.append_child(&controls)?;
    let label = doc().create_element("label")?;
    let check = doc()
        .create_element("input")?
        .dyn_into::<HtmlInputElement>()?;
    check.set_type("checkbox");
    check.set_checked(state.borrow().reduced);
    label.append_child(&check)?;
    let span = doc().create_element("span")?;
    span.set_text_content(Some(" Reduce motion"));
    label.append_child(&span)?;
    ui.append_child(&label)?;
    let s = state.clone();
    event(check.unchecked_ref(), "change", move |e| {
        if let Some(input) = e
            .target()
            .and_then(|v| v.dyn_into::<HtmlInputElement>().ok())
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
    help.set_text_content(Some("Drag or use arrow keys to change your view. Use the wheel or camera buttons to move closer. Open the proof laboratory to inspect the actual data and calculations."));
    ui.append_child(&help)?;
    let mut w = state.borrow_mut();
    let rect = w.canvas.get_bounding_client_rect();
    let aspect = rect.width().max(1.) / rect.height().max(1.);
    let camera = serde_json::to_value(&w.camera).map_err(err)?;
    let fitted = eval(&mut w, "fit_view", &[camera, json!(aspect)])?;
    let camera: Camera = serde_json::from_value(fitted).map_err(err)?;
    camera.validate().map_err(err)?;
    w.camera = camera.clone();
    w.goal = camera;
    w.receipt["scene_mode"] = json!("reference-model");
    set_camera(&mut w)?;
    Ok(())
}
