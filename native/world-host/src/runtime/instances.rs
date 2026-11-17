use super::*;

#[derive(Deserialize)]
pub(super) struct Placement {
    id: String,
    translation: [f64; 3],
    rotation: [f64; 4],
    scale: [f64; 3],
}

pub(super) fn bind(
    portal: &mut QualiaPortal,
    scene: &Scene,
    upload: &Value,
) -> Result<Value, JsValue> {
    bind_parts(portal, scene, upload, false)
}

/// Rebuild instance groups from the resident uploaded set (EWM first-paint).
pub(super) fn bind_resident(
    portal: &mut QualiaPortal,
    scene: &Scene,
    upload: &Value,
) -> Result<Value, JsValue> {
    bind_parts(portal, scene, upload, true)
}

fn bind_parts(
    portal: &mut QualiaPortal,
    scene: &Scene,
    upload: &Value,
    resident: bool,
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
    let count: usize = if resident {
        parts
            .iter()
            .map(|part| {
                part["id"]
                    .as_str()
                    .and_then(|id| scene.assets.iter().find(|a| a.id == id))
                    .map(|a| a.placements.len().max(1))
                    .unwrap_or(1)
            })
            .sum()
    } else {
        scene.assets.iter().map(|a| a.placements.len().max(1)).sum()
    };
    if count > 10_000 {
        return Err(err("Native prototype scene exceeds 10,000 source records"));
    }
    let mut transforms = Vec::with_capacity(count * 16);
    let mut ids = Vec::with_capacity(count * 2);
    let mut groups = Vec::with_capacity(parts.len() * 4);
    let mut semantic_ids = BTreeSet::new();
    let mut first_record = 0u32;
    let mut expanded = 0u64;
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
        let placement_count = asset.map(|a| a.placements.len().max(1)).unwrap_or(1) as u32;
        groups.extend([first_index, index_count, first_record, placement_count]);
        first_record += placement_count;
        expanded += u64::from(index_count / 3) * u64::from(placement_count);
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
            if placement
                .translation
                .iter()
                .chain(placement.scale.iter())
                .any(|v| !v.is_finite())
                || placement.scale.iter().any(|v| v.abs() < 1e-8)
            {
                return Err(err("Invalid authored prototype placement"));
            }
            let rotation = qualia_core_db::audio::qac::spatial_dsp::wigner_rotation::rotation_matrix_from_quaternion_f64(placement.rotation)
                .map_err(|e| err(format!("Invalid native placement quaternion: {e:?}")))?;
            for col in 0..3 {
                for row in 0..3 {
                    transforms.push((rotation[row][col] * placement.scale[col]) as f32);
                }
                transforms.push(0.);
            }
            transforms.extend(placement.translation.map(|n| n as f32));
            transforms.push(1.);
            let semantic = qualia_core_db::mini_parser::hash_token(&format!(
                "urn:webizen:placement:{}",
                placement.id
            ));
            if !semantic_ids.insert(semantic) {
                return Err(err("Duplicate native placement identity"));
            }
            ids.extend([semantic as u32, (semantic >> 32) as u32]);
        }
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
