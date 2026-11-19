//! Hierarchical World-to-Cosmos OCS Frame Tree & Camera-Relative Floating Origin,
//! plus OCS co-located infosphere (Noosphere) on Physical Earth. Public place IRIs only.
//! Does not invent RealmClass cyber.

#![allow(dead_code)]

use super::catalog::{ColocatedLayer, Place};
use serde::{Deserialize, Serialize};
use vibe::cosmic::colocated::{CoLocatedLayer, CoLocatedStack};
use vibe::cosmic::grounding::GroundingStatus;
use vibe::cosmic::transforms::Geodetic;
use vibe::cosmic::usri::RealmClass;
use wasm_bindgen::JsValue;

/// WGS84 semi-major axis in metres.
pub const WGS84_A_M: f64 = 6_378_137.0;
/// WGS84 flattening factor (`1 / 298.257223563`).
pub const WGS84_F: f64 = 1.0 / 298.257_223_563;
/// WGS84 first eccentricity squared: `e^2 = f * (2 - f)`.
pub const WGS84_E2: f64 = WGS84_F * (2.0 - WGS84_F);

/// Canonical scale-regime boundary altitudes (metres) with +/-15% hysteresis bands.
pub const LOCAL_TO_PLANETARY_M: f64 = 1.0e4;
pub const PLANETARY_TO_HELIOCENTRIC_M: f64 = 1.0e7;
pub const HELIOCENTRIC_TO_GALACTIC_M: f64 = 1.0e12;
pub const HYSTERESIS_BAND_FRACTION: f64 = 0.15;

/// Scale regimes spanning continuous zoom from a 1 cm leaf to the galactic cosmos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScaleRegime {
    /// Local catchment East-North-Up metres (`10^-2 m` to `10^4 m`), anchored to WGS84 geodetic origin.
    LocalEnu,
    /// Planetary WGS84 / ECEF globe + regional bioregion anchors (`10^4 m` to `10^7 m`).
    PlanetaryWgs84,
    /// Sun-centred ICRF J2000 solar system frame backed by JPL Horizons DE441 (`10^7 m` to `10^12 m`).
    HeliocentricIcrf,
    /// Extensible `.q42` celestial catalog layer (`> 10^12 m`).
    GalacticCosmos,
}

impl ScaleRegime {
    pub const fn as_str(self) -> &'static str {
        match self {
            ScaleRegime::LocalEnu => "LocalEnu",
            ScaleRegime::PlanetaryWgs84 => "PlanetaryWgs84",
            ScaleRegime::HeliocentricIcrf => "HeliocentricIcrf",
            ScaleRegime::GalacticCosmos => "GalacticCosmos",
        }
    }

    pub const fn min_altitude_m(self) -> f64 {
        match self {
            ScaleRegime::LocalEnu => 1.0e-2,
            ScaleRegime::PlanetaryWgs84 => LOCAL_TO_PLANETARY_M,
            ScaleRegime::HeliocentricIcrf => PLANETARY_TO_HELIOCENTRIC_M,
            ScaleRegime::GalacticCosmos => HELIOCENTRIC_TO_GALACTIC_M,
        }
    }

    pub const fn max_altitude_m(self) -> f64 {
        match self {
            ScaleRegime::LocalEnu => LOCAL_TO_PLANETARY_M,
            ScaleRegime::PlanetaryWgs84 => PLANETARY_TO_HELIOCENTRIC_M,
            ScaleRegime::HeliocentricIcrf => HELIOCENTRIC_TO_GALACTIC_M,
            ScaleRegime::GalacticCosmos => f64::INFINITY,
        }
    }

    pub const fn parent(self) -> Option<ScaleRegime> {
        match self {
            ScaleRegime::LocalEnu => Some(ScaleRegime::PlanetaryWgs84),
            ScaleRegime::PlanetaryWgs84 => Some(ScaleRegime::HeliocentricIcrf),
            ScaleRegime::HeliocentricIcrf => Some(ScaleRegime::GalacticCosmos),
            ScaleRegime::GalacticCosmos => None,
        }
    }

    pub const fn child(self) -> Option<ScaleRegime> {
        match self {
            ScaleRegime::LocalEnu => None,
            ScaleRegime::PlanetaryWgs84 => Some(ScaleRegime::LocalEnu),
            ScaleRegime::HeliocentricIcrf => Some(ScaleRegime::PlanetaryWgs84),
            ScaleRegime::GalacticCosmos => Some(ScaleRegime::HeliocentricIcrf),
        }
    }
}

