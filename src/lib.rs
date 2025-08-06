#![allow(dead_code, unused_imports, unsafe_op_in_unsafe_fn)]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(js_namespace = console, js_name = log)]
	fn clog(str: &str);
}

#[wasm_bindgen(start)]
fn start() {
	clog("hello");
}
