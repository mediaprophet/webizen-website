//! Soil mesh for the 600 m catchment.
//!
//! Metre frame. The origin is the basin centre. +Y is up. Horizontal x and z
//! use the same axes as `site/world/terrain.js`, in metres. The mesh covers
//! x and z in [-300, 300]. Vertex `(i, j)` lies at
//! `x = -300 + i * 2.5`, `z = -300 + j * 2.5`.
//!
//! `height_at`, `normal_at`, `material_at`, and `colour_at` are barycentric
//! samples of that triangle mesh (`production/ground/catchment-soil.bin`).
//! They are not a second copy of the catchment smoothstep. Outside the basin
//! the sample clamps to the edge triangle. Non-finite queries return a flat
//! 0 m datum.
//!
//! Authored plant y is 0 at the soil collar and negative below it.
//! [`fit_soil_joint`] is the only placement fit. It does not call
//! `compute_slope_conforming_root_transform` and it does not tilt a root disc.
//! Positive yaw is a right-handed rotation about +Y, matching a pure yaw
//! quaternion `(0, sin(yaw/2), 0, cos(yaw/2))`:
//! `x' = x cos yaw + z sin yaw`, `z' = -x sin yaw + z cos yaw`.
//! World xz is that yaw applied to the joint, then added to the trunk.
//! World y is `height_at` at the resulting xz plus the joint's authored y.

#![allow(dead_code)]

/// Triangle span of one grid cell, in metres.
pub const CELL_M: f64 = 2.5;
/// Vertices along one side of the 600 m grid, including both edges.
pub const GRID_SIDE: usize = 241;
/// Basin half-span. The mesh runs from `-HALF_SPAN_M` to `+HALF_SPAN_M`.
pub const HALF_SPAN_M: f64 = 300.0;
/// Full basin width.
pub const SPAN_M: f64 = 600.0;

const HEADER_LEN: usize = 24;
const VERTEX_COUNT: usize = GRID_SIDE * GRID_SIDE;
const HEIGHT_BYTES: usize = VERTEX_COUNT * 4;
const RECORD_BYTES: usize = VERTEX_COUNT * 4;
const MESH_LEN: usize = HEADER_LEN + HEIGHT_BYTES + RECORD_BYTES;

/// Little-endian soil mesh: header, f32 heights, then mat/r/g/b per vertex.
static MESH: &[u8] = include_bytes!("../../../../production/ground/catchment-soil.bin");

/// Semantic material stored on each soil vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GroundMaterial {
    FloodplainSilt = 1,
    LeafLitter = 2,
    Sandstone = 3,
    Granite = 4,
    Pebble = 5,
    WetBank = 6,
    CampsitePad = 7,
}

impl GroundMaterial {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => GroundMaterial::FloodplainSilt,
            2 => GroundMaterial::LeafLitter,
            3 => GroundMaterial::Sandstone,
            4 => GroundMaterial::Granite,
            5 => GroundMaterial::Pebble,
            6 => GroundMaterial::WetBank,
            7 => GroundMaterial::CampsitePad,
            _ => GroundMaterial::FloodplainSilt,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            GroundMaterial::FloodplainSilt => "floodplain silt",
            GroundMaterial::LeafLitter => "leaf litter",
            GroundMaterial::Sandstone => "sandstone",
            GroundMaterial::Granite => "granite",
            GroundMaterial::Pebble => "pebble",
            GroundMaterial::WetBank => "wet bank",
            GroundMaterial::CampsitePad => "campsite pad",
        }
    }
}

/// Base sRGB of a material, before the texture grain in the camp pictures.
/// Lockstep with `production/ground/author_mesh.py` `PALETTE`.
pub fn material_base_srgb(material: GroundMaterial) -> [u8; 3] {
    match material {
        GroundMaterial::FloodplainSilt => [168, 112, 64],
        GroundMaterial::LeafLitter => [68, 44, 24],
        GroundMaterial::Sandstone => [206, 184, 152],
        GroundMaterial::Granite => [108, 112, 118],
        GroundMaterial::Pebble => [146, 140, 128],
        GroundMaterial::WetBank => [42, 52, 40],
        GroundMaterial::CampsitePad => [118, 82, 48],
    }
}

fn mesh_bytes() -> &'static [u8] {
    debug_assert_eq!(MESH.len(), MESH_LEN);
    debug_assert_eq!(&MESH[0..4], b"SOIL");
    MESH
}

/// Byte length of the embedded soil mesh.
pub fn mesh_len() -> usize {
    mesh_bytes().len()
}