/// Double-precision (`f64`) 3D world position in metres within the active OCS frame.
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct WorldPos64 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl WorldPos64 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }

    pub fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }

    pub fn scale(self, s: f64) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn distance(self, other: Self) -> f64 {
        self.sub(other).length()
    }

    pub const fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

/// Camera-relative floating origin eliminating `f32` GPU jitter across 10^11+ m scales.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloatingOrigin {
    pub camera_world: WorldPos64,
    pub regime: ScaleRegime,
}

impl FloatingOrigin {
    pub const fn new(camera_world: WorldPos64, regime: ScaleRegime) -> Self {
        Self {
            camera_world,
            regime,
        }
    }

    pub fn for_altitude(camera_world: WorldPos64, altitude_m: f64) -> Self {
        Self {
            camera_world,
            regime: select_regime_for_altitude_m(altitude_m),
        }
    }

    pub fn update_altitude(&mut self, camera_world: WorldPos64, altitude_m: f64) -> ScaleRegime {
        self.camera_world = camera_world;
        self.regime = select_regime_with_hysteresis(self.regime, altitude_m);
        self.regime
    }
}

/// Rebase a double-precision world coordinate into camera-relative `f32` render coordinates.
/// Subtraction occurs in `f64` before narrowing to `f32`, eliminating precision jitter at orbital scales.
pub fn rebase_to_camera_f32(object_world: WorldPos64, origin: &FloatingOrigin) -> [f32; 3] {
    [
        (object_world.x - origin.camera_world.x) as f32,
        (object_world.y - origin.camera_world.y) as f32,
        (object_world.z - origin.camera_world.z) as f32,
    ]
}

/// Stateless canonical regime selector for a given camera altitude above the local reference surface.
pub fn select_regime_for_altitude_m(altitude_m: f64) -> ScaleRegime {
    let alt = altitude_m.max(0.0);
    if alt < LOCAL_TO_PLANETARY_M {
        ScaleRegime::LocalEnu
    } else if alt < PLANETARY_TO_HELIOCENTRIC_M {
        ScaleRegime::PlanetaryWgs84
    } else if alt < HELIOCENTRIC_TO_GALACTIC_M {
        ScaleRegime::HeliocentricIcrf
    } else {
        ScaleRegime::GalacticCosmos
    }
}

/// Hysteresis-guarded regime selector (+/-15% band) so continuous zoom never chatters at regime seams.
pub fn select_regime_with_hysteresis(current: ScaleRegime, altitude_m: f64) -> ScaleRegime {
    let alt = altitude_m.max(0.0);
    let up = 1.0 + HYSTERESIS_BAND_FRACTION;
    let down = 1.0 - HYSTERESIS_BAND_FRACTION;
    match current {
        ScaleRegime::LocalEnu => {
            if alt >= HELIOCENTRIC_TO_GALACTIC_M * up {
                ScaleRegime::GalacticCosmos
            } else if alt >= PLANETARY_TO_HELIOCENTRIC_M * up {
                ScaleRegime::HeliocentricIcrf
            } else if alt >= LOCAL_TO_PLANETARY_M * up {
                ScaleRegime::PlanetaryWgs84
            } else {
                ScaleRegime::LocalEnu
            }
        }
        ScaleRegime::PlanetaryWgs84 => {
            if alt < LOCAL_TO_PLANETARY_M * down {
                ScaleRegime::LocalEnu
            } else if alt >= HELIOCENTRIC_TO_GALACTIC_M * up {
                ScaleRegime::GalacticCosmos
            } else if alt >= PLANETARY_TO_HELIOCENTRIC_M * up {
                ScaleRegime::HeliocentricIcrf
            } else {
                ScaleRegime::PlanetaryWgs84
            }
        }
        ScaleRegime::HeliocentricIcrf => {
            if alt < LOCAL_TO_PLANETARY_M * down {
                ScaleRegime::LocalEnu
            } else if alt < PLANETARY_TO_HELIOCENTRIC_M * down {
                ScaleRegime::PlanetaryWgs84
            } else if alt >= HELIOCENTRIC_TO_GALACTIC_M * up {
                ScaleRegime::GalacticCosmos
            } else {
                ScaleRegime::HeliocentricIcrf
            }
        }
        ScaleRegime::GalacticCosmos => {
            if alt < LOCAL_TO_PLANETARY_M * down {
                ScaleRegime::LocalEnu
            } else if alt < PLANETARY_TO_HELIOCENTRIC_M * down {
                ScaleRegime::PlanetaryWgs84
            } else if alt < HELIOCENTRIC_TO_GALACTIC_M * down {
                ScaleRegime::HeliocentricIcrf
            } else {
                ScaleRegime::GalacticCosmos
            }
        }
    }
}

