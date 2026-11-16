//! Actual typed EMF admission and an isolated native diagnostic canvas.
use super::*;
use qualia_core_db::render::portal_emf_state::LoadedEmf;
struct Field {
    portal: Option<QualiaPortal>,
    loaded: Option<LoadedEmf>,
    manifest: Value,
    receipt: Value,
    page: usize,
}
type State = Rc<RefCell<Field>>;
fn value(v: JsValue) -> Result<Value, JsValue> {
    serde_json::from_str(
        &js_sys::JSON::stringify(&v)?
            .as_string()
            .ok_or(err("Invalid field receipt"))?,
    )
    .map_err(err)
}
pub(super) fn controls(lab: &Lab) -> Result<(), JsValue> {
    let root = doc()
        .get_element_by_id("native-lab-fields")
        .ok_or(err("Missing field instrument"))?;
    add(&root, "h3", "", "Explore a light-field model")?;
    add(&root,"p","","A constructed optical field lets you inspect how electric-field strength varies across a tiny local volume. These are model values, not measurements of a place or person.")?;
    let state = Rc::new(RefCell::new(Field {
        portal: None,
        loaded: None,
        manifest: Value::Null,
        receipt: Value::Null,
        page: 0,
    }));
    let load = button(&root, "native-field-load", "Open the light-field model")?;
    let s = state.clone();
    let l = lab.clone();
    event(load.unchecked_ref(), "click", move |_| {
        let s = s.clone();
        let l = l.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if l.borrow().busy {
                error(err("Finish or cancel the current operation first"));
                return;
            }
            l.borrow_mut().busy = true;
            let result = open(&s, &l).await;
            l.borrow_mut().busy = false;
            if let Err(e) = result {
                error(e)
            }
        });
    })?;
    let gain = input(
        &root,
        "native-field-gain",
        "Diagnostic display gain",
        "number",
        "1",
    )?;
    gain.set_min("0.000001");
    gain.set_max("1000000");
    gain.set_step("0.25");
    let apply = button(&root, "native-field-apply", "Apply display gain")?;
    let s = state.clone();
    let l = lab.clone();
    event(apply.unchecked_ref(), "click", move |_| {
        if l.borrow().busy {
            error(err("Finish or cancel the current operation first"));
            return;
        }
        if let Err(e) = draw(&s, &l) {
            error(e)
        }
    })?;
    let visible = button(&root, "native-field-toggle", "Hide preview")?;
    event(visible.unchecked_ref(), "click", move |event| {
        if let Some(canvas) = doc().get_element_by_id("native-field-canvas") {
            let hidden = canvas.has_attribute("hidden");
            let _ = if hidden {
                canvas.remove_attribute("hidden")
            } else {
                canvas.set_attribute("hidden", "")
            };
            if let Some(button) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                button.set_text_content(Some(if hidden {
                    "Hide preview"
                } else {
                    "Show preview"
                }));
            }
        }
    })?;
    let canvas =
        add(&root, "canvas", "native-field-canvas", "")?.dyn_into::<HtmlCanvasElement>()?;
    canvas.set_width(512);
    canvas.set_height(512);
    canvas.set_attribute(
        "aria-label",
        "Diagnostic optical field slice, z zero and time zero",
    )?;
    canvas.set_attribute("style", "width:min(100%,512px);height:auto;aspect-ratio:1")?;
    add(&root,"p","","The preview shows electric-field magnitude in greyscale at z = 0 and time = 0. Display gain changes its brightness only; the retained field values and their units stay unchanged.")?;
    let link = add(
        &root,
        "a",
        "native-field-download",
        "Download the original field (.emf)",
    )?;
    link.set_attribute("download", "")?;
    add(&root, "p", "native-field-summary", "")?;
    add(&root, "div", "native-field-values", "")?;
    for (id, label, next) in [
        ("native-field-prev", "Previous 24 cells", false),
        ("native-field-next", "Next 24 cells", true),
    ] {
        let b = button(&root, id, label)?;
        let s = state.clone();
        event(b.unchecked_ref(), "click", move |_| {
            let mut f = s.borrow_mut();
            let count = f
                .loaded
                .as_ref()
                .map(|x| x.display_cells().len())
                .unwrap_or(0);
            if next {
                if (f.page + 1) * 24 < count {
                    f.page += 1;
                }
            } else {
                f.page = f.page.saturating_sub(1);
            }
            drop(f);
            if let Err(e) = rows(&s) {
                error(e)
            }
        })?;
    }
    Ok(())
}
async fn open(s: &State, lab: &Lab) -> Result<(), JsValue> {
    text(
        "native-lab-status",
        "Opening and verifying the field model...",
    );
    let manifest = json_file("/production/aaa-field/manifest.json", 1024 * 1024).await?;
    if manifest["schema"] != "webizen.emf-proof/v1" {
        return Err(err("Unsupported field publication"));
    }
    let url = manifest["emf"]["url"]
        .as_str()
        .ok_or(err("Field has no .emf URL"))?;
    let raw = bytes(url, 16 * 1024 * 1024).await?;
    if hash(&raw) != manifest["emf"]["sha256"].as_str().unwrap_or("") {
        return Err(err("Field digest mismatch"));
    }
    let loaded =
        qualia_core_db::render::portal_load_emf::decode_emf_scientific(&raw).map_err(err)?;
    if loaded.display_cells().len() > 65_536 {
        return Err(err(
            "This field instrument accepts at most 65,536 cells; no sampling is applied",
        ));
    }
    let init = if s.borrow().portal.is_none() {
        let canvas = doc()
            .get_element_by_id("native-field-canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()?;
        let report = super::super::graphics::initialise(canvas.clone()).await?;
        let portal = QualiaPortal::new(canvas)?;
        s.borrow_mut().portal = Some(portal);
        Some(report)
    } else {
        None
    };
    let mut f = s.borrow_mut();
    let portal = f.portal.as_mut().unwrap();
    let inspection = value(portal.inspect_emf(&raw)?)?;
    let admission = value(portal.load_emf(&raw)?)?;
    f.receipt = json!({"schema":"webizen.field-instrument-run/v1","source":manifest,"source_sha256":hash(&raw),"inspection":inspection,"admission":admission,"graphics_initialisation":init,"interpretation":"authored model; local physical field; displayed and exported cell table is the native display derivative","served_build_identity":lab.borrow().world.borrow().receipt["served_build_identity"]});
    f.manifest = manifest.clone();
    f.loaded = Some(loaded);
    f.page = 0;
    drop(f);
    doc()
        .get_element_by_id("native-field-download")
        .unwrap()
        .set_attribute("href", url)?;
    draw(s, lab)
}
fn draw(s: &State, lab: &Lab) -> Result<(), JsValue> {
    let gain: f32 = input_value("native-field-gain")
        .parse()
        .map_err(|_| err("Enter a finite positive display gain"))?;
    let mut f = s.borrow_mut();
    let portal = f.portal.as_mut().ok_or(err("Open the field model first"))?;
    let draw = value(portal.render_emf_preview(gain)?)?;
    let state = portal.emf_display_state();
    f.receipt["draw"] = draw;
    f.receipt["display_state"] = json!(state);
    let loaded = f.loaded.as_ref().ok_or(err("No admitted field values"))?;
    let cells:Vec<Value>=loaded.display_cells().iter().enumerate().map(|(i,c)|{let optical=loaded.optical_cells().get(i);json!({"index":i,"electric_magnitude_v_per_m":c.magnitude_v_per_m,"valid":c.valid,"optical_display":optical.map(|o|json!({"stokes_i":o.stokes_i,"stokes_q":o.stokes_q,"stokes_u":o.stokes_u,"stokes_v":o.stokes_v,"spectral_sigma":o.spectral_sigma,"carrier_hz":o.carrier_hz,"phase_rad":o.phase_rad,"coherence":o.coherence,"valid":o.valid}))})}).collect();
    f.receipt["dimensions"] = json!(loaded.dimensions());
    f.receipt["derived_cells"] = json!(cells);
    let receipt = f.receipt.clone();
    drop(f);
    publish(
        lab,
        receipt.clone(),
        "Field admitted and diagnostic preview submitted",
    )?;
    let result = doc().get_element_by_id("native-lab-results").unwrap();
    let mut concise = receipt.clone();
    concise.as_object_mut().unwrap().remove("derived_cells");
    concise["cell_export"]=json!("All admitted derived cells are included in Export this result and evidence; original physical data remain in the .emf download.");
    evidence(&result, &concise)?;
    rows(s)?;
    text(
        "native-lab-status",
        "Field values verified. Change the display gain or explore the cells below.",
    );
    Ok(())
}
fn rows(s: &State) -> Result<(), JsValue> {
    let f = s.borrow();
    let loaded = f.loaded.as_ref().ok_or(err("Open the field model first"))?;
    let start = f.page * 24;
    let end = (start + 24).min(loaded.display_cells().len());
    text("native-field-summary",&format!("{} cells in a {:?} grid. Showing {} to {}. Values below are electric-field magnitude in V/m.",loaded.cell_count(),loaded.dimensions(),if end>0{start+1}else{0},end));
    let parent = doc().get_element_by_id("native-field-values").unwrap();
    parent.set_text_content(None);
    table(
        &parent,
        &["Cell", "Electric field (V/m)", "Valid"],
        &loaded.display_cells()[start..end]
            .iter()
            .enumerate()
            .map(|(i, c)| {
                vec![
                    (start + i).to_string(),
                    format!("{:.7}", c.magnitude_v_per_m),
                    c.valid.to_string(),
                ]
            })
            .collect::<Vec<_>>(),
    )?;
    Ok(())
}
