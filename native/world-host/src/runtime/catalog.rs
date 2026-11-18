//! EWM catalog from admitted Q42 volumes. SPARQL joins GIS place to scene assets.
//! Per-place Base lives on each PublicPlace (`placeBase` / `hasBaseAsset`).
//! Woodland first-paint membership stays 24 for field-studio boot.

use super::knowledge::{
    self, asset_id, QuinIndex, CATALOG_KIND, CATALOG_PACK, ENTRY_KIND, FIRST_PAINT_JOIN, GIS_JOIN,
    LAYER_KIND, PLACE_KIND, REL,
};
use qualia_core_db::vibe_host::PoetSnapshot;
use std::collections::BTreeSet;
use wasm_bindgen::JsValue;

const GIS_PLACE_ID: &str = "canberra-public-reference";
const FIELD_STUDIO_ID: &str = "field-studio";
const REQUIRED_NAMED_PLACES: [&str; 4] = ["field-studio", "river", "canopy", "commons"];

#[derive(Clone, Debug)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct CatalogEntry {
    pub entry_id: u32,
    pub scene_id: String,
    pub place_id: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
    pub triangles: u64,
    pub vertices: u64,
    pub role: String,
    pub obligation: String,
    pub aabb: Option<Aabb>,
    pub tiles: Vec<(i32, i32)>,
}

#[derive(Clone, Debug)]
pub struct CatalogEdge {
    pub from_entry_id: u32,
    pub to_entry_id: u32,
    pub reason: String,
    pub obligation: String,
}

#[derive(Clone, Debug)]
pub struct ColocatedLayer {
    pub id: String,
    pub label: String,
    pub realm_class: String,
    pub usri: String,
    pub grounding: String,
    pub anchor_iri: String,
}

#[derive(Clone, Debug)]
pub struct Place {
    pub id: String,
    pub title: String,
    pub crs: String,
    pub bbox_wsen: [f64; 4],
    pub elevation_m: [f64; 2],
    pub gis_terrain_id: String,
    pub authored_ground_id: String,
    pub usri: String,
    pub realm_class: String,
    pub colocated: Vec<ColocatedLayer>,
    pub base: Vec<String>,
    pub enhancement: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub descriptor_sha384: String,
    pub context_sha384: String,
    pub generation: u64,
    pub entries: u32,
}

#[derive(Clone, Debug)]
pub struct SparqlJoin {
    pub engine: String,
    pub gis_place: String,
    pub gis_terrain: String,
    pub first_paint: Vec<String>,
    pub sparql_hits: usize,
}

#[derive(Clone, Debug)]
pub struct Catalog {
    pub schema: String,
    pub pack_id: String,
    pub inner_ranges: bool,
    pub streaming_default: bool,
    pub places: Vec<Place>,
    pub candidate: Candidate,
    pub first_paint: Vec<String>,
    pub entries: Vec<CatalogEntry>,
    pub edges: Vec<CatalogEdge>,
    pub declared_bytes: u64,
    pub declared_triangles: u64,
    pub declared_vertices: u64,
    pub join: SparqlJoin,
}

impl Catalog {
    pub fn from_host(host: &mut PoetSnapshot) -> Result<Self, String> {
        let gis = knowledge::sparql(host, GIS_JOIN).unwrap_or(vibe::Value::Null);
        let paint = knowledge::sparql(host, FIRST_PAINT_JOIN).unwrap_or(vibe::Value::Null);
        let index = QuinIndex::from_host(host)?;
        Self::from_index(index, host, &gis, &paint)
    }