/// Convert WGS84 geodetic coordinates (degrees latitude, degrees longitude, ellipsoidal height in metres)
/// into Earth-Centred Earth-Fixed (ECEF) Cartesian metres.
pub fn geodetic_to_ecef_m(lat_deg: f64, lon_deg: f64, alt_m: f64) -> WorldPos64 {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();
    let n = WGS84_A_M / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    WorldPos64 {
        x: (n + alt_m) * cos_lat * cos_lon,
        y: (n + alt_m) * cos_lat * sin_lon,
        z: (n * (1.0 - WGS84_E2) + alt_m) * sin_lat,
    }
}

/// Convert local East-North-Up (`[east_m, north_m, up_m]`) offset anchored at a WGS84 geodetic origin
/// into Earth-Centred Earth-Fixed (ECEF) Cartesian metres.
pub fn enu_to_ecef(
    local_enu: [f64; 3],
    origin_lat_deg: f64,
    origin_lon_deg: f64,
    origin_alt_m: f64,
) -> WorldPos64 {
    let origin = geodetic_to_ecef_m(origin_lat_deg, origin_lon_deg, origin_alt_m);
    let lat = origin_lat_deg.to_radians();
    let lon = origin_lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();
    let [e, n, u] = local_enu;
    let dx = -sin_lon * e - sin_lat * cos_lon * n + cos_lat * cos_lon * u;
    let dy = cos_lon * e - sin_lat * sin_lon * n + cos_lat * sin_lon * u;
    let dz = cos_lat * n + sin_lat * u;
    WorldPos64 {
        x: origin.x + dx,
        y: origin.y + dy,
        z: origin.z + dz,
    }
}

/// Inverse transform: convert an ECEF position in metres into local East-North-Up metres
/// relative to a WGS84 geodetic origin.
pub fn ecef_to_enu(
    ecef: WorldPos64,
    origin_lat_deg: f64,
    origin_lon_deg: f64,
    origin_alt_m: f64,
) -> [f64; 3] {
    let origin = geodetic_to_ecef_m(origin_lat_deg, origin_lon_deg, origin_alt_m);
    let delta = ecef.sub(origin);
    let lat = origin_lat_deg.to_radians();
    let lon = origin_lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();
    let e = -sin_lon * delta.x + cos_lon * delta.y;
    let n = -sin_lat * cos_lon * delta.x - sin_lat * sin_lon * delta.y + cos_lat * delta.z;
    let u = cos_lat * cos_lon * delta.x + cos_lat * sin_lon * delta.y + sin_lat * delta.z;
    [e, n, u]
}

/// Single node in the hierarchical World-to-Cosmos OCS Frame Tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OcsFrameNode {
    pub id: String,
    pub label: String,
    pub usri: String,
    pub regime: ScaleRegime,
    pub parent_id: Option<String>,
    pub origin_world: WorldPos64,
    pub velocity_m_s: Option<[f64; 3]>,
    pub geodetic_deg: Option<[f64; 3]>,
    pub radius_m: f64,
    pub min_altitude_m: f64,
    pub max_altitude_m: f64,
}

/// Hierarchical World-to-Cosmos OCS Frame Tree bridging LocalEnu -> PlanetaryWgs84 -> HeliocentricIcrf -> GalacticCosmos.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OcsFrameTree {
    pub epoch_tdb: String,
    pub nodes: Vec<OcsFrameNode>,
}

