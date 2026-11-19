//! Hydrate native content-addressed material dependencies before scene upload.
use super::*;
#[derive(Clone, Deserialize)]
pub(crate) struct Texture {
    pub url: String,
    pub sha256: String,
    pub mime_type: String,
    pub role: u8,
    pub cutoff: Option<f32>,
}
pub(super) async fn hydrate(
    portal: &mut QualiaPortal,
    textures: &[Texture],
) -> Result<Value, JsValue> {
    if textures.len() > 128 {
        return Err(err("Native world exceeds 128 texture dependencies"));
    }
    let mut total = 0usize;
    let mut seen = BTreeSet::new();
    let mut receipts = Vec::new();
    for texture in textures {
        if texture.role > 3
            || !matches!(
                texture.mime_type.as_str(),
                "image/png" | "image/jpeg" | "image/ktx2"
            )
        {
            return Err(err("Unsupported native texture role or MIME type"));
        }
        let cutoff = texture.cutoff.unwrap_or(0.5);
        if !cutoff.is_finite() || !(0.0..=1.0).contains(&cutoff) {
            return Err(err("Invalid texture alpha cutoff"));
        }
        if !seen.insert((texture.sha256.clone(), texture.role, cutoff.to_bits())) {
            return Err(err("Duplicate native texture dependency"));
        }
        text(
            "native-world-status",
            &format!(
                "Opening native material maps — {} of {}",
                receipts.len() + 1,
                textures.len()
            ),
        );
        let encoded = bytes(&texture.url, 4 * 1024 * 1024).await?;
        total += encoded.len();
        if total > 128 * 1024 * 1024 {
            return Err(err("Native texture dependencies exceed 128 MiB"));
        }
        let digest: [u8; 32] = Sha256::digest(&encoded).into();
        if hash(&encoded) != texture.sha256 {
            return Err(err(format!("Texture digest mismatch: {}", texture.url)));
        }
        let uploaded = portal.load_texture_encoded(
            &digest,
            &texture.mime_type,
            &encoded,
            texture.role,
            cutoff,
        )?;
        if !uploaded {
            return Err(err(format!(
                "Native material texture could not be admitted: {}",
                texture.url
            )));
        }
        receipts.push(json!({"url":texture.url,"sha256":texture.sha256,"mime_type":texture.mime_type,"role":texture.role,"cutoff":cutoff,"encoded_bytes":encoded.len(),"native_resident":uploaded}));
    }
    Ok(
        json!({"loader":"QualiaPortal::load_texture_encoded","count":receipts.len(),"encoded_bytes":total,"dependencies":receipts}),
    )
}

