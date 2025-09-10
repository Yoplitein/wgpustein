use bevy_math::VectorSpace;

use crate::prelude::*;

#[derive(Clone, Debug, Default, Component)]
pub struct Transform {
	pub translation: Vec3,
	pub rotation: Quat,
}

impl Transform {
	pub const FORWARD: Vec3 = Vec3::Y;
	pub const RIGHT: Vec3 = Vec3::X;
	pub const UP: Vec3 = Vec3::Z;

	pub fn from_translation(translation: Vec3) -> Self {
		Self {
			translation,
			rotation: Quat::IDENTITY,
		}
	}

	pub fn from_rotation(rotation: Quat) -> Self {
		Self {
			translation: Vec3::ZERO,
			rotation,
		}
	}

	pub fn with_translation(&self, translation: Vec3) -> Self {
		let &Self { rotation, .. } = self;
		Self {
			translation,
			rotation,
		}
	}

	pub fn with_rotation(&self, rotation: Quat) -> Self {
		let &Self { translation, .. } = self;
		Self {
			translation,
			rotation,
		}
	}

	pub fn looking_at(&self, position: Vec3) -> Self {
		self.looking_along(position - self.translation)
	}

	pub fn looking_along(&self, forward: Vec3) -> Self {
		let axis = Self::FORWARD.cross(forward).normalize_or(Self::UP);
		let angle = signed_angle_between(Self::FORWARD, forward, axis);
		let new_rotation = Quat::from_axis_angle(axis, angle);
		self.with_rotation(new_rotation)
	}

	pub fn forward(&self) -> Vec3 {
		self.rotation.mul_vec3(Self::FORWARD)
	}

	pub fn right(&self) -> Vec3 {
		let right = self.forward().cross(Self::UP);
		if right.abs_diff_eq(Vec3::ZERO, 0.5) {
			Self::RIGHT
		} else {
			right
		}
	}

	pub fn as_model_matrix(&self) -> Mat4 {
		let &Self {
			translation,
			rotation,
		} = self;
		Mat4::from_rotation_translation(rotation, translation)
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

fn signed_angle_between(a: Vec3, b: Vec3, plane: Vec3) -> f32 {
	let (a, b) = (a.cross(b).dot(plane), a.dot(b));
	a.atan2(b)
}
