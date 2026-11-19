#[path = "../../native/world-host/src/runtime/ground.rs"]
mod ground;
#[path = "../../native/world-host/src/runtime/understorey.rs"]
mod understorey;

fn main() {
    println!("mesh bytes {}", ground::mesh_len());
    println!("tufts {}", understorey::TUFTS.len());
}
