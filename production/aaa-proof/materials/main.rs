use qualia_core_db::container_10d as c;
use qualia_core_db::bundle::cbor_ld_manifest::CborLdManifest;
use qualia_core_db::render::portal_load_10d::decode_load_10d;
use serde_json::{json,Value};
use std::{collections::BTreeMap,fs,io::Write,path::Path};
fn digest(bytes:&[u8])->String { CborLdManifest::compute_sha256_digest(bytes).strip_prefix("sha256-").unwrap().to_string() }
fn canonical(material:&c::MaterialRecord)->Result<(u64,String,Vec<u8>),String>{
 let mut m=*material;m.id=1;
 let range=c::SubmeshRange{first_index:0,index_count:3,material_id:1,semantic_id:0};
 let mut data=vec![0;c::material_section::encoded_len(1,1).ok_or("Canonical material size")?];
 let n=c::encode_material_section(&[m],&[range],3,&mut data).map_err(|e|e.to_string())?;data.truncate(n);
 let hash=digest(&data);let id=u64::from_str_radix(&hash[..16],16).map_err(|e|e.to_string())? | (1u64<<63);
 Ok((id,hash,data))
}
fn insert_identity(reg:&mut BTreeMap<u64,Vec<u8>>,id:u64,bytes:&[u8])->Result<(),String>{
 if let Some(old)=reg.get(&id){if old!=bytes{return Err(format!("Material identity collision: {id:016x}"))}}
 else{reg.insert(id,bytes.to_vec());} Ok(())
}
fn namespace(original:&[u8])->Result<(Vec<u8>,Value),String>{
 c::integrity::check_whole_file_crc32c(original).map_err(|e|e.to_string())?;
 let header=c::Container10dHeader::parse(original).map_err(|e|e.to_string())?;
 let descriptors=c::parse_section_table(original,&header).map_err(|e|e.to_string())?;
 let before=decode_load_10d(original)?;if before.governance_refused{return Err("Input governance refused".into())}
 let (nm,nr)=c::material_container_counts(original).map_err(|e|format!("Materials: {e:?}"))?;
 if nm>4096 || nr>4096{return Err("Material namespace profile exceeded".into())}
 let mut materials=vec![c::MaterialRecord::legacy_default();nm];
 let mut ranges=vec![c::SubmeshRange{first_index:0,index_count:0,material_id:0,semantic_id:0};nr];
 let indices=before.triangle_count.checked_mul(3).ok_or("Index overflow")?;
 c::read_material_section_into(original,indices,&mut materials,&mut ranges).map_err(|e|format!("Material read: {e:?}"))?;
 let original_ranges=ranges.clone();let mut mapping=BTreeMap::new();let mut identities=BTreeMap::new();let mut unique=BTreeMap::new();let mut receipt=Vec::new();
 for m in &materials {
  let (id,hash,encoded)=canonical(m)?;insert_identity(&mut identities,id,&encoded)?;
  if mapping.insert(m.id,id).is_some(){return Err("Duplicate source material ID".into())}
  let mut new=*m;new.id=id;unique.insert(id,new);
  receipt.push(json!({"old_id":format!("{:016x}",m.id),"new_id":format!("{id:016x}"),"canonical_sha256":hash,"canonical_hex":encoded.iter().map(|b|format!("{b:02x}")).collect::<String>()}));
 }
 for range in &mut ranges {range.material_id=*mapping.get(&range.material_id).ok_or("Unbound material range")?;}
 let namespaced:Vec<_>=unique.into_values().collect();
 let mut payload=vec![0;c::material_section::encoded_len(namespaced.len(),ranges.len()).ok_or("Material section size")?];
 let n=c::encode_material_section(&namespaced,&ranges,indices,&mut payload).map_err(|e|e.to_string())?;payload.truncate(n);
 let inputs:Vec<c::SectionInput>=descriptors.iter().map(|d|c::SectionInput{section_type:d.typ().unwrap(),alignment_tier:d.tier().unwrap(),stride:d.stride,element_count:d.element_count,payload:if d.typ()==Some(c::SectionType::Materials){&payload}else{&original[d.byte_offset as usize..(d.byte_offset+d.byte_length) as usize]}}).collect();
 let needed=match c::encode_container(&header,&inputs,&mut[]){Err(c::SectionTableError::OutputBufferTooSmall{needed,..})=>needed,other=>return Err(format!("Container sizing: {other:?}"))};
 let mut output=vec![0;needed];let written=c::encode_container(&header,&inputs,&mut output).map_err(|e|e.to_string())?;output.truncate(written);c::seal_whole_file_crc32c(&mut output);c::integrity::check_whole_file_crc32c(&output).map_err(|e|e.to_string())?;
 let after_header=c::Container10dHeader::parse(&output).map_err(|e|e.to_string())?;if header.flags!=after_header.flags{return Err("Policy flags changed".into())}
 let after_desc=c::parse_section_table(&output,&after_header).map_err(|e|e.to_string())?;if descriptors.len()!=after_desc.len(){return Err("Section count changed".into())}
 let mut preserved=Vec::new();
 for old in descriptors {
  let new=after_desc.iter().find(|d|d.section_type==old.section_type).ok_or("Section lost")?;
  if old.stride!=new.stride || old.element_count!=new.element_count{return Err("Section layout changed".into())}
  if old.typ()!=Some(c::SectionType::Materials){let left=&original[old.byte_offset as usize..(old.byte_offset+old.byte_length) as usize];let right=&output[new.byte_offset as usize..(new.byte_offset+new.byte_length) as usize];if left!=right{return Err("Non-material payload changed".into())}preserved.push(json!({"section_type":old.section_type,"sha256":digest(left),"bytes":left.len()}));}
 }
 let after=decode_load_10d(&output)?;
 if after.governance_refused || before.vertex_count!=after.vertex_count || before.triangle_count!=after.triangle_count || before.positions!=after.positions || before.indices!=after.indices || before.uv0!=after.uv0 || before.normals!=after.normals || before.tangents!=after.tangents || before.colors!=after.colors {return Err("Native decoded geometry or admission changed".into())}
 if after.ranges.len()!=original_ranges.len(){return Err("Range count changed".into())}
 for(old,new)in original_ranges.iter().zip(&after.ranges){if old.first_index!=new.first_index||old.index_count!=new.index_count||old.semantic_id!=new.semantic_id||mapping[&old.material_id]!=new.material_id{return Err("Material range mapping changed".into())}}
 for old in materials {let id=mapping[&old.id];let new=after.materials.iter().find(|m|m.id==id).ok_or("Material missing")?;if canonical(&old)?.2!=canonical(new)?.2{return Err("Material properties changed".into())}}
 let report=json!({"schema":"webizen.native-material-namespace/v1","passed":true,"method":"Native canonical MAT4 record with id1 and dummy range(0,3,1,0), SHA256 first64 bits OR highbit; remap ranges; canonical-identical material dedup","source_sha256":digest(original),"sha256":digest(&output),"bytes":output.len(),"vertices":after.vertex_count,"triangles":after.triangle_count,"source_material_count":nm,"material_count":after.materials.len(),"ranges":after.ranges.len(),"materials":receipt,"header_flags_preserved":true,"non_material_sections_byte_exact":true,"preserved_sections":preserved,"native_decoded_geometry_equal":true,"native_governance_refused":false});
 Ok((output,report))
}
fn run()->Result<(),String>{let args:Vec<String>=std::env::args().collect();if args.len()!=3{return Err("Usage: material-namespace.exe INPUT.10d OUTPUT.10d".into())}let input=Path::new(&args[1]);let output=Path::new(&args[2]);if input==output{return Err("Original must remain immutable".into())}if fs::metadata(input).map_err(|e|e.to_string())?.len()>128*1024*1024{return Err("Input exceeds128MiB".into())}let original=fs::read(input).map_err(|e|e.to_string())?;let(bytes,receipt)=namespace(&original)?;let mut file=fs::OpenOptions::new().write(true).create_new(true).open(output).map_err(|e|e.to_string())?;file.write_all(&bytes).map_err(|e|e.to_string())?;println!("{}",receipt);Ok(())}
fn main(){if let Err(e)=run(){eprintln!("{}",e);std::process::exit(1)}}
#[cfg(test)]mod tests {use super::*;
 #[test]fn same_content_different_old_ids_has_same_identity(){let a=c::MaterialRecord::legacy_default();let mut b=a;b.id=27;assert_eq!(canonical(&a).unwrap(),canonical(&b).unwrap())}
 #[test]fn different_content_same_old_id_has_distinct_identity(){let a=c::MaterialRecord::legacy_default();let mut b=a;b.roughness=0.123;assert_ne!(canonical(&a).unwrap().0,canonical(&b).unwrap().0);assert_ne!(canonical(&a).unwrap().2,canonical(&b).unwrap().2)}
 #[test]fn actual_collision_refuses(){let mut reg=BTreeMap::new();insert_identity(&mut reg,1,b"first").unwrap();assert!(insert_identity(&mut reg,1,b"different").is_err());}
 #[test]fn corrupt_container_refuses(){assert!(namespace(b"invalid").is_err());}
}

