extern crate qualia_core_db;
extern crate serde_json;
use qualia_core_db::{NQuin, QUINS_PER_BLOCK};
use qualia_core_db::q42_volume::{UnifiedVolumeBuilder,Q42Volume};
use qualia_core_db::mini_parser::hash_token;
use serde_json::{Value,json};
use std::{collections::HashMap,path::Path,error::Error};
fn main(){if let Err(e)=run(){eprintln!("{}",json!({"error":e.to_string()}));std::process::exit(1)}}
fn run()->Result<(),Box<dyn Error>>{
 let args:Vec<String>=std::env::args().collect();
 if args.len()<3{return Err("usage: q42_web build triples.json output.q42 | inspect volume.q42 | query volume.q42 predicate [subject] [object]".into())}
 if args[1]=="build"{
  if args.len()!=4{return Err("build needs input and new output".into())}
  if Path::new(&args[3]).exists(){return Err("output is create-only".into())}
  let data:Value=serde_json::from_slice(&std::fs::read(&args[2])?)?;
  let rows=data.as_array().ok_or("expected triple array")?;
  if rows.is_empty()||rows.len()>100000{return Err("triple count outside bounded publication limit".into())}
  let mut lex=HashMap::new();let mut quins=Vec::new();
  for row in rows{
   let row=row.as_array().ok_or("expected triple")?;if row.len()!=3{return Err("expected three terms".into())}
   let mut ids=[0;3];
   for i in 0..3{let term=row[i].as_str().ok_or("term must be text")?;if term.len()>16384{return Err("term too large".into())};ids[i]=hash_token(term);if let Some(old)=lex.insert(ids[i],term.to_string()){if old!=term{return Err("term hash collision".into())}}}
   quins.push(NQuin{subject:ids[0],predicate:ids[1],object:ids[2],context:0,metadata:0,parity:NQuin::calculate_parity(ids[0],ids[1],ids[2],0,0)});
  }
  quins.sort_by_key(|q|(q.object,q.subject,q.predicate));quins.dedup_by(|a,b|a.subject==b.subject&&a.predicate==b.predicate&&a.object==b.object);
  let mut builder=UnifiedVolumeBuilder::with_lex_map(&lex).map_err(|e|format!("lexicon: {e:?}"))?.with_permissive_commons();
  for(seq,chunk)in quins.chunks(QUINS_PER_BLOCK).enumerate(){builder.push_block(seq as u64,chunk)?;}
  builder.finish(Path::new(&args[3]))?;
  let volume=Q42Volume::open(Path::new(&args[3]))?;volume.verify_all_blocks()?;let loaded=volume.read_all_quins()?;
  if loaded.len()!=quins.len(){return Err("round-trip count mismatch".into())}
  for(a,b)in loaded.iter().zip(&quins){if (a.subject,a.predicate,a.object,a.context,a.metadata,a.parity)!=(b.subject,b.predicate,b.object,b.context,b.metadata,b.parity){return Err("round-trip value mismatch".into())}}
  let version=volume.header().version;
  println!("{}",json!({"writer":"qualia_core_db::q42_volume::UnifiedVolumeBuilder","reader":"qualia_core_db::q42_volume::Q42Volume","version":version,"quins":loaded.len(),"lexicon_terms":lex.len(),"verified_blocks":true,"roundtrip_equal":true}));return Ok(())
 }
 if args[1]!="inspect"&&args[1]!="query"{return Err("unknown command".into())}
 let volume=Q42Volume::open(Path::new(&args[2]))?;volume.verify_all_blocks()?;let lex=volume.lex_view().map_err(|e|format!("lexicon: {e:?}"))?;let rows=volume.read_all_quins()?;let version=volume.header().version;
 let pred=if args[1]=="query"{Some(hash_token(args.get(3).ok_or("query needs predicate")?))}else{None};
 let subj=args.get(4).filter(|x|x.as_str()!="*").map(|x|hash_token(x));let obj=args.get(5).filter(|x|x.as_str()!="*").map(|x|hash_token(x));
 let mut results=Vec::new();for q in &rows{if pred.map_or(false,|v|v!=q.predicate)||subj.map_or(false,|v|v!=q.subject)||obj.map_or(false,|v|v!=q.object){continue}results.push(json!([lex.lookup_owned(q.subject).ok_or("subject missing from lexicon")?,lex.lookup_owned(q.predicate).ok_or("predicate missing from lexicon")?,lex.lookup_owned(q.object).ok_or("object missing from lexicon")?]));}
 println!("{}",json!({"reader":"qualia_core_db::q42_volume::Q42Volume","version":version,"verified_blocks":true,"total_quins":rows.len(),"rows":results}));Ok(())
}