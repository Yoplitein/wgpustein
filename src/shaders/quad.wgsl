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
	model_0: vec4f,

	@location(1)
	model_1: vec4f,

	@location(2)
	model_2: vec4f,

	@location(3)
	model_3: vec4f,

	@location(4)
	size_etc: vec4f,
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
			vertex = vec4f(-0.5, 0.0, 0.5, 1.0);
			uv = vec2f(0.0, 1.0);
		}
		case 1 {
			vertex = vec4f(0.5, 0.0, 0.5, 1.0);
			uv = vec2f(1.0, 1.0);
		}
		case 2 {
			vertex = vec4f(-0.5, 0.0, -0.5, 1.0);
			uv = vec2f(0.0, 0.0);
		}
		case 3 {
			vertex = vec4f(0.5, 0.0, -0.5, 1.0);
			uv = vec2f(1.0, 0.0);
		}
		default {
			vertex = vec4f(0.0, 0.0, 0.0, 1.0);
			uv = vec2f(0.5, 0.5);
		}
	}
	let size = in.size_etc.xy;
	vertex = vertex * vec4f(size, 1.0, 1.0);
	// TODO: billboard flag, texture
	let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);
	return VOut(
		uniforms.projection * uniforms.view * model * vertex,
		uv,
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
