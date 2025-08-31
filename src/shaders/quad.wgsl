const PI: f32 = 3.14159265358979323846264338327950288;

struct Uniforms {
	projection: mat4x4f,
	view: mat4x4f,
	time: f32,
}

@group(0)
@binding(0)
var<uniform> uniforms: Uniforms;

struct VIn {
	@builtin(vertex_index)
	vertex: u32,

	@location(0)
	position: vec4f,
}

struct VOut {
	@builtin(position)
	position: vec4f,

	@location(0)
	uv: vec2f,
}

@vertex
fn vertex_main(in: VIn) -> VOut {
	var vertex: vec4f;
	var uv: vec2f;
	switch in.vertex {
		case 0 {
			vertex = vec4f(-0.5, 0.5, 0.0, 1.0);
			uv = vec2f(0.0, 1.0);
		}
		case 1 {
			vertex = vec4f(0.5, 0.5, 0.0, 1.0);
			uv = vec2f(1.0, 1.0);
		}
		case 2 {
			vertex = vec4f(-0.5, -0.5, 0.0, 1.0);
			uv = vec2f(0.0, 0.0);
		}
		case 3 {
			vertex = vec4f(0.5, -0.5, 0.0, 1.0);
			uv = vec2f(1.0, 0.0);
		}
		default {
			vertex = vec4f(0.0, 0.0, 0.0, 1.0);
			uv = vec2f(0.5, 0.5);
		}
	}
	let model = calc_model_matrix(in.position.xyz);
	return VOut(
		uniforms.projection * uniforms.view * model * vertex,
		uv,
	);
}

fn calc_model_matrix(pos: vec3f) -> mat4x4f {
	return mat4x4f(
		vec4f(1.0, 0.0, 0.0, 0.0),
		vec4f(0.0, 1.0, 0.0, 0.0),
		vec4f(0.0, 0.0, 1.0, 0.0),
		vec4f(pos, 1.0),
	);
}

@fragment
fn fragment_main(in: VOut) -> @location(0) vec4f {
	return vec4f(
		in.uv,
		cos(uniforms.time * 2.0 * PI),
		1.0
	);
}
