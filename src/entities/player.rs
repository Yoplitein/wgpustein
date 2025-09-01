use std::f32::consts::PI;

use crate::{gfx::Camera, prelude::*, transform::Transform};

app_setup_fn!(setup);
fn setup(app: &mut App) -> JsResult {
	app.add_systems(Startup, startup);
	app.add_systems(Update, orbit);

	Ok(())
}

fn startup(mut cmd: Commands) {
	cmd.spawn((Camera, Transform {
		translation: Vec3::new(0.0, -2.5, 1.5),
		rotation: Quat::from_rotation_x(-22.5f32.to_radians()),
	}));
}

fn orbit(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time<Virtual>>) {
	let mut transform = query.single_mut().unwrap();
	let yaw = ((time.elapsed_secs() * 2.0 * PI * 0.25).cos() * 45.0).to_radians();
	transform.rotation =
		Quat::from_rotation_z(yaw).mul_quat(Quat::from_rotation_x(-22.5f32.to_radians()));
}
