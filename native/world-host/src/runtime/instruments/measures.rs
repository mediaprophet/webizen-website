use super::*;
pub(super) fn controls(s: &Lab) -> Result<(), JsValue> {
    let root = doc().get_element_by_id("native-lab-checks").unwrap();
    add(&root, "h3", "", "Test the claim, then measure it")?;
    add(&root,"p","","Run real checks of consent withdrawal, permission expiry, WGS84 coordinates and the selected dataset queries. Benchmarking repeats the selected query after checking its published expected result count. Scene rendering pauses while measurement runs.")?;
    let check = button(&root, "native-checks-run", "Run the checks")?;
    let st = s.clone();
    event(check.unchecked_ref(), "click", move |_| {
        if let Err(e) = checks(&st) {
            error(e)
        }
    })?;
    let n = input(
        &root,
        "native-benchmark-repeats",
        "Measured repetitions",
        "number",
        "30",
    )?;
    n.set_min("5");
    n.set_max("200");
    let run = button(&root, "native-benchmark-run", "Measure this query")?;
    let st = s.clone();
    event(run.unchecked_ref(), "click", move |_| {
        let st = st.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = benchmark(&st).await {
                let mut w = st.borrow_mut();
                w.busy = false;
                w.world.borrow_mut().benchmarking = false;
                drop(w);
                error(e)
            }
        });
    })?;
    let cancel = button(&root, "native-benchmark-cancel", "Cancel measurement")?;
    let st = s.clone();
    event(cancel.unchecked_ref(), "click", move |_| {
        let mut w = st.borrow_mut();
        if w.busy {
            w.job += 1;
            text("native-lab-status","Cancelling at the next completed query. Completed samples will remain inspectable.");
        }
    })?;
    Ok(())
}
fn checks(s: &Lab) -> Result<(), JsValue> {
    if s.borrow().busy {
        return Err(err("Cancel the benchmark before running checks"));
    }
    let mut result = proof::checks();
    {
        let w = s.borrow();
        let d = w
            .dataset
            .as_ref()
            .ok_or(err("Load the actual dataset first"))?;
        let cases = result["cases"].as_array_mut().unwrap();
        for q in w.descriptor["queries"]
            .as_array()
            .ok_or(err("No published query fixtures"))?
        {
            let source = q["query"].as_str().ok_or(err("Invalid query fixture"))?;
            let expected = q["expected_count"]
                .as_u64()
                .ok_or(err("Query fixture lacks expected count"))?;
            let actual = d.query(source);
            let (pass, count, message) = match actual {
                Ok((rows, _)) => {
                    let verified = d.verify_fixture(q, &rows);
                    (verified.is_ok(), Some(rows.len()), verified.err())
                }
                Err(e) => (false, None, Some(e)),
            };
            cases.push(json!({"name":q["label"],"query":source,"expected_count":expected,"actual_count":count,"passed":pass,"error":message}));
        }
        result["dataset"] = w.descriptor.clone();
    }
    let cases = result["cases"].as_array().unwrap();
    let passed = cases.iter().filter(|x| x["passed"] == true).count();
    let total = cases.len();
    let rows = cases
        .iter()
        .map(|x| {
            vec![
                x["name"].as_str().unwrap_or("Check").into(),
                if x["passed"] == true {
                    "Passed".into()
                } else {
                    "Failed".into()
                },
            ]
        })
        .collect::<Vec<_>>();
    result["assertions"] = json!(total);
    result["passed"] = json!(passed);
    result["failed"] = json!(total - passed);
    publish(
        s,
        result.clone(),
        &format!("{passed} of {total} checks passed"),
    )?;
    let parent = doc().get_element_by_id("native-lab-results").unwrap();
    table(&parent, &["Actual check", "Outcome"], &rows)?;
    evidence(&parent, &result)?;
    text(
        "native-lab-status",
        "Checks completed. Results remain available to inspect and export.",
    );
    Ok(())
}
async fn benchmark(s: &Lab) -> Result<(), JsValue> {
    if s.borrow().busy {
        return Err(err("Only one measured job may run at a time"));
    }
    let repeat: usize = input_value("native-benchmark-repeats")
        .parse()
        .map_err(|_| err("Enter 5 to 200 repetitions"))?;
    if !(5..=200).contains(&repeat) {
        return Err(err("Enter 5 to 200 repetitions"));
    }
    let source = input_value("native-query");
    let (descriptor, fixture, expected) = {
        let w = s.borrow();
        let q=w.descriptor["queries"].as_array().and_then(|qs|qs.iter().find(|q|q["query"]==source)).ok_or(err("Select an unchanged published query before benchmarking; edited queries can be explored in Data and queries"))?;
        (
            w.descriptor.clone(),
            q.clone(),
            q["expected_count"]
                .as_u64()
                .ok_or(err("No expected result count"))?,
        )
    };
    let job = {
        let mut w = s.borrow_mut();
        w.busy = true;
        w.job += 1;
        w.world.borrow_mut().benchmarking = true;
        w.job
    };
    // Warm-up and correctness are outside the measured interval.
    for _ in 0..10 {
        let w = s.borrow();
        let (rows, _) = w
            .dataset
            .as_ref()
            .ok_or(err("No dataset"))?
            .query(&source)
            .map_err(err)?;
        w.dataset
            .as_ref()
            .unwrap()
            .verify_fixture(&fixture, &rows)
            .map_err(err)?;
    }
    let mut samples = Vec::new();
    let mut cancelled = false;
    let mut stats = Value::Null;
    for i in 0..repeat {
        yield_browser().await?;
        if s.borrow().job != job {
            cancelled = true;
            break;
        }
        let begin = now();
        let (rows, observed) = {
            let w = s.borrow();
            w.dataset.as_ref().unwrap().query(&source).map_err(err)?
        };
        let elapsed = now() - begin;
        if rows.len() as u64 != expected {
            return Err(err("Measured query result changed; run invalid"));
        }
        samples.push(elapsed);
        stats = observed;
        text(
            "native-lab-status",
            &format!(
                "Measured {} of {} queries. You can cancel between queries.",
                i + 1,
                repeat
            ),
        );
    }
    {
        let mut w = s.borrow_mut();
        w.busy = false;
        w.world.borrow_mut().benchmarking = false;
    }
    let mut sorted = samples.clone();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let p50 = if n == 0 {
        None
    } else if n % 2 == 0 {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.)
    } else {
        Some(sorted[n / 2])
    };
    let p95 = if n == 0 {
        None
    } else {
        Some(
            sorted[((n as f64 * 0.95).ceil() as usize)
                .saturating_sub(1)
                .min(n - 1)],
        )
    };
    let environment = {
        let window = web_sys::window().unwrap();
        let navigator = js_sys::Reflect::get(&window, &"navigator".into()).unwrap_or(JsValue::NULL);
        let user_agent = js_sys::Reflect::get(&navigator, &"userAgent".into())
            .ok()
            .and_then(|v| v.as_string());
        let cores = js_sys::Reflect::get(&navigator, &"hardwareConcurrency".into())
            .ok()
            .and_then(|v| v.as_f64());
        let isolated = js_sys::Reflect::get(&window, &"crossOriginIsolated".into())
            .ok()
            .and_then(|v| v.as_bool());
        let lab = s.borrow();
        let world = lab.world.borrow();
        json!({"user_agent":user_agent,"reported_logical_processors":cores,"cross_origin_isolated":isolated,"query_execution":"single native wasm32 call per sample","clock":"browser performance.now in milliseconds","native_renderer":world.receipt["renderer"],"world_q42_sha256":world.receipt["q42_sha256"],"vibe_sha256":world.receipt["vibe_sha256"],"host_package_version":env!("CARGO_PKG_VERSION"),"served_build_identity":world.receipt["served_build_identity"],"input_quins":lab.dataset.as_ref().unwrap().quins.len()})
    };
    let result = json!({"schema":"webizen.native-query-benchmark/v1","state":if cancelled{"cancelled"}else{"succeeded"},"dataset":descriptor,"query":source,"workload":"one native triple-pattern compile plus bytecode VM execution, result allocation and native statistics record; no SPARQL-join claim","timed":"query compilation + native VM + result buffers/statistics; excludes download, Q42 decode, warmup, DOM, yielding and rendering","warmup_runs":10,"requested_repeats":repeat,"completed_repeats":n,"expected_match_count":expected,"correctness":"published decoded rows compared where supplied; cardinality checked before and during timing","oracle_fixture":fixture,"samples_ms":samples,"median_ms":p50,"p95_ms":p95,"percentile_method":"nearest-rank p95; ordinary median","native":stats,"scene_rendering_paused":true,"profile":"same-browser wasm32; no competitor comparison","environment":environment});
    publish(
        s,
        result.clone(),
        if cancelled {
            "Measurement cancelled; completed samples retained"
        } else {
            "Query measurement completed"
        },
    )?;
    let parent = doc().get_element_by_id("native-lab-results").unwrap();
    table(
        &parent,
        &["Measure", "Observed value"],
        &[
            vec!["Completed repetitions".into(), n.to_string()],
            vec![
                "Median".into(),
                p50.map(|x| format!("{x:.4} ms"))
                    .unwrap_or("No samples".into()),
            ],
            vec![
                "95th percentile".into(),
                p95.map(|x| format!("{x:.4} ms"))
                    .unwrap_or("No samples".into()),
            ],
            vec!["Expected query matches".into(), expected.to_string()],
        ],
    )?;
    evidence(&parent, &result)?;
    text(
        "native-lab-status",
        if cancelled {
            "Measurement cancelled; world rendering resumed."
        } else {
            "Measurement completed; world rendering resumed."
        },
    );
    Ok(())
}
