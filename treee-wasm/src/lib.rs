use wasm_bindgen::prelude::*;

pub use wasm_bindgen_rayon::init_thread_pool;

#[wasm_bindgen]
pub async fn treee() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
	treee::try_main().await.unwrap();
}
