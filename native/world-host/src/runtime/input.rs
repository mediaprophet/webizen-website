use super::locomotion::MovementMode;
use super::*;

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

fn is_interactive_control_target(target: Option<web_sys::EventTarget>) -> bool {
    let Some(el) = target.and_then(|t| t.dyn_into::<Element>().ok()) else {
        return false;
    };
    let tag = el.tag_name();
    tag.eq_ignore_ascii_case("BUTTON")
        || tag.eq_ignore_ascii_case("A")
        || tag.eq_ignore_ascii_case("SUMMARY")
        || is_editable_target(Some(el.into()))
}

pub(super) fn install_input(state: &Shared) -> Result<(), JsValue> {
    let canvas = state.borrow().canvas.clone();
    canvas.set_attribute("tabindex", "0")?;
    canvas.set_attribute(
        "aria-label",
        "Interactive native world. Press 1 Observe, 2 Walk, 3 Fly, R Rhizosphere X-Ray, E Inspect, arrow keys or drag to look.",
    )?;
    canvas.set_attribute("style", "touch-action:none")?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "pointerdown", move |e| {
        if let Ok(p) = e.dyn_into::<PointerEvent>() {
            let mut w = s.borrow_mut();
            let _ = w.canvas.set_pointer_capture(p.pointer_id());
            w.drag = Some((
                p.pointer_id(),
                p.client_x() as f32,
                p.client_y() as f32,
                p.client_x() as f32,
                p.client_y() as f32,
            ));
        }
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "pointermove", move |e| {
        if let Ok(p) = e.dyn_into::<PointerEvent>() {
            let mut w = s.borrow_mut();
            if let Some((id, x, y, sx, sy)) = w.drag {
                if id != p.pointer_id() || now() - w.last_input < 12. {
                    return;
                }
                let nx = p.client_x() as f32;
                let ny = p.client_y() as f32;
                w.drag = Some((id, nx, ny, sx, sy));
                w.last_input = now();
                match super::locomotion::apply_look_input(&mut w, nx - x, ny - y) {
                    Ok(true) => {}
                    Ok(false) => {
                        let kind = if p.shift_key() { "pan" } else { "orbit" };
                        if let Err(e) = camera_input(&mut w, kind, nx - x, ny - y) {
                            show_error(e)
                        }
                    }
                    Err(e) => show_error(e),
                }
            }
        }
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "pointerup", move |e| {
        if let Ok(p) = e.dyn_into::<PointerEvent>() {
            let mut w = s.borrow_mut();
            if let Some((id, _, _, sx, sy)) = w.drag.take() {
                let _ = w.canvas.release_pointer_capture(id);
                if w.receipt["presentation_settings"]["scene_mode"] != "reference-model"
                    && (p.client_x() as f32 - sx).abs() + (p.client_y() as f32 - sy).abs() < 12.
                {
                    let rect = w.canvas.get_bounding_client_rect();
                    let width = w.canvas.width();
                    let height = w.canvas.height();
                    let x = (p.client_x() as f64 - rect.left()) * width as f64 / rect.width();
                    let y = (p.client_y() as f64 - rect.top()) * height as f64 / rect.height();
                    let interactive_mode =
                        w.play_mode || super::locomotion::current_mode() != MovementMode::ObserveOrbit;
                    if interactive_mode {
                        if let Err(e) =
                            super::play::pick_at(&mut w, x as f32, y as f32, width, height)
                        {
                            show_error(e)
                        }
                    } else if let Err(e) =
                        w.portal.select_node_at(x as f32, y as f32, width, height)
                    {
                        show_error(e)
                    } else {
                        w.pending_pick = true;
                        w.pending_pick_age = 0;
                    }
                }
            }
        }
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "click", move |e| {
        if let Ok(m) = e.dyn_into::<MouseEvent>() {
            let mut w = s.borrow_mut();
            if w.receipt["presentation_settings"]["scene_mode"] == "reference-model" {
                return;
            }
            let interactive_mode =
                w.play_mode || super::locomotion::current_mode() != MovementMode::ObserveOrbit;
            if !interactive_mode && w.pending_pick {
                return;
            }
            let rect = w.canvas.get_bounding_client_rect();
            let width = w.canvas.width();
            let height = w.canvas.height();
            let x = (m.client_x() as f64 - rect.left()) * width as f64 / rect.width();
            let y = (m.client_y() as f64 - rect.top()) * height as f64 / rect.height();
            if interactive_mode {
                if let Err(e) = super::play::pick_at(&mut w, x as f32, y as f32, width, height) {
                    show_error(e)
                }
            } else if let Err(e) = w.portal.select_node_at(x as f32, y as f32, width, height) {
                show_error(e)
            } else {
                w.pending_pick = true;
                w.pending_pick_age = 0;
            }
        }
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "pointercancel", move |_| {
        s.borrow_mut().drag = None
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "wheel", move |e| {
        e.prevent_default();
        if let Ok(e) = e.dyn_into::<WheelEvent>() {
            let mut w = s.borrow_mut();
            let dy = e.delta_y().clamp(-400., 400.) as f32;
            match super::locomotion::apply_zoom_input(&mut w, dy) {
                Ok(true) => {}
                Ok(false) => {
                    if let Err(e) = camera_input(&mut w, "zoom", 0., dy) {
                        show_error(e)
                    }
                }
                Err(e) => show_error(e),
            }
        }
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "keydown", move |e| {
        if let Ok(e) = e.dyn_into::<KeyboardEvent>() {
            if super::locomotion::current_mode() != MovementMode::ObserveOrbit {
                return;
            }
            let input = match e.key().as_str() {
                "ArrowLeft" => Some(("orbit", -30., 0.)),
                "ArrowRight" => Some(("orbit", 30., 0.)),
                "ArrowUp" => Some(("orbit", 0., -25.)),
                "ArrowDown" => Some(("orbit", 0., 25.)),
                "+" | "=" => Some(("zoom", 0., -90.)),
                "-" => Some(("zoom", 0., 90.)),
                _ => None,
            };
            if let Some((kind, dx, dy)) = input {
                e.prevent_default();
                if let Err(e) = camera_input(&mut s.borrow_mut(), kind, dx, dy) {
                    show_error(e)
                }
            }
        }
    })?;

    // Global keyboard shortcuts (1/2/3 modes, R Rhizosphere X-Ray, E Inspect) & fallback movement.
    if let Some(win) = web_sys::window() {
        let s = state.clone();
        event(win.unchecked_ref(), "keydown", move |e| {
            let Ok(k) = e.dyn_into::<KeyboardEvent>() else {
                return;
            };
            if is_editable_target(k.target()) {
                return;
            }
            let key = k.key();
            if !k.ctrl_key() && !k.meta_key() && !k.alt_key() {
                let handled = match key.as_str() {
                    "1" => {
                        k.prevent_default();
                        let mut w = s.borrow_mut();
                        if let Err(err) =
                            super::locomotion::set_mode(&mut w, MovementMode::ObserveOrbit)
                        {
                            show_error(err);
                        }
                        text("native-world-status", "Observe mode: orbit, chapters, and waypoints.");
                        true
                    }
                    "2" => {
                        k.prevent_default();
                        let mut w = s.borrow_mut();
                        if let Err(err) =
                            super::locomotion::set_mode(&mut w, MovementMode::WalkExplore)
                        {
                            show_error(err);
                        }
                        text(
                            "native-world-status",
                            "Walk mode: WASD or Arrows to walk, Shift sprint, Space hop, E inspect, R Rhizosphere X-Ray.",
                        );
                        true
                    }
                    "3" => {
                        k.prevent_default();
                        let mut w = s.borrow_mut();
                        if let Err(err) =
                            super::locomotion::set_mode(&mut w, MovementMode::FlyGlide)
                        {
                            show_error(err);
                        }
                        text(
                            "native-world-status",
                            "Fly mode: Space ascend, C or Ctrl descend, climb from creek to orbit.",
                        );
                        true
                    }
                    "r" | "R" => {
                        k.prevent_default();
                        super::locomotion::toggle_rhizosphere_xray(&mut s.borrow_mut());
                        true
                    }
                    "e" | "E" => {
                        k.prevent_default();
                        if let Err(err) = super::play::interact_nearest(&mut s.borrow_mut()) {
                            show_error(err);
                        }
                        true
                    }
                    _ => false,
                };
                if handled {
                    return;
                }
            }
            if !is_interactive_control_target(k.target())
                && super::locomotion::keydown_mods(
                    &mut s.borrow_mut(),
                    &key,
                    k.shift_key(),
                    k.ctrl_key(),
                )
            {
                k.prevent_default();
            }
        })?;
        let s = state.clone();
        event(win.unchecked_ref(), "keyup", move |e| {
            if let Ok(k) = e.dyn_into::<KeyboardEvent>() {
                if !is_editable_target(k.target()) {
                    let _ = super::locomotion::keyup(&mut s.borrow_mut(), &k.key());
                }
            }
        })?;
        event(win.unchecked_ref(), "blur", move |_| {
            super::locomotion::clear_all_inputs();
        })?;
    }
    Ok(())
}
