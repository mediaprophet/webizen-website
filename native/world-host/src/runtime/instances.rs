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

// Slope-disc root placement is retired. `wind::fit_tree` fits each lofted
// joint with `ground::fit_soil_joint`. The frame loop does not evaluate Vibe.

#[cfg(any(target_arch = "wasm32", test))]
#[path = "wind.rs"]
mod wind;

#[cfg(any(target_arch = "wasm32", test))]
pub use wind::{
    contour_demonstration_site, fit_tree, pose_wind, wind_and_root_mechanics, ContourSite, DrawView,
    FittedJoint, FittedTree, GumSeed, WindMechanics, WoodySpecies,
};

// Native `rustc --test` of this file, and of `gis_join.rs` which includes it,
// has no `runtime::ground` parent. The wasm crate uses `runtime::ground`.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "ground.rs"]
mod ground;

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
}
