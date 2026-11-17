//! Native renderer admission and observable initialisation diagnostics.
use super::*;
use qualia_core_db::render::portal::portal_gpu_backend;
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
fn publish(report: &Value) -> Result<(), JsValue> {
    let value = js_sys::JSON::parse(&report.to_string())?;
    js_sys::Reflect::set(
        &web_sys::window().unwrap(),
        &"__webizenNativeInit".into(),
        &value,
    )?;
    Ok(())
}
/// Honest live adapter from pending/adopted `PortalGpu`. Not `navigator.gpu`.
fn selected_backend_from_gpu() -> Value {
    match portal_gpu_backend().as_deref() {
        Some("webgpu") => json!("webgpu"),
        Some("webgl2") => json!("webgl2"),
        _ => Value::Null,
    }
}
pub(super) async fn initialise(canvas: HtmlCanvasElement) -> Result<Value, JsValue> {
    let started = now();
    let mut report = json!({"schema":"webizen.native-graphics-init/v1", "requested":"webgpu-or-webgl2", "state":"initialising", "canvas_width":canvas.width(), "canvas_height":canvas.height()});
    publish(&report)?;
    let outcome = portal_init_webgpu(canvas).await;
    report["duration_ms"] = json!(now() - started);
    match outcome {
        Ok(true) => {
            report["state"] = json!("ready");
            report["selected_backend"] = selected_backend_from_gpu();
            publish(&report)?;
            Ok(report)
        }
        failure => {
            // False is deliberately not assigned a fabricated reason: the core API
            // returns it for missing API, timeout, no adapter or aborted init.
            report["state"] = json!("refused");
            report["selected_backend"] = Value::Null;
            report["error"] = json!(match failure { Err(e) => js_error(&e), _ => "Native WebGPU initialisation returned false (missing API, timeout, unavailable adapter or aborted initialisation)".into() });
            report["reading_fallback"] = json!("/publication/reading.html");
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
