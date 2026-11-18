//! Consent-gated GIS ingest. v1 refuses live adapters without opt-in.
//! Create-only: new members schedule through the existing windowed Schedule.
//! Public boot does not call live adapters. Timeline.json is never stored.
//! Native job does ingest; wasm stays under 23.5 MiB and Ranges packed members.

#![allow(dead_code)]

use super::consent;
use serde_json::Value;
use wasm_bindgen::JsValue;

pub fn public_boot_calls_live_adapters() -> bool {
    false
}

pub fn output_is_create_only() -> bool {
    true
}

pub fn job_is_explicit_action_only() -> bool {
    true
}

pub fn gbif_live_parse_deferred() -> bool {
    true
}

/// Browser ingest stays off so wasm stays under 23.5 MiB. Native job does ingest.
pub fn wasm_ingest_opt_in_allowed() -> bool {
    false
}

/// LW-08 second public place. Packed OSM members are native `--opt-in` only.
pub fn second_public_place_id() -> &'static str {
    "river"
}

/// CI packs OSM/DEM/STAC from a consent fixture. Live hosts are not fetched.
pub fn consent_fixture_required_without_live_hosts() -> bool {
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

fn is_hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn scene_id_ok(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 128 {
        return false;
    }
    let first = bytes[0];
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-'))
}

fn member_url_ok(url: &str) -> bool {
    if url.starts_with("//") || url.contains("..") || url.contains('\\') {
        return false;
    }
    url.starts_with("/packages/") || url.starts_with("/production/")
}

fn validate_measured_member(catalog_row_json: &str) -> Result<Value, String> {
    let lowered = catalog_row_json.to_ascii_lowercase();
    if lowered.contains("timeline.json")
        || lowered.contains("inferred_work")
        || lowered.contains("placeids")
    {
        return Err("measured GIS member must not carry Timeline or personal place records".into());
    }
    let row: Value = serde_json::from_str(catalog_row_json)
        .map_err(|_| "measured GIS member must be JSON".to_string())?;
    let obj = row
        .as_object()
        .ok_or_else(|| "measured GIS member must be a catalog row object".to_string())?;
    if obj.get("epistemic_class").and_then(Value::as_str) != Some("measured-gis") {
        return Err("measured GIS member epistemic_class must be measured-gis".into());
    }
    let scene_id = obj
        .get("scene_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "measured GIS member scene_id is required".to_string())?;
    if !scene_id_ok(scene_id) {
        return Err("measured GIS member scene_id is empty or invalid".into());
    }
    let url = obj
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "measured GIS member url is required".to_string())?;
    if !member_url_ok(url) {
        return Err("measured GIS member url must start with /packages/ or /production/".into());
    }
    let sha = obj
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| "measured GIS member sha256 is required".to_string())?;
    if !is_hex64(sha) {
        return Err("measured GIS member sha256 must be 64 lowercase hex characters".into());
    }
    if sha.bytes().all(|b| b == b'0') {
        return Err("measured GIS member sha256 must be non-zero".into());
    }
    Ok(row)
}

/// Validate a measured-gis catalog row. New members enter the existing
/// windowed Schedule as catalog rows, not a camera-tick scrape. No HTTP.
pub fn append_measured_member(catalog_row_json: &str) -> Result<Value, JsValue> {
    validate_measured_member(catalog_row_json).map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_row() -> String {
        r#"{"scene_id":"canberra-reference-terrain","epistemic_class":"measured-gis","url":"/packages/by-sha256/bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de","sha256":"bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de"}"#.into()
    }

    fn river_row() -> String {
        r#"{"scene_id":"river-osm-waterway","epistemic_class":"measured-gis","url":"/packages/by-sha256/c4f1b0e29a7d6c8f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7081","sha256":"c4f1b0e29a7d6c8f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7081"}"#.into()
    }

    #[test]
    fn public_boot_stays_default_deny() {
        assert!(!public_boot_calls_live_adapters());
        assert!(output_is_create_only());
        assert!(job_is_explicit_action_only());
        assert!(gbif_live_parse_deferred());
        assert!(!wasm_ingest_opt_in_allowed());
        assert_eq!(second_public_place_id(), "river");
        assert!(consent_fixture_required_without_live_hosts());
    }

    #[test]
    fn river_osm_member_is_measured_gis_enhancement_row() {
        let row = validate_measured_member(&river_row()).expect("valid river measured-gis row");
        assert_eq!(row["epistemic_class"], "measured-gis");
        assert_eq!(row["scene_id"], "river-osm-waterway");
        assert!(row["url"].as_str().unwrap().starts_with("/packages/"));
    }

    #[test]
    fn empty_registry_fails_closed_for_live_gis() {
        assert!(consent::live_gis_denied(
            "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/13/7487/4954.png"
        ));
        assert!(consent::live_gis_denied("https://api.gbif.org/v1/occurrence/search"));
        assert!(!consent::live_gis_denied(
            "/packages/by-sha256/bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de"
        ));
    }

    #[test]
    fn measured_member_is_a_catalog_row() {
        let row = validate_measured_member(&valid_row()).expect("valid measured-gis row");
        assert_eq!(row["epistemic_class"], "measured-gis");
    }

    #[test]
    fn measured_member_rejects_live_url_and_wrong_class() {
        assert!(validate_measured_member(
            r#"{"scene_id":"canberra-reference-terrain","epistemic_class":"authored-artwork","url":"/packages/x","sha256":"bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de"}"#
        )
        .is_err());
        assert!(validate_measured_member(
            r#"{"scene_id":"canberra-reference-terrain","epistemic_class":"measured-gis","url":"https://example.invalid/dem.10d","sha256":"bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de"}"#
        )
        .is_err());
        assert!(validate_measured_member(
            r#"{"scene_id":"","epistemic_class":"measured-gis","url":"/packages/x","sha256":"bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de"}"#
        )
        .is_err());
        assert!(validate_measured_member(
            r#"{"scene_id":"canberra-reference-terrain","epistemic_class":"measured-gis","url":"/packages/x","sha256":"abc"}"#
        )
        .is_err());
    }

    fn row_with_note(note: &str) -> String {
        format!(
            r#"{{"scene_id":"canberra-reference-terrain","epistemic_class":"measured-gis","url":"/packages/by-sha256/bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de","sha256":"bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de","note":"{note}"}}"#
        )
    }

    #[test]
    fn measured_member_rejects_timeline_inferred_work_and_place_ids() {
        for token in [
            "Timeline.json",
            "INFERRED_WORK",
            "placeIds",
            "TimeLine.JSON",
            "PlaceIDs",
        ] {
            let row = row_with_note(token);
            assert!(
                validate_measured_member(&row).is_err(),
                "validate should refuse {token}"
            );
            assert!(
                append_measured_member(&row).is_err(),
                "append should refuse {token}"
            );
        }
    }

    #[test]
    fn measured_member_rejects_all_zero_sha256() {
        let row = r#"{"scene_id":"canberra-reference-terrain","epistemic_class":"measured-gis","url":"/packages/by-sha256/0000000000000000000000000000000000000000000000000000000000000000","sha256":"0000000000000000000000000000000000000000000000000000000000000000"}"#;
        assert!(validate_measured_member(row).is_err());
        assert!(append_measured_member(row).is_err());
    }
}
