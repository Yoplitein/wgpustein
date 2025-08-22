use crate::prelude::*;

app_setup_fn!(setup);
fn setup(app: &mut App) -> JsResult {
	app.init_resource::<FpsState>();
	app.add_systems(Update, fps_frame);
	app.add_systems(FixedUpdate, fps_tick);

	Ok(())
}

#[derive(Default, Resource)]
struct FpsState {
	accum: f64,
	frames: usize,
	ticks: usize,
}

fn fps_frame(mut state: ResMut<FpsState>, time: Res<Time<Real>>) {
	state.frames += 1;
	state.accum += time.delta_secs_f64();
	let update = state.accum >= 1.0;
	state.accum = state.accum.fract();

	if update {
		let window = web_sys::window().unwrap();
		let document = window.document().unwrap();
		document.set_title(&format!(
			"wgpustein | {} fps {} tps",
			state.frames, state.ticks
		));
		state.ticks = 0;
		state.frames = 0;
	}
}

fn fps_tick(mut state: ResMut<FpsState>) {
	state.ticks += 1;
}
