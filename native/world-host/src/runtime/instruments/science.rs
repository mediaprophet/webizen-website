use super::*;
pub(super) fn controls(s: &Lab) -> Result<(), JsValue> {
    let root = doc().get_element_by_id("native-lab-rights").unwrap();
    add(&root, "h3", "", "When may information be shared?")?;
    add(&root,"p","","This example permits sharing group statistics while protecting information about an individual. Withdraw consent or move past the expiry to see how the permission changes.")?;
    let label = add(&root, "label", "", "Requested information")?;
    label.set_attribute("for", "native-rights-scope")?;
    let choice = add(&root, "select", "native-rights-scope", "")?;
    option(&choice, "aggregate", "Aggregate information")?;
    option(&choice, "individual", "An individual's information")?;
    input(
        &root,
        "native-rights-withdraw",
        "Withdraw aggregate permission",
        "checkbox",
        "",
    )?;
    input(
        &root,
        "native-rights-now",
        "Evaluation time (Unix seconds)",
        "number",
        "1763337600",
    )?;
    input(
        &root,
        "native-rights-expiry",
        "Permission expiry (Unix seconds; 0 means no expiry)",
        "number",
        "1794873600",
    )?;
    let run = button(&root, "native-rights-run", "Evaluate this request")?;
    let st = s.clone();
    event(run.unchecked_ref(), "click", move |_| {
        if let Err(e) = rights(&st) {
            error(e)
        }
    })?;
    let root = doc().get_element_by_id("native-lab-space").unwrap();
    add(
        &root,
        "h3",
        "",
        "Where are we on Earth, and where is the Sun?",
    )?;
    add(&root,"p","","Change a public reference location and UTC date. QualiaDB converts WGS84 coordinates into Earth-centred metres, checks the return transformation, and computes analytical Sun and Moon positions. These are modelled quantities, not live observations. UTC approximates UT1 in this proof.")?;
    input(
        &root,
        "native-space-lat",
        "Latitude (degrees)",
        "number",
        "-35",
    )?
    .set_step("0.01");
    input(
        &root,
        "native-space-lon",
        "Longitude (degrees)",
        "number",
        "149",
    )?
    .set_step("0.01");
    input(
        &root,
        "native-space-alt",
        "Height above WGS84 ellipsoid (metres)",
        "number",
        "600",
    )?;
    input(
        &root,
        "native-space-date",
        "UTC date and time",
        "datetime-local",
        "2026-11-16T00:00",
    )?;
    let run = button(
        &root,
        "native-space-run",
        "Calculate Earth and sky coordinates",
    )?;
    let st = s.clone();
    event(run.unchecked_ref(), "click", move |_| {
        if let Err(e) = space(&st) {
            error(e)
        }
    })?;
    Ok(())
}
fn rights(s: &Lab) -> Result<(), JsValue> {
    if s.borrow().busy {
        return Err(err("Cancel the benchmark before another operation"));
    }
    let withdrawn = doc()
        .get_element_by_id("native-rights-withdraw")
        .unwrap()
        .dyn_into::<HtmlInputElement>()?
        .checked();
    let now = input_value("native-rights-now")
        .parse()
        .map_err(|_| err("Invalid evaluation time"))?;
    let expiry = input_value("native-rights-expiry")
        .parse()
        .map_err(|_| err("Invalid expiry time"))?;
    let result =
        proof::rights(&select_value("native-rights-scope"), withdrawn, now, expiry).map_err(err)?;
    let summary = if result["permitted"] == true {
        "The scenario permits this request"
    } else {
        "The scenario does not permit this request"
    };
    publish(s, result.clone(), summary)?;
    let parent = doc().get_element_by_id("native-lab-results").unwrap();
    let rows = result["verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            vec![
                match v["opcode"].as_u64() {
                    Some(17) => "Permission",
                    Some(18) => "Prohibition",
                    _ => "Norm",
                }
                .into(),
                v["status"].as_str().unwrap_or("").into(),
                v["defeat_kind"].as_str().unwrap_or("").into(),
            ]
        })
        .collect::<Vec<_>>();
    table(
        &parent,
        &["Norm", "Observed status", "Exception type"],
        &rows,
    )?;
    evidence(&parent, &result)?;
    text("native-lab-status", "Permission evaluation completed.");
    Ok(())
}
fn space(s: &Lab) -> Result<(), JsValue> {
    if s.borrow().busy {
        return Err(err("Cancel the benchmark before another operation"));
    }
    let number = |id: &str| {
        input_value(id)
            .parse::<f64>()
            .map_err(|_| err("Enter numeric coordinates"))
    };
    let date = input_value("native-space-date");
    let utc = if date.len() == 16 {
        format!("{date}:00Z")
    } else {
        format!("{date}Z")
    };
    let unix = js_sys::Date::parse(&utc) / 1000.;
    let result = proof::space(
        number("native-space-lat")?,
        number("native-space-lon")?,
        number("native-space-alt")?,
        unix,
    )
    .map_err(err)?;
    publish(s, result.clone(), "Earth and sky coordinates calculated")?;
    let parent = doc().get_element_by_id("native-lab-results").unwrap();
    let f = |v: &Value, unit: &str| format!("{:.6} {unit}", v.as_f64().unwrap_or(f64::NAN));
    let rows = vec![
        vec!["Earth-centred X".into(), f(&result["ecef_m"][0], "m")],
        vec!["Earth-centred Y".into(), f(&result["ecef_m"][1], "m")],
        vec!["Earth-centred Z".into(), f(&result["ecef_m"][2], "m")],
        vec![
            "Return latitude".into(),
            f(&result["roundtrip"]["latitude_deg"], "degrees"),
        ],
        vec![
            "Return longitude".into(),
            f(&result["roundtrip"]["longitude_deg"], "degrees"),
        ],
        vec![
            "Return ellipsoid height".into(),
            f(&result["roundtrip"]["altitude_m"], "m"),
        ],
        vec!["Julian date".into(), f(&result["julian_date"], "days")],
        vec![
            "Sun elevation".into(),
            f(&result["sun"]["elevation_deg"], "degrees"),
        ],
        vec![
            "Sun azimuth".into(),
            f(&result["sun"]["azimuth_deg"], "degrees"),
        ],
        vec![
            "Earth-Sun distance".into(),
            f(&result["sun"]["distance_au"], "AU"),
        ],
        vec![
            "Moon elevation".into(),
            f(&result["moon"]["elevation_deg"], "degrees"),
        ],
        vec![
            "Moon illuminated fraction".into(),
            f(&result["moon"]["illuminated_fraction"], ""),
        ],
    ];
    table(&parent, &["Calculated quantity", "Value and units"], &rows)?;
    evidence(&parent, &result)?;
    text("native-lab-status", "Earth and sky calculations completed.");
    Ok(())
}