    fn from_index(
        index: QuinIndex,
        host: &PoetSnapshot,
        gis: &vibe::Value,
        paint: &vibe::Value,
    ) -> Result<Self, String> {
        let pack = index.subject_of_kind(CATALOG_KIND)?;
        if pack != CATALOG_PACK {
            return Err("Catalog Q42 pack IRI mismatch".into());
        }
        let schema = index.text(&pack, "schema")?;
        if schema != "webizen.ewm-catalog/v1" {
            return Err("Unsupported EWM catalog schema".into());
        }
        let streaming_default = index.boolean(&pack, "streamingDefault")?;
        if streaming_default {
            return Err("EWM catalog must keep streaming.default false".into());
        }
        let inner_ranges = index.boolean(&pack, "innerRanges")?;
        if inner_ranges {
            return Err("EWM catalog must keep inner_ranges false".into());
        }
        let mut places = Vec::new();
        for uri in index.rel(&pack, "hasPlace") {
            let kind = index
                .many(&uri, &format!("{REL}kind"))
                .into_iter()
                .next()
                .unwrap_or_default();
            if kind != PLACE_KIND {
                return Err("Catalog hasPlace must name a public place".into());
            }
            places.push(place_from_index(&index, &uri)?);
        }
        if places.is_empty() {
            return Err("Catalog Q42 has no public place".into());
        }
        if !places.iter().any(|p| p.id == GIS_PLACE_ID) {
            return Err("Catalog must keep the woodland GIS public place".into());
        }
        for id in REQUIRED_NAMED_PLACES {
            if !places.iter().any(|p| p.id == id) {
                return Err(format!("Catalog missing public place {id}"));
            }
        }
        let first_paint: Vec<String> = index
            .rel(&pack, "firstPaint")
            .into_iter()
            .filter_map(|iri| asset_id(&iri).map(str::to_string))
            .collect();
        if first_paint.len() != 24 {
            return Err("Catalog first-paint membership must stay 24".into());
        }
        let studio = places
            .iter()
            .find(|p| p.id == FIELD_STUDIO_ID)
            .ok_or("Catalog missing field-studio public place")?;
        let paint_set: BTreeSet<&str> = first_paint.iter().map(String::as_str).collect();
        let studio_set: BTreeSet<&str> = studio.base.iter().map(String::as_str).collect();
        if studio.base.len() != 24 || studio_set != paint_set {
            return Err("field-studio Base must stay the woodland first-paint 24".into());
        }
        let sparql_paint: std::collections::BTreeSet<String> = knowledge::sparql_iris(host, paint, 0)
            .into_iter()
            .filter_map(|iri| asset_id(&iri).map(str::to_string))
            .collect();
        let gis_places = knowledge::sparql_iris(host, gis, 0);
        let gis_terrain = knowledge::sparql_iris(host, gis, 1);
        let gis_home = places
            .iter()
            .find(|p| p.id == GIS_PLACE_ID)
            .or_else(|| places.first())
            .ok_or("Catalog Q42 has no public place")?;
        let gis_place_iri = format!("https://webizen.au/catalog/place/{GIS_PLACE_ID}");
        let join = SparqlJoin {
            engine: "GraphDatabase.sparql".into(),
            gis_place: gis_places
                .iter()
                .find(|iri| iri.as_str() == gis_place_iri)
                .cloned()
                .or_else(|| gis_places.first().cloned())
                .unwrap_or_else(|| format!("https://webizen.au/catalog/place/{}", gis_home.id)),
            gis_terrain: gis_terrain
                .first()
                .and_then(|iri| asset_id(iri).map(str::to_string))
                .unwrap_or_else(|| gis_home.gis_terrain_id.clone()),
            first_paint: first_paint.clone(),
            sparql_hits: sparql_paint.len(),
        };
        if join.gis_terrain.is_empty() {
            return Err("Catalog GIS place did not name a terrain asset".into());
        }
        let mut entries = Vec::new();
        for uri in index.rel(&pack, "hasEntry") {
            let kind = index
                .many(&uri, &format!("{REL}kind"))
                .into_iter()
                .next()
                .unwrap_or_default();
            if kind != ENTRY_KIND {
                return Err("Catalog hasEntry must name an entry".into());
            }
            entries.push(entry_from_index(&index, &uri)?);
        }
        entries.sort_by_key(|e| e.entry_id);
        if entries.len() < 85 {
            return Err("Catalog must describe the 85 woodland organs".into());
        }
        let mut edges = Vec::new();
        for uri in index.rel(&pack, "hasEdge") {
            edges.push(CatalogEdge {
                from_entry_id: index.u64(&uri, "fromEntryId").unwrap_or(0) as u32,
                to_entry_id: index.u64(&uri, "toEntryId").unwrap_or(0) as u32,
                reason: index.text(&uri, "reason")?,
                obligation: index.text(&uri, "obligation")?,
            });
        }
        Ok(Self {
            schema,
            pack_id: index.text(&pack, "packId")?,
            inner_ranges,
            streaming_default,
            places,
            candidate: Candidate {
                descriptor_sha384: index.text(&pack, "descriptorSha384")?,
                context_sha384: index.text(&pack, "contextSha384")?,
                generation: index.u64(&pack, "candidateGeneration").unwrap_or(0),
                entries: index.u64(&pack, "candidateEntries").unwrap_or(0) as u32,
            },
            first_paint,
            entries,
            edges,
            declared_bytes: index.u64(&pack, "declaredBytes").unwrap_or(0),
            declared_triangles: index.u64(&pack, "declaredTriangles").unwrap_or(0),
            declared_vertices: index.u64(&pack, "declaredVertices").unwrap_or(0),
            join,
        })
    }

    pub fn place(&self) -> Result<&Place, JsValue> {
        self.places
            .iter()
            .find(|p| p.id == GIS_PLACE_ID)
            .or_else(|| self.places.first())
            .ok_or_else(|| JsValue::from_str("EWM catalog has no public place"))
    }

    pub fn place_named(&self, id: &str) -> Option<&Place> {
        self.places.iter().find(|p| p.id == id)
    }

