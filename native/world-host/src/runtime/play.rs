//! Play mode: the visitor shifts from observer to agent.
//!
//! Play is a host stance, not a sequence action (it does not occupy an
//! action slot). Clicking a mesh opens a toolbox of permitted Q42 actions
//! authored `rel:aboutGroup` that group. SPARQL is queried at pick time;
//! QuinIndex `about` is the source of truth when SPARQL returns no rows.
//! Do not mint camp.rs. Do not special-case group ids in this file.

use super::*;
use qualia_core_db::mini_parser::hash_token;
use qualia_core_db::render::camera::{orbit_view_projection_with_near, CAMERA_NEAR_PLANE};
use std::collections::BTreeSet;

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

pub(super) fn follow_camera(w: &mut World) -> Result<(), JsValue> {
    if !w.play_mode {
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
    text("native-world-status", PLAY_STATUS);
    Ok(())
}

fn leave_play(state: &Shared) -> Result<(), JsValue> {
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

fn fill_toolbox(w: &mut World, placement_id: &str, asset_id: &str) -> Result<(), JsValue> {
    let group_id = w
        .macros
        .as_ref()
        .and_then(|live| live.group_id_for_asset(asset_id))
        .map(str::to_string);
    let Some(group_id) = group_id else {
        w.play_focus_group = None;
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
        if asset.id == "gpu-suffix" {
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

fn html_class(add: bool, name: &str) {
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
