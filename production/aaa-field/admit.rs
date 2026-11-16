pub use qualia_core_db::{q42, container_10d};
pub mod render {
 #[path="C:/github/qualiaDB/crates/qualia-core-db/src/render/portal_emf_state.rs"] pub mod portal_emf_state;
 #[path="C:/github/qualiaDB/crates/qualia-core-db/src/render/portal_load_emf.rs"] pub mod portal_load_emf;
}
fn main(){
 let p=std::env::args().nth(1).expect("EMF path");
 let bytes=std::fs::read(p).unwrap();
 let v=render::portal_load_emf::decode_emf_scientific(&bytes).expect("current native Portal admission");
 let s=v.selection();
 let max=v.display_cells().iter().filter(|c|c.valid).map(|c|c.magnitude_v_per_m).fold(0f64,f64::max);
 let mut corrupt=bytes.clone();let last=corrupt.len()-1;corrupt[last]^=1;
 assert!(render::portal_load_emf::decode_emf_scientific(&corrupt).is_err(),"corrupt input must be refused");
 assert_eq!(v.source_bytes(),bytes);
 println!("{}",serde_json::json!({"valid":true,"decoder":"current-source render::portal_load_emf::decode_emf_scientific","dimensions":v.dimensions(),"cell_count":v.cell_count(),"bounds_metres":v.local_bounds(),"time_ticks":v.time_ticks(),"temporal":v.is_temporal(),"valid_display_cells":v.display_cells().iter().filter(|c|c.valid).count(),"maximum_e_magnitude_v_per_m":max,"selection":{"descriptor_id":s.descriptor_id,"frame_id":s.frame_id,"clock_id":s.clock_id,"stream_id":s.stream_id},"exact_source_preserved":true,"corrupt_payload_rejected":true}));
}

