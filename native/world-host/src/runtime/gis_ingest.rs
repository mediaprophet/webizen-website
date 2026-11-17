//! Consent-gated GIS ingest. v1 refuses live adapters without opt-in.
//! Create-only: new members schedule through the existing windowed Schedule.
//! Public boot does not call live adapters. Timeline.json is never stored.

#![allow(dead_code)]

use super::consent;
use wasm_bindgen::JsValue;

pub fn public_boot_calls_live_adapters() -> bool {
    false
}

pub fn output_is_create_only() -> bool {
    true
}

pub fn refuse_live_without_consent(url: &str) -> Result<(), JsValue> {
    if consent::live_gis_denied(url) {
        Err(JsValue::from_str(
            "Live GIS ingest is default-deny until the visitor opts in",
        ))
    } else {
        Ok(())
    }
}
