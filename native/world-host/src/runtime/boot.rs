use super::*;
use super::stream::selection::unload;
use std::sync::atomic::AtomicBool;

const WOODLAND_SCENE_VOLUME: &str = "/production/lush-woodland/scene.q42";
const WOODLAND_LOGIC_VOLUME: &str = "/data/ewm/camp.q42";

fn query_has_token(query: &str, token: &str) -> bool {
    query
        .trim_start_matches('?')
        .split('&')
        .any(|part| part == token)
}

/// LW-06: one-line fetch of `/data/ewm/places/<id>.q42` when it is not the woodland scene.
pub(super) fn place_scene_volume_if_differs(place_id: &str) -> Option<String> {
    if place_id.is_empty()
        || !place_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }
    let url = format!("/data/ewm/places/{place_id}.q42");
    if url == WOODLAND_SCENE_VOLUME {
        None
    } else {
        Some(url)
    }
}

/// LW-06: admit a logic volume only when it is not woodland camp.q42.
#[allow(dead_code)]
pub(super) fn place_logic_volume_if_differs(url: Option<&str>) -> Option<String> {
    let url = url.filter(|u| !u.is_empty() && *u != WOODLAND_LOGIC_VOLUME)?;
    Some(url.to_string())
}

pub async fn boot() -> Result<(), JsValue> {
    element("native-world-status", "p")?.set_attribute("role", "status")?;
    text("native-world-status", "Opening the native living world…");
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
    let q42 = bytes("/publication/world.q42", 16 * 1024 * 1024).await?;
    let q42_hash = hash(&q42);
    let mut host = PoetSnapshot::default();
    super::knowledge::admit(&mut host, &q42, "publication", "world.q42").map_err(err)?;
    let graph = graph::from_host(&host).map_err(err)?;
    let settings = policy::evaluate(&mut host, &source, "world_settings", &[]).map_err(err)?;
    let camera: Camera = serde_json::from_value(settings["camera"].clone()).map_err(err)?;
    camera.validate().map_err(err)?;
    let location = js_sys::Reflect::get(&web_sys::window().unwrap(), &"location".into())?;
    let query = js_sys::Reflect::get(&location, &"search".into())?
        .as_string()
        .unwrap_or_default();
    let gpu_timing = query
        .trim_start_matches('?')
        .split('&')
        .any(|p| p == "gpu-timing=1");
    // Query flag defaults on. Catchment streaming stays an explicit ?ewm=1 opt-in.
    let query_flag = super::stream::ewm_query_enabled(&query, &canvas);
    let canvas_stream = canvas.get_attribute("data-ewm-streaming");
    let extra_streaming = if query_has_token(&query, "ewm=0") {
        false
    } else if query_has_token(&query, "ewm=1") {
        true
    } else {
        canvas_stream.as_deref() == Some("1")
    };
    let preview_scene = canvas.get_attribute("data-scene").unwrap_or_default();
    if preview_scene.ends_with(".q42")
        && preview_scene != WOODLAND_SCENE_VOLUME
        && !extra_streaming
    {
        return Err(err(
            "The woodland streams when you add ?ewm=1. The reading view remains available.",
        ));
    }
    let raw_dpr = web_sys::window().unwrap().device_pixel_ratio();
    let settings_max_dpr = settings["max_dpr"].as_f64();
    let pre_requested =
        super::graphics::parse_requested_backend(&query, canvas.get_attribute("data-backend").as_deref());
    let pre_budget =
        super::graphics::DeviceTier::pre_init_estimate(pre_requested, super::graphics::detect_mobile_viewport())
            .budget();
    let pre_dpr = super::graphics::clamp_dpr(raw_dpr, settings_max_dpr, &pre_budget);
    let rect = canvas.get_bounding_client_rect();
    let base_w = if rect.width() > 0. { rect.width() } else { 640. };
    let base_h = if rect.height() > 0. { rect.height() } else { 480. };
    canvas.set_width((base_w * pre_dpr) as u32);
    canvas.set_height((base_h * pre_dpr) as u32);
    let mut graphics_init =
        super::graphics::initialise_with_query(canvas.clone(), &query).await?;
    let dpr = super::graphics::clamp_dpr(raw_dpr, settings_max_dpr, &graphics_init.tier_budget);
    let clamped_w = (base_w * dpr).max(1.) as u32;
    let clamped_h = (base_h * dpr).max(1.) as u32;
    let dpr_changed = clamped_w != canvas.width() || clamped_h != canvas.height();
    if dpr_changed {
        canvas.set_width(clamped_w);
        canvas.set_height(clamped_h);
        graphics_init.report["canvas_width"] = json!(clamped_w);
        graphics_init.report["canvas_height"] = json!(clamped_h);
        super::graphics::publish(&graphics_init.report)?;
    }
    let backend = graphics_init
        .selected_backend
        .as_deref()
        .unwrap_or("");
    let mut portal = QualiaPortal::new(canvas.clone())?;
    if dpr_changed {
        portal.resize(canvas.clone(), clamped_w, clamped_h)?;
    }
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
    let mut scene_url = if (backend == "webgl2"
        || graphics_init.requested_backend == "webgl2"
        || graphics_init.is_mobile)
        && canvas.has_attribute("data-scene-mobile")
    {
        canvas.get_attribute("data-scene-mobile").unwrap_or_default()
    } else {
        canvas.get_attribute("data-scene").unwrap_or_default()
    };
    if scene_url.is_empty() {
        scene_url = if query_flag {
            WOODLAND_SCENE_VOLUME.into()
        } else {
            "/production/native-world/scene.json".into()
        };
    }
    let resident_scene = scene_url == WOODLAND_SCENE_VOLUME;
    // First-paint Q42 draws the camp. The catchment loop is the ?ewm=1 extra.
    let ewm_streaming = extra_streaming || resident_scene;
    let mut scene_graph_receipt = Value::Null;
    let mut admitted_catalog = None;
    let mut logic_url = None;
    let mut scene: Scene = if scene_url.ends_with(".q42") {
        if !resident_scene && !extra_streaming {
            return Err(err(
                "The woodland streams when you add ?ewm=1. The reading view remains available.",
            ));
        }
        let catalog_bytes = bytes("/data/ewm/catalog.q42", 2 * 1024 * 1024).await?;
        super::knowledge::admit(&mut host, &catalog_bytes, "catalog", "catalog.q42").map_err(err)?;
        if let Ok(species_bytes) = bytes("/data/ewm/species-catalog.q42", 2 * 1024 * 1024).await {
            let _ = super::knowledge::admit(
                &mut host,
                &species_bytes,
                "species-catalog",
                "species-catalog.q42",
            );
        }
        if let Ok(cosmos_bytes) = bytes("/data/ewm/cosmos-catalog.q42", 1024 * 1024).await {
            let _ = super::knowledge::admit(
                &mut host,
                &cosmos_bytes,
                "cosmos-catalog",
                "cosmos-catalog.q42",
            );
        }
        let q42_bytes = bytes(&scene_url, 16 * 1024 * 1024).await?;
        let scene_hash = hash(&q42_bytes);
        super::knowledge::admit(&mut host, &q42_bytes, "scene", "scene.q42").map_err(err)?;
        let catalog = super::catalog::Catalog::from_host(&mut host).map_err(err)?;
        let projected = super::scene_q42::from_host(&host).map_err(err)?;
        let water_raw = bytes(&projected.water_url, 1024 * 1024).await?;
        if hash(&water_raw) != projected.water_sha256 {
            return Err(err("Water geometry digest mismatch"));
        }
        let mut scene = projected.scene;
        scene.water = Some(super::scene_q42::water_from_bytes(&water_raw).map_err(err)?);
        scene_graph_receipt = json!({
            "format":"q42",
            "url":scene_url,
            "sha256":scene_hash,
            "reader":"PoetSnapshot::admit_q42_slice",
            "encoding":"q42vol-v4-cbor-ld",
            "engine":"GraphDatabase.sparql",
            "catalog_url":"/data/ewm/catalog.q42",
            "catalog_sha256":hash(&catalog_bytes),
            "gis_terrain":catalog.join.gis_terrain,
            "sparql_hits":catalog.join.sparql_hits,
            "first_paint":catalog.join.first_paint,
            "water_url":projected.water_url,
            "water_sha256":projected.water_sha256,
            "logic_url":projected.logic_url.clone()
        });
        logic_url = projected.logic_url;
        admitted_catalog = Some(catalog);
        scene
    } else if scene_url.ends_with(".json") {
        if ewm_streaming {
            return Err(err(
                "Streaming woodland uses a Q42 scene graph, not a JSON catalog",
            ));
        }
        serde_json::from_value(json_file(&scene_url, 16 * 1024 * 1024).await?).map_err(err)?
    } else {
        return Err(err("Unsupported native world scene"));
    };
    if scene.schema != "webizen.native-world/v1"
        || scene.assets.is_empty()
        || scene.assets.len() > 512
    {
        return Err(err("Unsupported native world artifact manifest"));
    }
    let macros_live = if ewm_streaming {
        if let Some(url) = logic_url.as_deref() {
            let logic_bytes = bytes(url, 2 * 1024 * 1024).await?;
            let logic_name = url.rsplit('/').next().unwrap_or("logic.q42");
            super::knowledge::admit(&mut host, &logic_bytes, "logic", logic_name).map_err(err)?;
            Some(super::macros::Live::from_world(&mut host, &source).map_err(err)?)
        } else {
            None
        }
    } else {
        None
    };
    let texture_receipts = super::textures::hydrate(&mut portal, &scene.textures).await?;
    let mut total_bytes = 0usize;
    let mut pointer_sha256 = String::new();
    let mut overlapping = Vec::new();
    let mut infosphere = false;
    let mut ewm_live = None;
    let (uploaded, mut upload_receipt, mut asset_receipts, texture_bindings) = if ewm_streaming {
        let catalog = admitted_catalog.ok_or(err("Streaming woodland needs the catalog Q42"))?;
        let paint = super::stream::load_first_paint(&mut portal, &scene, &camera, catalog).await?;
        total_bytes = paint.total_bytes;
        pointer_sha256 = paint.pointer_sha256;
        overlapping = paint.overlapping;
        infosphere = paint.infosphere;
        if extra_streaming {
            ewm_live = Some(EwmLive {
                catalog: paint.catalog,
                selection: paint.selection,
                colours: paint.colours,
                textures: scene.textures.clone(),
                // Catchment streaming loop replaces legacy 64-asset white-trunk enhancement
                // immediately after first paint without stalling `ewm.busy` for 30+ seconds.
                enhancement_pending: false,
                place_volume_pending: false,
                busy: Rc::new(AtomicBool::new(false)),
                abort: Rc::new(RefCell::new(None)),
            });
        }
        (
            paint.uploaded,
            paint.upload_receipt,
            paint.asset_receipts,
            paint.texture_bindings,
        )
    } else {
        let assets = js_sys::Array::new();
        let mut texture_bindings = 0;
        let mut material_registry = Vec::new();
        let mut asset_receipts = Vec::new();
        for (n, a) in scene.assets.iter().enumerate() {
            text(
                "native-world-status",
                &format!(
                    "Opening the landscape — {} of {}",
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
        let upload_receipt: Value = serde_json::from_str(
            &js_sys::JSON::stringify(&uploaded)?
                .as_string()
                .ok_or(err("Invalid native upload receipt"))?,
        )
        .map_err(err)?;
        (uploaded, upload_receipt, asset_receipts, texture_bindings)
    };
    js_sys::Reflect::set(
        &web_sys::window().unwrap(),
        &"__webizenNativeUpload".into(),
        &uploaded,
    )?;
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
    let mut instance_receipt = if ewm_streaming {
        super::instances::bind_resident(
            &mut portal,
            &scene,
            &upload_receipt,
            macros_live.as_ref(),
        )?
    } else {
        super::instances::bind(&mut portal, &scene, &upload_receipt, None)?
    };
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
    if !ewm_streaming {
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
    }
    // Bare homepage: keep the van, the car, and the dishes, and draw the
    // catchment soil plus the eco-intent tufts. ?ewm=1 keeps the catchment loop.
    let (tufts, wood, road_field) = if resident_scene && !extra_streaming {
        let mounted = super::resident_ground::mount(
            &mut portal,
            &mut scene,
            &upload_receipt,
            macros_live.as_ref(),
            &mut host,
        )
        .await?;
        upload_receipt = mounted.upload;
        instance_receipt = mounted.instances;
        asset_receipts = mounted.asset_receipts;
        total_bytes = mounted.total_bytes;
        (Some(mounted.tufts), Some(mounted.wood), mounted.road)
    } else {
        (None, None, serde_json::Value::Null)
    };
    let mut camera = camera;
    let show_roots = query_has_token(&query, "roots=1");
    let show_road = query_has_token(&query, "road=1");
    if let Some(wood) = wood.as_ref() {
        if query_has_token(&query, "gum=1") || show_roots {
            let (target, yaw, pitch, zoom) = wood.proof_view(show_roots);
            camera.target = target;
            camera.yaw = yaw;
            camera.pitch = pitch;
            camera.zoom = zoom;
        } else if show_road {
            if let Some((target, yaw, pitch, zoom)) = super::gis_join::road_proof_view() {
                camera.target = target;
                camera.yaw = yaw;
                camera.pitch = pitch;
                camera.zoom = zoom;
            }
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
    if let Some(ref water) = scene.water {
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
    let live_place = ewm_live.as_ref().map(|e| {
        if e.selection.place_id.is_empty() {
            "field-studio".to_string()
        } else {
            e.selection.place_id.clone()
        }
    });
    let live_place_base = match (&ewm_live, &live_place) {
        (Some(ewm), Some(id)) => ewm.catalog.live_place_base(id),
        _ => Vec::new(),
    };
    let ewm_unload = if extra_streaming {
        unload::V1_UNLOAD
    } else {
        ""
    };
    let device_telemetry = frame_telemetry::DeviceTelemetry {
        selected_backend: graphics_init.selected_backend.clone(),
        requested_backend: graphics_init.requested_backend,
        device_tier: graphics_init.device_tier.as_str(),
        tier_budget: frame_telemetry::TierBudgetTelemetry {
            tier_id: graphics_init.tier_budget.tier_id,
            max_resident_triangles: graphics_init.tier_budget.max_resident_triangles,
            max_gpu_memory_bytes: graphics_init.tier_budget.max_gpu_memory_bytes,
            max_first_paint_bytes: graphics_init.tier_budget.max_first_paint_bytes,
            max_texture_dim: graphics_init.tier_budget.max_texture_dim,
            max_dpr: graphics_init.tier_budget.max_dpr,
            preferred_lod: graphics_init.tier_budget.preferred_lod,
        },
        is_mobile: graphics_init.is_mobile,
        reading_fallback: super::graphics::READING_FALLBACK_URL,
    };
    let receipt = json!({"schema":"webizen.native-world-receipt/v1","gpu_timing_requested":gpu_timing,"ewm_streaming":extra_streaming,"ewm_catalog_engine":pointer_sha256,"ewm_overlapping":overlapping,"ewm_infosphere":infosphere,"ewm_live_place":live_place,"ewm_live_place_base":live_place_base,"ewm_unload":ewm_unload,"streaming":{"enabled":extra_streaming,"default":false},"resident_ground":resident_scene && !extra_streaming,"woody":wood.is_some(),"road_field":road_field,"scene_graph":scene_graph_receipt,"served_build_identity":runtime_identity,"renderer":backend,"requested_backend":graphics_init.requested_backend,"device_tier":graphics_init.device_tier.as_str(),"tier_budget":super::graphics::tier_budget_json(&graphics_init.tier_budget),"is_mobile":graphics_init.is_mobile,"reading_fallback":super::graphics::READING_FALLBACK_URL,"q42_sha256":q42_hash,"q42_rows":graph.rows,"q42_reader":"PoetSnapshot::admit_q42_slice","q42_engine":"GraphDatabase.sparql","vibe_sha256":hash(&source_bytes),"vibe_evaluator":"PoetSnapshot::eval_fn","assets":asset_receipts,"asset_bytes":total_bytes,"default_language":"en-AU","authority":"anonymous-public","controller":"Rust/WASM","upload":upload_receipt,"instances":instance_receipt,"graphics_initialisation":graphics_init.report,"presentation_settings":settings,"textures":texture_receipts,"verified_texture_bindings":texture_bindings,"camp":macros_live.as_ref().map(|live| live.receipt()).unwrap_or(serde_json::Value::Null)});
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
        timings: frame_telemetry::FrameTelemetry::with_device_context(device_telemetry),
        pending_pick: false,
        pending_pick_age: 0,
        benchmarking: false,
        receipt,
        scene,
        ewm: ewm_live,
        macros: macros_live,
        upload: upload_receipt,
        play_mode: false,
        play_observer: None,
        play_purpose: String::new(),
        play_focus_group: None,
        tufts,
        tuft_shift: 0.0,
        wood,
        woody_shift: 0.0,
    }));
    set_camera(&mut state.borrow_mut())?;
    ui::install_panel_toggle()?;
    play::install(&state)?;
    ui::install_controls(&state)?;
    input::install_input(&state)?;
    if show_roots {
        super::locomotion::set_rhizosphere_xray(&mut state.borrow_mut(), true);
    }
    // Place selection writes the campsite goal after the first proof-camera
    // assignment. Put the gum framing back on both camera and goal so the
    // frame blend does not walk back to the camp.
    if query_has_token(&query, "gum=1") || show_roots {
        let framed = state
            .borrow()
            .wood
            .as_ref()
            .map(|wood| wood.proof_view(show_roots));
        if let Some((target, yaw, pitch, zoom)) = framed {
            let mut w = state.borrow_mut();
            w.camera.target = target;
            w.camera.yaw = yaw;
            w.camera.pitch = pitch;
            w.camera.zoom = zoom;
            w.goal = w.camera.clone();
            set_camera(&mut w)?;
        }
    } else if show_road {
        if let Some((target, yaw, pitch, zoom)) = super::gis_join::road_proof_view() {
            let mut w = state.borrow_mut();
            w.camera.target = target;
            w.camera.yaw = yaw;
            w.camera.pitch = pitch;
            w.camera.zoom = zoom;
            w.goal = w.camera.clone();
            set_camera(&mut w)?;
        }
    }
    play::init_ewm_bridge(
        &state,
        graphics_init.device_tier.as_str(),
        graphics_init.tier_budget.max_resident_triangles,
        graphics_init.tier_budget.max_gpu_memory_bytes as u64,
    );
    if extra_streaming {
        super::stream::spawn_catchment_streaming_loop(
            state.clone(),
            graphics_init.device_tier.as_str().to_string(),
            graphics_init.tier_budget,
        );
    }
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
