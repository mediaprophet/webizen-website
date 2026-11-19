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
const SPECIES_CATALOG_SCHEMA: &str = "webizen.species-catalog/v1";
const DWC_SCIENTIFIC_NAME: &str = "http://rs.tdwg.org/dwc/terms/scientificName";
const DWC_VERNACULAR_NAME: &str = "http://rs.tdwg.org/dwc/terms/vernacularName";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

#[derive(Clone, Debug, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesLodAsset {
    pub tier: String,
    pub distance_band: String,
    pub url: String,
    pub remote_url: String,
    pub sha256: String,
    pub bytes: u64,
    pub triangles: u64,
    pub vertices: u64,
    pub aabb: Option<Aabb>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesCatalogEntry {
    pub taxon_iri: String,
    pub slug: String,
    pub scientific_name: String,
    pub vernacular_name: String,
    pub kingdom: String,
    pub ecological_zone: String,
    pub url: String,
    pub remote_url: String,
    pub sha256: String,
    pub bytes: u64,
    pub triangles: u64,
    pub vertices: u64,
    pub aabb: Option<Aabb>,
    pub sidecar_taxon_q42: Option<(String, String)>,
    pub sidecar_field_emf: Option<(String, String)>,
    pub sidecar_substrate_m64: Option<(String, String)>,
    pub lod0: Option<SpeciesLodAsset>,
    pub lod1: Option<SpeciesLodAsset>,
    pub lod2: Option<SpeciesLodAsset>,
    pub aboveground_lod1: Option<SpeciesLodAsset>,
    pub rhizosphere: Option<SpeciesLodAsset>,
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
    pub species: Vec<SpeciesCatalogEntry>,
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
        let species = species_from_index(&index);
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
            species,
        })
    }

    #[allow(dead_code)]
    pub fn species_from_host(host: &PoetSnapshot) -> Result<Vec<SpeciesCatalogEntry>, String> {
        let index = QuinIndex::from_host(host)?;
        Ok(species_from_index(&index))
    }

    #[allow(dead_code)]
    pub fn species_by_slug(&self, slug: &str) -> Option<&SpeciesCatalogEntry> {
        self.species.iter().find(|s| s.slug == slug)
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

fn strip_rdf_literal(raw: &str) -> String {
    let t = raw.trim();
    let without_lang = if let Some(idx) = t.rfind("\"@") {
        &t[..idx + 1]
    } else if let Some(idx) = t.rfind("\"^^") {
        &t[..idx + 1]
    } else {
        t
    };
    if without_lang.starts_with('"') {
        if let Ok(s) = serde_json::from_str::<String>(without_lang) {
            return s;
        }
        if without_lang.len() >= 2 && without_lang.ends_with('"') {
            return without_lang[1..without_lang.len() - 1].to_string();
        }
    }
    without_lang.to_string()
}

fn first_pred_text(index: &QuinIndex, subject: &str, pred: &str) -> Option<String> {
    index
        .many(subject, pred)
        .into_iter()
        .next()
        .map(|v| strip_rdf_literal(&v))
}

fn optional_aabb(index: &QuinIndex, uri: &str) -> Option<Aabb> {
    if index.rel(uri, "aabbMinX").len() == 1 {
        let min = index.f32x3(uri, "aabbMin").ok()?;
        let max = index.f32x3(uri, "aabbMax").ok()?;
        Some(Aabb { min, max })
    } else {
        None
    }
}

fn species_lod_from_index(index: &QuinIndex, uri: &str) -> Option<SpeciesLodAsset> {
    let tier = index.text(uri, "tier").ok()?;
    let url = index.text(uri, "url").ok()?;
    let sha256 = index.text(uri, "sha256").ok()?;
    Some(SpeciesLodAsset {
        tier,
        distance_band: index.text(uri, "distanceBand").unwrap_or_default(),
        url,
        remote_url: index.text(uri, "remoteUrl").unwrap_or_default(),
        sha256,
        bytes: index.u64(uri, "bytes").unwrap_or(0),
        triangles: index.u64(uri, "triangles").unwrap_or(0),
        vertices: index.u64(uri, "vertices").unwrap_or(0),
        aabb: optional_aabb(index, uri),
    })
}

fn optional_sidecar(
    index: &QuinIndex,
    uri: &str,
    url_pred: &str,
    sha_pred: &str,
) -> Option<(String, String)> {
    let url = index.text(uri, url_pred).ok()?;
    let sha = index.text(uri, sha_pred).ok()?;
    Some((url, sha))
}

fn species_entry_from_index(index: &QuinIndex, taxon_iri: &str) -> Option<SpeciesCatalogEntry> {
    let slug = index.text(taxon_iri, "slug").ok()?;
    let url = index.text(taxon_iri, "canonicalUrl").ok()?;
    let sha256 = index.text(taxon_iri, "canonicalSha256").ok()?;
    let scientific_name = first_pred_text(index, taxon_iri, DWC_SCIENTIFIC_NAME).unwrap_or_default();
    let vernacular_name = first_pred_text(index, taxon_iri, DWC_VERNACULAR_NAME)
        .or_else(|| first_pred_text(index, taxon_iri, RDFS_LABEL))
        .unwrap_or_else(|| slug.clone());

    let mut lod0 = None;
    let mut lod1 = None;
    let mut lod2 = None;
    let mut aboveground_lod1 = None;
    let mut rhizosphere = None;
    for lod_iri in index.rel(taxon_iri, "hasLod") {
        let Some(lod) = species_lod_from_index(index, &lod_iri) else {
            continue;
        };
        match lod.tier.as_str() {
            "lod0" => lod0 = Some(lod),
            "lod1" => lod1 = Some(lod),
            "lod2" => lod2 = Some(lod),
            "aboveground_lod1" => aboveground_lod1 = Some(lod),
            "rhizosphere" => rhizosphere = Some(lod),
            _ => {}
        }
    }

    Some(SpeciesCatalogEntry {
        taxon_iri: taxon_iri.to_string(),
        slug,
        scientific_name,
        vernacular_name,
        kingdom: index.text(taxon_iri, "kingdom").unwrap_or_default(),
        ecological_zone: index.text(taxon_iri, "ecologicalZone").unwrap_or_default(),
        url,
        remote_url: index
            .text(taxon_iri, "canonicalRemoteUrl")
            .unwrap_or_default(),
        sha256,
        bytes: index.u64(taxon_iri, "canonicalBytes").unwrap_or(0),
        triangles: index.u64(taxon_iri, "canonicalTriangles").unwrap_or(0),
        vertices: index.u64(taxon_iri, "canonicalVertices").unwrap_or(0),
        aabb: optional_aabb(index, taxon_iri),
        sidecar_taxon_q42: optional_sidecar(
            index,
            taxon_iri,
            "sidecarTaxonQ42",
            "sidecarTaxonQ42Sha256",
        ),
        sidecar_field_emf: optional_sidecar(
            index,
            taxon_iri,
            "sidecarFieldEmf",
            "sidecarFieldEmfSha256",
        ),
        sidecar_substrate_m64: optional_sidecar(
            index,
            taxon_iri,
            "sidecarSubstrateM64",
            "sidecarSubstrateM64Sha256",
        ),
        lod0,
        lod1,
        lod2,
        aboveground_lod1,
        rhizosphere,
    })
}

/// Parse optional `species-catalog.q42` (`webizen.species-catalog/v1`) entries if admitted
/// into the graph. Returns an empty `Vec` when no species catalog is present so existing
/// woodland `Catalog::from_host` callers and tests remain unaffected.
pub fn species_from_index(index: &QuinIndex) -> Vec<SpeciesCatalogEntry> {
    let Ok(pack) = index.subject_of_schema(SPECIES_CATALOG_SCHEMA) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for taxon_iri in index.rel(&pack, "hasSpecies") {
        if let Some(entry) = species_entry_from_index(index, &taxon_iri) {
            out.push(entry);
        }
    }
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    out
}