impl OcsFrameTree {
    pub fn new(epoch_tdb: impl Into<String>) -> Self {
        Self {
            epoch_tdb: epoch_tdb.into(),
            nodes: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: OcsFrameNode) {
        self.nodes.push(node);
    }

    pub fn get(&self, id: &str) -> Option<&OcsFrameNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn children(&self, parent_id: &str) -> Vec<&OcsFrameNode> {
        self.nodes
            .iter()
            .filter(|n| n.parent_id.as_deref() == Some(parent_id))
            .collect()
    }

    pub fn nodes_in_regime(&self, regime: ScaleRegime) -> Vec<&OcsFrameNode> {
        self.nodes.iter().filter(|n| n.regime == regime).collect()
    }

    /// Walk upward from `id` to the root frame (cycle-safe, bounded by node count).
    pub fn ancestors(&self, id: &str) -> Vec<&OcsFrameNode> {
        let mut chain = Vec::new();
        let mut cursor = self.get(id);
        let mut steps = 0;
        while let Some(node) = cursor {
            chain.push(node);
            steps += 1;
            if steps > self.nodes.len() {
                break;
            }
            cursor = node.parent_id.as_deref().and_then(|pid| self.get(pid));
        }
        chain
    }

    /// Canonical Webizen World-to-Cosmos frame tree anchored on JPL Horizons DE441 (2026-11-16 TDB)
    /// and the four SE Australian bioregions + Canberra public reference catchment.
    pub fn default_webizen_cosmos() -> Self {
        let mut tree = Self::new("2026-11-16T00:00:00Z");

        // GalacticCosmos root
        tree.add_node(OcsFrameNode {
            id: "did:q42:cosmos:milky-way".into(),
            label: "Milky Way Galactic Cosmos".into(),
            usri: "urn:omni:v1:physical:observable:standard:cosmos:galactic".into(),
            regime: ScaleRegime::GalacticCosmos,
            parent_id: None,
            origin_world: WorldPos64::zero(),
            velocity_m_s: None,
            geodetic_deg: None,
            radius_m: 5.0e20,
            min_altitude_m: HELIOCENTRIC_TO_GALACTIC_M,
            max_altitude_m: f64::INFINITY,
        });

        // HeliocentricIcrf (JPL Horizons DE441, JD 2461360.5 = 2026-Nov-16 00:00:00 TDB)
        tree.add_node(OcsFrameNode {
            id: "did:q42:cosmos:sun".into(),
            label: "Sun (Sol)".into(),
            usri: "urn:omni:v1:physical:observable:standard:sun:icrf".into(),
            regime: ScaleRegime::HeliocentricIcrf,
            parent_id: Some("did:q42:cosmos:milky-way".into()),
            origin_world: WorldPos64::zero(),
            velocity_m_s: Some([0.0, 0.0, 0.0]),
            geodetic_deg: None,
            radius_m: 695_700_000.0,
            min_altitude_m: PLANETARY_TO_HELIOCENTRIC_M,
            max_altitude_m: HELIOCENTRIC_TO_GALACTIC_M,
        });

        let earth_icrf = WorldPos64::new(
            88_500_030_005.589_98,
            108_798_923_991.076_6,
            47_161_666_865.260_02,
        );
        tree.add_node(OcsFrameNode {
            id: "did:q42:cosmos:earth".into(),
            label: "Earth (WGS84 / JPL 399)".into(),
            usri: "urn:omni:v1:physical:observable:standard:earth:wgs84".into(),
            regime: ScaleRegime::HeliocentricIcrf,
            parent_id: Some("did:q42:cosmos:sun".into()),
            origin_world: earth_icrf,
            velocity_m_s: Some([-24_365.727_941_551_98, 16_238.386_443_689_1, 7_038.046_127_295_474]),
            geodetic_deg: None,
            radius_m: 6_371_010.0,
            min_altitude_m: LOCAL_TO_PLANETARY_M,
            max_altitude_m: HELIOCENTRIC_TO_GALACTIC_M,
        });

        tree.add_node(OcsFrameNode {
            id: "did:q42:cosmos:moon".into(),
            label: "Moon (Luna / JPL 301)".into(),
            usri: "urn:omni:v1:physical:observable:standard:moon:icrf".into(),
            regime: ScaleRegime::HeliocentricIcrf,
            parent_id: Some("did:q42:cosmos:earth".into()),
            origin_world: WorldPos64::new(384_400_000.0, 0.0, 0.0),
            velocity_m_s: Some([0.0, 1_022.0, 0.0]),
            geodetic_deg: None,
            radius_m: 1_737_400.0,
            min_altitude_m: PLANETARY_TO_HELIOCENTRIC_M,
            max_altitude_m: HELIOCENTRIC_TO_GALACTIC_M,
        });

        tree.add_node(OcsFrameNode {
            id: "did:q42:cosmos:mars".into(),
            label: "Mars (JPL 499)".into(),
            usri: "urn:omni:v1:physical:observable:standard:mars:icrf".into(),
            regime: ScaleRegime::HeliocentricIcrf,
            parent_id: Some("did:q42:cosmos:sun".into()),
            origin_world: WorldPos64::new(
                -72_391_939_208.309_78,
                208_026_428_048.865_2,
                97_369_318_095.567_12,
            ),
            velocity_m_s: Some([
                -22_190.850_396_625_03,
                -4_969.050_404_339_35,
                -1_680.704_404_674_567,
            ]),
            geodetic_deg: None,
            radius_m: 3_389_920.0,
            min_altitude_m: PLANETARY_TO_HELIOCENTRIC_M,
            max_altitude_m: HELIOCENTRIC_TO_GALACTIC_M,
        });

        // PlanetaryWgs84 SE Australian bioregions from data/ewm/gis/bioregions-se-au.json
        let bioregions = [
            (
                "did:q42:bioregion:northern-rivers-coastal",
                "Northern Rivers & Far North Coast (NSW)",
                -28.59,
                153.41,
                575.0,
                55_000.0,
            ),
            (
                "did:q42:bioregion:sydney-basin-sandstone",
                "Sydney Basin Sandstone Plateaus & Coastal Gorges",
                -33.80,
                150.80,
                625.0,
                75_000.0,
            ),
            (
                "did:q42:bioregion:snowy-mountains-alpine",
                "Snowy Mountains, Australian Alps & Monaro Tablelands",
                -35.95,
                148.625,
                1339.0,
                95_000.0,
            ),
            (
                "did:q42:bioregion:victorian-central-highlands",
                "Victorian Central Highlands & Yarra-Goulburn Catchments",
                -37.575,
                145.825,
                845.0,
                65_000.0,
            ),
        ];
        for (id, label, lat, lon, alt, radius_m) in bioregions {
            tree.add_node(OcsFrameNode {
                id: id.into(),
                label: label.into(),
                usri: "urn:omni:v1:physical:observable:standard:earth:wgs84".into(),
                regime: ScaleRegime::PlanetaryWgs84,
                parent_id: Some("did:q42:cosmos:earth".into()),
                origin_world: geodetic_to_ecef_m(lat, lon, alt),
                velocity_m_s: None,
                geodetic_deg: Some([lat, lon, alt]),
                radius_m,
                min_altitude_m: LOCAL_TO_PLANETARY_M,
                max_altitude_m: PLANETARY_TO_HELIOCENTRIC_M,
            });
        }

        // LocalEnu anchor: Canberra public reference catchment (-35.29, 149.07, 541 m)
        tree.add_node(OcsFrameNode {
            id: "canberra-public-reference".into(),
            label: "Canberra Southern Tablelands Catchment (Local ENU)".into(),
            usri: "urn:omni:v1:physical:observable:standard:earth:wgs84".into(),
            regime: ScaleRegime::LocalEnu,
            parent_id: Some("did:q42:bioregion:snowy-mountains-alpine".into()),
            origin_world: geodetic_to_ecef_m(-35.29, 149.07, 541.0),
            velocity_m_s: None,
            geodetic_deg: Some([-35.29, 149.07, 541.0]),
            radius_m: 300.0,
            min_altitude_m: 1.0e-2,
            max_altitude_m: LOCAL_TO_PLANETARY_M,
        });

        tree
    }
}

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

