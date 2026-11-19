//! Native renderer admission, capability tiering, and observable initialisation diagnostics.

pub(crate) const READING_FALLBACK_URL: &str = "/publication/reading.html";

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeviceTier {
    TierA_DesktopWebGPU,
    TierB_DesktopWebGL2_MobileWebGPU,
    TierC_MobileWebGL2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TierBudget {
    pub tier_id: &'static str,
    pub max_resident_triangles: u64,
    pub max_gpu_memory_bytes: usize,
    pub max_first_paint_bytes: usize,
    pub max_texture_dim: u32,
    pub max_dpr: f64,
    pub preferred_lod: &'static str,
}

impl DeviceTier {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::TierA_DesktopWebGPU => "tier-a",
            Self::TierB_DesktopWebGL2_MobileWebGPU => "tier-b",
            Self::TierC_MobileWebGL2 => "tier-c",
        }
    }

    pub(crate) const fn budget(self) -> TierBudget {
        match self {
            Self::TierA_DesktopWebGPU => TierBudget {
                tier_id: "tier-a",
                max_resident_triangles: 650_000,
                max_gpu_memory_bytes: 384 * 1024 * 1024,
                max_first_paint_bytes: 24 * 1024 * 1024,
                max_texture_dim: 2048,
                max_dpr: 1.5,
                preferred_lod: "lod0",
            },
            Self::TierB_DesktopWebGL2_MobileWebGPU => TierBudget {
                tier_id: "tier-b",
                max_resident_triangles: 320_000,
                max_gpu_memory_bytes: 192 * 1024 * 1024,
                max_first_paint_bytes: 17 * 1024 * 1024,
                max_texture_dim: 1024,
                max_dpr: 1.35,
                preferred_lod: "lod1",
            },
            Self::TierC_MobileWebGL2 => TierBudget {
                tier_id: "tier-c",
                max_resident_triangles: 150_000,
                max_gpu_memory_bytes: 96 * 1024 * 1024,
                max_first_paint_bytes: 12 * 1024 * 1024,
                max_texture_dim: 1024,
                max_dpr: 1.15,
                preferred_lod: "lod1",
            },
        }
    }

    /// Classify capability tier from the honest selected GPU backend and mobile heuristic.
    pub(crate) fn classify(selected_backend: Option<&str>, is_mobile: bool) -> Self {
        match (selected_backend, is_mobile) {
            (Some("webgpu"), false) => Self::TierA_DesktopWebGPU,
            (Some("webgpu"), true) => Self::TierB_DesktopWebGL2_MobileWebGPU,
            (Some("webgl2"), false) => Self::TierB_DesktopWebGL2_MobileWebGPU,
            (Some("webgl2"), true) => Self::TierC_MobileWebGL2,
            (_, false) => Self::TierB_DesktopWebGL2_MobileWebGPU,
            (_, true) => Self::TierC_MobileWebGL2,
        }
    }

    /// Classify capability tier while honouring a deterministic `?backend=webgl2` / `data-backend="webgl2"` cap.
    pub(crate) fn classify_with_request(
        selected_backend: Option<&str>,
        requested_backend: &str,
        is_mobile: bool,
    ) -> Self {
        if requested_backend.eq_ignore_ascii_case("webgl2") {
            Self::classify(Some("webgl2"), is_mobile)
        } else {
            Self::classify(selected_backend, is_mobile)
        }
    }

    /// Conservative pre-initialisation tier used to bound initial canvas framebuffer allocation.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn pre_init_estimate(requested_backend: &str, is_mobile: bool) -> Self {
        if requested_backend.eq_ignore_ascii_case("webgl2") {
            Self::classify(Some("webgl2"), is_mobile)
        } else if is_mobile {
            Self::TierB_DesktopWebGL2_MobileWebGPU
        } else {
            Self::TierA_DesktopWebGPU
        }
    }
}

/// Mobile/tablet viewport heuristic combining pointer coarseness and viewport dimensions.
pub(crate) fn is_mobile_viewport(width: f64, height: f64, coarse_pointer: bool) -> bool {
    if coarse_pointer {
        return true;
    }
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return false;
    }
    width <= 900.0 || height <= 500.0
}