    /// Resident Base for a live PublicPlace. field-studio (and any place whose
    /// Base is that set) returns the woodland first-paint 24 in catalog order.
    /// Unknown or empty Base falls back to first-paint so boot stays 24.
    pub fn live_place_base(&self, place_id: &str) -> Vec<String> {
        let Some(place) = self.place_named(place_id) else {
            return self.first_paint.clone();
        };
        if place.base.is_empty() {
            return self.first_paint.clone();
        }
        let paint: BTreeSet<&str> = self.first_paint.iter().map(String::as_str).collect();
        let base: BTreeSet<&str> = place.base.iter().map(String::as_str).collect();
        if base == paint {
            self.first_paint.clone()
        } else {
            place.base.clone()
        }
    }

    pub fn entry(&self, scene_id: &str) -> Option<&CatalogEntry> {
        self.entries.iter().find(|e| e.scene_id == scene_id)
    }
}

fn place_from_index(index: &QuinIndex, uri: &str) -> Result<Place, String> {
    let gis = index
        .rel(uri, "gisTerrain")
        .into_iter()
        .next()
        .and_then(|iri| asset_id(&iri).map(str::to_string))
        .ok_or("Public place missing GIS terrain asset")?;
    let ground = index
        .rel(uri, "authoredGround")
        .into_iter()
        .next()
        .and_then(|iri| asset_id(&iri).map(str::to_string))
        .unwrap_or_default();
    let mut colocated = Vec::new();
    for layer in index.rel(uri, "hasColocated") {
        let kind = index
            .many(&layer, &format!("{REL}kind"))
            .into_iter()
            .next()
            .unwrap_or_default();
        if kind != LAYER_KIND {
            return Err("Place hasColocated must name a Noosphere layer".into());
        }
        colocated.push(ColocatedLayer {
            id: index.text(&layer, "id")?,
            label: index.text(&layer, "label")?,
            realm_class: index.text(&layer, "realmClass")?,
            usri: index.text(&layer, "usri")?,
            grounding: index.text(&layer, "grounding")?,
            anchor_iri: index.text(&layer, "anchorIri")?,
        });
    }
    let mut base = asset_ids(index, uri, "hasBaseAsset");
    if base.is_empty() {
        base = asset_ids(index, uri, "placeBase");
    }
    Ok(Place {
        id: index.text(uri, "id")?,
        title: index.text(uri, "title")?,
        crs: index.text(uri, "crs")?,
        bbox_wsen: [
            index.number(uri, "bboxW").unwrap_or(0.0),
            index.number(uri, "bboxS").unwrap_or(0.0),
            index.number(uri, "bboxE").unwrap_or(0.0),
            index.number(uri, "bboxN").unwrap_or(0.0),
        ],
        elevation_m: [
            index.number(uri, "elevationMin").unwrap_or(0.0),
            index.number(uri, "elevationMax").unwrap_or(0.0),
        ],
        gis_terrain_id: gis,
        authored_ground_id: ground,
        usri: index.text(uri, "usri")?,
        realm_class: index.text(uri, "realmClass")?,
        colocated,
        base,
        enhancement: asset_ids(index, uri, "hasEnhancementAsset"),
    })
}

fn asset_ids(index: &QuinIndex, uri: &str, rel: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for iri in index.rel(uri, rel) {
        let Some(id) = asset_id(&iri) else {
            continue;
        };
        if seen.insert(id.to_string()) {
            out.push(id.to_string());
        }
    }
    out
}

fn entry_from_index(index: &QuinIndex, uri: &str) -> Result<CatalogEntry, String> {
    let scene = index
        .rel(uri, "sceneAsset")
        .into_iter()
        .next()
        .and_then(|iri| asset_id(&iri).map(str::to_string))
        .ok_or("Catalog entry missing scene asset")?;
    let aabb = if index.rel(uri, "aabbMinX").len() == 1 {
        Some(Aabb {
            min: index.f32x3(uri, "aabbMin")?,
            max: index.f32x3(uri, "aabbMax")?,
        })
    } else {
        None
    };
    let mut tiles = Vec::new();
    for tile in index.rel(uri, "hasTile") {
        tiles.push((
            index.i64(&tile, "tileX").unwrap_or(0) as i32,
            index.i64(&tile, "tileZ").unwrap_or(0) as i32,
        ));
    }
    Ok(CatalogEntry {
        entry_id: index.u64(uri, "entryId").unwrap_or(0) as u32,
        scene_id: scene,
        place_id: index.text(uri, "placeId").unwrap_or_default(),
        url: index.text(uri, "url")?,
        sha256: index.text(uri, "sha256")?,
        bytes: index.u64(uri, "bytes").unwrap_or(0),
        triangles: index.u64(uri, "triangles").unwrap_or(0),
        vertices: index.u64(uri, "vertices").unwrap_or(0),
        role: index.text(uri, "role")?,
        obligation: index.text(uri, "obligation")?,
        aabb,
        tiles,
    })
}