    #[test]
    fn floating_origin_eliminates_f32_jitter_at_one_au() {
        // Earth at ~1 AU (~1.496e11 m) from the Sun in ICRF; two objects 0.05 m apart.
        let camera = WorldPos64::new(
            88_500_030_005.589_98,
            108_798_923_991.076_6,
            47_161_666_865.260_02,
        );
        let leaf = WorldPos64::new(camera.x + 0.05, camera.y + 1.25, camera.z - 0.75);
        let origin = FloatingOrigin::new(camera, ScaleRegime::HeliocentricIcrf);
        let rel = rebase_to_camera_f32(leaf, &origin);
        // At 1 AU (~10^11 m), f64 ULP is ~15 micrometres vs ~8.2 km in naive f32.
        assert!((rel[0] - 0.05_f32).abs() < 1e-4);
        assert!((rel[1] - 1.25_f32).abs() < 1e-4);
        assert!((rel[2] + 0.75_f32).abs() < 1e-4);
    }

    #[test]
    fn wgs84_geodetic_ecef_and_enu_roundtrip() {
        let eq = geodetic_to_ecef_m(0.0, 0.0, 0.0);
        assert!((eq.x - WGS84_A_M).abs() < 1e-6);
        assert!(eq.y.abs() < 1e-6);
        assert!(eq.z.abs() < 1e-6);

        let pole = geodetic_to_ecef_m(90.0, 0.0, 0.0);
        let wgs84_b = WGS84_A_M * (1.0 - WGS84_F);
        assert!(pole.x.abs() < 1e-6);
        assert!((pole.z - wgs84_b).abs() < 1e-6);

        // Canberra catchment local ENU -> ECEF -> ENU round-trip
        let local = [125.5, -240.25, 37.8];
        let ecef = enu_to_ecef(local, -35.29, 149.07, 541.0);
        let back = ecef_to_enu(ecef, -35.29, 149.07, 541.0);
        assert!((back[0] - local[0]).abs() < 1e-6);
        assert!((back[1] - local[1]).abs() < 1e-6);
        assert!((back[2] - local[2]).abs() < 1e-6);
    }

