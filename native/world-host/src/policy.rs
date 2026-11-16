use qualia_core_db::vibe_host::PoetSnapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::collections::BTreeMap;
use vibe::Value;
pub fn into_vibe(v: &Json) -> Value {
    match v {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => {
            if let Some(i) = n.as_u64() {
                Value::U64(i)
            } else if let Some(i) = n.as_i64() {
                Value::I64(i)
            } else {
                Value::F64(n.as_f64().unwrap_or(0.))
            }
        }
        Json::String(s) => Value::String(s.clone()),
        Json::Array(a) => Value::List(a.iter().map(into_vibe).collect()),
        Json::Object(o) => Value::Record(
            o.iter()
                .map(|(k, v)| (k.clone(), into_vibe(v)))
                .collect::<BTreeMap<_, _>>(),
        ),
    }
}
fn from_vibe(v: Value) -> Result<Json, String> {
    Ok(match v {
        Value::Null => Json::Null,
        Value::Bool(b) => b.into(),
        Value::I64(n) => n.into(),
        Value::U64(n) => n.into(),
        Value::F64(n) => serde_json::Number::from_f64(n)
            .ok_or("Non-finite Vibe result")?
            .into(),
        Value::String(s) | Value::Iri(s) => s.into(),
        Value::List(a) => Json::Array(a.into_iter().map(from_vibe).collect::<Result<_, _>>()?),
        Value::Record(o) => Json::Object(
            o.into_iter()
                .map(|(k, v)| Ok((k, from_vibe(v)?)))
                .collect::<Result<_, String>>()?,
        ),
        _ => return Err("Unsupported Vibe world result type".into()),
    })
}
pub fn evaluate(
    host: &mut PoetSnapshot,
    source: &str,
    name: &str,
    args: &[Json],
) -> Result<Json, String> {
    from_vibe(
        host.eval_fn(source, name, args.iter().map(into_vibe).collect())
            .map_err(|e| e.to_string())?,
    )
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub target: [f32; 3],
}
impl Camera {
    pub fn validate(&self) -> Result<(), String> {
        if ![
            self.yaw,
            self.pitch,
            self.zoom,
            self.target[0],
            self.target[1],
            self.target[2],
        ]
        .iter()
        .all(|v| v.is_finite())
            || self.zoom <= 0.
            || self.zoom > 10000.
        {
            Err("Invalid Vibe camera".into())
        } else {
            Ok(())
        }
    }
    pub fn place(p: &Json) -> Result<Self, String> {
        let target: [f32; 3] =
            serde_json::from_value(p["position"].clone()).map_err(|e| e.to_string())?;
        let eye: [f32; 3] =
            serde_json::from_value(p["camera"].clone()).map_err(|e| e.to_string())?;
        let delta = [eye[0] - target[0], eye[1] - target[1], eye[2] - target[2]];
        let zoom = (delta.iter().map(|x| x * x).sum::<f32>()).sqrt();
        let out = Self {
            yaw: delta[0].atan2(delta[2]),
            pitch: (delta[1] / zoom).asin(),
            zoom,
            target,
        };
        out.validate()?;
        Ok(out)
    }
}