fn vertex_height(i: usize, j: usize) -> f64 {
    let idx = j * GRID_SIDE + i;
    let o = HEADER_LEN + idx * 4;
    let bytes = mesh_bytes();
    f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as f64
}

fn vertex_record(i: usize, j: usize) -> (u8, [u8; 3]) {
    let idx = j * GRID_SIDE + i;
    let o = HEADER_LEN + HEIGHT_BYTES + idx * 4;
    let bytes = mesh_bytes();
    (bytes[o], [bytes[o + 1], bytes[o + 2], bytes[o + 3]])
}

struct Tri {
    i0: usize,
    j0: usize,
    i1: usize,
    j1: usize,
    i2: usize,
    j2: usize,
    w0: f64,
    w1: f64,
    w2: f64,
}

fn locate(x: f64, z: f64) -> Tri {
    let u = ((x - (-HALF_SPAN_M)) / CELL_M).clamp(0.0, (GRID_SIDE - 1) as f64);
    let v = ((z - (-HALF_SPAN_M)) / CELL_M).clamp(0.0, (GRID_SIDE - 1) as f64);
    let mut i = u.floor() as usize;
    let mut j = v.floor() as usize;
    if i >= GRID_SIDE - 1 {
        i = GRID_SIDE - 2;
    }
    if j >= GRID_SIDE - 1 {
        j = GRID_SIDE - 2;
    }
    let s = (u - i as f64).clamp(0.0, 1.0);
    let t = (v - j as f64).clamp(0.0, 1.0);
    if s + t <= 1.0 {
        Tri {
            i0: i,
            j0: j,
            i1: i + 1,
            j1: j,
            i2: i,
            j2: j + 1,
            w0: 1.0 - s - t,
            w1: s,
            w2: t,
        }
    } else {
        Tri {
            i0: i + 1,
            j0: j,
            i1: i + 1,
            j1: j + 1,
            i2: i,
            j2: j + 1,
            w0: 1.0 - t,
            w1: s + t - 1.0,
            w2: 1.0 - s,
        }
    }
}

