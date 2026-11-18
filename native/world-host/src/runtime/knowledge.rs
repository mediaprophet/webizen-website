//! Admit Q42VOL v4 CBOR-LD SuperBlocks of NQuins into PoetSnapshot.
//! Visitors never read those files. SPARQL and the admitted lexicon are the
//! query surface. Literal terms are RDF-typed (IRI, number, boolean, text).

use qualia_core_db::vibe_host::PoetSnapshot;
use std::collections::BTreeMap;
use vibe::Value as Vibe;

pub const REL: &str = "https://webizen.au/relation/";
pub const SCENE_ASSET: &str = "https://webizen.au/scene/asset";
pub const ASSET_NS: &str = "https://webizen.au/scene/asset/";
pub const CATALOG_PACK: &str = "https://webizen.au/catalog/canberra-public-place";
pub const PLACE_KIND: &str = "https://webizen.au/catalog/PublicPlace";
pub const ENTRY_KIND: &str = "https://webizen.au/catalog/Entry";
pub const CATALOG_KIND: &str = "https://webizen.au/catalog/Catalog";
pub const LAYER_KIND: &str = "https://webizen.au/catalog/NoosphereLayer";
pub const GEO_WKT: &str = "http://www.opengis.net/ont/geosparql#asWKT";

pub const GIS_JOIN: &str = concat!(
    "SELECT ?place ?terrain WHERE { ",
    "?place <https://webizen.au/relation/kind> <https://webizen.au/catalog/PublicPlace> . ",
    "?place <https://webizen.au/relation/gisTerrain> ?terrain . ",
    "?terrain <https://webizen.au/relation/kind> <https://webizen.au/scene/asset> ",
    "}"
);

pub const FIRST_PAINT_JOIN: &str = concat!(
    "SELECT ?asset WHERE { ",
    "<https://webizen.au/catalog/canberra-public-place> <https://webizen.au/relation/firstPaint> ?asset . ",
    "?asset <https://webizen.au/relation/kind> <https://webizen.au/scene/asset> ",
    "}"
);

pub const CAMP_GROUP_NS: &str = "https://webizen.au/camp/group/";
pub const CAMP_ACTION_KIND: &str = "https://webizen.au/camp/Action";

/// SPARQL over admitted Q42: actions authored about a logic-sequence group.
/// Group id is the last path segment of the group IRI.
pub fn actions_about_group_query(group_id: &str) -> Option<String> {
    if group_id.is_empty()
        || !group_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(format!(
        "SELECT ?action WHERE {{ ?action <{REL}kind> <{CAMP_ACTION_KIND}> . ?action <{REL}aboutGroup> <{CAMP_GROUP_NS}{group_id}> }}"
    ))
}

pub fn admit(
    host: &mut PoetSnapshot,
    bytes: &[u8],
    identity: &str,
    key: &str,
) -> Result<usize, String> {
    if bytes.len() < 8 || bytes[..3] != *b"Q42" {
        return Err(format!("{identity} is not a Q42VOL volume"));
    }
    let receipt = host.admit_q42_slice(bytes, identity, key, None, true)?;
    Ok(receipt.quin_count)
}

pub fn sparql(host: &mut PoetSnapshot, query: &str) -> Result<Vibe, String> {
    host.invoke_id("GraphDatabase.sparql", Vibe::String(query.into()))
        .map_err(|e| e.to_string())
}

pub fn resolve(host: &PoetSnapshot, hash: u64) -> Option<String> {
    host.resolve_admitted_lexicon_term(hash)
        .map(|s| s.to_string())
}

pub fn sparql_iris(host: &PoetSnapshot, result: &Vibe, slot: u64) -> Vec<String> {
    let Vibe::Record(rec) = result else {
        return Vec::new();
    };
    let Some(Vibe::List(rows)) = rec.get("rows") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        let Vibe::List(cells) = row else {
            continue;
        };
        for cell in cells {
            let Vibe::Record(m) = cell else {
                continue;
            };
            if m.get("slot") != Some(&Vibe::U64(slot)) {
                continue;
            }
            let Some(Vibe::U64(hash)) = m.get("value") else {
                continue;
            };
            if let Some(iri) = resolve(host, *hash) {
                out.push(iri);
            }
        }
    }
    out
}

pub fn asset_id(iri: &str) -> Option<&str> {
    iri.strip_prefix(ASSET_NS)
}

pub struct QuinIndex {
    pub index: BTreeMap<(String, String), Vec<String>>,
    pub rows: usize,
}

