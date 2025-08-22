use std::{cell::Cell, f64::consts::PI};

use bevy_app::MainScheduleOrder;
use bevy_math::DVec2;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::CanvasRenderingContext2d;

use crate::{DomElements, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RenderPre;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct Render;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RenderPost;

pub struct GraphicsContext {
	ctx: CanvasRenderingContext2d,
}

#[derive(Clone, Copy, Debug, Event)]
pub struct WindowResized(pub UVec2);

thread_local! {
	static PENDING_RESIZE: Cell<Option<UVec2>> = panic!("trying to use PENDING_RESIZE from background thread");
}

app_setup_fn!(setup);
fn setup(app: &mut App) -> JsResult {
	log::info!("setting up graphics context");

	let DomElements { window, canvas, .. } = app.world().non_send_resource::<DomElements>();
	let ctx = canvas
		.get_context("2d")?
		.ok_or("could not get canvas context")?
		.dyn_into()?;
	let resize = {
		let window = window.clone();
		let canvas = canvas.clone();
		move || -> JsResult<()> {
			let size = UVec2::new(
				window.inner_width()?.as_f64().unwrap() as u32,
				window.inner_height()?.as_f64().unwrap() as u32,
			);
			canvas.set_width(size.x);
			canvas.set_height(size.y);
			PENDING_RESIZE.set(Some(size));
			Ok(())
		}
	};
	let resize = Closure::<dyn Fn() -> JsResult<()>>::new(resize);
	window.add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref())?;
	resize.forget();
	window.dispatch_event(&web_sys::Event::new("resize")?)?;

	app.insert_non_send_resource(GraphicsContext { ctx });

	app.init_schedule(RenderPre);
	app.init_schedule(Render);
	app.init_schedule(RenderPost);
	let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
	order.insert_after(Update, RenderPre);
	order.insert_after(RenderPre, Render);
	order.insert_after(Render, RenderPost);

	app.add_event::<WindowResized>();

	app.add_systems(RenderPre, frame_start);
	app.add_systems(Render, frame);

	Ok(())
}

fn dispatch_resize(mut resize: EventWriter<WindowResized>, _: NonSend<NonSendMarker>) {
	let Some(new_size) = PENDING_RESIZE.take() else {
		return;
	};
	log::trace!("resizing to {new_size}");
	resize.write(WindowResized(new_size));

	// TODO: resize framebuffer?
}

fn frame_start(ctx: NonSend<GraphicsContext>) {
	let ctx = &ctx.ctx;
	ctx.set_fill_style_str("black");
	let (width, height) = {
		let canvas = ctx.canvas().unwrap();
		(canvas.width() as f64, canvas.height() as f64)
	};
	ctx.fill_rect(0.0, 0.0, width, height);
}

fn frame(ctx: NonSend<GraphicsContext>, time: Res<Time<Virtual>>) {
	let ctx = &ctx.ctx;
	let (width, height) = {
		let canvas = ctx.canvas().unwrap();
		(canvas.width() as f64, canvas.height() as f64)
	};
	let size = DVec2::new(width, height);
	let half_size = size / 2.0;

	ctx.set_stroke_style_str("white");
	ctx.begin_path();
	let dist = size.min_element() * 0.95 / 2.0;
	let (x, y) =
		(DVec2::from_angle(time.elapsed_secs_f64() * 2.0 * PI * 0.25) * dist + half_size).into();
	ctx.move_to(half_size.x, half_size.y);
	ctx.line_to(x, y);
	ctx.stroke();
}
