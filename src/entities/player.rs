use std::f32::consts::PI;

use crate::{
	gfx::{Camera, Sprite, SpriteBundle, SpriteMode},
	prelude::*,
	transform::Transform,
};

app_setup_fn!(setup);
fn setup(app: &mut App) -> JsResult {
	app.add_systems(Startup, (startup, place_quads));
	app.add_systems(Update, orbit);

	Ok(())
}

fn startup(mut cmd: Commands) {
	cmd.spawn((Camera, Transform {
		translation: Vec3::new(0.0, -1.5, 1.5),
		rotation: Quat::from_rotation_x(-22.5f32.to_radians()),
	}));
}

fn orbit(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time<Virtual>>) {
	let mut transform = query.single_mut().unwrap();
	let yaw = ((time.elapsed_secs() * 2.0 * PI * 0.25).cos() * 45.0).to_radians();
	transform.rotation = Quat::from_rotation_z(yaw) * Quat::from_rotation_x(-22.5f32.to_radians());
}

fn place_quads(mut cmd: Commands) {
	for (x, y, forward) in [
		(-0.75, 0.0, Transform::RIGHT),
		(0.75, 0.0, -Transform::RIGHT),
		(-1.0, 1.0, Transform::UP),
		(1.0, 1.0, -Transform::UP),
		(-1.0, 2.0, -Transform::FORWARD),
		(1.0, 2.0, -Transform::FORWARD),
		(-1.0, 3.0, -Transform::FORWARD),
		(1.0, 3.0, -Transform::FORWARD),
		(-1.0, 4.0, -Transform::FORWARD),
		(1.0, 4.0, -Transform::FORWARD),
	] {
		cmd.spawn(SpriteBundle {
			sprite: Sprite {
				mode: SpriteMode::Fixed,
				..default()
			},
			transform: Transform::from_translation(Vec3::new(x, y, 0.5)).looking_along(forward),
			..default()
		});
	}
}
