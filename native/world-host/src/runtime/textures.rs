//! Hydrate native content-addressed material dependencies before scene upload.
use super::*;
#[derive(Deserialize)]
pub(super) struct Texture {
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
        if let Some((owner, prior)) = registry.iter().find(|(_, prior)| prior.id == material.id) {
            if prior != material {
                return Err(err(format!("Asset {id} conflicts with asset {owner}: native material ID {} has different MAT1 records; reseal unique producer IDs before loading", material.id)));
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
