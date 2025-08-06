#![allow(dead_code, unused_imports, unsafe_op_in_unsafe_fn)]

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn start() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	console_log::init_with_level(
		if cfg!(debug_assertions) {
			log::Level::Trace
		} else {
			log::Level::Info
		}
	).unwrap();

	log::trace!("trace");
	log::debug!("debug");
	log::info!("info");
	log::warn!("warn");
	log::error!("error");
	panic!("panic");
}
