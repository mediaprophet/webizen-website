//! Forestry track mesh for WR-03.
//!
//! World frame matches `ground::height_at`: metres, +Y up, basin origin.
//! The centreline runs north from the camp clearing at (-22, 28) to a
//! community-grounds pad. Each vertex is draped on the soil at that (x, z):
//! world y is `height_at` plus a small crown, shoulder, or cut-side drain
//! offset. The carriageway half-widths are the existing plant-clearance
//! values: 4.5 m dirt, 5.0 m gravel, 6.5 m sealed. This file does not change
//! the tile-profile fields in `gis_join.rs`.

#![allow(dead_code)]

/// Dirt forestry carriageway half-width. Same as the forest-campsite clearance.
pub const HALF_DIRT_M: f32 = 4.5;
/// Short gravel join. Same as the community-grounds clearance.
pub const HALF_GRAVEL_M: f32 = 5.0;
/// Sealed approach. Same as the road-corridor clearance.
pub const HALF_SEALED_M: f32 = 6.5;

/// Sealed pad and the gravel loop around it, in world metres.
pub const PAD_X: f32 = -22.0;
pub const PAD_Z: f32 = 154.0;
pub const PAD_INNER_M: f32 = 9.0;
pub const PAD_OUTER_M: f32 = 13.5;

const SPACING_M: f64 = 2.0;
const CROWN_DY: f64 = 0.12;
const DRAIN_DY: f64 = -0.26;

const NCOL: usize = 9;
const COLUMN_T: [f64; NCOL] = [-1.0, -0.82, -0.66, -0.36, 0.0, 0.36, 0.66, 0.82, 1.0];

/// Camp-clearing mouth, then north along the west bank to the pad.
const CONTROLS: [[f64; 2]; 7] = [
    [-19.0, 38.0],
    [-18.2, 58.0],
    [-19.0, 82.0],
    [-20.4, 106.0],
    [-21.6, 128.0],
    [-22.0, 142.0],
    [PAD_X as f64, PAD_Z as f64],
];

const HEADER_LEN: usize = 48;
const VERTEX_LEN: usize = 16;
const CENTRELINE_LEN: usize = 24;

pub const ROLE_EDGE: u8 = 0;
pub const ROLE_SHOULDER: u8 = 1;
pub const ROLE_DRAIN: u8 = 2;
pub const ROLE_LANE: u8 = 3;
pub const ROLE_CROWN: u8 = 4;

/// Surface the drive packet reads. Centreline samples are never [`RoadSurface::Grass`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RoadSurface {
    Dirt = 1,
    Gravel = 2,
    Sealed = 3,
    Grass = 4,
    Creek = 5,
}

