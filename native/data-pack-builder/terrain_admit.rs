//! Project-specific packaging using existing QualiaDB native codecs. No header-policy bypass.
use qualia_core_db::container_10d::{Container10dHeader,parse_section_table,SectionInput,AlignmentTier,SectionType,encode_container,SectionTableError,seal_whole_file_crc32c,verify_whole_file_crc32c};
use qualia_core_db::container_10d::provenance_section::{ProvenanceSidecar,encoded_len,encode_provenance_section,decode_provenance_section,validate_provenance};
use qualia_core_db::render::portal_load_10d::decode_load_10d;
use std::{fs,env};

fn main(){
 let a:Vec<String>=env::args().collect(); assert_eq!(a.len(),4,"compiled.10d source-terrain.10d output.10d");
 let mut original=fs::read(&a[1]).unwrap(); verify_whole_file_crc32c(&mut original).unwrap();
 let source_container=fs::read(&a[2]).unwrap(); let source_header=Container10dHeader::parse(&source_container).unwrap(); let source_sections=parse_section_table(&source_container,&source_header).unwrap(); let source_pd=source_sections.iter().find(|d|d.typ()==Some(SectionType::ProvenanceSidecar)).expect("real DEM provenance missing"); let payload=source_container[source_pd.byte_offset as usize..(source_pd.byte_offset+source_pd.byte_length) as usize].to_vec(); let source_view=decode_provenance_section(&payload).unwrap();validate_provenance(&source_view).unwrap();let source=source_view.source_bytes().to_vec(); let h=Container10dHeader::parse(&original).unwrap();let ds=parse_section_table(&original,&h).unwrap();
 assert!(!ds.iter().any(|d|d.typ()==Some(SectionType::ProvenanceSidecar)),"Already attested; original input required");
 // Preserve the original source raster bundle, attribution and licence byte-for-byte.

 let mut inputs:Vec<SectionInput>=ds.iter().map(|d|SectionInput{section_type:d.typ().unwrap(),alignment_tier:d.tier().unwrap(),stride:d.stride,element_count:d.element_count,payload:&original[d.byte_offset as usize..(d.byte_offset+d.byte_length) as usize]}).collect();
 inputs.push(SectionInput{section_type:SectionType::ProvenanceSidecar,alignment_tier:AlignmentTier::CacheLine,stride:0,element_count:0,payload:&payload});
 let needed=match encode_container(&h,&inputs,&mut[]){Err(SectionTableError::OutputBufferTooSmall{needed,..})=>needed,other=>panic!("Unexpected sizing: {:?}",other)};
 let mut out=vec![0;needed];let written=encode_container(&h,&inputs,&mut out).unwrap();out.truncate(written);seal_whole_file_crc32c(&mut out);verify_whole_file_crc32c(&mut out).unwrap();
 let h2=Container10dHeader::parse(&out).unwrap();assert_eq!(h.flags,h2.flags);let ds2=parse_section_table(&out,&h2).unwrap();assert_eq!(ds2.len(),ds.len()+1);
 for old in ds {let new=ds2.iter().find(|d|d.section_type==old.section_type).unwrap();assert_eq!(old.stride,new.stride);assert_eq!(old.element_count,new.element_count);assert_eq!(&original[old.byte_offset as usize..(old.byte_offset+old.byte_length) as usize],&out[new.byte_offset as usize..(new.byte_offset+new.byte_length) as usize]);}
 let pd=ds2.iter().find(|d|d.typ()==Some(SectionType::ProvenanceSidecar)).unwrap();let view=decode_provenance_section(&out[pd.byte_offset as usize..(pd.byte_offset+pd.byte_length) as usize]).unwrap();validate_provenance(&view).unwrap();assert_eq!(view.source_bytes(),source.as_slice());
 let gpu=decode_load_10d(&out).unwrap();assert!(!gpu.governance_refused);assert!(!std::path::Path::new(&a[3]).exists(),"create-only output");fs::write(&a[3],&out).unwrap();
 println!("{{\"passed\":true,\"original_sections_unchanged\":true,\"header_flags_preserved\":true,\"native_governance_refused\":false,\"vertices\":{},\"triangles\":{},\"bytes\":{},\"source_bytes\":{}}}",gpu.vertex_count,gpu.triangle_count,out.len(),source.len());
}

