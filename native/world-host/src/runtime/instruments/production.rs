//! Bounded native client for genuine same-origin Synth production.
use super::*;
struct Production {
    recipes: Vec<Value>,
    generation: u64,
    active: bool,
}
type State = Rc<RefCell<Production>>;
pub(super) async fn install(lab: &Lab) -> Result<(), JsValue> {
    let root = doc()
        .get_element_by_id("native-lab-production")
        .ok_or(err("Missing production instrument"))?;
    add(
        &root,
        "h3",
        "",
        "Make something, then inspect how it was made",
    )?;
    add(&root,"p","","These examples run Synth on this site's production service. Your source is sent to that service when you choose Make. Every returned file is checked against its digest before it is shown. Sound plays only when you choose Play.")?;
    let catalogue = json_file(
        "/data/publications/aaa-production/catalogue.json",
        256 * 1024,
    )
    .await?;
    if catalogue["schema"] != "webizen.production-recipes/v1" {
        return Err(err("Unknown recipe catalogue"));
    }
    let recipes = catalogue["recipes"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 16)
        .ok_or(err("Invalid recipe count"))?
        .clone();
    for r in &recipes {
        if hash(r["source"].as_str().unwrap_or("").as_bytes())
            != r["source_sha256"].as_str().unwrap_or("")
        {
            return Err(err("Recipe source digest mismatch"));
        }
    }
    let state = Rc::new(RefCell::new(Production {
        recipes,
        generation: 0,
        active: false,
    }));
    let label = add(&root, "label", "", "Choose something to make")?;
    label.set_attribute("for", "native-production-recipe")?;
    let select = add(&root, "select", "native-production-recipe", "")?;
    for r in &state.borrow().recipes {
        option(
            &select,
            r["id"].as_str().unwrap_or(""),
            r["title"].as_str().unwrap_or(""),
        )?;
    }
    add(&root, "p", "native-production-description", "")?;
    add(&root, "div", "native-production-parameters", "")?;
    let details = add(&root, "details", "", "")?;
    add(
        &details,
        "summary",
        "",
        "Read or edit the VibeScript source",
    )?;
    let custom = input(
        &details,
        "native-production-custom",
        "Use my edited source",
        "checkbox",
        "",
    )?;
    custom.set_checked(false);
    let label = add(&details, "label", "", "VibeScript source")?;
    label.set_attribute("for", "native-production-source")?;
    let source = add(&details, "textarea", "native-production-source", "")?;
    source.set_attribute("rows", "12")?;
    source.set_attribute("spellcheck", "false")?;
    source.set_attribute("style", "width:100%;font-family:monospace")?;
    let run = button(&root, "native-production-run", "Make this")?;
    let cancel = button(&root, "native-production-cancel", "Discard pending result")?;
    add(
        &root,
        "p",
        "native-production-status",
        "Choose an example and adjust its parameters.",
    )?
    .set_attribute("role", "status")?;
    add(&root, "div", "native-production-output", "")?;
    let st = state.clone();
    event(select.unchecked_ref(), "change", move |_| {
        if let Err(e) = choose(&st) {
            error(e)
        }
    })?;
    let st = state.clone();
    let l = lab.clone();
    event(run.unchecked_ref(), "click", move |_| {
        let st = st.clone();
        let l = l.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if l.borrow().busy {
                error(err("Finish or discard the current operation first"));
                return;
            }
            let job = {
                let mut p = st.borrow_mut();
                p.generation += 1;
                p.active = true;
                p.generation
            };
            l.borrow_mut().busy = true;
            let result = produce(&st, &l, job).await;
            if st.borrow().generation == job {
                st.borrow_mut().active = false;
                l.borrow_mut().busy = false;
                if let Err(e) = result {
                    text(
                        "native-production-status",
                        &e.as_string().unwrap_or("Production failed".into()),
                    );
                    error(e);
                }
            }
        });
    })?;
    let st = state.clone();
    let l = lab.clone();
    event(cancel.unchecked_ref(), "click", move |_| {
        if !st.borrow().active {
            return;
        }
        st.borrow_mut().generation += 1;
        st.borrow_mut().active = false;
        l.borrow_mut().busy = false;
        text(
            "native-production-status",
            "Pending result discarded. The server may finish its bounded production job.",
        );
    })?;
    choose(&state)
}
fn selected(s: &State) -> Result<Value, JsValue> {
    s.borrow()
        .recipes
        .iter()
        .find(|r| r["id"] == select_value("native-production-recipe"))
        .cloned()
        .ok_or(err("Unknown production recipe"))
}
fn set_source(value: &str) -> Result<(), JsValue> {
    let e = doc()
        .get_element_by_id("native-production-source")
        .ok_or(err("Missing source editor"))?;
    js_sys::Reflect::set(&e, &"value".into(), &value.into())?;
    Ok(())
}
fn choose(s: &State) -> Result<(), JsValue> {
    let r = selected(s)?;
    text(
        "native-production-description",
        r["description"].as_str().unwrap_or(""),
    );
    let root = doc()
        .get_element_by_id("native-production-parameters")
        .unwrap();
    root.set_text_content(None);
    for p in r["parameters"]
        .as_array()
        .ok_or(err("Recipe has no parameter definitions"))?
    {
        let key = p["key"].as_str().ok_or(err("Parameter has no key"))?;
        let v = if let Some(v) = p["default"].as_str() {
            v.to_string()
        } else {
            p["default"].to_string()
        };
        let e = input(
            &root,
            &format!("native-production-param-{key}"),
            p["label"].as_str().unwrap_or(key),
            p["kind"].as_str().unwrap_or("text"),
            &v,
        )?;
        for name in ["min", "max", "step"] {
            if let Some(v) = p.get(name) {
                e.set_attribute(name, &v.to_string())?;
            }
        }
        let st = s.clone();
        event(e.unchecked_ref(), "input", move |_| {
            if let Ok(source) = source_for(&st, false) {
                let _ = set_source(&source);
            }
        })?;
    }
    if let Some(e) = doc()
        .get_element_by_id("native-production-custom")
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
    {
        e.set_checked(false);
    }
    set_source(&source_for(s, false)?)
}
fn source_for(s: &State, allow_custom: bool) -> Result<String, JsValue> {
    if allow_custom
        && doc()
            .get_element_by_id("native-production-custom")
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            .map(|e| e.checked())
            .unwrap_or(false)
    {
        let e = doc().get_element_by_id("native-production-source").unwrap();
        let source = js_sys::Reflect::get(&e, &"value".into())?
            .as_string()
            .ok_or(err("Invalid edited source"))?;
        if source.len() > 65536 {
            return Err(err("Edited source exceeds 64 KiB"));
        }
        return Ok(source);
    }
    let r = selected(s)?;
    let mut source = r["source"].as_str().unwrap_or("").to_string();
    for p in r["parameters"].as_array().unwrap() {
        let key = p["key"].as_str().unwrap();
        let raw = input_value(&format!("native-production-param-{key}"));
        let value = if p["kind"] == "number" {
            let v = raw
                .parse::<f64>()
                .map_err(|_| err("Enter a number for each numeric parameter"))?;
            if !v.is_finite()
                || v < p["min"].as_f64().unwrap_or(f64::MIN)
                || v > p["max"].as_f64().unwrap_or(f64::MAX)
            {
                return Err(err(format!(
                    "{} is outside its permitted range",
                    p["label"].as_str().unwrap_or(key)
                )));
            }
            v.to_string()
        } else {
            if raw.chars().count() > p["max_length"].as_u64().unwrap_or(256) as usize {
                return Err(err("Text is too long"));
            }
            serde_json::to_string(&raw).map_err(err)?
        };
        source = source.replace(&format!("__{key}__"), &value);
    }
    Ok(source)
}
async fn produce(s: &State, lab: &Lab, job: u64) -> Result<(), JsValue> {
    let source = source_for(s, true)?;
    let recipe = selected(s)?;
    text(
        "native-production-status",
        "Synth is producing and strictly replaying your source...",
    );
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    let headers = js_sys::Object::new();
    js_sys::Reflect::set(&headers, &"Content-Type".into(), &"application/json".into())?;
    opts.set_headers(&headers);
    opts.set_body(&json!({"source":source,"profile":"files","include_html":false,"include_resources":true,"include_package":true}).to_string().into());
    let response: web_sys::Response = JsFuture::from(
        web_sys::window()
            .unwrap()
            .fetch_with_str_and_init("/api/produce", &opts),
    )
    .await?
    .dyn_into()?;
    if s.borrow().generation != job {
        return Ok(());
    }
    let raw = js_sys::Uint8Array::new(&JsFuture::from(response.array_buffer()?).await?).to_vec();
    if raw.len() > 32 * 1024 * 1024 {
        return Err(err("Production response exceeds 32 MiB"));
    }
    let body: Value = serde_json::from_slice(&raw)
        .map_err(|_| err("The Synth production service did not return a valid response"))?;
    if !response.ok() || body["ok"] != true {
        return Err(err(body["error"]
            .as_str()
            .unwrap_or("Synth refused this source")));
    }
    if body["strict_replay_verified"] != true {
        return Err(err("Production did not verify strict replay"));
    }
    let resources = body["resources"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 32)
        .ok_or(err("Invalid production resource count"))?;
    if body["resource_count"].as_u64() != Some(resources.len() as u64) {
        return Err(err("Production resource count mismatch"));
    }
    let mut files = Vec::new();
    let mut total = 0;
    for r in resources {
        let encoded = r["base64"].as_str().ok_or(err("Missing resource bytes"))?;
        let bytes = decode(encoded, 8 * 1024 * 1024)?;
        total += bytes.len();
        if total > 16 * 1024 * 1024
            || r["byte_length"].as_u64() != Some(bytes.len() as u64)
            || hash(&bytes) != r["sha256"].as_str().unwrap_or("")
        {
            return Err(err("Resource digest or size validation failed"));
        }
        files.push((r.clone(), bytes));
    }
    let package = body["package_base64"]
        .as_str()
        .ok_or(err("Missing production package"))?;
    let hmc = decode(package, 16 * 1024 * 1024)?;
    if hash(&hmc) != body["package_sha256"].as_str().unwrap_or("")
        || body["package_byte_length"].as_u64() != Some(hmc.len() as u64)
    {
        return Err(err("HMC package digest mismatch"));
    }
    if s.borrow().generation != job {
        return Ok(());
    }
    let output = doc().get_element_by_id("native-production-output").unwrap();
    output.set_text_content(None);
    for (r, bytes) in &files {
        show(&output, r, bytes)?;
    }
    let a = add(
        &output,
        "a",
        "native-production-package",
        "Download the complete HMC package",
    )?;
    a.set_attribute(
        "href",
        &format!("data:application/octet-stream;base64,{package}"),
    )?;
    a.set_attribute("download", "webizen-production.hmc")?;
    let receipt = json!({"schema":"webizen.production-run/v1","recipe":recipe["id"],"source":source,"source_sha256":hash(source.as_bytes()),"service":"same-origin Synth /api/produce","strict_replay_verified":true,"package_sha256":body["package_sha256"],"package_bytes":hmc.len(),"document_digest":body["document_digest"],"resources":resources.iter().map(|r|json!({"key":r["key"],"kind":r["kind"],"byte_length":r["byte_length"],"sha256":r["sha256"]})).collect::<Vec<_>>(),"browser_verified_all_digests":true});
    publish(lab, receipt.clone(), "Your work is ready")?;
    evidence(&output, &receipt)?;
    text(
        "native-production-status",
        &format!(
            "Made and verified {} files. Inspect, play or download them below.",
            files.len()
        ),
    );
    Ok(())
}
fn decode(encoded: &str, limit: usize) -> Result<Vec<u8>, JsValue> {
    if encoded.len() > limit * 4 / 3 + 8 {
        return Err(err("Encoded resource exceeds budget"));
    }
    let raw = web_sys::window().unwrap().atob(encoded)?;
    let bytes: Vec<u8> = raw.chars().map(|c| c as u8).collect();
    if bytes.len() > limit {
        return Err(err("Decoded resource exceeds budget"));
    }
    Ok(bytes)
}
fn show(root: &Element, r: &Value, bytes: &[u8]) -> Result<(), JsValue> {
    let key = r["key"].as_str().unwrap_or("resource");
    let encoded = r["base64"].as_str().unwrap();
    let figure = add(root, "figure", "", "")?;
    add(&figure, "figcaption", "", key)?;
    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE") {
        "audio/wav"
    } else if r["kind"] == "svg"
        && std::str::from_utf8(bytes)
            .map(|s| s.contains("<svg"))
            .unwrap_or(false)
    {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    };
    let url = format!("data:{mime};base64,{encoded}");
    if mime.starts_with("image/") {
        let img = add(&figure, "img", "", "")?;
        img.set_attribute("src", &url)?;
        img.set_attribute("alt", &format!("Produced {key}"))?;
        img.set_attribute(
            "style",
            "max-width:100%;max-height:420px;object-fit:contain",
        )?;
    }
    if mime == "audio/wav" {
        let audio = add(&figure, "audio", "", "")?;
        audio.set_attribute("src", &url)?;
        audio.set_attribute("controls", "")?;
        audio.set_attribute("preload", "none")?;
        audio.set_attribute("aria-label", &format!("Play produced {key}"))?;
    }
    let link = add(
        &figure,
        "a",
        "",
        &format!("Download {key} ({} bytes)", bytes.len()),
    )?;
    link.set_attribute("href", &url)?;
    link.set_attribute("download", key)?;
    Ok(())
}
