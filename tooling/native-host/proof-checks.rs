#[path="../../native/world-host/src/proof.rs"] mod proof;
use serde_json::json;
fn main(){
 let checks=proof::checks();assert_eq!(checks["failed"],0,"{checks}");
 let mut datasets=Vec::new();let mut rights=Vec::new();
 let stage=std::path::Path::new("C:/github/synth/production/webizen-swarm");
 let catalogue=stage.join("data/publications/aaa/catalogue.json");
 if catalogue.exists(){
  let c:serde_json::Value=serde_json::from_slice(&std::fs::read(catalogue).unwrap()).unwrap();
  if let Some(cases)=c["rights"]["cases"].as_array(){for case in cases{let result=proof::rights_fixture(case).unwrap();assert_eq!(result["passed"],true);rights.push(result);}}
  for desc in c["datasets"].as_array().unwrap(){
   let path=stage.join(desc["q42_url"].as_str().unwrap().trim_start_matches('/'));
   let bytes=std::fs::read(path).unwrap();let d=proof::Dataset::open(&bytes).unwrap();let mut queries=Vec::new();
   for fixture in desc["queries"].as_array().unwrap(){let (rows,stats)=d.query(fixture["query"].as_str().unwrap()).unwrap();d.verify_fixture(fixture,&rows).unwrap();queries.push(json!({"id":fixture["id"],"actual_count":rows.len(),"expected_count":fixture["expected_count"],"native":stats,"rows":d.rows(&rows)}));}
   assert!(d.query("").is_err());
   assert!(d.query("?s ?p ?o extra").is_err());
   let mut corrupt=bytes.clone();corrupt[0]^=0xFF;assert!(proof::Dataset::open(&corrupt).is_err());
   let fixture=&desc["queries"][0];let(rows,_)=d.query(fixture["query"].as_str().unwrap()).unwrap();let mut wrong=fixture.clone();wrong["expected_count"]=json!(rows.len()+1);assert!(d.verify_fixture(&wrong,&rows).is_err());
   datasets.push(json!({"id":desc["id"],"rows":d.quins.len(),"queries":queries,"malformed_query_refused":true,"extra_query_terms_refused":true,"corrupt_header_refused":true,"wrong_expected_count_refused":true}));
  }
 }
 println!("{}",json!({"native_checks":checks,"datasets":datasets,"rights_fixtures":rights,"engine":"linked actual qualia_core_db; same proof.rs as browser WASM"}));
}
