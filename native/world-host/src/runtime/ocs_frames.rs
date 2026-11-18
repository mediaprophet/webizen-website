//! OCS co-located infosphere (Noosphere) on Physical Earth. Public place IRIs only.
//! Does not invent RealmClass cyber. Camera zoom is not a coordinate demonstration.

#![allow(dead_code)]

use super::catalog::{ColocatedLayer, Place};
use vibe::cosmic::colocated::{CoLocatedLayer, CoLocatedStack};
use vibe::cosmic::grounding::GroundingStatus;
use vibe::cosmic::transforms::Geodetic;
use vibe::cosmic::usri::RealmClass;
use wasm_bindgen::JsValue;

pub fn physical_usri(place: &Place) -> Option<&str> {
    Some(place.usri.as_str())
}

pub fn infosphere_layer(place: &Place) -> Option<&ColocatedLayer> {
    place.colocated.iter().find(|layer| {
        layer.realm_class == "noosphere" && layer.grounding == "EmpiricallyAnchored"
    })
}

pub fn realm_class(token: &str) -> Option<RealmClass> {
    RealmClass::from_str(token)
}

pub fn infosphere_grounding(layer: &ColocatedLayer) -> Option<GroundingStatus> {
    if layer.grounding != "EmpiricallyAnchored" {
        return None;
    }
    Some(GroundingStatus::EmpiricallyAnchored {
        anchor_iri: layer.anchor_iri.clone(),
        confidence: 1.0,
        stratum_or_epoch: None,
    })
}

fn geodetic_from_place(place: &Place) -> Option<Geodetic> {
    let [west, south, east, north] = place.bbox_wsen;
    let alt = (place.elevation_m[0] + place.elevation_m[1]) * 0.5;
    Some(Geodetic {
        lat_deg: (south + north) * 0.5,
        lon_deg: (west + east) * 0.5,
        alt_m: alt,
    })
}

/// Catalog-backed stack: Physical Earth base plus a Noosphere infosphere layer.
pub fn colocated_stack(place: &Place) -> Result<CoLocatedStack, JsValue> {
    if realm_class(&place.realm_class) != Some(RealmClass::Physical) {
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
    if realm_class(&layer.realm_class) != Some(RealmClass::Noosphere) {
        return Err(JsValue::from_str("Infosphere realm_class must be noosphere"));
    }
    let _grounding = infosphere_grounding(layer)
        .ok_or_else(|| JsValue::from_str("Infosphere grounding must be EmpiricallyAnchored"))?;
    if layer.realm_class.eq_ignore_ascii_case("cyber")
        || layer.usri.split(':').any(|part| part.eq_ignore_ascii_case("cyber"))
    {
        return Err(JsValue::from_str(
            "Infosphere layer uses RealmClass::Noosphere on a public place IRI",
        ));
    }
    let name = if layer.label.is_empty() {
        "infosphere"
    } else {
        layer.label.as_str()
    };
    stack.add_layer(CoLocatedLayer {
        usri: layer.usri.clone(),
        name: name.into(),
        geodetic: geo,
        is_physical_base: false,
    });
    if stack.layers.len() < 2 {
        return Err(JsValue::from_str("Infosphere layer was not co-located on the physical base"));
    }
    Ok(stack)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public_place() -> Place {
        Place {
            id: "canberra-public-reference".into(),
            title: "Southern Tablelands".into(),
            crs: "EPSG:4326".into(),
            bbox_wsen: [149.06, -35.30, 149.08, -35.28],
            elevation_m: [541.0, 677.0],
            gis_terrain_id: "canberra-reference-terrain".into(),
            authored_ground_id: "habitat-terrain".into(),
            usri: "urn:omni:v1:physical:observable:standard:earth:wgs84".into(),
            realm_class: "physical".into(),
            colocated: vec![ColocatedLayer {
                id: "canberra-public-infosphere".into(),
                label: "infosphere".into(),
                realm_class: "noosphere".into(),
                usri: "urn:omni:v1:noosphere:webizen:public:world/canberra-public-reference".into(),
                grounding: "EmpiricallyAnchored".into(),
                anchor_iri: "https://webizen.au/world/canberra-public-reference".into(),
            }],
        }
    }

    #[test]
    fn colocated_stack_is_physical_earth_plus_noosphere() {
        let place = public_place();
        let stack = colocated_stack(&place).expect("public place");
        assert_eq!(stack.layers.len(), 2);
        assert!(stack.layers[0].is_physical_base);
        assert!(!stack.layers[1].is_physical_base);
        assert_eq!(
            physical_usri(&place),
            Some("urn:omni:v1:physical:observable:standard:earth:wgs84")
        );
        assert_eq!(realm_class("noosphere"), Some(RealmClass::Noosphere));
        assert_eq!(realm_class("cyber"), None);
    }

    #[test]
    fn infosphere_rejects_cyber_realm_and_keeps_public_iris() {
        let mut place = public_place();
        place.colocated[0].realm_class = "cyber".into();
        assert!(colocated_stack(&place).is_err());
        place.colocated[0].realm_class = "noosphere".into();
        place.colocated[0].usri = "urn:omni:v1:cyber:example".into();
        assert!(colocated_stack(&place).is_err());
        let layer = infosphere_layer(&public_place()).expect("noosphere layer");
        assert_eq!(
            layer.anchor_iri,
            "https://webizen.au/world/canberra-public-reference"
        );
    }
}