fn vertex_world(i: usize, j: usize) -> [f64; 3] {
    [
        -HALF_SPAN_M + i as f64 * CELL_M,
        vertex_height(i, j),
        -HALF_SPAN_M + j as f64 * CELL_M,
    ]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize_up(v: [f64; 3]) -> [f64; 3] {
    let mut n = v;
    if n[1] < 0.0 {
        n = [-n[0], -n[1], -n[2]];
    }
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if !len.is_finite() || len < 1e-12 {
        [0.0, 1.0, 0.0]
    } else {
        [n[0] / len, n[1] / len, n[2] / len]
    }
}

/// Soil height in metres at `(x, z)`, sampled from the triangle under that point.
pub fn height_at(x: f64, z: f64) -> f64 {
    if !x.is_finite() || !z.is_finite() {
        return 0.0;
    }
    let tri = locate(x, z);
    vertex_height(tri.i0, tri.j0) * tri.w0
        + vertex_height(tri.i1, tri.j1) * tri.w1
        + vertex_height(tri.i2, tri.j2) * tri.w2
}

/// Unit soil normal at `(x, z)`, from the same triangle as [`height_at`].
pub fn normal_at(x: f64, z: f64) -> [f64; 3] {
    if !x.is_finite() || !z.is_finite() {
        return [0.0, 1.0, 0.0];
    }
    let tri = locate(x, z);
    let p0 = vertex_world(tri.i0, tri.j0);
    let p1 = vertex_world(tri.i1, tri.j1);
    let p2 = vertex_world(tri.i2, tri.j2);
    normalize_up(cross(sub(p1, p0), sub(p2, p0)))
}

/// Material of the triangle vertex with the greatest barycentric weight.
pub fn material_at(x: f64, z: f64) -> GroundMaterial {
    if !x.is_finite() || !z.is_finite() {
        return GroundMaterial::FloodplainSilt;
    }
    let tri = locate(x, z);
    let (m0, _) = vertex_record(tri.i0, tri.j0);
    let (m1, _) = vertex_record(tri.i1, tri.j1);
    let (m2, _) = vertex_record(tri.i2, tri.j2);
    let (id, _, _) = [(m0, tri.w0, 0u8), (m1, tri.w1, 1), (m2, tri.w2, 2)]
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    GroundMaterial::from_u8(id)
}

/// Authored vertex colour, barycentric, in sRGB 0–255.
pub fn colour_at(x: f64, z: f64) -> [f64; 3] {
    if !x.is_finite() || !z.is_finite() {
        return [0.0, 0.0, 0.0];
    }
    let tri = locate(x, z);
    let (_, c0) = vertex_record(tri.i0, tri.j0);
    let (_, c1) = vertex_record(tri.i1, tri.j1);
    let (_, c2) = vertex_record(tri.i2, tri.j2);
    [
        c0[0] as f64 * tri.w0 + c1[0] as f64 * tri.w1 + c2[0] as f64 * tri.w2,
        c0[1] as f64 * tri.w0 + c1[1] as f64 * tri.w1 + c2[1] as f64 * tri.w2,
        c0[2] as f64 * tri.w0 + c1[2] as f64 * tri.w1 + c2[2] as f64 * tri.w2,
    ]
}

/// Place one authored joint on the soil mesh.
///
/// `trunk_xz` is the plant origin. `yaw` is radians about +Y. `joint_xyz`
/// is the asset-local joint: y is 0 at the collar and negative below it.
/// Returns world `[x, y, z]`.
pub fn fit_soil_joint(trunk_xz: [f64; 2], yaw: f64, joint_xyz: [f64; 3]) -> [f64; 3] {
    let jx = if joint_xyz[0].is_finite() { joint_xyz[0] } else { 0.0 };
    let jy = if joint_xyz[1].is_finite() { joint_xyz[1] } else { 0.0 };
    let jz = if joint_xyz[2].is_finite() { joint_xyz[2] } else { 0.0 };
    let yaw = if yaw.is_finite() { yaw } else { 0.0 };
    let tx = if trunk_xz[0].is_finite() { trunk_xz[0] } else { 0.0 };
    let tz = if trunk_xz[1].is_finite() { trunk_xz[1] } else { 0.0 };
    let (s, c) = yaw.sin_cos();
    let x = tx + jx * c + jz * s;
    let z = tz - jx * s + jz * c;
    [x, height_at(x, z) + jy, z]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bary_from_bytes(x: f64, z: f64) -> f64 {
        let u = ((x + HALF_SPAN_M) / CELL_M).clamp(0.0, (GRID_SIDE - 1) as f64);
        let v = ((z + HALF_SPAN_M) / CELL_M).clamp(0.0, (GRID_SIDE - 1) as f64);
        let mut i = u.floor() as usize;
        let mut j = v.floor() as usize;
        if i >= GRID_SIDE - 1 {
            i = GRID_SIDE - 2;
        }
        if j >= GRID_SIDE - 1 {
            j = GRID_SIDE - 2;
        }
        let s = u - i as f64;
        let t = v - j as f64;
        let h = |ii: usize, jj: usize| vertex_height(ii, jj);
        if s + t <= 1.0 {
            h(i, j) * (1.0 - s - t) + h(i + 1, j) * s + h(i, j + 1) * t
        } else {
            h(i + 1, j) * (1.0 - t) + h(i + 1, j + 1) * (s + t - 1.0) + h(i, j + 1) * (1.0 - s)
        }
    }

    #[test]
    fn mesh_header_and_span() {
        assert_eq!(MESH.len(), MESH_LEN);
        assert_eq!(&MESH[0..4], b"SOIL");
        assert_eq!(u32::from_le_bytes(MESH[4..8].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(MESH[8..12].try_into().unwrap()) as usize, GRID_SIDE);
        let cell = f32::from_le_bytes(MESH[12..16].try_into().unwrap()) as f64;
        assert!((cell - CELL_M).abs() < 1e-6);
        assert!((SPAN_M - CELL_M * (GRID_SIDE - 1) as f64).abs() < 1e-9);
    }

    #[test]
    fn height_matches_the_mesh_within_five_centimetres() {
        let points = [
            (-22.0, 28.0),
            (0.0, 16.0),
            (165.0, -265.0),
            (255.0, 80.0),
            (-300.0, -300.0),
            (300.0, 300.0),
            (-14.2, 31.7),
            (12.4, -8.8),
            (-160.0, -116.0),
        ];
        for (x, z) in points {
            let err = (height_at(x, z) - bary_from_bytes(x, z)).abs();
            assert!(err < 0.05, "{x},{z} err {err}");
            assert!(err < 1e-4, "{x},{z} err {err}");
        }
        for j in (0..GRID_SIDE).step_by(20) {
            for i in (0..GRID_SIDE).step_by(17) {
                let x = -HALF_SPAN_M + (i as f64 + 0.37) * CELL_M;
                let z = -HALF_SPAN_M + (j as f64 + 0.62) * CELL_M;
                let err = (height_at(x, z) - bary_from_bytes(x, z)).abs();
                assert!(err < 0.05, "{x},{z} err {err}");
            }
        }
        let x = -HALF_SPAN_M + 40.0 * CELL_M;
        let z = -HALF_SPAN_M + 80.0 * CELL_M;
        assert!((height_at(x, z) - vertex_height(40, 80)).abs() < 1e-4);
    }

    #[test]
    fn normal_is_the_triangle_normal() {
        let samples = [(-22.0, 28.0), (4.2, 18.5), (-40.0, -20.0), (180.0, -200.0)];
        for (x, z) in samples {
            let n = normal_at(x, z);
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-6);
            assert!(n[1] > 0.0);
            let tri = locate(x, z);
            let p0 = vertex_world(tri.i0, tri.j0);
            let p1 = vertex_world(tri.i1, tri.j1);
            let p2 = vertex_world(tri.i2, tri.j2);
            let face = normalize_up(cross(sub(p1, p0), sub(p2, p0)));
            let dot = n[0] * face[0] + n[1] * face[1] + n[2] * face[2];
            assert!(dot > 0.999, "{dot}");
        }
        let pad = normal_at(-22.0, 28.0);
        assert!(pad[1] > 0.98, "{pad:?}");
    }

    #[test]
    fn campsite_pad_is_flat_and_meets_soil_colour() {
        let centre = height_at(-22.0, 28.0);
        for (x, z) in [(-22.0, 28.0), (-18.0, 30.0), (-26.0, 26.0), (-17.5, 28.0)] {
            assert!((height_at(x, z) - centre).abs() < 0.02, "{x},{z}");
        }
        assert_eq!(material_at(-22.0, 28.0), GroundMaterial::CampsitePad);
        let pad = colour_at(-22.0, 28.0);
        let soil = colour_at(-22.0, 28.0 + 16.0);
        let pad_l = pad[0] + pad[1] + pad[2];
        let soil_l = soil[0] + soil[1] + soil[2];
        assert!((pad_l - soil_l).abs() > 8.0 || material_at(-22.0, 44.0) != GroundMaterial::CampsitePad);
        assert_ne!(material_at(-22.0, 44.0), GroundMaterial::CampsitePad);
    }

    #[test]
    fn fit_soil_joint_uses_authored_depth_not_a_tilted_disc() {
        let yaw = std::f64::consts::FRAC_PI_2;
        let placed = fit_soil_joint([10.0, 20.0], yaw, [3.0, -0.2, 0.0]);
        assert!((placed[0] - 10.0).abs() < 1e-9);
        assert!((placed[2] - 17.0).abs() < 1e-9);
        assert!((placed[1] - (height_at(10.0, 17.0) - 0.2)).abs() < 1e-9);

        let collar = fit_soil_joint([-22.0, 28.0], 0.0, [0.0, 0.0, 0.0]);
        assert!((collar[1] - height_at(-22.0, 28.0)).abs() < 1e-9);

        let mut trunk = [0.0, 0.0];
        let mut found = false;
        for j in (10..GRID_SIDE - 10).step_by(11) {
            for i in (10..GRID_SIDE - 10).step_by(13) {
                let x = -HALF_SPAN_M + i as f64 * CELL_M;
                let z = -HALF_SPAN_M + j as f64 * CELL_M;
                let left = height_at(x - 4.0, z);
                let right = height_at(x + 4.0, z);
                if (left - right).abs() > 0.2 {
                    trunk = [x, z];
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found);
        let a = fit_soil_joint(trunk, 0.0, [4.0, -0.3, 0.0]);
        let b = fit_soil_joint(trunk, 0.0, [-4.0, -0.3, 0.0]);
        assert!((a[1] - (height_at(trunk[0] + 4.0, trunk[1]) - 0.3)).abs() < 1e-9);
        assert!((b[1] - (height_at(trunk[0] - 4.0, trunk[1]) - 0.3)).abs() < 1e-9);
        assert!((a[1] - b[1]).abs() > 0.15);
        assert!(a[1] < height_at(a[0], a[2]));
        assert!(b[1] < height_at(b[0], b[2]));
    }

    #[test]
    fn non_finite_query_is_a_flat_datum() {
        assert_eq!(height_at(f64::NAN, 0.0), 0.0);
        assert_eq!(normal_at(0.0, f64::INFINITY), [0.0, 1.0, 0.0]);
    }
}
