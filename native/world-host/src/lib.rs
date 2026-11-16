#![cfg(target_arch = "wasm32")]
mod graph;
mod policy;
mod proof;
mod runtime;
use wasm_bindgen::prelude::*;
#[wasm_bindgen(start)]
pub fn start() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = runtime::boot().await {
            runtime::fail(&error);
        }
    });
}
