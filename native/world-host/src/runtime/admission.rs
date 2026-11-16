use super::*;
/// Diagnose the existing native admission gate; never alter its flags or payload.
pub(super) fn check(id: &str, bytes: &[u8]) -> Result<(), JsValue> {
    use qualia_core_db::container_10d as codec;
    let header =
        codec::Container10dHeader::parse(bytes).map_err(|e| err(format!("Asset {id}: {e}")))?;
    codec::integrity::check_whole_file_crc32c(bytes)
        .map_err(|e| err(format!("Asset {id}: {e}")))?;
    let sections =
        codec::parse_section_table(bytes, &header).map_err(|e| err(format!("Asset {id}: {e}")))?;
    if header.flags & codec::FLAG_DEFAULT_DISPOSITION_REFUSE == 0 {
        return Ok(());
    }
    let descriptor = sections
        .iter()
        .find(|d| d.typ() == Some(codec::SectionType::ProvenanceSidecar))
        .ok_or_else(|| {
            err(format!(
                "Asset {id} requires embedded source provenance for native scene admission"
            ))
        })?;
    let begin = descriptor.byte_offset as usize;
    let end = begin
        .checked_add(descriptor.byte_length as usize)
        .filter(|end| *end <= bytes.len())
        .ok_or(err("Invalid provenance section bounds"))?;
    let provenance = codec::decode_provenance_section(&bytes[begin..end])
        .map_err(|e| err(format!("Asset {id} provenance: {e}")))?;
    codec::validate_provenance(&provenance)
        .map_err(|e| err(format!("Asset {id} provenance: {e}")))?;
    Ok(())
}
