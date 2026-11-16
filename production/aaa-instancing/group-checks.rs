extern crate etalon_render;
mod render { pub mod instance_culling { pub use etalon_render::instance_culling::*; } }
#[path = "C:/github/qualiaDB/crates/qualia-core-db/src/render/gpu/instance_groups.rs"]
mod groups;
use std::alloc::{GlobalAlloc,Layout,System};
use std::cell::Cell;
thread_local! { static MEASURE:Cell<bool>=const{Cell::new(false)};static ALLOCATIONS:Cell<usize>=const{Cell::new(0)}; }
struct Count;
unsafe impl GlobalAlloc for Count {
 unsafe fn alloc(&self,l:Layout)->*mut u8 { MEASURE.with(|m|if m.get(){ALLOCATIONS.with(|n|n.set(n.get()+1))});unsafe{System.alloc(l)} }
 unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe{System.dealloc(p,l)}}
 unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8 { MEASURE.with(|m|if m.get(){ALLOCATIONS.with(|v|v.set(v.get()+1))});unsafe{System.realloc(p,l,n)} }
}
#[global_allocator] static ALLOCATOR:Count=Count;
#[test] fn group_culling_and_submission_allocate_zero_heap() {
 let identity=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]];
 let records=[render::instance_culling::GpuInstanceRecord::new(identity,7);2];
 let mut groups=groups::compile_groups(&[0,3,0,1,3,3,1,1],&[[-0.2,-0.2,0.5],[0.2,-0.2,0.5],[0.,0.2,0.5]],&[0,1,2,0,1,2],2).unwrap();
 let mut output=records;let mut scratch=[0;2];
 ALLOCATIONS.with(|n|n.set(0));MEASURE.with(|m|m.set(true));
 let count=groups::compact_group_visibility(&mut groups,&records,identity,identity,&mut scratch,&mut output).unwrap();
 let draws=groups::draw_ranges(&groups,0,6,2,true).count();
 MEASURE.with(|m|m.set(false));
 assert_eq!(count,2);assert_eq!(draws,2);assert_eq!(ALLOCATIONS.with(Cell::get),0);
}
