use super::*;

pub(super) fn install_input(state: &Shared) -> Result<(), JsValue> {
    let canvas = state.borrow().canvas.clone();
    canvas.set_attribute("tabindex", "0")?;
    canvas.set_attribute(
        "aria-label",
        "Interactive native world. Use arrow keys to look around, plus and minus to zoom.",
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
                if id != p.pointer_id() || now() - w.last_input < 40. {
                    return;
                }
                let nx = p.client_x() as f32;
                let ny = p.client_y() as f32;
                w.drag = Some((id, nx, ny, sx, sy));
                w.last_input = now();
                let kind = if p.shift_key() { "pan" } else { "orbit" };
                if let Err(e) = camera_input(&mut w, kind, nx - x, ny - y) {
                    show_error(e)
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
                    && (p.client_x() as f32 - sx).abs() + (p.client_y() as f32 - sy).abs() < 6.
                {
                    let rect = w.canvas.get_bounding_client_rect();
                    let width = w.canvas.width();
                    let height = w.canvas.height();
                    let x = (p.client_x() as f64 - rect.left()) * width as f64 / rect.width();
                    let y = (p.client_y() as f64 - rect.top()) * height as f64 / rect.height();
                    if let Err(e) = w.portal.select_node_at(x as f32, y as f32, width, height) {
                        show_error(e)
                    }
                    w.pending_pick = true;
                }
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
            if let Err(e) = camera_input(
                &mut s.borrow_mut(),
                "zoom",
                0.,
                e.delta_y().clamp(-400., 400.) as f32,
            ) {
                show_error(e)
            }
        }
    })?;
    let s = state.clone();
    event(canvas.unchecked_ref(), "keydown", move |e| {
        if let Ok(e) = e.dyn_into::<KeyboardEvent>() {
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
    Ok(())
}
