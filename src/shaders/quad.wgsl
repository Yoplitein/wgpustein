const PI: f32 = 3.14159265358979323846264338327950288;

struct Uniforms {
	time: f32,
	// projection: mat4f,
	// view: mat4f,
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

@vertex
fn vertex_main(in: VIn) -> @builtin(position) vec4f {
	var vertex: vec4f;
	switch in.vertex {
		case 0 {
			vertex = vec4f(0.0, 0.1, 0.0, 1.0);
		}
		case 1 {
			vertex = vec4f(-0.1, -0.1, 0.0, 1.0);
		}
		case 2 {
			vertex = vec4f(0.1, -0.1, 0.0, 1.0);
		}
		default {
			vertex = vec4f(0.0, 0.0, 0.0, 1.0);
		}
	}
	return vec4f(in.position.xyz, 0.0) + vertex;
}

@fragment
fn fragment_main() -> @location(0) vec4f {
	return vec4f(
		cos(uniforms.time * 0.9 * 2.0 * PI),
		sin(uniforms.time * 2.0 * PI),
		cos(uniforms.time * 0.8 * 2.0 * PI),
		1.0
	);
}
