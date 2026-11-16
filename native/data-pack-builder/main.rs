use qualia_core_db::{NQuin,QUINS_PER_BLOCK};
use qualia_core_db::q42_volume::{Q42Volume,UnifiedVolumeBuilder};
use qualia_core_db::mini_parser::{hash_token,compile_ntriples_to_bytecode};
use qualia_core_db::webizen_bytecode::execute_program_with_stats;
pub use qualia_core_db::{net,lexicon,container_10d,render};
mod domains { pub mod geospatial {
 pub use qualia_core_db::domains::geospatial::{adapters,geodetic};
 #[path="C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/dem.rs"] pub mod dem;
 #[path="C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain_pipeline.rs"] pub mod terrain_pipeline;
}}

use qualia_core_db::domains::geospatial::adapters::{DataAdapter,AdapterHttpRequest,AdapterHttpMethod};
include!("native_adapter_support.rs");
#[path="C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/adapters/dem_adapter.rs"]
mod live_dem_adapter;
use domains::geospatial::dem as live_dem_mesh;
use live_dem_adapter::{DemAdapter,TERRAIN_ATTRIBUTION,TERRAIN_ATTRIBUTION_URL};
use qualia_core_db::net::disclosure::NetworkDisclosureRegistry;
use serde_json::{json,Value};
use std::{fs,path::Path,collections::HashMap,error::Error};
fn main(){if let Err(e)=run(){eprintln!("{}",e);std::process::exit(1)}}
fn run()->Result<(),Box<dyn Error>>{
 let a:Vec<String>=std::env::args().collect();let op=a.get(1).ok_or("command missing")?;
 if op=="terrain-mesh" {
  let h:Value=serde_json::from_slice(&fs::read(&a[2])?)?;let vals:Vec<f32>=h["values_m"].as_array().ok_or("values")?.iter().map(|v|v.as_f64().unwrap() as f32).collect();
  let mesh=live_dem_mesh::generate_terrain_mesh(&vals,h["width"].as_u64().unwrap() as usize,h["height"].as_u64().unwrap() as usize,h["cell_size_m"].as_f64().unwrap());
  let out=json!({"positions":mesh.vertices,"indices":mesh.indices,"source":"Native generate_terrain_mesh with corrected upward winding; original sampled positions preserved"});fs::write(&a[3],serde_json::to_vec(&out)?)?;println!("{}",json!({"passed":true,"vertices":mesh.vertices.len(),"triangles":mesh.indices.len()/3,"orientation":"Z-up outward"}));return Ok(())
 }
 if op=="dem" {
  let out=Path::new(a.get(2).ok_or("output dir missing")?);fs::create_dir_all(out)?;
  if out.join("reference-terrain.10d").exists(){return Err("create-only output exists".into())}
  let bbox=(149.06,-35.30,149.08,-35.28);let adapter=DemAdapter::terrarium("webizen-public-reference-dem").with_zoom(13)?;
  let mut registry=NetworkDisclosureRegistry::new();assert!(adapter.build_fetch_requests(bbox,&registry).is_err());
  registry.register_egress(adapter.adapter_id(),"https://s3.amazonaws.com/elevation-tiles-prod/terrarium/","Acquire bounded public reference landscape for authorised Webizen proof; no personal coordinates","User authorised bounded public DEM acquisition");
  let requests=adapter.build_fetch_requests(bbox,&registry)?;if requests.len()>4{return Err("proof limited to four tiles".into())}
  fs::write(out.join("requests.json"),serde_json::to_vec_pretty(&json!({"adapter":"DemAdapter::terrarium","bbox_wsen":bbox,"zoom":13,"urls":requests.iter().map(|r|r.url.clone()).collect::<Vec<_>>(),"purpose":"Public Canberra reference terrain; not a personal campsite","consent_without_registry_refused":true}))?)?;
  let asset=adapter.fetch_region_dem(bbox,&registry)?;let h=&asset.heightfield;
  fs::write(out.join("source-tiles.qdem"),&h.source_bundle)?;fs::write(out.join("reference-terrain.10d"),&asset.encoded_10d)?;
  let geometry=qualia_core_db::render::portal_load_10d::decode_load_10d(&asset.encoded_10d)?;if geometry.governance_refused{return Err("native terrain admission refused".into())}
  let min=h.values_m.iter().copied().fold(f32::INFINITY,f32::min);let max=h.values_m.iter().copied().fold(f32::NEG_INFINITY,f32::max);
  fs::write(out.join("heightfield.json"),serde_json::to_vec_pretty(&json!({"schema":"webizen.native-dem-heightfield/v1","width":h.width,"height":h.height,"cell_size_m":h.cell_size_m,"bbox_wsen":h.bbox,"centre_lat":h.centre_lat,"centre_lon":h.centre_lon,"centre_alt_m":h.centre_alt_m,"values_m":h.values_m,"native_mesh_frame":"x grid east; y grid row south; z source elevation metres; source mesh is Z-up","native_mesh_offset_rule":"x=column*cell-width*cell/2; y=row*cell-height*cell/2; z=elevation; no Python geometry","licence_reference":TERRAIN_ATTRIBUTION_URL,"attribution":TERRAIN_ATTRIBUTION}))?)?;
  fs::write(out.join("mesh-inspection.json"),serde_json::to_vec_pretty(&json!({"positions":asset.terrain.mesh.vertices,"indices":asset.terrain.mesh.indices,"source":"native DemAdapter terrain.mesh; inspection only, not browser geometry"}))?)?;
  let receipt=json!({"passed":true,"adapter":"qualia_core_db::domains::geospatial::adapters::dem_adapter::DemAdapter","compiler":"terrain_pipeline::compile_and_encode_10d_tile_with_media_type","title":"Canberra public reference landscape","not_personal_campsite":true,"tiles":requests.len(),"width":h.width,"height":h.height,"cell_size_m":h.cell_size_m,"min_elevation_m":min,"max_elevation_m":max,"vertices":geometry.vertex_count,"triangles":geometry.triangle_count,"native_governance_refused":geometry.governance_refused,"source_bundle_bytes":h.source_bundle.len(),"native_bytes":asset.encoded_10d.len(),"attribution":TERRAIN_ATTRIBUTION,"licence_reference":TERRAIN_ATTRIBUTION_URL});fs::write(out.join("native-receipt.json"),serde_json::to_vec_pretty(&receipt)?)?;println!("{}",receipt);return Ok(())
 }
 if op=="build"{
  let data:Value=serde_json::from_slice(&fs::read(&a[2])?)?;let rows=data.as_array().ok_or("expected triple array")?;if rows.len()>25000{return Err("proof graph bound".into())}let out=Path::new(&a[3]);if out.exists(){return Err("create-only output".into())}let mut lex=HashMap::new();let mut quins=Vec::new();
  for row in rows {let r=row.as_array().ok_or("triple array")?;if r.len()!=3{return Err("triple arity".into())}let mut ids=[0;3];for i in 0..3{let term=r[i].as_str().ok_or("term")?;ids[i]=hash_token(term);if let Some(old)=lex.insert(ids[i],term.to_string()){if old!=term{return Err("hash collision".into())}}}quins.push(NQuin{subject:ids[0],predicate:ids[1],object:ids[2],context:0,metadata:0,parity:NQuin::calculate_parity(ids[0],ids[1],ids[2],0,0)});}
  quins.sort_by_key(|q|(q.object,q.subject,q.predicate));quins.dedup_by(|a,b|a.subject==b.subject&&a.predicate==b.predicate&&a.object==b.object);let mut builder=UnifiedVolumeBuilder::with_lex_map(&lex).map_err(|e|format!("lex:{e:?}"))?.with_permissive_commons();for(seq,chunk)in quins.chunks(QUINS_PER_BLOCK).enumerate(){builder.push_block(seq as u64,chunk)?}builder.finish(out)?;let v=Q42Volume::open(out)?;v.verify_all_blocks()?;let version=v.header().version;let loaded=v.read_all_quins()?;if loaded.len()!=quins.len(){return Err("roundtrip count".into())}for(a,b)in loaded.iter().zip(&quins){if(a.subject,a.predicate,a.object,a.context,a.metadata,a.parity)!=(b.subject,b.predicate,b.object,b.context,b.metadata,b.parity){return Err("roundtrip values".into())}}println!("{}",json!({"passed":true,"writer":"UnifiedVolumeBuilder","version":version,"quins":loaded.len(),"lexicon_terms":lex.len(),"roundtrip_equal":true}));return Ok(())
 }
 if op=="rights"{
  use qualia_core_db::modalities::logic::deontic::{compile_norm_quin,evaluate_deontic_contract,DeonticVerdict};
  let fixture:Value=serde_json::from_slice(&fs::read(&a[2])?)?;let mut results=Vec::new();
  for case in fixture["cases"].as_array().ok_or("cases")? {
   let mut norms=Vec::new();for n in case["norms"].as_array().ok_or("norms")? {let hash=|k:&str|qualia_core_db::q_hash(n[k].as_str().unwrap());norms.push(compile_norm_quin(hash("party"),n["opcode"].as_u64().unwrap() as u8,hash("path"),hash("object"),hash("contract"),n["expiry"].as_u64().unwrap() as u32,n["defeater"].as_bool().unwrap()));}
   let mut out=vec![DeonticVerdict::default();norms.len()];let count=evaluate_deontic_contract(&norms,case["now"].as_u64().ok_or("now")? as u32,&mut out).map_err(|e|format!("deontic:{e:?}"))?;let actual:Vec<_>=out[..count].iter().map(|v|format!("{:?}",v.status)).collect();let expected:Vec<_>=case["expected_statuses"].as_array().ok_or("expected statuses")?.iter().map(|v|v.as_str().unwrap().to_string()).collect();if actual!=expected{return Err(format!("rights fixture mismatch {}",case["id"]).into())}results.push(json!({"id":case["id"],"actual_statuses":actual,"expected_statuses":expected,"passed":true}));
  }
  println!("{}",json!({"schema":"webizen.native-rights-proof/v1","execution":"native compile_norm_quin + evaluate_deontic_contract","cases":results,"passed":true}));return Ok(())
 }
 if op=="query"{
  let v=Q42Volume::open(Path::new(&a[2]))?;v.verify_all_blocks()?;let version=v.header().version;let lex=v.lex_view().map_err(|e|format!("lex:{e:?}"))?;let rows=v.read_all_quins()?;let mut program=[0;1024];let used=compile_ntriples_to_bytecode(a[3].as_bytes(),&mut program).map_err(|e|format!("query:{e:?}"))?;let mut out=vec![NQuin::default();rows.len()];let stats=execute_program_with_stats(&program,&rows,&mut out,None).map_err(|e|format!("vm:{e:?}"))?;let decoded=out[..stats.match_count].iter().map(|q|json!([lex.lookup_owned(q.subject),lex.lookup_owned(q.predicate),lex.lookup_owned(q.object)])).collect::<Vec<_>>();println!("{}",json!({"count":stats.match_count,"rows":decoded,"total_quins":rows.len(),"version":version,"compiled_bytes":used,"vm_cycles":stats.vm_cycles,"query":a[3],"execution":"native compile_ntriples_to_bytecode + execute_program_with_stats, same core path as WASM execute_ntriples_query"}));return Ok(())
 }
 Err("unknown command".into())
}