impl RoadSurface {
    pub const fn as_str(self) -> &'static str {
        match self {
            RoadSurface::Dirt => "dirt",
            RoadSurface::Gravel => "gravel",
            RoadSurface::Sealed => "sealed",
            RoadSurface::Grass => "grass",
            RoadSurface::Creek => "creek",
        }
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(RoadSurface::Dirt),
            2 => Some(RoadSurface::Gravel),
            3 => Some(RoadSurface::Sealed),
            4 => Some(RoadSurface::Grass),
            5 => Some(RoadSurface::Creek),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CentrelineSample {
    pub x: f32,
    pub z: f32,
    pub y: f32,
    pub chainage_m: f32,
    pub half_width_m: f32,
    pub surface: RoadSurface,
    pub cut_right: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RoadHit {
    pub surface: RoadSurface,
    pub lateral_m: f64,
    pub half_width_m: f64,
    pub chainage_m: f64,
    pub cut_right: bool,
    pub on_pad: bool,
}

#[derive(Clone, Copy)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    surface: RoadSurface,
    role: u8,
    cut_right: bool,
}

pub struct BuiltRoad {
    pub bytes: Vec<u8>,
    pub samples: Vec<CentrelineSample>,
    pub max_grade: f64,
    pub drain_vertices: usize,
}

/// Crown, shoulders, and the cut-side drain, as a vertical offset in metres.
/// `t` is -1 on the left of travel and +1 on the right. `t == 0` is the crown.
pub fn profile_dy(t: f64, cut_right: bool) -> (f64, u8) {
    let t = t.clamp(-1.0, 1.0);
    let a = t.abs();
    let cut_sign = if cut_right { 1.0 } else { -1.0 };
    let on_cut = t * cut_sign > 0.55;
    if on_cut && a > 0.70 && a < 0.94 {
        return (DRAIN_DY, ROLE_DRAIN);
    }
    if a < 0.04 {
        return (CROWN_DY, ROLE_CROWN);
    }
    if a < 0.46 {
        let u = a / 0.46;
        return (CROWN_DY - 0.07 * u, ROLE_LANE);
    }
    if a < 0.74 {
        return (0.035, ROLE_SHOULDER);
    }
    (0.0, ROLE_EDGE)
}

pub fn author_mesh(height_at: impl Fn(f64, f64) -> f64) -> BuiltRoad {
    let pts = resample_centreline();
    let mut samples = Vec::with_capacity(pts.len());
    let mut chain = 0.0_f64;
    for (i, p) in pts.iter().enumerate() {
        if i > 0 {
            chain += hypot2(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
        }
        let (surface, half) = surface_for_chainage(chain, total_length(&pts));
        let tangent = tangent_at(&pts, i);
        let right = (tangent[1], -tangent[0]);
        let left_y = height_at(p[0] - right.0 * 4.0, p[1] - right.1 * 4.0);
        let right_y = height_at(p[0] + right.0 * 4.0, p[1] + right.1 * 4.0);
        let cut_right = if (right_y - left_y).abs() <= 0.02 {
            // Flat table: keep the drain on the left of travel, which is west
            // while the track heads north and away from the creek.
            right.0 < 0.0
        } else {
            right_y > left_y
        };
        let y = height_at(p[0], p[1]) + CROWN_DY;
        samples.push(CentrelineSample {
            x: p[0] as f32,
            z: p[1] as f32,
            y: y as f32,
            chainage_m: chain as f32,
            half_width_m: half,
            surface,
            cut_right,
        });
    }

    let mut max_grade = 0.0_f64;
    for w in samples.windows(2) {
        let ds = hypot2((w[1].x - w[0].x) as f64, (w[1].z - w[0].z) as f64);
        if ds > 0.2 {
            let g = ((w[1].y - w[0].y) as f64).abs() / ds;
            if g > max_grade {
                max_grade = g;
            }
        }
    }

    let mut verts = Vec::new();
    for (i, sample) in samples.iter().enumerate() {
        let tangent = tangent_at(&pts, i);
        let right = (tangent[1], -tangent[0]);
        for &t in &COLUMN_T {
            let off = t * sample.half_width_m as f64;
            let x = sample.x as f64 + right.0 * off;
            let z = sample.z as f64 + right.1 * off;
            let (dy, role) = profile_dy(t, sample.cut_right);
            verts.push(Vertex {
                x: x as f32,
                y: (height_at(x, z) + dy) as f32,
                z: z as f32,
                surface: sample.surface,
                role,
                cut_right: sample.cut_right,
            });
        }
    }
    let strip_count = verts.len();
    let stations = samples.len();
    let mut indices: Vec<u32> = Vec::new();
    for i in 0..stations.saturating_sub(1) {
        for j in 0..NCOL - 1 {
            let a = (i * NCOL + j) as u32;
            let b = ((i + 1) * NCOL + j) as u32;
            let c = (i * NCOL + j + 1) as u32;
            let d = ((i + 1) * NCOL + j + 1) as u32;
            indices.extend_from_slice(&[a, b, c, c, b, d]);
        }
    }

    let pad_centre_index = verts.len() as u32;
    let pad_y = height_at(PAD_X as f64, PAD_Z as f64);
    verts.push(Vertex {
        x: PAD_X,
        y: (pad_y + CROWN_DY) as f32,
        z: PAD_Z,
        surface: RoadSurface::Sealed,
        role: ROLE_CROWN,
        cut_right: false,
    });
    let sectors = 28usize;
    let rings: [(f64, RoadSurface, u8); 4] = [
        (3.0, RoadSurface::Sealed, ROLE_LANE),
        (6.0, RoadSurface::Sealed, ROLE_LANE),
        (PAD_INNER_M as f64, RoadSurface::Sealed, ROLE_SHOULDER),
        (PAD_OUTER_M as f64, RoadSurface::Gravel, ROLE_EDGE),
    ];
    let mut ring_start = Vec::new();
    for (radius, surface, role) in rings {
        ring_start.push(verts.len() as u32);
        for s in 0..sectors {
            let ang = (s as f64) * std::f64::consts::TAU / sectors as f64;
            let x = PAD_X as f64 + ang.cos() * radius;
            let z = PAD_Z as f64 + ang.sin() * radius;
            let dy = if surface == RoadSurface::Sealed { 0.06 } else { 0.02 };
            verts.push(Vertex {
                x: x as f32,
                y: (height_at(x, z) + dy) as f32,
                z: z as f32,
                surface,
                role,
                cut_right: false,
            });
        }
    }
    for s in 0..sectors {
        let s2 = ((s + 1) % sectors) as u32;
        indices.extend_from_slice(&[
            pad_centre_index,
            ring_start[0] + s as u32,
            ring_start[0] + s2,
        ]);
    }
    for r in 0..rings.len() - 1 {
        for s in 0..sectors {
            let s2 = (s + 1) % sectors;
            let a = ring_start[r] + s as u32;
            let b = ring_start[r + 1] + s as u32;
            let c = ring_start[r] + s2 as u32;
            let d = ring_start[r + 1] + s2 as u32;
            indices.extend_from_slice(&[a, b, c, c, b, d]);
        }
    }

    let strip_tris = strip_tri_count(stations);
    if indices.len() >= 3 && triangle_up(&verts, &indices[..3]) < 0.0 {
        for tri in indices.chunks_exact_mut(3).take(strip_tris) {
            tri.swap(1, 2);
        }
    }
    let pad_at = strip_tris * 3;
    if indices.len() >= pad_at + 3 && triangle_up(&verts, &indices[pad_at..pad_at + 3]) < 0.0 {
        for tri in indices[pad_at..].chunks_exact_mut(3) {
            tri.swap(1, 2);
        }
    }
    debug_assert!(strip_count <= verts.len());

    let drain_vertices = verts.iter().filter(|v| v.role == ROLE_DRAIN).count();
    let bytes = encode(&verts, &indices, &samples);
    BuiltRoad {
        bytes,
        samples,
        max_grade,
        drain_vertices,
    }
}

fn strip_tri_count(stations: usize) -> usize {
    stations.saturating_sub(1) * (NCOL - 1) * 2
}

fn triangle_up(verts: &[Vertex], tri: &[u32]) -> f64 {
    let a = verts[tri[0] as usize];
    let b = verts[tri[1] as usize];
    let c = verts[tri[2] as usize];
    let abx = (b.x - a.x) as f64;
    let abz = (b.z - a.z) as f64;
    let acx = (c.x - a.x) as f64;
    let acz = (c.z - a.z) as f64;
    abz * acx - abx * acz
}

pub fn centreline(mesh: &[u8]) -> Vec<CentrelineSample> {
    let Some(layout) = layout(mesh) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(layout.centreline_count);
    for i in 0..layout.centreline_count {
        let o = layout.centreline_at + i * CENTRELINE_LEN;
        let Some(surface) = RoadSurface::from_u8(mesh[o + 20]) else {
            continue;
        };
        out.push(CentrelineSample {
            x: f32_at(mesh, o),
            z: f32_at(mesh, o + 4),
            y: f32_at(mesh, o + 8),
            chainage_m: f32_at(mesh, o + 12),
            half_width_m: f32_at(mesh, o + 16),
            surface,
            cut_right: mesh[o + 21] != 0,
        });
    }
    out
}

pub fn classify(mesh: &[u8], x: f64, z: f64) -> Option<RoadHit> {
    if !x.is_finite() || !z.is_finite() {
        return None;
    }
    let samples = centreline(mesh);
    if samples.len() < 2 {
        return None;
    }
    let mut best_dist = f64::MAX;
    let mut best: Option<RoadHit> = None;
    for w in samples.windows(2) {
        let (dist, t, lateral, _along) = segment_closest(
            w[0].x as f64,
            w[0].z as f64,
            w[1].x as f64,
            w[1].z as f64,
            x,
            z,
        );
        if dist < best_dist {
            best_dist = dist;
            let half = w[0].half_width_m as f64
                + (w[1].half_width_m as f64 - w[0].half_width_m as f64) * t;
            let surface = if t < 0.5 { w[0].surface } else { w[1].surface };
            let cut_right = if t < 0.5 { w[0].cut_right } else { w[1].cut_right };
            let chainage = w[0].chainage_m as f64
                + (w[1].chainage_m as f64 - w[0].chainage_m as f64) * t;
            best = Some(RoadHit {
                surface,
                lateral_m: lateral,
                half_width_m: half,
                chainage_m: chainage,
                cut_right,
                on_pad: false,
            });
        }
    }
    let (px, pz, inner, outer) = pad(mesh);
    let pad_dist = hypot2(x - px, z - pz);
    let on_strip = best
        .as_ref()
        .map(|hit| best_dist <= hit.half_width_m + 0.02)
        .unwrap_or(false);
    if on_strip {
        let mut hit = best.unwrap();
        hit.on_pad = pad_dist <= outer as f64;
        return Some(hit);
    }
    if pad_dist <= inner as f64 {
        return Some(RoadHit {
            surface: RoadSurface::Sealed,
            lateral_m: 0.0,
            half_width_m: inner as f64,
            chainage_m: samples.last().map(|s| s.chainage_m as f64).unwrap_or(0.0),
            cut_right: false,
            on_pad: true,
        });
    }
    if pad_dist <= outer as f64 {
        return Some(RoadHit {
            surface: RoadSurface::Gravel,
            lateral_m: pad_dist,
            half_width_m: outer as f64,
            chainage_m: samples.last().map(|s| s.chainage_m as f64).unwrap_or(0.0),
            cut_right: false,
            on_pad: true,
        });
    }
    None
}

/// On the authored mesh, the surface is dirt, gravel, or sealed.
/// Off it, `creek` selects creek, otherwise grass.
pub fn surface_at_with(mesh: &[u8], x: f64, z: f64, creek: bool) -> RoadSurface {
    if let Some(hit) = classify(mesh, x, z) {
        return hit.surface;
    }
    if creek {
        RoadSurface::Creek
    } else {
        RoadSurface::Grass
    }
}

pub fn plant_blocked(mesh: &[u8], x: f64, z: f64, extra_m: f64) -> bool {
    if !x.is_finite() || !z.is_finite() {
        return false;
    }
    let samples = centreline(mesh);
    let mut best = f64::MAX;
    let mut half = 0.0_f64;
    if samples.len() >= 2 {
        for w in samples.windows(2) {
            let (dist, t, _, _) = segment_closest(
                w[0].x as f64,
                w[0].z as f64,
                w[1].x as f64,
                w[1].z as f64,
                x,
                z,
            );
            if dist < best {
                best = dist;
                half = w[0].half_width_m as f64
                    + (w[1].half_width_m as f64 - w[0].half_width_m as f64) * t;
            }
        }
        if best < half + extra_m {
            return true;
        }
    }
    let (px, pz, _inner, outer) = pad(mesh);
    hypot2(x - px, z - pz) < outer as f64 + extra_m
}

/// One formed-surface vertex. The surface byte is the ontology key, not a colour.
#[derive(Clone, Copy, Debug)]
pub struct RoadVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub surface: RoadSurface,
}

/// Positions and triangle indices of the formed track. Grass and creek are not
/// vertices of this mesh. `None` when the file is not a closed ROAD v1.
pub fn road_vertices(mesh: &[u8]) -> Option<(Vec<RoadVertex>, Vec<u32>)> {
    if mesh.len() < HEADER_LEN || &mesh[0..4] != b"ROAD" || u32_at(mesh, 4) != 1 {
        return None;
    }
    let vertex_count = u32_at(mesh, 8) as usize;
    let index_count = u32_at(mesh, 12) as usize;
    let centreline_count = u32_at(mesh, 16) as usize;
    if index_count % 3 != 0 {
        return None;
    }
    let centreline_at = HEADER_LEN + vertex_count * VERTEX_LEN + index_count * 4;
    if centreline_at + centreline_count * CENTRELINE_LEN != mesh.len() {
        return None;
    }
    let mut vertices = Vec::with_capacity(vertex_count);
    for i in 0..vertex_count {
        let o = HEADER_LEN + i * VERTEX_LEN;
        let surface = RoadSurface::from_u8(mesh[o + 12])?;
        if !matches!(
            surface,
            RoadSurface::Dirt | RoadSurface::Gravel | RoadSurface::Sealed
        ) {
            return None;
        }
        vertices.push(RoadVertex {
            x: f32_at(mesh, o),
            y: f32_at(mesh, o + 4),
            z: f32_at(mesh, o + 8),
            surface,
        });
    }
    let mut indices = Vec::with_capacity(index_count);
    for i in 0..index_count {
        let index = u32_at(mesh, centreline_at - index_count * 4 + i * 4);
        if index as usize >= vertex_count {
            return None;
        }
        indices.push(index);
    }
    Some((vertices, indices))
}

pub fn pad(mesh: &[u8]) -> (f64, f64, f32, f32) {
    if mesh.len() < HEADER_LEN || &mesh[0..4] != b"ROAD" {
        return (PAD_X as f64, PAD_Z as f64, PAD_INNER_M, PAD_OUTER_M);
    }
    (
        f32_at(mesh, 20) as f64,
        f32_at(mesh, 24) as f64,
        f32_at(mesh, 28),
        f32_at(mesh, 32),
    )
}

struct Layout {
    centreline_count: usize,
    centreline_at: usize,
}

fn layout(mesh: &[u8]) -> Option<Layout> {
    if mesh.len() < HEADER_LEN || &mesh[0..4] != b"ROAD" {
        return None;
    }
    if u32_at(mesh, 4) != 1 {
        return None;
    }
    let vertex_count = u32_at(mesh, 8) as usize;
    let index_count = u32_at(mesh, 12) as usize;
    let centreline_count = u32_at(mesh, 16) as usize;
    let centreline_at = HEADER_LEN + vertex_count * VERTEX_LEN + index_count * 4;
    let end = centreline_at + centreline_count * CENTRELINE_LEN;
    if end != mesh.len() {
        return None;
    }
    Some(Layout {
        centreline_count,
        centreline_at,
    })
}

fn encode(verts: &[Vertex], indices: &[u32], samples: &[CentrelineSample]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        HEADER_LEN + verts.len() * VERTEX_LEN + indices.len() * 4 + samples.len() * CENTRELINE_LEN,
    );
    bytes.extend_from_slice(b"ROAD");
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&(verts.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(indices.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&PAD_X.to_le_bytes());
    bytes.extend_from_slice(&PAD_Z.to_le_bytes());
    bytes.extend_from_slice(&PAD_INNER_M.to_le_bytes());
    bytes.extend_from_slice(&PAD_OUTER_M.to_le_bytes());
    bytes.extend_from_slice(&HALF_DIRT_M.to_le_bytes());
    bytes.extend_from_slice(&HALF_GRAVEL_M.to_le_bytes());
    bytes.extend_from_slice(&HALF_SEALED_M.to_le_bytes());
    debug_assert_eq!(bytes.len(), HEADER_LEN);
    for v in verts {
        bytes.extend_from_slice(&v.x.to_le_bytes());
        bytes.extend_from_slice(&v.y.to_le_bytes());
        bytes.extend_from_slice(&v.z.to_le_bytes());
        bytes.push(v.surface as u8);
        bytes.push(v.role);
        bytes.push(u8::from(v.cut_right));
        bytes.push(0);
    }
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    for sample in samples {
        bytes.extend_from_slice(&sample.x.to_le_bytes());
        bytes.extend_from_slice(&sample.z.to_le_bytes());
        bytes.extend_from_slice(&sample.y.to_le_bytes());
        bytes.extend_from_slice(&sample.chainage_m.to_le_bytes());
        bytes.extend_from_slice(&sample.half_width_m.to_le_bytes());
        bytes.push(sample.surface as u8);
        bytes.push(u8::from(sample.cut_right));
        bytes.extend_from_slice(&0u16.to_le_bytes());
    }
    bytes
}

fn surface_for_chainage(chainage: f64, total: f64) -> (RoadSurface, f32) {
    let dirt_end = total * 0.46;
    let gravel_end = total * 0.62;
    if chainage < dirt_end {
        (RoadSurface::Dirt, HALF_DIRT_M)
    } else if chainage < gravel_end {
        (RoadSurface::Gravel, HALF_GRAVEL_M)
    } else {
        (RoadSurface::Sealed, HALF_SEALED_M)
    }
}

fn total_length(pts: &[[f64; 2]]) -> f64 {
    pts.windows(2)
        .map(|w| hypot2(w[1][0] - w[0][0], w[1][1] - w[0][1]))
        .sum()
}

fn resample_centreline() -> Vec<[f64; 2]> {
    let n = CONTROLS.len();
    let mut dense = Vec::new();
    for i in 0..n - 1 {
        let p0 = CONTROLS[i.saturating_sub(1)];
        let p1 = CONTROLS[i];
        let p2 = CONTROLS[i + 1];
        let p3 = CONTROLS[(i + 2).min(n - 1)];
        let steps = 24;
        for s in 0..steps {
            let t = s as f64 / steps as f64;
            dense.push(catmull(p0, p1, p2, p3, t));
        }
    }
    dense.push(*CONTROLS.last().unwrap());
    let mut out = vec![dense[0]];
    let mut acc = 0.0_f64;
    let mut next = SPACING_M;
    for w in dense.windows(2) {
        let d = hypot2(w[1][0] - w[0][0], w[1][1] - w[0][1]);
        if d < 1e-8 {
            continue;
        }
        while acc + d >= next - 1e-8 {
            let t = ((next - acc) / d).clamp(0.0, 1.0);
            out.push([
                w[0][0] + (w[1][0] - w[0][0]) * t,
                w[0][1] + (w[1][1] - w[0][1]) * t,
            ]);
            next += SPACING_M;
        }
        acc += d;
    }
    let end = *dense.last().unwrap();
    let last = *out.last().unwrap();
    if hypot2(last[0] - end[0], last[1] - end[1]) > 0.35 {
        out.push(end);
    } else {
        *out.last_mut().unwrap() = end;
    }
    out
}

fn catmull(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], t: f64) -> [f64; 2] {
    let t2 = t * t;
    let t3 = t2 * t;
    let mut out = [0.0, 0.0];
    for k in 0..2 {
        out[k] = 0.5
            * ((2.0 * p1[k])
                + (-p0[k] + p2[k]) * t
                + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * t2
                + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * t3);
    }
    out
}

fn tangent_at(pts: &[[f64; 2]], i: usize) -> [f64; 2] {
    let prev = pts[i.saturating_sub(1)];
    let next = pts[(i + 1).min(pts.len() - 1)];
    let dx = next[0] - prev[0];
    let dz = next[1] - prev[1];
    let len = hypot2(dx, dz).max(1e-8);
    [dx / len, dz / len]
}

fn segment_closest(ax: f64, az: f64, bx: f64, bz: f64, x: f64, z: f64) -> (f64, f64, f64, f64) {
    let dx = bx - ax;
    let dz = bz - az;
    let len2 = dx * dx + dz * dz;
    let len = len2.sqrt();
    if len < 1e-8 {
        return (hypot2(x - ax, z - az), 0.0, 0.0, 0.0);
    }
    let t = (((x - ax) * dx + (z - az) * dz) / len2).clamp(0.0, 1.0);
    let px = ax + dx * t;
    let pz = az + dz * t;
    let ox = x - px;
    let oz = z - pz;
    let rx = dz / len;
    let rz = -dx / len;
    (hypot2(ox, oz), t, ox * rx + oz * rz, t * len)
}

fn hypot2(x: f64, z: f64) -> f64 {
    (x * x + z * z).sqrt()
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn f32_at(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
