use super::*;
pub(super) fn controls(s: &Lab) -> Result<(), JsValue> {
    let root = doc().get_element_by_id("native-lab-data").unwrap();
    add(&root, "h3", "", "What does the data say?")?;
    let label = add(&root, "label", "", "Public dataset")?;
    label.set_attribute("for", "native-dataset")?;
    let selector = add(&root, "select", "native-dataset", "")?;
    let state = s.clone();
    event(selector.unchecked_ref(), "change", move |_| {
        if state.borrow().busy {
            if let Some(select) = doc()
                .get_element_by_id("native-dataset")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
            {
                select.set_value(state.borrow().descriptor["id"].as_str().unwrap_or(""));
            }
            error(err("Cancel the current benchmark before changing datasets"));
            return;
        }
        let selected = select_value("native-dataset");
        let descriptor = state.borrow().catalogue["datasets"]
            .as_array()
            .and_then(|rows| rows.iter().find(|d| d["id"] == selected))
            .cloned();
        if let Some(d) = descriptor {
            let st = state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Err(e) = load(&st, d).await {
                    error(e)
                }
            });
        }
    })?;
    add(&root, "p", "native-dataset-description", "")?;
    add(&root, "p", "native-dataset-count", "")?;
    let provenance = add(&root, "details", "native-dataset-provenance", "")?;
    add(&provenance, "summary", "", "Dataset source and publication")?;
    add(&provenance, "div", "native-dataset-source", "")?;
    let download = add(
        &root,
        "a",
        "native-dataset-download",
        "Download this dataset (Q42)",
    )?;
    download.set_attribute("download", "")?;
    add(&root, "div", "native-dataset-table", "")?;
    let pages = add(&root, "nav", "native-dataset-pages", "")?;
    pages.set_attribute("aria-label", "Dataset pages")?;
    for (id, label, forward) in [
        ("native-data-prev", "Previous 25 rows", false),
        ("native-data-next", "Next 25 rows", true),
    ] {
        let b = button(&pages, id, label)?;
        let st = s.clone();
        event(b.unchecked_ref(), "click", move |_| {
            let mut w = st.borrow_mut();
            let count = w.dataset.as_ref().map(|d| d.quins.len()).unwrap_or(0);
            if forward {
                if (w.page + 1) * 25 < count {
                    w.page += 1;
                }
            } else {
                w.page = w.page.saturating_sub(1);
            }
            drop(w);
            if let Err(e) = draw(&st) {
                error(e)
            }
        })?;
    }
    let l = add(&root, "label", "", "Example query")?;
    l.set_attribute("for", "native-query-preset")?;
    let preset = add(&root, "select", "native-query-preset", "")?;
    let st = s.clone();
    event(preset.unchecked_ref(), "change", move |_| {
        let id = select_value("native-query-preset");
        if let Some(q) = st.borrow().descriptor["queries"]
            .as_array()
            .and_then(|qs| qs.iter().find(|q| q["id"] == id))
        {
            if let Some(e) = doc()
                .get_element_by_id("native-query")
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            {
                e.set_value(q["query"].as_str().unwrap_or(""));
            }
        }
    })?;
    input(
        &root,
        "native-query",
        "Relationship to find",
        "text",
        "?s ?p ?o",
    )?;
    add(&root,"p","","Choose an example, or search for a subject, relationship and value. Variables begin with ?, and named entities use angle brackets.")?;
    let run = button(&root, "native-query-run", "Run this query")?;
    let st = s.clone();
    event(run.unchecked_ref(), "click", move |_| {
        if let Err(e) = run_query(&st) {
            error(e)
        }
    })?;
    Ok(())
}
pub(super) async fn load(s: &Lab, d: Value) -> Result<(), JsValue> {
    let job = {
        let mut w = s.borrow_mut();
        if w.busy {
            return Err(err("Cancel the current benchmark before changing datasets"));
        }
        w.job += 1;
        w.dataset = None;
        w.descriptor = Value::Null;
        w.job
    };
    text(
        "native-lab-status",
        "Verifying and reading actual Q42 data...",
    );
    text(
        "native-dataset-count",
        "The selected dataset is being verified.",
    );
    for id in [
        "native-dataset-table",
        "native-dataset-source",
        "native-query-preset",
    ] {
        if let Some(element) = doc().get_element_by_id(id) {
            element.set_text_content(None);
        }
    }
    if let Some(download) = doc().get_element_by_id("native-dataset-download") {
        download.remove_attribute("href")?;
    }
    if let Some(status) = doc().get_element_by_id("native-lab-status") {
        status.remove_attribute("data-state")?;
        status.set_attribute("role", "status")?;
    }
    let url = d["q42_url"]
        .as_str()
        .ok_or(err("Dataset has no native Q42 URL"))?;
    let raw = bytes(url, 32 * 1024 * 1024).await?;
    if hash(&raw) != d["sha256"].as_str().unwrap_or("") {
        return Err(err("Dataset digest mismatch"));
    }
    let dataset = proof::Dataset::open(&raw).map_err(err)?;
    if s.borrow().job != job {
        return Ok(());
    }
    if let Some(expected) = d["row_count"].as_u64() {
        if dataset.quins.len() as u64 != expected {
            return Err(err("Dataset row count differs from publication"));
        }
    }
    {
        let mut w = s.borrow_mut();
        w.dataset = Some(dataset);
        w.descriptor = d.clone();
        w.page = 0;
    }
    text(
        "native-dataset-description",
        d["description"]
            .as_str()
            .unwrap_or("Verified public dataset"),
    );
    doc()
        .get_element_by_id("native-dataset-download")
        .unwrap()
        .set_attribute("href", url)?;
    let options = doc().get_element_by_id("native-query-preset").unwrap();
    options.set_text_content(None);
    if let Some(qs) = d["queries"].as_array() {
        for q in qs {
            option(
                &options,
                q["id"].as_str().unwrap_or(""),
                q["label"].as_str().unwrap_or("Query"),
            )?;
        }
        if let Some(q) = qs.first() {
            doc()
                .get_element_by_id("native-query")
                .unwrap()
                .dyn_into::<HtmlInputElement>()?
                .set_value(q["query"].as_str().unwrap_or("?s ?p ?o"));
        }
    }
    let provenance = doc().get_element_by_id("native-dataset-source").unwrap();
    provenance.set_text_content(None);
    let source = &d["source"];
    let mut metadata = vec![
        vec![
            "Dataset identity".into(),
            d["id"].as_str().unwrap_or("").into(),
        ],
        vec!["Version".into(), d["version"].as_str().unwrap_or("").into()],
    ];
    if let Some(fields) = source.as_object() {
        for (key, value) in fields {
            metadata.push(vec![
                key.replace('_', " "),
                value
                    .as_str()
                    .map(String::from)
                    .unwrap_or_else(|| value.to_string()),
            ]);
        }
    }
    table(
        &provenance,
        &["Publication field", "Actual source record"],
        &metadata,
    )?;
    draw(s)?;
    text(
        "native-lab-status",
        "Data verified. Explore its records or run a query.",
    );
    Ok(())
}
fn draw(s: &Lab) -> Result<(), JsValue> {
    let w = s.borrow();
    let d = w.dataset.as_ref().ok_or(err("No dataset loaded"))?;
    let start = w.page * 25;
    let end = (start + 25).min(d.quins.len());
    text(
        "native-dataset-count",
        &format!(
            "{} records. Showing {} to {}. Source version: {}.",
            d.quins.len(),
            if end > 0 { start + 1 } else { 0 },
            end,
            w.descriptor["version"].as_str().unwrap_or("unspecified")
        ),
    );
    let parent = doc().get_element_by_id("native-dataset-table").unwrap();
    parent.set_text_content(None);
    table(
        &parent,
        &["Subject", "Relationship", "Value"],
        &d.rows(&d.quins[start..end])
            .into_iter()
            .map(Vec::from)
            .collect::<Vec<_>>(),
    )
}
fn run_query(s: &Lab) -> Result<(), JsValue> {
    if s.borrow().busy {
        return Err(err(
            "A benchmark is running; cancel it before another operation",
        ));
    }
    let query = input_value("native-query");
    let started = now();
    let (rows, stats, descriptor) = {
        let w = s.borrow();
        let d = w.dataset.as_ref().ok_or(err("Load a dataset first"))?;
        let (q, stats) = d.query(&query).map_err(err)?;
        (d.rows(&q), stats, w.descriptor.clone())
    };
    let result = json!({"schema":"webizen.native-query-run/v1","dataset":descriptor,"query":query,"rows":rows,"match_count":rows.len(),"elapsed_ms":now()-started,"native":stats});
    publish(
        s,
        result.clone(),
        &format!("{} matching relationships", rows.len()),
    )?;
    let parent = doc().get_element_by_id("native-lab-results").unwrap();
    table(
        &parent,
        &["Subject", "Relationship", "Value"],
        &rows
            .iter()
            .take(100)
            .cloned()
            .map(Vec::from)
            .collect::<Vec<_>>(),
    )?;
    if rows.len() > 100 {
        add(
            &parent,
            "p",
            "",
            "The first 100 matches are shown. Export includes every returned match.",
        )?;
    }
    evidence(&parent, &result)?;
    text(
        "native-lab-status",
        "Query completed. Inspect the matching records below.",
    );
    Ok(())
}