/// Parse `?backend=webgl2` (highest priority) or `<canvas data-backend="webgl2">`.
pub(crate) fn parse_requested_backend(query: &str, canvas_data_backend: Option<&str>) -> &'static str {
    for pair in query.trim_start_matches('?').split('&') {
        if let Some(value) = pair.strip_prefix("backend=") {
            if value.eq_ignore_ascii_case("webgl2") {
                return "webgl2";
            }
            if value.eq_ignore_ascii_case("webgpu") {
                return "webgpu";
            }
        }
    }
    if let Some(attr) = canvas_data_backend {
        let trimmed = attr.trim();
        if trimmed.eq_ignore_ascii_case("webgl2") {
            return "webgl2";
        }
        if trimmed.eq_ignore_ascii_case("webgpu") {
            return "webgpu";
        }
    }
    "webgpu-or-webgl2"
}

/// Return `true` when the resolved backend request explicitly forces WebGL2 (`Backends::GL`).
pub(crate) fn should_force_webgl2(requested_backend: &str) -> bool {
    requested_backend.eq_ignore_ascii_case("webgl2")
}

/// Clamp device pixel ratio against Vibe settings and the resolved `TierBudget::max_dpr`.
pub(crate) fn clamp_dpr(
    device_pixel_ratio: f64,
    settings_max_dpr: Option<f64>,
    budget: &TierBudget,
) -> f64 {
    let raw_dpr = if device_pixel_ratio.is_finite() && device_pixel_ratio > 0.0 {
        device_pixel_ratio
    } else {
        1.0
    };
    let settings_cap = settings_max_dpr
        .filter(|v| v.is_finite())
        .unwrap_or(1.5)
        .clamp(1.0, 2.0);
    raw_dpr.min(settings_cap).min(budget.max_dpr)
}

#[cfg(target_arch = "wasm32")]
use super::*;
#[cfg(target_arch = "wasm32")]
use qualia_core_db::render::portal::{portal_gpu_backend, portal_init_with_backend};

#[cfg(target_arch = "wasm32")]
pub(super) struct GraphicsInit {
    pub report: Value,
    pub selected_backend: Option<String>,
    pub requested_backend: &'static str,
    pub device_tier: DeviceTier,
    pub tier_budget: TierBudget,
    pub is_mobile: bool,
}

