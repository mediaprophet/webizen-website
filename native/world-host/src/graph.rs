use qualia_core_db::{q42::q42_volume::WasmSliceQ42Reader, NQuin};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub struct Graph {
    pub quins: Vec<NQuin>,
    pub places: Value,
    pub edges: Value,
    pub chapters: Value,
    pub chapter_method: String,
    pub rows: usize,
}
const W: &str = "https://webizen.au/world/";
const R: &str = "https://webizen.au/relation/";
const J: &str = "https://webizen.au/journey/";
fn local(s: &str) -> String {
    for prefix in [W, R, "https://webizen.au/purpose/"] {
        if let Some(v) = s.strip_prefix(prefix) {
            return v.into();
        }
    }
    s.into()
}
pub fn decode(bytes: &[u8]) -> Result<Graph, String> {
    let reader = WasmSliceQ42Reader::open(bytes).map_err(|e| e.to_string())?;
    if !reader.is_unified_volume() || reader.version() != 4 {
        return Err("Expected native Q42VOL v4 publication".into());
    }
    let quins = reader.collect_all_quins().map_err(|e| e.to_string())?;
    if quins.len() > 20000 {
        return Err("Public world graph exceeds host budget".into());
    }
    let mut names = BTreeMap::new();
    let mut scratch = vec![0; 65536];
    let mut text = vec![0; 65536];
    for q in &quins {
        for hash in [q.subject, q.predicate, q.object] {
            if names.contains_key(&hash) {
                continue;
            }
            let n = reader
                .lookup_lexicon_hash_into(hash, &mut scratch, &mut text)
                .map_err(|e| e.to_string())?
                .ok_or("Missing native Q42 lexical term")?;
            names.insert(
                hash,
                std::str::from_utf8(&text[..n])
                    .map_err(|e| e.to_string())?
                    .to_string(),
            );
        }
    }
    let rows: Vec<[String; 3]> = quins
        .iter()
        .map(|q| {
            [
                names[&q.subject].clone(),
                names[&q.predicate].clone(),
                names[&q.object].clone(),
            ]
        })
        .collect();
    let property = |s: &str, p: &str| -> Result<Value, String> {
        let matches: Vec<_> = rows
            .iter()
            .filter(|r| r[0] == s && r[1] == format!("{R}{p}"))
            .collect();
        if matches.len() != 1 {
            return Err(format!("Missing or ambiguous graph property {s} {p}"));
        }
        serde_json::from_str(&matches[0][2]).map_err(|e| e.to_string())
    };
    let ids: BTreeSet<_> = rows
        .iter()
        .filter(|r| r[0].starts_with(W) && r[1] == format!("{R}kind"))
        .map(|r| r[0].clone())
        .collect();
    let mut places = Vec::new();
    for uri in ids {
        places.push(json!({"id":uri.trim_start_matches(W),"uri":uri,"title":property(&uri,"title")?,"description":property(&uri,"description")?,"reading_route":property(&uri,"readingRoute")?,"position":property(&uri,"position")?,"camera":property(&uri,"camera")?}));
    }
    if places.is_empty() {
        return Err("No public places in native graph".into());
    }
    let edges = rows
        .iter()
        .map(|r| json!({"subject":local(&r[0]),"predicate":local(&r[1]),"object":local(&r[2])}))
        .collect::<Vec<_>>();
    let mut chapters = Vec::new();
    for uri in rows
        .iter()
        .filter(|r| r[0] == format!("{W}field-studio") && r[1] == format!("{R}hasChapter"))
        .map(|r| &r[2])
    {
        if !uri.starts_with(J) {
            return Err("Chapter outside journey namespace".into());
        }
        let mut c = serde_json::Map::new();
        for key in [
            "id",
            "title",
            "month_bands",
            "relative_order",
            "stay_band",
            "artistic_palette",
            "landscape_copy",
        ] {
            c.insert(key.into(), property(uri, key)?);
        }
        if let Some(palette) = c.get("artistic_palette").and_then(Value::as_array) {
            let linear = palette
                .iter()
                .map(|v| colour_linear(v.as_str().ok_or("Invalid chapter colour")?))
                .collect::<Result<Vec<_>, String>>()?;
            c.insert("palette_linear".into(), json!(linear));
        }
        chapters.push(Value::Object(c));
    }
    chapters.sort_by_key(|c| c["relative_order"].as_u64().unwrap_or(0));
    let method = property(&format!("{J}publication"), "method")?
        .as_str()
        .ok_or("Invalid journey method")?
        .into();
    Ok(Graph {
        rows: rows.len(),
        quins,
        places: json!(places),
        edges: json!(edges),
        chapters: json!(chapters),
        chapter_method: method,
    })
}

fn colour_linear(value: &str) -> Result<[f32; 4], String> {
    if value.len() != 7 || !value.starts_with('#') || !value[1..].is_ascii() {
        return Err("Invalid public palette colour".into());
    }
    let mut rgba = [0., 0., 0., 1.];
    for i in 0..3 {
        let c = u8::from_str_radix(&value[1 + i * 2..3 + i * 2], 16)
            .map_err(|_| "Invalid public palette colour")? as f32
            / 255.;
        rgba[i] = if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        };
    }
    Ok(rgba)
}