    #[test]
    fn scale_regime_transitions_and_hysteresis() {
        assert_eq!(select_regime_for_altitude_m(1.0), ScaleRegime::LocalEnu);
        assert_eq!(select_regime_for_altitude_m(600.0), ScaleRegime::LocalEnu);
        assert_eq!(
            select_regime_for_altitude_m(6_371_000.0),
            ScaleRegime::PlanetaryWgs84
        );
        assert_eq!(
            select_regime_for_altitude_m(1.496e11),
            ScaleRegime::HeliocentricIcrf
        );
        assert_eq!(
            select_regime_for_altitude_m(2.0e12),
            ScaleRegime::GalacticCosmos
        );

        // Hysteresis prevents chatter around 10 km seam (8.5 km .. 11.5 km)
        assert_eq!(
            select_regime_with_hysteresis(ScaleRegime::LocalEnu, 10_500.0),
            ScaleRegime::LocalEnu
        );
        assert_eq!(
            select_regime_with_hysteresis(ScaleRegime::LocalEnu, 12_000.0),
            ScaleRegime::PlanetaryWgs84
        );
        assert_eq!(
            select_regime_with_hysteresis(ScaleRegime::PlanetaryWgs84, 9_500.0),
            ScaleRegime::PlanetaryWgs84
        );
        assert_eq!(
            select_regime_with_hysteresis(ScaleRegime::PlanetaryWgs84, 8_000.0),
            ScaleRegime::LocalEnu
        );
    }

    #[test]
    fn default_webizen_cosmos_tree_spans_all_four_regimes() {
        let tree = OcsFrameTree::default_webizen_cosmos();
        assert_eq!(tree.nodes_in_regime(ScaleRegime::GalacticCosmos).len(), 1);
        assert_eq!(tree.nodes_in_regime(ScaleRegime::HeliocentricIcrf).len(), 4);
        assert_eq!(tree.nodes_in_regime(ScaleRegime::PlanetaryWgs84).len(), 4);
        assert_eq!(tree.nodes_in_regime(ScaleRegime::LocalEnu).len(), 1);
        let chain = tree.ancestors("canberra-public-reference");
        assert_eq!(chain.len(), 5);
        assert_eq!(chain[0].id, "canberra-public-reference");
        assert_eq!(chain[1].id, "did:q42:bioregion:snowy-mountains-alpine");
        assert_eq!(chain[2].id, "did:q42:cosmos:earth");
        assert_eq!(chain[3].id, "did:q42:cosmos:sun");
        assert_eq!(chain[4].id, "did:q42:cosmos:milky-way");
    }
}
