use crate::prelude::*;

#[derive(Clone, Debug, Component)]
pub struct Transform {
	pub translation: Vec3,
	pub rotation: Quat,
}

impl Transform {
	pub const FORWARD: Vec3 = Vec3::Y;
	pub const RIGHT: Vec3 = Vec3::X;
	pub const UP: Vec3 = Vec3::Z;

	pub fn as_model_matrix(&self) -> Mat4 {
		let &Self {
			translation,
			rotation,
		} = self;
		let translation = Mat4::from_translation(translation);
		let rotation = Mat4::from_quat(rotation);
		rotation * translation
	}

	pub fn as_view_matrix(&self) -> Mat4 {
		let &Self {
			translation,
			rotation,
		} = self;
		let forward = rotation.mul_vec3(Self::FORWARD);
		Mat4::look_to_rh(translation, forward, Self::UP)
	}
}
