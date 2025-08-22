#![allow(
	dead_code,
	unused_imports,
	unused_variables,
	unused_assignments,
	unused_mut
)]

#[cfg(debug_assertions)]
pub mod fps_counter;
pub mod gfx;
pub mod input;

pub mod prelude {
	pub use bevy_app::prelude::*;
	pub use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::SystemParam};
	pub use bevy_input::prelude::*;
	pub use bevy_math::prelude::*;
	pub use bevy_time::prelude::*;

	pub use crate::JsResult;

	macro_rules! app_setup_fn {
		($f:ident) => {
			inventory::submit!($crate::SetupFn($f));
		};
	}
	pub(crate) use app_setup_fn;

	pub fn default<T: Default>() -> T {
		T::default()
	}
}

use std::{cell::OnceCell, rc::Rc, time::Duration};

use bevy_app::PluginsState;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, Event, HtmlCanvasElement};

use crate::prelude::*;

pub type JsResult<T = ()> = Result<T, JsValue>;

pub struct SetupFn(fn(&mut App) -> JsResult<()>);
inventory::collect!(SetupFn);

pub struct DomElements {
	pub window: web_sys::Window,
	pub document: web_sys::Document,
	pub canvas: web_sys::HtmlCanvasElement,
}

unsafe extern "C" {
	fn __wasm_call_ctors();
}

#[wasm_bindgen(start)]
fn start() -> JsResult {
	unsafe {
		__wasm_call_ctors();
	}
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	console_log::init_with_level(if cfg!(debug_assertions) {
		log::Level::Trace
	} else {
		log::Level::Info
	})
	.unwrap();

	let window = web_sys::window().ok_or("could not get DOM Window")?;
	let document = window.document().ok_or("DOM Window has no Document")?;
	let canvas: HtmlCanvasElement = document
		.query_selector("canvas")?
		.ok_or("could not find <canvas>")?
		.dyn_into()?;
	let dom_elements = DomElements {
		window,
		document,
		canvas,
	};

	let mut app = App::new();
	app.add_plugins(bevy_app::TaskPoolPlugin::default());
	app.add_plugins(bevy_time::TimePlugin);
	app.add_plugins(bevy_input::InputPlugin);

	app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f64(
		1.0 / 30.0,
	)));
	app.insert_non_send_resource(dom_elements);

	#[cfg(debug_assertions)]
	app.add_systems(
		Update,
		|input: Res<ButtonInput<KeyCode>>,
		 mut exit: EventWriter<AppExit>,
		 dom: NonSend<DomElements>| {
			if input.just_pressed(KeyCode::Pause) {
				dom.document.set_title("wgpustein | exited");
				exit.write(AppExit::Success);
				log::info!("requesting app exit");
			}
		},
	);

	for SetupFn(f) in inventory::iter::<SetupFn> {
		f(&mut app)?;
	}

	app.set_runner(|mut app: App| {
		app.finish();
		app.cleanup();

		let window = app
			.world()
			.non_send_resource::<DomElements>()
			.window
			.clone();

		let on_frame = Rc::new(OnceCell::<Closure<dyn FnMut() -> JsResult>>::new());
		let on_frame_fn = {
			let window = window.clone();
			let on_frame = on_frame.clone();
			move || {
				if app.should_exit().is_some() {
					log::info!("app exited");
					return Ok(());
				}

				app.update();
				window.request_animation_frame(
					on_frame
						.get()
						.unwrap_or_else(|| unreachable!())
						.as_ref()
						.unchecked_ref(),
				)?;
				Ok(())
			}
		};
		let on_frame_fn = Closure::new(on_frame_fn);
		on_frame.set(on_frame_fn).unwrap_or_else(|_| unreachable!());

		window
			.request_animation_frame(
				on_frame
					.get()
					.unwrap_or_else(|| unreachable!())
					.as_ref()
					.unchecked_ref(),
			)
			.unwrap_or_else(|err| panic!("could not request initial frame: {err:?}"));

		AppExit::Success
	});
	log::info!("app setup");
	app.run();
	log::info!("app running");

	Ok(())
}