#[cfg(target_arch = "wasm32")]
fn js_error(value: &JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            js_sys::Reflect::get(value, &"message".into())
                .ok()
                .and_then(|v| v.as_string())
        })
        .unwrap_or_else(|| format!("{value:?}"))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn publish(report: &Value) -> Result<(), JsValue> {
    let value = js_sys::JSON::parse(&report.to_string())?;
    js_sys::Reflect::set(
        &web_sys::window().unwrap(),
        &"__webizenNativeInit".into(),
        &value,
    )?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn tier_budget_json(budget: &TierBudget) -> Value {
    json!({
        "tier_id": budget.tier_id,
        "max_resident_triangles": budget.max_resident_triangles,
        "max_gpu_memory_bytes": budget.max_gpu_memory_bytes,
        "max_first_paint_bytes": budget.max_first_paint_bytes,
        "max_texture_dim": budget.max_texture_dim,
        "max_dpr": budget.max_dpr,
        "preferred_lod": budget.preferred_lod,
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) fn detect_mobile_viewport() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let width = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let height = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let coarse_pointer = window
        .match_media("(pointer: coarse)")
        .ok()
        .flatten()
        .map(|m| m.matches())
        .unwrap_or(false);
    is_mobile_viewport(width, height, coarse_pointer)
}

/// Honest live adapter from pending/adopted `PortalGpu`. Not `navigator.gpu`.
#[cfg(target_arch = "wasm32")]
fn selected_backend_from_gpu() -> Option<String> {
    match portal_gpu_backend().as_deref() {
        Some("webgpu") => Some("webgpu".to_string()),
        Some("webgl2") => Some("webgl2".to_string()),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn initialise(canvas: HtmlCanvasElement) -> Result<Value, JsValue> {
    let query = web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"location".into()).ok())
        .and_then(|loc| js_sys::Reflect::get(&loc, &"search".into()).ok())
        .and_then(|s| s.as_string())
        .unwrap_or_default();
    Ok(initialise_with_query(canvas, &query).await?.report)
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn initialise_with_query(
    canvas: HtmlCanvasElement,
    query: &str,
) -> Result<GraphicsInit, JsValue> {
    let started = now();
    let requested_backend =
        parse_requested_backend(query, canvas.get_attribute("data-backend").as_deref());
    let is_mobile = detect_mobile_viewport();
    let pre_tier = DeviceTier::pre_init_estimate(requested_backend, is_mobile);
    let pre_budget = pre_tier.budget();
    let mut report = json!({
        "schema": "webizen.native-graphics-init/v1",
        "requested": requested_backend,
        "requested_backend": requested_backend,
        "state": "initialising",
        "selected_backend": Value::Null,
        "device_tier": pre_tier.as_str(),
        "tier_budget": tier_budget_json(&pre_budget),
        "is_mobile": is_mobile,
        "reading_fallback": READING_FALLBACK_URL,
        "canvas_width": canvas.width(),
        "canvas_height": canvas.height(),
    });
    publish(&report)?;
    // When `?backend=webgl2` or `data-backend="webgl2"` is set, force `Backends::GL` (WebGL2)
    // directly in `qualia-core-db` so `portal_gpu_backend()` and `selected_backend` genuinely
    // report `"webgl2"` even on WebGPU-capable desktop browsers. Otherwise `portal_init_webgpu`
    // tries WebGPU first and retries GL (WebGL2) once.
    let outcome = if should_force_webgl2(requested_backend) {
        portal_init_with_backend(canvas, true).await
    } else {
        portal_init_webgpu(canvas).await
    };
    report["duration_ms"] = json!(now() - started);
    match outcome {
        Ok(true) => {
            let selected_backend = selected_backend_from_gpu();
            let device_tier = DeviceTier::classify_with_request(
                selected_backend.as_deref(),
                requested_backend,
                is_mobile,
            );
            let tier_budget = device_tier.budget();
            report["state"] = json!("ready");
            report["selected_backend"] = match selected_backend.as_deref() {
                Some(b) => json!(b),
                None => Value::Null,
            };
            report["requested_backend"] = json!(requested_backend);
            report["device_tier"] = json!(device_tier.as_str());
            report["tier_budget"] = tier_budget_json(&tier_budget);
            report["is_mobile"] = json!(is_mobile);
            report["reading_fallback"] = json!(READING_FALLBACK_URL);
            publish(&report)?;
            Ok(GraphicsInit {
                report,
                selected_backend,
                requested_backend,
                device_tier,
                tier_budget,
                is_mobile,
            })
        }
        failure => {
            let device_tier = DeviceTier::classify_with_request(None, requested_backend, is_mobile);
            let tier_budget = device_tier.budget();
            // False is deliberately not assigned a fabricated reason: the core API
            // returns it for missing API, timeout, no adapter or aborted init.
            report["state"] = json!("refused");
            report["selected_backend"] = Value::Null;
            report["requested_backend"] = json!(requested_backend);
            report["device_tier"] = json!(device_tier.as_str());
            report["tier_budget"] = tier_budget_json(&tier_budget);
            report["is_mobile"] = json!(is_mobile);
            report["error"] = json!(match failure {
                Err(e) => js_error(&e),
                _ => "Native WebGPU initialisation returned false (missing API, timeout, unavailable adapter or aborted initialisation)".into(),
            });
            report["reading_fallback"] = json!(READING_FALLBACK_URL);
            publish(&report)?;
            web_sys::console::error_2(
                &"Webizen native WebGPU initialisation failed".into(),
                &report.to_string().into(),
            );
            portal_abort_webgpu();
            // This existing native probe uses detached canvases and does not request
            // a second device. Run only after failure to avoid normal boot overhead.
            match probe_portal_graphics().await {
                Ok(v) => {
                    report["capabilities"] = js_sys::JSON::stringify(&v)
                        .ok()
                        .and_then(|s| s.as_string())
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or(Value::Null)
                }
                Err(e) => report["capability_probe_error"] = json!(js_error(&e)),
            }
            publish(&report)?;
            Err(err("This landscape needs WebGPU or WebGL2 to display its native materials. Your browser could not start either. Please use the reading view to explore the same places and evidence."))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_desktop_webgpu_as_tier_a() {
        assert_eq!(
            DeviceTier::classify(Some("webgpu"), false),
            DeviceTier::TierA_DesktopWebGPU
        );
        assert_eq!(DeviceTier::TierA_DesktopWebGPU.as_str(), "tier-a");
    }

    #[test]
    fn classifies_desktop_webgl2_and_mobile_webgpu_as_tier_b() {
        assert_eq!(
            DeviceTier::classify(Some("webgl2"), false),
            DeviceTier::TierB_DesktopWebGL2_MobileWebGPU
        );
        assert_eq!(
            DeviceTier::classify(Some("webgpu"), true),
            DeviceTier::TierB_DesktopWebGL2_MobileWebGPU
        );
        assert_eq!(
            DeviceTier::TierB_DesktopWebGL2_MobileWebGPU.as_str(),
            "tier-b"
        );
    }

    #[test]
    fn classifies_mobile_webgl2_as_tier_c() {
        assert_eq!(
            DeviceTier::classify(Some("webgl2"), true),
            DeviceTier::TierC_MobileWebGL2
        );
        assert_eq!(DeviceTier::TierC_MobileWebGL2.as_str(), "tier-c");
    }

    #[test]
    fn mobile_viewport_heuristic_handles_dimensions_and_coarse_pointer() {
        assert!(!is_mobile_viewport(1440.0, 900.0, false));
        assert!(!is_mobile_viewport(1280.0, 720.0, false));
        assert!(is_mobile_viewport(800.0, 600.0, false));
        assert!(is_mobile_viewport(390.0, 844.0, true));
        assert!(is_mobile_viewport(1366.0, 1024.0, true));
        assert!(is_mobile_viewport(932.0, 430.0, false));
        assert!(!is_mobile_viewport(0.0, 0.0, false));
        assert!(is_mobile_viewport(0.0, 0.0, true));
    }

    #[test]
    fn parses_backend_override_from_query_and_canvas_attribute() {
        assert_eq!(parse_requested_backend("?backend=webgl2", None), "webgl2");
        assert_eq!(
            parse_requested_backend("?ewm=1&backend=WebGL2&gpu-timing=1", None),
            "webgl2"
        );
        assert_eq!(
            parse_requested_backend("?ewm=1", Some("webgl2")),
            "webgl2"
        );
        assert_eq!(
            parse_requested_backend("?backend=webgpu", Some("webgl2")),
            "webgpu"
        );
        assert_eq!(
            parse_requested_backend("?ewm=1", None),
            "webgpu-or-webgl2"
        );
        assert_eq!(
            parse_requested_backend("?backend=unknown", Some("invalid")),
            "webgpu-or-webgl2"
        );
    }

    #[test]
    fn webgl2_request_enforces_tier_b_desktop_and_tier_c_mobile_caps() {
        assert_eq!(
            DeviceTier::classify_with_request(Some("webgpu"), "webgl2", false),
            DeviceTier::TierB_DesktopWebGL2_MobileWebGPU
        );
        assert_eq!(
            DeviceTier::classify_with_request(Some("webgpu"), "webgl2", true),
            DeviceTier::TierC_MobileWebGL2
        );
        assert_eq!(
            DeviceTier::classify_with_request(Some("webgpu"), "webgpu-or-webgl2", false),
            DeviceTier::TierA_DesktopWebGPU
        );
    }

    #[test]
    fn tier_budgets_match_specification_and_monotonic_invariants() {
        let a = DeviceTier::TierA_DesktopWebGPU.budget();
        let b = DeviceTier::TierB_DesktopWebGL2_MobileWebGPU.budget();
        let c = DeviceTier::TierC_MobileWebGL2.budget();

        assert_eq!(a.tier_id, "tier-a");
        assert_eq!(a.max_resident_triangles, 650_000);
        assert_eq!(a.max_gpu_memory_bytes, 384 * 1024 * 1024);
        assert_eq!(a.max_first_paint_bytes, 24 * 1024 * 1024);
        assert_eq!(a.max_texture_dim, 2048);
        assert_eq!(a.max_dpr, 1.5);
        assert_eq!(a.preferred_lod, "lod0");

        assert_eq!(b.tier_id, "tier-b");
        assert_eq!(b.max_resident_triangles, 320_000);
        assert_eq!(b.max_gpu_memory_bytes, 192 * 1024 * 1024);
        assert_eq!(b.max_first_paint_bytes, 17 * 1024 * 1024);
        assert_eq!(b.max_texture_dim, 1024);
        assert_eq!(b.max_dpr, 1.35);
        assert_eq!(b.preferred_lod, "lod1");

        assert_eq!(c.tier_id, "tier-c");
        assert_eq!(c.max_resident_triangles, 150_000);
        assert_eq!(c.max_gpu_memory_bytes, 96 * 1024 * 1024);
        assert_eq!(c.max_first_paint_bytes, 12 * 1024 * 1024);
        assert_eq!(c.max_texture_dim, 1024);
        assert_eq!(c.max_dpr, 1.15);
        assert_eq!(c.preferred_lod, "lod1");

        assert!(a.max_resident_triangles > b.max_resident_triangles);
        assert!(b.max_resident_triangles > c.max_resident_triangles);
        assert!(a.max_gpu_memory_bytes > b.max_gpu_memory_bytes);
        assert!(b.max_gpu_memory_bytes > c.max_gpu_memory_bytes);
        assert!(a.max_first_paint_bytes > b.max_first_paint_bytes);
        assert!(b.max_first_paint_bytes > c.max_first_paint_bytes);
        assert!(a.max_texture_dim >= b.max_texture_dim);
        assert!(b.max_texture_dim >= c.max_texture_dim);
        assert!(a.max_dpr > b.max_dpr);
        assert!(b.max_dpr > c.max_dpr);
    }

    #[test]
    fn dpr_clamping_respects_tier_budget() {
        let a = DeviceTier::TierA_DesktopWebGPU.budget();
        let b = DeviceTier::TierB_DesktopWebGL2_MobileWebGPU.budget();
        let c = DeviceTier::TierC_MobileWebGL2.budget();
        assert_eq!(clamp_dpr(3.0, Some(2.0), &a), 1.5);
        assert_eq!(clamp_dpr(3.0, Some(2.0), &b), 1.35);
        assert_eq!(clamp_dpr(3.0, Some(2.0), &c), 1.15);
        assert_eq!(clamp_dpr(1.0, Some(1.5), &c), 1.0);
        assert_eq!(clamp_dpr(f64::NAN, None, &c), 1.0);
        assert_eq!(READING_FALLBACK_URL, "/publication/reading.html");
    }

    #[test]
    fn forced_webgl2_switch_triggers_only_on_explicit_webgl2_request() {
        assert!(should_force_webgl2("webgl2"));
        assert!(should_force_webgl2("WebGL2"));
        assert!(!should_force_webgl2("webgpu"));
        assert!(!should_force_webgl2("webgpu-or-webgl2"));
        assert!(should_force_webgl2(parse_requested_backend(
            "?backend=webgl2",
            None
        )));
        assert!(should_force_webgl2(parse_requested_backend(
            "?ewm=1",
            Some("webgl2")
        )));
        assert!(!should_force_webgl2(parse_requested_backend(
            "?backend=webgpu",
            Some("webgl2")
        )));
        assert!(!should_force_webgl2(parse_requested_backend("", None)));
    }
}
