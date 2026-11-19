#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use super::*;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(target_arch = "wasm32", derive(Deserialize))]
pub struct Placement {
    pub id: String,
    pub translation: [f64; 3],
    pub rotation: [f64; 4],
    pub scale: [f64; 3],
}

/// 64-byte deterministic phenotype descriptor for GPU-instanced ecological individuals.
/// Allows one shared species `.10d` asset to instantiate hundreds of unique individuals
/// across a 600 m catchment with deterministic yaw, trunk lean, ontogenetic height,
/// and vigor-modulated scale variation.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceState64 {
    pub seed: u64,
    pub ontogenetic_age_norm: f32,
    pub height_scale: f32,
    pub site_vigor: f32,
    pub environmental_stress: [f32; 4],
    pub _reserved: [u32; 7],
}

const _: () = assert!(std::mem::size_of::<InstanceState64>() == 64);

impl InstanceState64 {
    pub const fn new(
        seed: u64,
        ontogenetic_age_norm: f32,
        height_scale: f32,
        site_vigor: f32,
        environmental_stress: [f32; 4],
    ) -> Self {
        Self {
            seed,
            ontogenetic_age_norm,
            height_scale,
            site_vigor,
            environmental_stress,
            _reserved: [0u32; 7],
        }
    }
}

/// Deterministic `splitmix64` step for reproducible phenotype derivation.
#[inline]
fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Map a `splitmix64` output uniformly to `[0.0, 1.0)`.
#[inline]
fn next_unit_f64(state: &mut u64) -> f64 {
    let bits = splitmix64_next(state) >> 11;
    (bits as f64) * (1.0 / ((1u64 << 53) as f64))
}

/// Map a `splitmix64` output uniformly to `[-1.0, 1.0)`.
#[inline]
fn next_signed_f64(state: &mut u64) -> f64 {
    next_unit_f64(state) * 2.0 - 1.0
}

#[inline]
fn sanitize_f64(v: f32, default: f64, min: f64, max: f64) -> f64 {
    if v.is_finite() {
        (v as f64).clamp(min, max)
    } else {
        default
    }
}