/// Refuse silent neutral-map substitution using the actual native MAT1 and UV01 readers.
pub(super) fn verify_asset(
    id: &str,
    bytes: &[u8],
    textures: &[Texture],
    registry: &mut Vec<(String, qualia_core_db::container_10d::MaterialRecord)>,
) -> Result<usize, JsValue> {
    use qualia_core_db::container_10d as c;
    let (nm, nr) = match c::material_container_counts(bytes) {
        Ok(v) => v,
        Err(c::MaterialContainerError::Missing) => return Ok(0),
        Err(e) => return Err(err(format!("Asset {id} materials: {e:?}"))),
    };
    if nm > 4096 || nr > 4096 {
        return Err(err("Native material table exceeds host budget"));
    }
    let h = c::Container10dHeader::parse(bytes).map_err(err)?;
    let ds = c::parse_section_table(bytes, &h).map_err(err)?;
    let md = ds
        .iter()
        .find(|d| d.typ() == Some(c::SectionType::QuantizedMesh))
        .ok_or(err("Missing mesh section"))?;
    let start = md.byte_offset as usize;
    let end = start
        .checked_add(md.byte_length as usize)
        .filter(|e| *e <= bytes.len())
        .ok_or(err("Invalid mesh section bounds"))?;
    let (mesh, _) = c::mesh_section::parse_mesh_header(&bytes[start..end])
        .map_err(|e| err(format!("Mesh header: {e:?}")))?;
    let mut materials = vec![c::MaterialRecord::legacy_default(); nm];
    let mut ranges = vec![
        c::SubmeshRange {
            first_index: 0,
            index_count: 0,
            material_id: 0,
            semantic_id: 0
        };
        nr
    ];
    c::read_material_section_into(
        bytes,
        mesh.triangle_count
            .checked_mul(3)
            .ok_or(err("Mesh index overflow"))?,
        &mut materials,
        &mut ranges,
    )
    .map_err(|e| err(format!("Asset {id} MAT1: {e:?}")))?;
    for material in &materials {
        // QualiaPortal loads each organ from its own bytes. Cross-asset MAT1
        // integer ids are not a GPU merge: measured-gis DEM and authored
        // woodland both use producer id 1 with different records. Conflict
        // only inside one scene_id.
        if let Some((_, prior)) = registry
            .iter()
            .find(|(owner, prior)| owner == id && prior.id == material.id)
        {
            if prior != material {
                return Err(err(format!("Asset {id}: native material ID {} has different MAT1 records; reseal unique producer IDs before loading", material.id)));
            }
        } else {
            registry.push((id.to_string(), *material));
        }
    }
    let used: BTreeSet<_> = ranges.iter().map(|r| r.material_id).collect();
    let mut referenced = 0;
    for m in materials.iter().filter(|m| used.contains(&m.id)) {
        let base_role = if m.opacity_mode == c::OpacityMode::Mask {
            3
        } else {
            0
        };
        for (digest, role) in [
            (m.base_color_texture, base_role),
            (m.normal_texture, 1),
            (m.metallic_roughness_texture, 2),
            (m.occlusion_texture, 2),
            (m.emissive_texture, 0),
            (m.stylized_ramp_texture, 0),
        ] {
            if digest == [0; 32] {
                continue;
            }
            let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
            if !textures.iter().any(|t| {
                t.sha256 == hex
                    && t.role == role
                    && (role != 3 || (t.cutoff.unwrap_or(0.5) - m.alpha_cutoff).abs() < 1e-6)
            }) {
                return Err(err(format!(
                    "Asset {id} material {} has no hydrated dependency for role {role}: {hex}",
                    m.id
                )));
            }
            referenced += 1;
        }
    }
    if referenced > 0 {
        if mesh.vertex_count > 1_048_576 {
            return Err(err("Textured asset exceeds tangent workspace budget"));
        }
        let mut uv = vec![[0.; 2]; mesh.vertex_count as usize];
        c::read_texture_coordinate_section_into(bytes, mesh.vertex_count as usize, &mut uv)
            .map_err(|e| {
                err(format!(
                    "Asset {id} requires valid vertex-aligned UV01: {e:?}"
                ))
            })?;
    }
    Ok(referenced)
}

fn detect_texture_mime(encoded: &[u8]) -> Option<&'static str> {
    if encoded.len() >= 8 && &encoded[..8] == b"\x89PNG\r\n\x1a\n" {
        Some("image/png")
    } else if encoded.len() >= 3 && encoded[..3] == [0xff, 0xd8, 0xff] {
        Some("image/jpeg")
    } else if encoded.len() >= 12 && &encoded[..12] == b"\xabKTX 20\xbb\r\n\x1a\n" {
        Some("image/ktx2")
    } else {
        None
    }
}

