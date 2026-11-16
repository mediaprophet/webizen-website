//! Native in-experience proof laboratory; no authored JavaScript controller.
mod data;
mod fields;
mod measures;
mod production;
mod references;
mod science;
use super::*;
use crate::proof;
type Lab = Rc<RefCell<LabState>>;
struct LabState {
    catalogue: Value,
    descriptor: Value,
    dataset: Option<proof::Dataset>,
    result: Value,
    page: usize,
    job: u64,
    busy: bool,
    world: Shared,
}
fn add(parent: &Element, tag: &str, id: &str, value: &str) -> Result<Element, JsValue> {
    let e = doc().create_element(tag)?;
    if !id.is_empty() {
        e.set_id(id);
    }
    e.set_text_content(Some(value));
    parent.append_child(&e)?;
    Ok(e)
}
fn button(parent: &Element, id: &str, label: &str) -> Result<Element, JsValue> {
    let b = add(parent, "button", id, label)?;
    b.set_attribute("type", "button")?;
    Ok(b)
}
fn input(
    parent: &Element,
    id: &str,
    label: &str,
    kind: &str,
    value: &str,
) -> Result<HtmlInputElement, JsValue> {
    let l = add(parent, "label", "", label)?;
    l.set_attribute("for", id)?;
    let e = add(parent, "input", id, "")?.dyn_into::<HtmlInputElement>()?;
    e.set_type(kind);
    e.set_value(value);
    Ok(e)
}
fn input_value(id: &str) -> String {
    doc()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .map(|e| e.value())
        .unwrap_or_default()
}
fn select_value(id: &str) -> String {
    doc()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
        .map(|e| e.value())
        .unwrap_or_default()
}
fn option(parent: &Element, value: &str, label: &str) -> Result<(), JsValue> {
    let e = add(parent, "option", "", label)?;
    e.set_attribute("value", value)
}
fn error(e: JsValue) {
    web_sys::console::error_1(&e);
    if let Some(status) = doc().get_element_by_id("native-lab-status") {
        let _ = status.set_attribute("data-state", "error");
        let _ = status.set_attribute("role", "alert");
    }
    text(
        "native-lab-status",
        &e.as_string()
            .unwrap_or("The operation could not complete".into()),
    );
}
fn table(parent: &Element, heads: &[&str], rows: &[Vec<String>]) -> Result<(), JsValue> {
    let t = add(parent, "table", "", "")?;
    let h = add(&t, "thead", "", "")?;
    let tr = add(&h, "tr", "", "")?;
    for name in heads {
        let th = add(&tr, "th", "", name)?;
        th.set_attribute("scope", "col")?;
    }
    let body = add(&t, "tbody", "", "")?;
    for row in rows {
        let tr = add(&body, "tr", "", "")?;
        for v in row {
            add(&tr, "td", "", v)?;
        }
    }
    Ok(())
}
fn publish(state: &Lab, value: Value, summary: &str) -> Result<(), JsValue> {
    if let Some(status) = doc().get_element_by_id("native-lab-status") {
        status.remove_attribute("data-state")?;
        status.set_attribute("role", "status")?;
    }
    let result = doc()
        .get_element_by_id("native-lab-results")
        .ok_or(err("Missing lab results"))?;
    result.set_text_content(None);
    add(&result, "h3", "", summary)?;
    let encoded = js_sys::encode_uri_component(&value.to_string());
    let a = add(
        &result,
        "a",
        "native-lab-export",
        "Export this result and evidence",
    )?;
    a.set_attribute(
        "href",
        &format!("data:application/json;charset=utf-8,{encoded}"),
    )?;
    a.set_attribute("download", "webizen-native-result.json")?;
    state.borrow_mut().result = value.clone();
    js_sys::Reflect::set(
        &web_sys::window().unwrap(),
        &"__webizenNativeInstrument".into(),
        &js_sys::JSON::parse(&value.to_string())?,
    )?;
    Ok(())
}
fn evidence(parent: &Element, value: &Value) -> Result<(), JsValue> {
    let d = add(parent, "details", "", "")?;
    add(&d, "summary", "", "Inspect native execution receipt")?;
    add(
        &d,
        "pre",
        "",
        &serde_json::to_string_pretty(value).map_err(err)?,
    )?;
    Ok(())
}
async fn yield_browser() -> Result<(), JsValue> {
    let p = js_sys::Promise::new(&mut |resolve, _| {
        let cb = Closure::once(move || {
            let _ = resolve.call0(&JsValue::NULL);
        });
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 0);
        cb.forget();
    });
    JsFuture::from(p).await?;
    Ok(())
}
pub(super) async fn install(world: Shared) -> Result<(), JsValue> {
    let root = element("native-proof-lab", "details")?;
    root.set_class_name("native-proof-lab");
    add(&root, "summary", "", "Open the proof laboratory")?;
    add(&root, "h2", "", "Explore the evidence")?;
    add(&root,"p","","Inspect real public data, change a query, test a permission scenario or calculate Earth and sky coordinates. Data instruments run the native QualiaDB engines in your browser; Make with Synth uses this site's native production service.")?;
    let status = add(
        &root,
        "p",
        "native-lab-status",
        "Opening the dataset catalogue...",
    )?;
    status.set_attribute("role", "status")?;
    status.set_attribute("aria-live", "polite")?;
    let state = Rc::new(RefCell::new(LabState {
        catalogue: Value::Null,
        descriptor: Value::Null,
        dataset: None,
        result: Value::Null,
        page: 0,
        job: 0,
        busy: false,
        world,
    }));
    let nav = add(&root, "nav", "native-lab-tabs", "")?;
    nav.set_attribute("aria-label", "Laboratory instruments")?;
    for (id, label) in [
        ("data", "Data and queries"),
        ("rights", "Rights and consent"),
        ("space", "Earth and sky"),
        ("checks", "Tests and benchmarks"),
        ("fields", "Light and fields"),
        ("production", "Make with Synth"),
    ] {
        let b = button(&nav, &format!("native-lab-tab-{id}"), label)?;
        b.set_attribute("aria-controls", &format!("native-lab-{id}"))?;
        b.set_attribute("aria-current", if id == "data" { "true" } else { "false" })?;
        let section = add(&root, "section", &format!("native-lab-{id}"), "")?;
        if id != "data" {
            section.set_attribute("hidden", "")?;
        }
        let selected = id.to_string();
        event(b.unchecked_ref(), "click", move |_| {
            for key in ["data", "rights", "space", "checks", "fields", "production"] {
                if let Some(tab) = doc().get_element_by_id(&format!("native-lab-tab-{key}")) {
                    let _ = tab.set_attribute(
                        "aria-current",
                        if key == selected { "true" } else { "false" },
                    );
                }
                if let Some(e) = doc().get_element_by_id(&format!("native-lab-{key}")) {
                    let _ = if key == selected {
                        e.remove_attribute("hidden")
                    } else {
                        e.set_attribute("hidden", "")
                    };
                }
            }
        })?;
    }
    let results = add(&root, "section", "native-lab-results", "")?;
    results.set_attribute("aria-label", "Instrument results")?;
    data::controls(&state)?;
    science::controls(&state)?;
    measures::controls(&state)?;
    fields::controls(&state)?;
    let catalogue = json_file("/data/publications/aaa/catalogue.json", 1024 * 1024).await?;
    if catalogue["schema"] != "webizen.proof-data/v1" {
        return Err(err("Unsupported proof dataset catalogue"));
    }
    let rows = catalogue["datasets"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 32)
        .ok_or(err("Dataset catalogue must contain 1..32 descriptors"))?;
    let selector = doc()
        .get_element_by_id("native-dataset")
        .ok_or(err("Missing dataset selector"))?;
    for d in rows {
        option(
            &selector,
            d["id"].as_str().ok_or(err("Dataset has no ID"))?,
            d["title"].as_str().ok_or(err("Dataset has no title"))?,
        )?;
    }
    let first = rows[0].clone();
    state.borrow_mut().catalogue = catalogue;
    data::load(&state, first).await?;
    references::install(&state).await?;
    if let Err(e) = production::install(&state).await {
        error(e);
    }
    Ok(())
}
