//! Named public places from the EWM catalog Q42. Camera-in-place uses catalog AABB.
//! Display frame is Y-up; tiles are (floor(x/100), floor(z/100)). Not XOR tile ids.

#![allow(dead_code)]

use super::catalog::{Catalog, CatalogEntry, Place};
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

pub fn place_from_catalog(catalog: &Catalog) -> Result<&Place, JsValue> {
    catalog.place()
}

pub fn bbox_wsen(place: &Place) -> [f64; 4] {
    place.bbox_wsen
}

/// Circle in the Y-up x/z plane versus an AABB, expanded by `radius` metres.
pub fn aabb_overlaps(min: [f32; 3], max: [f32; 3], pose: &CameraPose, radius: f32) -> bool {
    let nx = pose.x.clamp(min[0], max[0]);
    let nz = pose.z.clamp(min[2], max[2]);
    let dx = pose.x - nx;
    let dz = pose.z - nz;
    dx * dx + dz * dz <= radius * radius
}

pub fn tile_membership(entry: &CatalogEntry, pose: &CameraPose) -> bool {
    let (ix, iy) = tile_xz(pose.x, pose.z);
    entry.tiles.iter().any(|t| *t == (ix, iy))
}

/// Circle-vs-AABB (plus radius) or catalog tile membership of the camera target.
pub fn entry_overlaps(entry: &CatalogEntry, pose: &CameraPose, radius: f32) -> bool {
    entry
        .aabb
        .as_ref()
        .map(|aabb| aabb_overlaps(aabb.min, aabb.max, pose, radius))
        .unwrap_or(false)
        || tile_membership(entry, pose)
}

/// Entries whose AABB (plus radius) or tile contains the camera target. Windowed to 64.
pub fn overlapping_entries<'a>(
    entries: &'a [CatalogEntry],
    pose: &CameraPose,
    radius: f32,
) -> Vec<&'a CatalogEntry> {
    let mut out = Vec::new();
    for entry in entries {
        if entry_overlaps(entry, pose, radius) {
            out.push(entry);
            if out.len() >= MAX_WINDOW {
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::catalog::Aabb;

    fn unit_entry() -> CatalogEntry {
        CatalogEntry {
            entry_id: 1,
            scene_id: "unit".into(),
            place_id: "canberra-public-reference".into(),
            url: "/production/unit.10d".into(),
            sha256: "00".into(),
            bytes: 1,
            triangles: 1,
            vertices: 1,
            role: "Base".into(),
            obligation: "Mandatory".into(),
            aabb: Some(Aabb {
                min: [-0.5, -0.5, -0.5],
                max: [0.5, 0.5, 0.5],
            }),
            tiles: vec![(0, 0)],
        }
    }

    #[test]
    fn tile_xz_is_floor_division_not_xor() {
        assert_eq!(TILE_METRES, 100.0);
        assert_eq!(tile_xz(0.0, 0.0), (0, 0));
        assert_eq!(tile_xz(100.0, 0.0), (1, 0));
        assert_eq!(tile_xz(-0.1, -0.1), (-1, -1));
        assert_eq!(tile_xz(100.0, 200.0), (1, 2));
        assert_eq!(tile_xz(250.0, -150.0), (2, -2));
    }

    #[test]
    fn camera_pose_copies_xz_yaw_pitch_without_zoom() {
        let pose = camera_pose([12.5, 99.0, -3.25], 1.5, -0.25);
        assert_eq!(pose.x, 12.5);
        assert_eq!(pose.z, -3.25);
        assert_eq!(pose.yaw, 1.5);
        assert_eq!(pose.pitch, -0.25);
        assert_eq!(pose.y, 99.0);
    }

    #[test]
    fn aabb_overlaps_unit_at_origin_within_80m() {
        let pose = camera_pose([0.0, 0.0, 0.0], 0.0, 0.0);
        assert!(aabb_overlaps(
            [-0.5, -0.5, -0.5],
            [0.5, 0.5, 0.5],
            &pose,
            DEFAULT_RADIUS_M
        ));
        assert!(!aabb_overlaps(
            [200.0, -1.0, 200.0],
            [201.0, 1.0, 201.0],
            &pose,
            DEFAULT_RADIUS_M
        ));
    }

    #[test]
    fn overlapping_entries_windows_to_max_window() {
        let pose = camera_pose([0.0, 0.0, 0.0], 0.0, 0.0);
        let entries: Vec<CatalogEntry> = (0..MAX_WINDOW + 1).map(|_| unit_entry()).collect();
        let window = overlapping_entries(&entries, &pose, DEFAULT_RADIUS_M);
        assert_eq!(MAX_WINDOW, 64);
        assert_eq!(window.len(), MAX_WINDOW);
    }
}