/// Compose an intrinsic Y-X-Z Euler rotation (yaw around Y, slight trunk lean in X/Z)
/// into a normalized `[x, y, z, w]` quaternion.
fn quat_from_yaw_and_lean(yaw: f64, pitch_x: f64, roll_z: f64) -> [f64; 4] {
    let (sy, cy) = (yaw * 0.5).sin_cos();
    let (sx, cx) = (pitch_x * 0.5).sin_cos();
    let (sz, cz) = (roll_z * 0.5).sin_cos();

    // q = q_y * q_x * q_z
    let x = cy * sx * cz + sy * cx * sz;
    let y = sy * cx * cz - cy * sx * sz;
    let z = cy * cx * sz - sy * sx * cz;
    let w = cy * cx * cz + sy * sx * sz;

    let norm = (x * x + y * y + z * z + w * w).sqrt();
    if norm > 1e-12 && norm.is_finite() {
        [x / norm, y / norm, z / norm, w / norm]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// Deterministically derive a world-space [`Placement`] from `base_pos` and an
/// [`InstanceState64`] phenotype descriptor.
pub fn derive_placement_from_state64(base_pos: [f32; 3], state: &InstanceState64) -> Placement {
    let mut rng = state.seed;

    let age = sanitize_f64(state.ontogenetic_age_norm, 0.5, 0.0, 1.0);
    let height_scale = sanitize_f64(state.height_scale, 1.0, 0.25, 3.0);
    let site_vigor = sanitize_f64(state.site_vigor, 1.0, 0.25, 2.0);
    let stress_mean = state
        .environmental_stress
        .iter()
        .map(|&s| sanitize_f64(s, 0.0, 0.0, 1.0))
        .sum::<f64>()
        / 4.0;
    let wind_stress = sanitize_f64(state.environmental_stress[0], 0.0, 0.0, 1.0);

    // 1. Natural yaw in [0, 2*PI).
    let yaw = next_unit_f64(&mut rng) * std::f64::consts::TAU;

    // 2. Slight natural trunk lean (up to ~3.5 deg baseline, up to ~6.5 deg under wind stress).
    let max_lean_rad = 0.06 + 0.055 * wind_stress;
    let lean_x = next_signed_f64(&mut rng) * max_lean_rad;
    let lean_z = next_signed_f64(&mut rng) * max_lean_rad;
    let rotation = quat_from_yaw_and_lean(yaw, lean_x, lean_z);

    // 3. Ontogenetic height & crown/stem scale with natural deterministic variation.
    //    Saplings (age=0) start at ~0.35x; mature individuals (age=1) reach 1.0x.
    let ontogenetic_factor = 0.35 + 0.65 * age.powf(0.75);
    let stress_damping = 1.0 - 0.18 * stress_mean;
    let vigor_factor = 0.75 + 0.25 * site_vigor;

    let radial_jitter_x = 1.0 + 0.12 * next_signed_f64(&mut rng);
    let radial_jitter_z = 1.0 + 0.12 * next_signed_f64(&mut rng);
    let height_jitter = 1.0 + 0.10 * next_signed_f64(&mut rng);

    let sy = (ontogenetic_factor * height_scale * vigor_factor * stress_damping * height_jitter)
        .clamp(0.15, 4.0);
    let radial_base = (ontogenetic_factor * (0.8 + 0.2 * height_scale) * vigor_factor * stress_damping)
        .clamp(0.15, 4.0);
    let sx = (radial_base * radial_jitter_x).clamp(0.15, 4.0);
    let sz = (radial_base * radial_jitter_z).clamp(0.15, 4.0);

    let translation = [
        if base_pos[0].is_finite() { base_pos[0] as f64 } else { 0.0 },
        if base_pos[1].is_finite() { base_pos[1] as f64 } else { 0.0 },
        if base_pos[2].is_finite() { base_pos[2] as f64 } else { 0.0 },
    ];

    Placement {
        id: format!("inst-{:016x}", state.seed),
        translation,
        rotation,
        scale: [sx, sy, sz],
    }
}

/// Default topsoil cover depth (metres) above the uppermost subterranean root plate
/// so lateral roots remain buried on steep montane slopes.
pub const DEFAULT_ROOT_SOIL_COVER_M: f64 = 0.08;

/// Subterranean 5-order root plate & mycorrhizal network transform aligned to the
/// terrain surface normal and sunk below the downhill slope face.
///
/// Above-ground tree trunks remain vertical (geotropic) via
/// [`derive_placement_from_state64`], while subterranean root plates use
/// [`compute_slope_conforming_root_transform`] so no root crown or lateral root
/// ever pokes into mid-air on the downhill side of a slope.
#[derive(Clone, Debug, PartialEq)]
pub struct SlopeRootPlacement {
    /// World-space `[x, y, z]` origin for the subterranean root plate.
    pub root_translation: [f64; 3],
    /// Unit quaternion `[x, y, z, w]` pitching local `+Y` onto `terrain_normal`.
    pub root_rotation: [f64; 4],
    /// Vertical burial sink applied (`sin(slope_rad) * root_radius_m + soil_cover_m`).
    pub sink_depth_m: f64,
    /// Topsoil cover margin applied (metres).
    pub soil_cover_m: f64,
    /// Effective slope angle in radians (`[0, FRAC_PI_2)`).
    pub slope_rad: f64,
    /// Normalized upward-facing terrain surface normal `[nx, ny, nz]`.
    pub terrain_normal: [f64; 3],
}

/// Normalize a terrain surface normal, guaranteeing a finite unit vector with `ny > 0`.
fn normalize_upward_normal(n: [f32; 3]) -> [f64; 3] {
    if !n.iter().all(|v| v.is_finite()) {
        return [0.0, 1.0, 0.0];
    }
    let mut nx = n[0] as f64;
    let mut ny = n[1] as f64;
    let mut nz = n[2] as f64;
    if ny < 0.0 {
        nx = -nx;
        ny = -ny;
        nz = -nz;
    }
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len < 1e-9 || !len.is_finite() {
        return [0.0, 1.0, 0.0];
    }
    [nx / len, ny / len, nz / len]
}

/// Shortest-arc unit quaternion `[x, y, z, w]` rotating `+Y = [0, 1, 0]` to unit `normal` (`ny >= 0`).
fn quat_from_up_to_normal(normal: [f64; 3]) -> [f64; 4] {
    let [nx, ny, nz] = normal;
    let x = nz;
    let y = 0.0;
    let z = -nx;
    let w = 1.0 + ny.max(0.0);
    let norm = (x * x + y * y + z * z + w * w).sqrt();
    if norm > 1e-12 && norm.is_finite() {
        [x / norm, y / norm, z / norm, w / norm]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// Rotate a 3D vector by a unit quaternion `[x, y, z, w]`.
pub fn rotate_vec_by_quat(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let [qx, qy, qz, qw] = q;
    let [vx, vy, vz] = v;
    let tx = 2.0 * (qy * vz - qz * vy);
    let ty = 2.0 * (qz * vx - qx * vz);
    let tz = 2.0 * (qx * vy - qy * vx);
    [
        vx + qw * tx + (qy * tz - qz * ty),
        vy + qw * ty + (qz * tx - qx * tz),
        vz + qw * tz + (qx * ty - qy * tx),
    ]
}

/// Compute a slope-conforming subterranean root plate transform for a flora individual
/// at `pos` on terrain with `terrain_normal`.
///
/// The root plate pitches to match `terrain_normal` and sinks vertically by
/// `sin(effective_slope) * root_radius_m + soil_cover_m`, guaranteeing that every
/// point on the top of the root disc (`|r| <= root_radius_m`) lies at or below both
/// the sloped terrain surface `ground_y(x, z)` and `pos[1]`.
pub fn compute_slope_conforming_root_transform(
    pos: [f32; 3],
    terrain_normal: [f32; 3],
    root_radius_m: f32,
    slope_rad: f32,
) -> SlopeRootPlacement {
    compute_slope_conforming_root_transform_with_cover(
        pos,
        terrain_normal,
        root_radius_m,
        slope_rad,
        DEFAULT_ROOT_SOIL_COVER_M as f32,
    )
}

/// Variant of [`compute_slope_conforming_root_transform`] accepting an explicit `soil_cover_m`.
pub fn compute_slope_conforming_root_transform_with_cover(
    pos: [f32; 3],
    terrain_normal: [f32; 3],
    root_radius_m: f32,
    slope_rad: f32,
    soil_cover_m: f32,
) -> SlopeRootPlacement {
    let normal = normalize_upward_normal(terrain_normal);
    let normal_slope = normal[1].clamp(0.0, 1.0).acos();
    let caller_slope = if slope_rad.is_finite() {
        (slope_rad as f64).abs().clamp(0.0, std::f64::consts::FRAC_PI_2 - 1e-4)
    } else {
        0.0
    };
    // Use the steeper of caller-supplied slope_rad and the normal's geometric slope
    // so roots never under-sink if either estimate is steeper.
    let eff_slope = caller_slope.max(normal_slope);
    let radius = if root_radius_m.is_finite() {
        (root_radius_m as f64).max(0.0)
    } else {
        0.0
    };
    let cover = if soil_cover_m.is_finite() {
        (soil_cover_m as f64).max(0.0)
    } else {
        DEFAULT_ROOT_SOIL_COVER_M
    };

    let sink_depth_m = eff_slope.sin() * radius + cover;
    let px = if pos[0].is_finite() { pos[0] as f64 } else { 0.0 };
    let py = if pos[1].is_finite() { pos[1] as f64 } else { 0.0 };
    let pz = if pos[2].is_finite() { pos[2] as f64 } else { 0.0 };

    SlopeRootPlacement {
        root_translation: [px, py - sink_depth_m, pz],
        root_rotation: quat_from_up_to_normal(normal),
        sink_depth_m,
        soil_cover_m: cover,
        slope_rad: eff_slope,
        terrain_normal: normal,
    }
}

impl SlopeRootPlacement {
    /// Convert this subterranean root transform into a renderable [`Placement`].
    pub fn to_placement(&self, id: impl Into<String>, scale: [f64; 3]) -> Placement {
        Placement {
            id: id.into(),
            translation: self.root_translation,
            rotation: self.root_rotation,
            scale,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn bind(
    portal: &mut QualiaPortal,
    scene: &Scene,
    upload: &Value,
    overlay: Option<&super::macros::Live>,
) -> Result<Value, JsValue> {
    bind_parts(portal, scene, upload, false, overlay)
}

/// Rebuild instance groups from the resident uploaded set (EWM first-paint).
#[cfg(target_arch = "wasm32")]
pub(super) fn bind_resident(
    portal: &mut QualiaPortal,
    scene: &Scene,
    upload: &Value,
    overlay: Option<&super::macros::Live>,
) -> Result<Value, JsValue> {
    bind_parts(portal, scene, upload, true, overlay)
}

#[cfg(target_arch = "wasm32")]
fn bind_parts(
    portal: &mut QualiaPortal,
    scene: &Scene,
    upload: &Value,
    resident: bool,
    overlay: Option<&super::macros::Live>,
) -> Result<Value, JsValue> {
    if !resident && scene.assets.iter().all(|a| a.placements.is_empty()) {
        return Ok(Value::Null);
    }
    let parts = upload["parts"]
        .as_array()
        .ok_or(err("Missing native prototype index spans"))?;
    if !resident && parts.len() != scene.assets.len() {
        return Err(err("Native prototype part admission mismatch"));
    }
    if !resident {
        for (asset, part) in scene.assets.iter().zip(parts.iter()) {
            if part["id"] != asset.id {
                return Err(err(
                    "Native prototype order differs from its source manifest",
                ));
            }
        }
    }
    let mut transforms = Vec::new();
    let mut ids = Vec::new();
    let mut groups = Vec::with_capacity(parts.len() * 4 + 4);
    let mut semantic_ids = BTreeSet::new();
    let mut first_record = 0u32;
    let mut expanded = 0u64;
    let mut covered_indices = 0u32;
    for part in parts {
        let part_id = part["id"]
            .as_str()
            .ok_or(err("Native prototype part missing id"))?;
        let asset = scene.assets.iter().find(|a| a.id == part_id);
        let first_index = part["first_index"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(err("Invalid native first index"))?;
        let index_count = part["index_count"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(err("Invalid native index count"))?;
        if index_count == 0 {
            continue;
        }
        let placement_count = asset.map(|a| a.placements.len().max(1)).unwrap_or(1) as u32;
        groups.extend([first_index, index_count, first_record, placement_count]);
        first_record += placement_count;
        expanded += u64::from(index_count / 3) * u64::from(placement_count);
        covered_indices = covered_indices.max(first_index.saturating_add(index_count));
        let identity = Placement {
            id: part_id.to_string(),
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        };
        let placements: &[Placement] = match asset {
            Some(a) if !a.placements.is_empty() => &a.placements,
            _ => std::slice::from_ref(&identity),
        };
        for placement in placements {
            push_instance(
                &mut transforms,
                &mut ids,
                &mut semantic_ids,
                placement,
                overlay
                    .map(|live| {
                        live.overlay(
                            part_id,
                            !asset.map(|a| a.placements.is_empty()).unwrap_or(true),
                        )
                    })
                    .unwrap_or([0.; 3]),
                overlay
                    .map(|live| live.overlay_scale(part_id))
                    .unwrap_or([1.; 3]),
            )?;
        }
    }
    if resident {
        if let Some(remainder) = suffix_indices(portal, covered_indices)? {
            let identity = Placement {
                id: "urn:webizen:placement:gpu-suffix".into(),
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
            };
            groups.extend([covered_indices, remainder, first_record, 1]);
            expanded += u64::from(remainder / 3);
            push_instance(
                &mut transforms,
                &mut ids,
                &mut semantic_ids,
                &identity,
                [0.; 3],
                [1.; 3],
            )?;
        }
    }
    let count = ids.len() / 2;
    if count == 0 {
        return Err(err("Native prototype instance groups are empty"));
    }
    if count > 10_000 {
        return Err(err("Native prototype scene exceeds 10,000 source records"));
    }
    if !resident && scene.expanded_triangle_count != Some(expanded) {
        return Err(err(
            "Expanded prototype triangle count differs from its actual source geometry",
        ));
    }
    let mut packed = vec![0u8; count * portal.mesh_instance_record_stride() as usize];
    portal.pack_mesh_instance_records(&transforms, &ids, &mut packed)?;
    let result =
        portal.set_mesh_instance_groups(portal.mesh_instance_record_version(), &groups, &packed)?;
    let mut receipt: Value = serde_json::from_str(
        &js_sys::JSON::stringify(&result)?
            .as_string()
            .ok_or(err("Invalid native instance receipt"))?,
    )
    .map_err(err)?;
    receipt["ecology_placements"] = json!(if resident {
        parts
            .iter()
            .filter_map(|part| part["id"].as_str())
            .filter_map(|id| scene.assets.iter().find(|a| a.id == id))
            .map(|a| a.placements.len())
            .sum::<usize>()
    } else {
        scene
            .assets
            .iter()
            .map(|a| a.placements.len())
            .sum::<usize>()
    });
    receipt["packed_record_bytes"] = json!(packed.len());
    receipt["source_geometry_triangles"] = upload["total_triangles"].clone();
    receipt["resident_subset"] = json!(resident);
    receipt["expanded_triangles"] = json!(expanded);
    receipt["transform_provider"] = json!(
        "QualiaDB / Etalon Hamilton quaternion matrix and native ABI inverse-transpose frames"
    );
    Ok(receipt)
}

#[cfg(target_arch = "wasm32")]
fn suffix_indices(portal: &QualiaPortal, covered: u32) -> Result<Option<u32>, JsValue> {
    let admitted = portal.scene_render_receipt()?;
    let gpu_indices = js_sys::Reflect::get(&admitted, &"index_count".into())?
        .as_f64()
        .unwrap_or(0.) as u64;
    let gpu_indices = u32::try_from(gpu_indices).unwrap_or(0);
    if gpu_indices <= covered {
        return Ok(None);
    }
    let remainder = gpu_indices - covered;
    if remainder % 3 != 0 {
        return Err(err(
            "instance group partition leaves unowned geometry or records",
        ));
    }
    Ok(Some(remainder))
}

#[cfg(target_arch = "wasm32")]
fn push_instance(
    transforms: &mut Vec<f32>,
    ids: &mut Vec<u32>,
    semantic_ids: &mut BTreeSet<u64>,
    placement: &Placement,
    extra: [f64; 3],
    extra_scale: [f64; 3],
) -> Result<(), JsValue> {
    if placement
        .translation
        .iter()
        .chain(placement.scale.iter())
        .chain(extra_scale.iter())
        .any(|v| !v.is_finite())
        || placement.scale.iter().any(|v| v.abs() < 1e-8)
        || extra_scale.iter().any(|v| v.abs() < 1e-8)
    {
        return Err(err("Invalid authored prototype placement"));
    }
    let rotation = qualia_core_db::audio::qac::spatial_dsp::wigner_rotation::rotation_matrix_from_quaternion_f64(
        placement.rotation,
    )
    .map_err(|e| err(format!("Invalid native placement quaternion: {e:?}")))?;
    for col in 0..3 {
        for row in 0..3 {
            transforms.push((rotation[row][col] * placement.scale[col] * extra_scale[col]) as f32);
        }
        transforms.push(0.);
    }
    transforms.extend([
        (placement.translation[0] + extra[0]) as f32,
        (placement.translation[1] + extra[1]) as f32,
        (placement.translation[2] + extra[2]) as f32,
    ]);
    transforms.push(1.);
    let semantic = qualia_core_db::mini_parser::hash_token(&format!(
        "urn:webizen:placement:{}",
        placement.id
    ));
    if !semantic_ids.insert(semantic) {
        return Err(err("Duplicate native placement identity"));
    }
    ids.extend([semantic as u32, (semantic >> 32) as u32]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_state64_is_exact_64_bytes_repr_c() {
        assert_eq!(std::mem::size_of::<InstanceState64>(), 64);
        assert_eq!(std::mem::align_of::<InstanceState64>(), 8);
    }

    #[test]
    fn derive_placement_from_state64_is_deterministic_and_produces_unit_quaternion() {
        let state = InstanceState64::new(0x0123_4567_89ab_cdef, 0.85, 1.15, 1.05, [0.2, 0.1, 0.0, 0.05]);
        let p1 = derive_placement_from_state64([120.5, 4.25, -240.0], &state);
        let p2 = derive_placement_from_state64([120.5, 4.25, -240.0], &state);
        assert_eq!(p1, p2);
        assert_eq!(p1.id, "inst-0123456789abcdef");
        assert_eq!(p1.translation, [120.5, 4.25, -240.0]);

        // Quaternion must be finite and unit-length.
        let q = p1.rotation;
        assert!(q.iter().all(|v| v.is_finite()));
        let norm = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        assert!((norm - 1.0).abs() < 1e-12, "expected unit quaternion, got norm={norm}");

        // Scale must be strictly positive and finite.
        assert!(p1.scale.iter().all(|v| v.is_finite() && *v >= 0.15));
    }

    #[test]
    fn distinct_seeds_yield_unique_individuals_and_ontogenetic_height_scales_monotonically() {
        let sapling = InstanceState64::new(42, 0.05, 1.0, 1.0, [0.0; 4]);
        let mature = InstanceState64::new(42, 1.0, 1.0, 1.0, [0.0; 4]);
        let p_sapling = derive_placement_from_state64([0.0, 0.0, 0.0], &sapling);
        let p_mature = derive_placement_from_state64([0.0, 0.0, 0.0], &mature);

        // Same seed => same yaw/lean, but mature individual is substantially taller than sapling.
        assert_eq!(p_sapling.rotation, p_mature.rotation);
        assert!(
            p_mature.scale[1] > p_sapling.scale[1] * 1.8,
            "expected mature height ({}) >> sapling height ({})",
            p_mature.scale[1],
            p_sapling.scale[1]
        );

        // Distinct seeds => distinct ids, rotations, and scales across a 600 m catchment.
        let mut ids = std::collections::BTreeSet::new();
        let mut rotations = Vec::new();
        for seed in 1u64..=200 {
            let st = InstanceState64::new(seed, 0.6, 1.0, 1.0, [0.15, 0.1, 0.05, 0.0]);
            let p = derive_placement_from_state64([(seed as f32) * 3.0 - 300.0, 0.0, 0.0], &st);
            assert!(ids.insert(p.id.clone()));
            rotations.push(p.rotation);
        }
        assert_eq!(ids.len(), 200);
        assert_ne!(rotations[0], rotations[1]);
        assert_ne!(rotations[1], rotations[2]);
    }

    #[test]
    fn derive_placement_sanitizes_non_finite_inputs() {
        let bad = InstanceState64::new(999, f32::NAN, f32::INFINITY, -5.0, [f32::NAN; 4]);
        let p = derive_placement_from_state64([f32::NAN, 1.0, f32::NEG_INFINITY], &bad);
        assert!(p.translation.iter().all(|v| v.is_finite()));
        assert!(p.rotation.iter().all(|v| v.is_finite()));
        assert!(p.scale.iter().all(|v| v.is_finite() && *v >= 0.15));
    }

    #[test]
    fn slope_conforming_root_burial_keeps_root_top_below_ground_on_25_deg_slope() {
        let slope_deg = 25.0_f64;
        let slope_rad = slope_deg.to_radians();
        // Sloped terrain normal tilting downhill toward +X: n = (sin(25°), cos(25°), 0)
        let normal = [slope_rad.sin() as f32, slope_rad.cos() as f32, 0.0_f32];
        let ground_pos = [42.0_f32, 488.5_f32, -15.0_f32];
        let root_radius_m = 3.2_f32;

        let root = compute_slope_conforming_root_transform(
            ground_pos,
            normal,
            root_radius_m,
            slope_rad as f32,
        );

        // 1. Verify quaternion aligns local +Y directly onto the terrain normal.
        let rotated_up = rotate_vec_by_quat(root.root_rotation, [0.0, 1.0, 0.0]);
        for i in 0..3 {
            assert!(
                (rotated_up[i] - root.terrain_normal[i]).abs() < 1e-9,
                "rotated up[{i}] = {}, expected normal[{i}] = {}",
                rotated_up[i],
                root.terrain_normal[i]
            );
        }

        // 2. Verify sink depth equals sin(slope_rad) * root_radius_m + soil_cover_m.
        let expected_sink = slope_rad.sin() * (root_radius_m as f64) + DEFAULT_ROOT_SOIL_COVER_M;
        assert!((root.sink_depth_m - expected_sink).abs() < 1e-6);

        // 3. Verify EVERY point on the perimeter of the root plate (downhill, uphill, cross-slope)
        //    has world root_top_y <= local sloped ground_y(x, z) AND <= ground_pos[1].
        let [nx, ny, nz] = root.terrain_normal;
        for step in 0..32 {
            let theta = (step as f64) * std::f64::consts::TAU / 32.0;
            let local_pt = [
                (root_radius_m as f64) * theta.cos(),
                0.0,
                (root_radius_m as f64) * theta.sin(),
            ];
            let offset = rotate_vec_by_quat(root.root_rotation, local_pt);
            let root_top_y = root.root_translation[1] + offset[1];
            // Local terrain tangent plane height at (px + offset[0], pz + offset[2]):
            let local_ground_y =
                (ground_pos[1] as f64) - (nx * offset[0] + nz * offset[2]) / ny;
            assert!(
                root_top_y <= local_ground_y - DEFAULT_ROOT_SOIL_COVER_M + 1e-6,
                "step {step}: root_top_y ({root_top_y}) poked above sloped ground_y ({local_ground_y})"
            );
            assert!(
                root_top_y <= (ground_pos[1] as f64) - DEFAULT_ROOT_SOIL_COVER_M + 1e-6,
                "step {step}: root_top_y ({root_top_y}) exceeded stem base ground_y ({})",
                ground_pos[1]
            );
        }

        // 4. Above-ground trunk remains vertical (geotropic) with only slight natural lean,
        //    independent of the 25-degree slope pitch applied to the subterranean root plate.
        let trunk_state = InstanceState64::new(0xBEEF, 0.9, 1.1, 1.0, [0.0; 4]);
        let trunk = derive_placement_from_state64(ground_pos, &trunk_state);
        let trunk_up = rotate_vec_by_quat(trunk.rotation, [0.0, 1.0, 0.0]);
        assert!(
            trunk_up[1] > 0.99,
            "above-ground trunk must stay nearly vertical (geotropic), got up.y={}",
            trunk_up[1]
        );
    }
}
