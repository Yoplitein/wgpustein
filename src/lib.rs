#![allow(dead_code, unused_imports, unused_variables, unused_assignments)]

use log::trace;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, Event, HtmlCanvasElement};

pub type JsResult<T> = Result<T, JsValue>;

#[wasm_bindgen(start)]
fn start() -> JsResult<()> {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	console_log::init_with_level(
		if cfg!(debug_assertions) {
			log::Level::Trace
		} else {
			log::Level::Info
		}
	).unwrap();

	let window = web_sys::window().ok_or("could not get DOM Window")?;
	let document = window.document().ok_or("DOM Window has no Document")?;
	let canvas: HtmlCanvasElement = document.query_selector("canvas")?.ok_or("could not find <canvas>")?.dyn_into()?;

	// temp
	let context: CanvasRenderingContext2d = canvas.get_context("2d")?.ok_or("could not get canvas context")?.dyn_into()?;
	let render = {
		let canvas = canvas.clone();
		let ctx = context.clone();
		move |mut now: f64| -> JsResult<()> {
			now /= 1000.0;

			let (width, height) = (canvas.width() as f64, canvas.height() as f64);

			ctx.set_fill_style_str("black");
			ctx.fill_rect(0.0, 0.0, width, height);

			ctx.set_stroke_style_str("white");
			ctx.begin_path();
			ctx.move_to(0.0, 0.0);
			ctx.line_to(width / 2.0, height / 2.0);
			ctx.stroke();

			Ok(())
		}
	};
	let render = Closure::<dyn Fn(f64) -> JsResult<()>>::new(render);

	let resize = {
		let window = window.clone();
		let canvas = canvas.clone();
		move || -> JsResult<()> {
			let width = window.inner_width()?.as_f64().unwrap() as u32;
			let height = window.inner_height()?.as_f64().unwrap() as u32;
			trace!("resizing to {width}x{height}");
			canvas.set_width(width);
			canvas.set_height(height);
			// TODO: dispatch ECS event
			window.request_animation_frame(render.as_ref().unchecked_ref())?;
			Ok(())
		}
	};
	let resize = Closure::<dyn Fn() -> JsResult<()>>::new(resize);
	window.add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref())?;
	resize.forget();
	window.dispatch_event(&Event::new("resize")?)?;

	Ok(())
}
