//! OCS co-located infosphere (Noosphere) on Physical Earth. Public place IRIs only.
//! Does not invent RealmClass cyber. Camera zoom is not a coordinate demonstration.

#![allow(dead_code)]

use serde_json::Value;
use vibe::cosmic::colocated::{CoLocatedLayer, CoLocatedStack};
use vibe::cosmic::grounding::GroundingStatus;
use vibe::cosmic::transforms::Geodetic;
use vibe::cosmic::usri::RealmClass;
use wasm_bindgen::JsValue;

pub fn physical_usri(place: &Value) -> Option<&str> {
    place.get("usri")?.as_str()
}

pub fn infosphere_layer(place: &Value) -> Option<&Value> {
    place.get("colocated")?.as_array()?.iter().find(|layer| {
        layer.get("realm_class").and_then(|v| v.as_str()) == Some("noosphere")
            && layer.get("grounding").and_then(|v| v.as_str()) == Some("EmpiricallyAnchored")
    })
}

pub fn realm_class(token: &str) -> Option<RealmClass> {
    RealmClass::from_str(token)
}

pub fn infosphere_grounding(layer: &Value) -> Option<GroundingStatus> {
    if layer.get("grounding").and_then(Value::as_str) != Some("EmpiricallyAnchored") {
        return None;
    }
    Some(GroundingStatus::EmpiricallyAnchored {
        anchor_iri: layer.get("anchor_iri")?.as_str()?.to_string(),
        confidence: 1.0,
        stratum_or_epoch: None,
    })
}

fn geodetic_from_place(place: &Value) -> Option<Geodetic> {
    let b = place.get("bbox_wsen")?.as_array()?;
    if b.len() != 4 {
        return None;
    }
    let west = b[0].as_f64()?;
    let south = b[1].as_f64()?;
    let east = b[2].as_f64()?;
    let north = b[3].as_f64()?;
    let elev = place.get("elevation_m").and_then(Value::as_array);
    let alt = match elev {
        Some(e) if e.len() == 2 => (e[0].as_f64()? + e[1].as_f64()?) * 0.5,
        _ => 0.0,
    };
    Some(Geodetic {
        lat_deg: (south + north) * 0.5,
        lon_deg: (west + east) * 0.5,
        alt_m: alt,
    })
}

/// Catalog-backed stack: Physical Earth base plus a Noosphere infosphere layer.
pub fn colocated_stack(place: &Value) -> Result<CoLocatedStack, JsValue> {
    if realm_class(place.get("realm_class").and_then(Value::as_str).unwrap_or(""))
        != Some(RealmClass::Physical)
    {
        return Err(JsValue::from_str("EWM place realm_class must be physical"));
    }
    let usri = physical_usri(place).ok_or_else(|| JsValue::from_str("EWM place missing usri"))?;
    if usri != "urn:omni:v1:physical:observable:standard:earth:wgs84" {
        return Err(JsValue::from_str("EWM physical USRI must be WGS84 Earth"));
    }
    let geo = geodetic_from_place(place)
        .ok_or_else(|| JsValue::from_str("EWM place bbox_wsen is required for OCS frames"))?;
    let mut base = CoLocatedLayer::physical_base(geo);
    base.usri = usri.into();
    let mut stack = CoLocatedStack::new(base);
    let layer = infosphere_layer(place)
        .ok_or_else(|| JsValue::from_str("EWM place needs a Noosphere infosphere layer"))?;
    if realm_class(layer.get("realm_class").and_then(Value::as_str).unwrap_or(""))
        != Some(RealmClass::Noosphere)
    {
        return Err(JsValue::from_str("Infosphere realm_class must be noosphere"));
    }
    let _grounding = infosphere_grounding(layer)
        .ok_or_else(|| JsValue::from_str("Infosphere grounding must be EmpiricallyAnchored"))?;
    let layer_usri = layer
        .get("usri")
        .and_then(Value::as_str)
        .ok_or_else(|| JsValue::from_str("Infosphere layer missing usri"))?;
    if layer
        .get("realm_class")
        .and_then(Value::as_str)
        .map(|s| s.eq_ignore_ascii_case("cyber"))
        .unwrap_or(false)
        || layer_usri.split(':').any(|part| part.eq_ignore_ascii_case("cyber"))
    {
        return Err(JsValue::from_str(
            "Infosphere layer uses RealmClass::Noosphere on a public place IRI",
        ));
    }
    let name = layer.get("label").and_then(Value::as_str).unwrap_or("infosphere");
    stack.add_layer(CoLocatedLayer {
        usri: layer_usri.into(),
        name: name.into(),
        geodetic: geo,
        is_physical_base: false,
    });
    if stack.layers.len() < 2 {
        return Err(JsValue::from_str("Infosphere layer was not co-located on the physical base"));
    }
    Ok(stack)
}