/// Extract all non-zero content-addressed PBR texture references `(sha256, role, cutoff)`
/// declared in a `.10d` container's `MAT1` section.
pub(super) fn collect_material_texture_refs(bytes: &[u8]) -> Vec<([u8; 32], u8, f32)> {
    use qualia_core_db::container_10d as c;
    let Ok((nm, nr)) = c::material_container_counts(bytes) else {
        return Vec::new();
    };
    if nm == 0 || nm > 4096 || nr > 4096 {
        return Vec::new();
    }
    let Ok(h) = c::Container10dHeader::parse(bytes) else {
        return Vec::new();
    };
    let Ok(ds) = c::parse_section_table(bytes, &h) else {
        return Vec::new();
    };
    let Some(md) = ds
        .iter()
        .find(|d| d.typ() == Some(c::SectionType::QuantizedMesh))
    else {
        return Vec::new();
    };
    let start = md.byte_offset as usize;
    let Some(end) = start
        .checked_add(md.byte_length as usize)
        .filter(|e| *e <= bytes.len())
    else {
        return Vec::new();
    };
    let Ok((mesh, _)) = c::mesh_section::parse_mesh_header(&bytes[start..end]) else {
        return Vec::new();
    };
    let Some(index_count) = mesh.triangle_count.checked_mul(3) else {
        return Vec::new();
    };
    let mut materials = vec![c::MaterialRecord::legacy_default(); nm];
    let mut ranges = vec![
        c::SubmeshRange {
            first_index: 0,
            index_count: 0,
            material_id: 0,
            semantic_id: 0,
        };
        nr
    ];
    if c::read_material_section_into(bytes, index_count, &mut materials, &mut ranges).is_err() {
        return Vec::new();
    }
    let used: BTreeSet<_> = ranges.iter().map(|r| r.material_id).collect();
    let mut out = Vec::new();
    let mut seen: BTreeSet<([u8; 32], u8)> = BTreeSet::new();
    for m in materials.iter().filter(|m| used.contains(&m.id)) {
        let base_role = if m.opacity_mode == c::OpacityMode::Mask {
            3
        } else {
            0
        };
        let cutoff = if m.alpha_cutoff.is_finite() && (0.0..=1.0).contains(&m.alpha_cutoff) {
            m.alpha_cutoff
        } else {
            0.5
        };
        for (digest, role) in [
            (m.base_color_texture, base_role),
            (m.normal_texture, 1),
            (m.metallic_roughness_texture, 2),
            (m.occlusion_texture, 2),
            (m.emissive_texture, 0),
            (m.stylized_ramp_texture, 0),
        ] {
            if digest == [0u8; 32] {
                continue;
            }
            if seen.insert((digest, role)) {
                out.push((digest, role, cutoff));
            }
        }
    }
    out
}

/// Fetch unhydrated content-addressed PBR textures referenced by streamed catchment `.10d`
/// assets (`catchment-600m-lod0`, `catchment-600m-lod1`, `catchment-600m-water`) from
/// `/production/terrain-v2/textures/sha256/<hex>` (with named PNG fallback) and verify SHA-256.
pub(super) async fn fetch_catchment_textures(
    refs: &[([u8; 32], u8, f32)],
    already_loaded: &BTreeSet<([u8; 32], u8)>,
) -> Vec<([u8; 32], &'static str, u8, f32, Vec<u8>)> {
    let mut out = Vec::new();
    let mut seen_this_batch: BTreeSet<([u8; 32], u8)> = BTreeSet::new();
    for &(digest, role, cutoff) in refs {
        if already_loaded.contains(&(digest, role)) || !seen_this_batch.insert((digest, role)) {
            continue;
        }
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let mut candidates = vec![format!("/production/terrain-v2/textures/sha256/{hex}")];
        match hex.as_str() {
            "cbfbdc9f89885b2390146928156051a8d1262f89218f100f7f422e329bf016b0" => {
                candidates.push("/production/terrain-v2/terrain-albedo.png".into());
            }
            "46fdc63a6bcabd92f1866ba468aa4dd823fadb75a07e9c662fd5caa897460e4d" => {
                candidates.push("/production/terrain-v2/terrain-normal.png".into());
            }
            "76f910055e78df16e951bff7f51bffee7906e21783c547f4e2e618572392f5ba" => {
                candidates.push("/production/terrain-v2/terrain-orm.png".into());
            }
            "7e878f0e92d3170e8b772ebc1a1c154cfc08ebb05ad46fafa09dc4ca541667e1" => {
                candidates.push("/production/terrain-v2/textures/water-albedo.png".into());
            }
            "0d401a9d9964c6f46d70708bb77b88243c52568a39be5b525f8b37a311b7df5d" => {
                candidates.push("/production/terrain-v2/textures/water-normal.png".into());
            }
            "0c5a7cb018dc82971a19d7e2d530d9626491770e8e273879f8854d155877baca" => {
                candidates.push("/production/terrain-v2/textures/water-orm.png".into());
            }
            _ => {}
        }
        for url in candidates {
            if let Ok(encoded) = bytes(&url, 8 * 1024 * 1024).await {
                let actual: [u8; 32] = Sha256::digest(&encoded).into();
                if actual == digest {
                    if let Some(mime) = detect_texture_mime(&encoded) {
                        out.push((digest, mime, role, cutoff, encoded));
                        break;
                    }
                }
            }
        }
    }
    out
}
