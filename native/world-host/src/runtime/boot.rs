use super::*;

pub async fn boot() -> Result<(), JsValue> {
    element("native-world-status", "p")?.set_attribute("role", "status")?;
    text("native-world-status", "Opening the native living world...");
    let canvas = element("native-world", "canvas")?.dyn_into::<HtmlCanvasElement>()?;
    let source_url = canvas
        .get_attribute("data-vibe")
        .unwrap_or("/world/living-world.vibe".into());
    let source_bytes = bytes(&source_url, 512 * 1024).await?;
    let runtime_identity = json_file("/pkg/native-world/runtime-identity.json", 512 * 1024).await?;
    if runtime_identity["schema"] != "webizen.native-runtime-identity/v1" {
        return Err(err("Unsupported served native build identity"));
    }
    let source = String::from_utf8(source_bytes.clone()).map_err(err)?;
    let pointer = json_file("/publication/current.json", 65536).await?;
    let volume = pointer["q42"]
        .as_str()
        .ok_or(err("No Q42 publication pointer"))?;
    let q42 = bytes(&format!("/publication/{volume}"), 16 * 1024 * 1024).await?;
    let q42_hash = hash(&q42);
    if Some(q42_hash.as_str()) != pointer["sha256"].as_str() {
        return Err(err("Q42 publication digest mismatch"));
    }
    let graph = graph::decode(&q42).map_err(err)?;
    let mut host = PoetSnapshot::default();
    host.committed = graph.quins.clone();
    let settings = policy::evaluate(&mut host, &source, "world_settings", &[]).map_err(err)?;
    let camera: Camera = serde_json::from_value(settings["camera"].clone()).map_err(err)?;
    camera.validate().map_err(err)?;
    let dpr = web_sys::window()
        .unwrap()
        .device_pixel_ratio()
        .min(settings["max_dpr"].as_f64().unwrap_or(1.5).clamp(1., 2.));
    let rect = canvas.get_bounding_client_rect();
    canvas.set_width(
        ((if rect.width() > 0. {
            rect.width()
        } else {
            640.
        }) * dpr) as u32,
    );
    canvas.set_height(
        ((if rect.height() > 0. {
            rect.height()
        } else {
            480.
        }) * dpr) as u32,
    );
    let graphics_init = super::graphics::initialise(canvas.clone()).await?;
    let backend = graphics_init["selected_backend"]
        .as_str()
        .unwrap_or("webgpu");
    let mut portal = QualiaPortal::new(canvas.clone())?;
    let location = js_sys::Reflect::get(&web_sys::window().unwrap(), &"location".into())?;
    let query = js_sys::Reflect::get(&location, &"search".into())?
        .as_string()
        .unwrap_or_default();
    let gpu_timing = query
        .trim_start_matches('?')
        .split('&')
        .any(|p| p == "gpu-timing=1");
    portal.set_gpu_pass_timing_enabled(gpu_timing);
    portal.set_preserve_authored_frame(true);
    portal.set_tensor_projection_enabled(false);
    portal.set_ambient_enabled(false);
    portal.set_camera_near_plane(0.05)?;
    portal.set_sky_preset(settings["sky_preset"].as_u64().unwrap_or(1) as u32);
    let sun: [f32; 5] = serde_json::from_value(settings["sun"].clone()).map_err(err)?;
    portal.set_lighting(sun[0], sun[1], sun[2], sun[3], sun[4]);
    if let Some(ev) = settings["exposure_ev"].as_f64() {
        if !ev.is_finite() || !(-8.0..=8.0).contains(&ev) {
            return Err(err("Invalid Vibe exposure"));
        }
        portal.set_exposure_compensation(ev as f32);
    }
    if let Some(rgb) = settings["sky_colour"].as_array() {
        if rgb.len() != 3 {
            return Err(err("Invalid Vibe sky colour"));
        }
        let values: Vec<f32> = rgb
            .iter()
            .map(|v| {
                v.as_f64()
                    .filter(|n| n.is_finite() && (0.0..=1.0).contains(n))
                    .map(|n| n as f32)
                    .ok_or(err("Invalid Vibe sky colour"))
            })
            .collect::<Result<_, _>>()?;
        portal.set_clear_color(values[0] as f64, values[1] as f64, values[2] as f64, 1.);
    }
    if !settings["environment_fallback"].is_null() || !settings["environment_probes"].is_null() {
        let rgb: [f32; 3] = if settings["environment_fallback"].is_null() {
            [0.; 3]
        } else {
            serde_json::from_value(settings["environment_fallback"].clone()).map_err(err)?
        };
        if !rgb
            .iter()
            .all(|v| v.is_finite() && (0.0..=16.0).contains(v))
        {
            return Err(err("Invalid Vibe environment fallback radiance"));
        }
        let probes: Vec<Vec<f32>> = if settings["environment_probes"].is_null() {
            Vec::new()
        } else {
            serde_json::from_value(settings["environment_probes"].clone()).map_err(err)?
        };
        if probes.len() > 4
            || probes
                .iter()
                .any(|p| p.len() != 25 || p.iter().any(|v| !v.is_finite()) || p[3] <= 0.)
        {
            return Err(err("Vibe environment probes require up to 4 finite 25-number records with positive radius"));
        }
        let packed: Vec<f32> = probes.into_iter().flatten().collect();
        portal.set_environment_probe_data(&packed, &rgb)?;
    }
    portal.set_shadows_enabled(true);
    portal.set_screen_space_ao_enabled(true);
    let is_mobile_viewport = web_sys::window()
        .unwrap()
        .match_media("(max-width: 900px)")?
        .map(|m| m.matches())
        .unwrap_or(false);
    let scene_url = if (backend == "webgl2" || is_mobile_viewport)
        && canvas.has_attribute("data-scene-mobile")
    {
        canvas
            .get_attribute("data-scene-mobile")
            .unwrap_or("/production/lush-woodland/scene.json".into())
    } else {
        canvas
            .get_attribute("data-scene")
            .unwrap_or("/production/native-world/scene.json".into())
    };
    let scene: Scene =
        serde_json::from_value(json_file(&scene_url, 16 * 1024 * 1024).await?).map_err(err)?;
    if scene.schema != "webizen.native-world/v1"
        || scene.assets.is_empty()
        || scene.assets.len() > 512
    {
        return Err(err("Unsupported native world artifact manifest"));
    }
    let texture_receipts = super::textures::hydrate(&mut portal, &scene.textures).await?;
    let assets = js_sys::Array::new();
    let mut total_bytes = 0;
    let mut texture_bindings = 0;
    let mut material_registry = Vec::new();
    let mut asset_receipts = Vec::new();
    for (n, a) in scene.assets.iter().enumerate() {
        text(
            "native-world-status",
            &format!(
                "Opening the landscape - {} of {}",
                n + 1,
                scene.assets.len()
            ),
        );
        let raw = bytes(&a.url, 64 * 1024 * 1024).await?;
        total_bytes += raw.len();
        if total_bytes > 256 * 1024 * 1024 {
            return Err(err("Native world exceeds its asset byte budget"));
        }
        if hash(&raw) != a.sha256 {
            return Err(err(format!("Asset digest mismatch: {}", a.id)));
        }
        super::admission::check(&a.id, &raw)?;
        texture_bindings +=
            super::textures::verify_asset(&a.id, &raw, &scene.textures, &mut material_registry)?;
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"bytes".into(),
            &js_sys::Uint8Array::from(raw.as_slice()),
        )?;
        js_sys::Reflect::set(&obj, &"id".into(), &a.id.clone().into())?;
        for (name, value) in ["r", "g", "b", "a"].iter().zip(a.colour) {
            js_sys::Reflect::set(&obj, &(*name).into(), &JsValue::from_f64(value as f64))?;
        }
        assets.push(&obj);
        asset_receipts.push(json!({"id":a.id,"sha256":a.sha256}));
    }
    let uploaded = portal.load_scene_assets(&assets)?;
    js_sys::Reflect::set(
        &web_sys::window().unwrap(),
        &"__webizenNativeUpload".into(),
        &uploaded,
    )?;
    let upload_receipt: Value = serde_json::from_str(
        &js_sys::JSON::stringify(&uploaded)?
            .as_string()
            .ok_or(err("Invalid native upload receipt"))?,
    )
    .map_err(err)?;
    if texture_bindings > 0 && upload_receipt["material_pipeline"].as_bool() != Some(true) {
        return Err(err(
            "Native textured scene could not bind its actual material pipeline",
        ));
    }
    let refused = js_sys::Reflect::get(&uploaded, &"organs_refused".into())?
        .as_f64()
        .unwrap_or(0.);
    if refused != 0. {
        return Err(err(format!("Native scene refused {refused} assets")));
    }
    let instance_receipt = super::instances::bind(&mut portal, &scene, &upload_receipt)?;
    let admitted = portal.scene_render_receipt()?;
    let got_indices = js_sys::Reflect::get(&admitted, &"index_count".into())?
        .as_f64()
        .unwrap_or(0.) as u64;
    let authored_triangles = js_sys::Reflect::get(&uploaded, &"total_triangles".into())?
        .as_f64()
        .unwrap_or(0.) as u64;
    if got_indices != authored_triangles * 3 {
        return Err(err("Native GPU admission reduced the authored scene; use the reading view while a smaller native scene is prepared"));
    }
    if let Some(expected) = scene.triangle_count {
        if expected != authored_triangles {
            return Err(err("Native triangle total differs from published manifest"));
        }
    }
    if let Some(expected) = scene.vertex_count {
        let got = js_sys::Reflect::get(&admitted, &"vertex_count".into())?
            .as_f64()
            .unwrap_or(0.) as u64;
        if expected != got {
            return Err(err("Native vertex total differs from published manifest"));
        }
    }
    // Native tensor markers use graph positions; identity stays in the exact graph order.
    use qualia_core_db::tensor::{
        buffer_export::{write_tensor_buffer, TensorBufferHeader},
        Tensor10D,
    };
    let marker_height = settings["marker_height"].as_f64().unwrap_or(3.) as f32;
    let marker_sigma = settings["marker_sigma"].as_f64().unwrap_or(0.42) as f32;
    let marker_alpha = settings["marker_alpha"].as_f64().unwrap_or(1.) as f32;
    let mut nodes = Vec::new();
    for p in graph.places.as_array().unwrap() {
        let xyz: [f32; 3] = serde_json::from_value(p["position"].clone()).map_err(err)?;
        nodes.push(Tensor10D {
            x: xyz[0],
            y: xyz[1] + marker_height,
            z: xyz[2],
            alpha: marker_alpha,
            sigma: marker_sigma,
            ..Tensor10D::default()
        });
    }
    let mut tensor_bytes = vec![0; TensorBufferHeader::total_bytes(nodes.len())];
    write_tensor_buffer(&nodes, &mut tensor_bytes).map_err(err)?;
    portal.upload_tensor_buffer(&tensor_bytes)?;
    portal.set_temporal_slice(0., 0.08);
    portal.set_tensor_projection_enabled(
        settings["scene_mode"] != "reference-model"
            && settings["markers_visible"].as_bool().unwrap_or(true),
    );
    if let Some(water) = scene.water {
        let admitted = portal.set_water_geometry(&water.positions, &water.indices)?;
        if admitted {
            let cfg = &settings["water"];
            let rgba: [f32; 4] = serde_json::from_value(cfg["colour"].clone()).map_err(err)?;
            portal.set_water_surface(
                rgba[0],
                rgba[1],
                rgba[2],
                rgba[3],
                cfg["amplitude"].as_f64().unwrap_or(0.025) as f32,
                cfg["frequency"].as_f64().unwrap_or(1.) as f32,
                cfg["quality"].as_u64().unwrap_or(1) as u32,
            );
        }
    }
    let reduced = web_sys::window()
        .unwrap()
        .match_media("(prefers-reduced-motion: reduce)")?
        .map(|m| m.matches())
        .unwrap_or(false);
    portal.set_animation_reduced_motion(reduced);
    let receipt = json!({"schema":"webizen.native-world-receipt/v1","gpu_timing_requested":gpu_timing,"served_build_identity":runtime_identity,"renderer":backend,"q42_sha256":q42_hash,"q42_rows":graph.rows,"q42_reader":"WasmSliceQ42Reader","vibe_sha256":hash(&source_bytes),"vibe_evaluator":"PoetSnapshot::eval_fn","assets":asset_receipts,"asset_bytes":total_bytes,"default_language":"en-AU","authority":"anonymous-public","controller":"Rust/WASM","upload":upload_receipt,"instances":instance_receipt,"graphics_initialisation":graphics_init,"presentation_settings":settings,"textures":texture_receipts,"verified_texture_bindings":texture_bindings});
    let state = Rc::new(RefCell::new(World {
        portal,
        canvas: canvas.clone(),
        host,
        source,
        graph,
        camera: camera.clone(),
        goal: camera,
        reduced,
        drag: None,
        last_input: 0.,
        last_frame: now(),
        last_receipt: now(),
        frames: 0,
        timings: frame_telemetry::FrameTelemetry::default(),
        pending_pick: false,
        benchmarking: false,
        receipt,
    }));
    set_camera(&mut state.borrow_mut())?;
    ui::install_controls(&state)?;
    ui::install_panel_toggle()?;
    input::install_input(&state)?;
    let s = state.clone();
    event(
        web_sys::window().unwrap().unchecked_ref(),
        "resize",
        move |_| {
            let mut w = s.borrow_mut();
            let rect = w.canvas.get_bounding_client_rect();
            let c = w.canvas.clone();
            if let Err(e) = w.portal.resize(
                c,
                (rect.width().max(1.) * dpr) as u32,
                (rect.height().max(1.) * dpr) as u32,
            ) {
                show_error(e)
            }
        },
    )?;
    text(
        "native-world-status",
        "Explore the world. Your chosen path changes the presentation, never your rights.",
    );
    let lab_world = state.clone();
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = super::instruments::install(lab_world).await {
            web_sys::console::error_1(&e);
            text(
                "native-lab-status",
                &e.as_string()
                    .unwrap_or("Proof instruments could not load".into()),
            );
        }
    });
    start_frames(state)?;
    Ok(())
}
