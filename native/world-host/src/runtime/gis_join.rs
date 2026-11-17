//! Named public places from the EWM catalog. Camera-in-place uses catalog AABB.
//! Display frame is Y-up; tiles are (floor(x/100), floor(z/100)). Not XOR tile ids.

#![allow(dead_code)]

use serde_json::Value;
use wasm_bindgen::JsValue;

pub const DEFAULT_RADIUS_M: f32 = 80.0;
pub const TILE_METRES: f32 = 100.0;
pub const MAX_WINDOW: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
}

pub fn camera_pose(target: [f32; 3], yaw: f32, pitch: f32) -> CameraPose {
    CameraPose {
        x: target[0],
        y: target[1],
        z: target[2],
        yaw,
        pitch,
    }
}

pub fn tile_xz(x: f32, z: f32) -> (i32, i32) {
    ((x / TILE_METRES).floor() as i32, (z / TILE_METRES).floor() as i32)
}

pub fn place_from_catalog(catalog: &Value) -> Result<&Value, JsValue> {
    let places = catalog["places"]
        .as_array()
        .ok_or_else(|| JsValue::from_str("EWM catalog has no public place"))?;
    places
        .iter()
        .find(|p| p["id"].as_str() == Some("canberra-public-reference"))
        .or_else(|| places.first())
        .ok_or_else(|| JsValue::from_str("EWM catalog has no public place"))
}

pub fn bbox_wsen(place: &Value) -> Option<[f64; 4]> {
    let b = place.get("bbox_wsen")?.as_array()?;
    if b.len() != 4 {
        return None;
    }
    Some([
        b[0].as_f64()?,
        b[1].as_f64()?,
        b[2].as_f64()?,
        b[3].as_f64()?,
    ])
}

fn aabb_xyz(entry: &Value) -> Option<([f32; 3], [f32; 3])> {
    let aabb = entry.get("aabb")?;
    let min = aabb.get("min")?.as_array()?;
    let max = aabb.get("max")?.as_array()?;
    if min.len() != 3 || max.len() != 3 {
        return None;
    }
    Some((
        [
            min[0].as_f64()? as f32,
            min[1].as_f64()? as f32,
            min[2].as_f64()? as f32,
        ],
        [
            max[0].as_f64()? as f32,
            max[1].as_f64()? as f32,
            max[2].as_f64()? as f32,
        ],
    ))
}

/// Circle in the Y-up x/z plane versus an AABB, expanded by `radius` metres.
pub fn aabb_overlaps(min: [f32; 3], max: [f32; 3], pose: &CameraPose, radius: f32) -> bool {
    let nx = pose.x.clamp(min[0], max[0]);
    let nz = pose.z.clamp(min[2], max[2]);
    let dx = pose.x - nx;
    let dz = pose.z - nz;
    dx * dx + dz * dz <= radius * radius
}

pub fn tile_membership(entry: &Value, pose: &CameraPose) -> bool {
    let Some(tiles) = entry.get("tiles").and_then(Value::as_array) else {
        return false;
    };
    let (ix, iy) = tile_xz(pose.x, pose.z);
    tiles.iter().any(|t| {
        t.as_array()
            .filter(|p| p.len() == 2)
            .and_then(|p| Some((p[0].as_i64()? as i32, p[1].as_i64()? as i32)))
            == Some((ix, iy))
    })
}

/// Entries whose AABB (plus radius) or tile contains the camera target. Windowed to 64.
pub fn overlapping_entries<'a>(
    entries: &'a [Value],
    pose: &CameraPose,
    radius: f32,
) -> Vec<&'a Value> {
    let mut out = Vec::new();
    for entry in entries {
        let hit = aabb_xyz(entry)
            .map(|(min, max)| aabb_overlaps(min, max, pose, radius))
            .unwrap_or(false)
            || tile_membership(entry, pose);
        if hit {
            out.push(entry);
            if out.len() >= MAX_WINDOW {
                break;
            }
        }
    }
    out
}