impl QuinIndex {
    pub fn from_host(host: &PoetSnapshot) -> Result<Self, String> {
        let mut names: BTreeMap<u64, String> = BTreeMap::new();
        for q in &host.committed {
            for hash in [q.subject, q.predicate, q.object] {
                if names.contains_key(&hash) {
                    continue;
                }
                if let Some(term) = resolve(host, hash) {
                    names.insert(hash, term);
                }
            }
        }
        let mut index: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        let mut rows = 0usize;
        for q in &host.committed {
            let Some(s) = names.get(&q.subject) else {
                continue;
            };
            let Some(p) = names.get(&q.predicate) else {
                continue;
            };
            let Some(o) = names.get(&q.object) else {
                continue;
            };
            index
                .entry((s.clone(), p.clone()))
                .or_default()
                .push(o.clone());
            rows += 1;
        }
        if rows == 0 {
            return Err("Admitted Q42 volumes have no readable lexicon terms".into());
        }
        Ok(Self { index, rows })
    }

    pub fn many(&self, s: &str, p: &str) -> Vec<String> {
        self.index
            .get(&(s.to_string(), p.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    pub fn rel(&self, s: &str, p: &str) -> Vec<String> {
        self.many(s, &format!("{REL}{p}"))
    }

    pub fn one(&self, s: &str, p: &str) -> Result<String, String> {
        let found = self.rel(s, p);
        if found.len() != 1 {
            return Err(format!("Missing or ambiguous graph property {p}"));
        }
        Ok(found[0].clone())
    }

    pub fn text(&self, s: &str, p: &str) -> Result<String, String> {
        Ok(lex_text(&self.one(s, p)?))
    }

    pub fn boolean(&self, s: &str, p: &str) -> Result<bool, String> {
        match lex_text(&self.one(s, p)?).as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => Err(format!("{p} must be a boolean, got {other}")),
        }
    }

    pub fn number(&self, s: &str, p: &str) -> Result<f64, String> {
        lex_number(&self.one(s, p)?).ok_or_else(|| format!("{p} must be a finite number"))
    }

    pub fn u64(&self, s: &str, p: &str) -> Result<u64, String> {
        let n = self.number(s, p)?;
        if n.fract() == 0.0 && n >= 0.0 && n <= u64::MAX as f64 {
            Ok(n as u64)
        } else {
            Err(format!("{p} must be a whole number"))
        }
    }

    pub fn i64(&self, s: &str, p: &str) -> Result<i64, String> {
        let n = self.number(s, p)?;
        if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            Ok(n as i64)
        } else {
            Err(format!("{p} must be a whole number"))
        }
    }

    pub fn f64x3(&self, s: &str, base: &str) -> Result<[f64; 3], String> {
        Ok([
            self.number(s, &format!("{base}X"))?,
            self.number(s, &format!("{base}Y"))?,
            self.number(s, &format!("{base}Z"))?,
        ])
    }

    pub fn f32x3(&self, s: &str, base: &str) -> Result<[f32; 3], String> {
        let n = self.f64x3(s, base)?;
        Ok([n[0] as f32, n[1] as f32, n[2] as f32])
    }

    pub fn f32x4(&self, s: &str, names: [&str; 4]) -> Result<[f32; 4], String> {
        Ok([
            self.number(s, names[0])? as f32,
            self.number(s, names[1])? as f32,
            self.number(s, names[2])? as f32,
            self.number(s, names[3])? as f32,
        ])
    }

    pub fn f64x4(&self, s: &str, names: [&str; 4]) -> Result<[f64; 4], String> {
        Ok([
            self.number(s, names[0])?,
            self.number(s, names[1])?,
            self.number(s, names[2])?,
            self.number(s, names[3])?,
        ])
    }

    pub fn subject_of_kind(&self, kind: &str) -> Result<String, String> {
        self.index
            .iter()
            .find(|((_, p), objects)| p == &format!("{REL}kind") && objects.iter().any(|o| o == kind))
            .map(|((s, _), _)| s.clone())
            .ok_or_else(|| format!("Q42 is missing {kind}"))
    }

    pub fn subject_of_schema(&self, schema: &str) -> Result<String, String> {
        self.index
            .iter()
            .find(|((_, p), objects)| {
                p == &format!("{REL}schema") && objects.iter().any(|o| lex_text(o) == schema)
            })
            .map(|((s, _), _)| s.clone())
            .ok_or_else(|| format!("Q42 is missing schema {schema}"))
    }
}

fn lex_text(raw: &str) -> String {
    let t = raw.trim();
    if t.starts_with('"') {
        if let Ok(s) = serde_json::from_str::<String>(t) {
            return s;
        }
        if t.len() >= 2 && t.ends_with('"') {
            return t[1..t.len() - 1].to_string();
        }
    }
    t.to_string()
}

fn lex_number(raw: &str) -> Option<f64> {
    let t = lex_text(raw);
    let n = t.parse::<f64>().ok()?;
    n.is_finite().then_some(n)
}


