#![cfg(target_arch = "wasm32")]
mod graph;
mod policy;
mod proof;
mod runtime;
use wasm_bindgen::prelude::*;
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("PANIC: {info}").into());
    }));
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = runtime::boot().await {
            runtime::fail(&error);
        }
    });
}
