use std::{borrow::Cow, cell::Cell, f64::consts::PI, num::NonZero, ptr::NonNull};

use bevy_app::MainScheduleOrder;
use bevy_math::DVec2;
use futures_util::FutureExt;
use js_sys::{Array as JsArray, Object as JsObject};
use wasm_bindgen::{
	JsCast,
	JsError,
	JsValue,
	prelude::{Closure, wasm_bindgen},
};
use wasm_bindgen_futures::JsFuture;
use wgpu::{
	BufferUsages,
	ShaderStages,
	rwh::{
		RawDisplayHandle,
		RawWindowHandle,
		WebCanvasWindowHandle,
		WebDisplayHandle,
		WebWindowHandle,
	},
};

use crate::{DomElements, prelude::*, transform::Transform};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RenderPre;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct Render;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RenderPost;

pub struct GraphicsContext {
	pub instance: wgpu::Instance,
	pub surface: wgpu::Surface<'static>,
	pub adapter: wgpu::Adapter,
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
}

pub struct Pipelines {
	pub uniforms: wgpu::Buffer,
	pub uniforms_group: wgpu::BindGroup,

	pub instances: wgpu::Buffer,

	pub pipeline: wgpu::RenderPipeline,
}

#[derive(Clone, Copy, Debug, Event)]
pub struct WindowResized(pub UVec2);

#[derive(Clone, Copy, Debug, Component)]
pub struct Camera;

thread_local! {
	static PENDING_RESIZE: Cell<Option<UVec2>> =
		panic!("trying to use PENDING_RESIZE from background thread");
}

app_setup_fn!(async setup);
fn setup(app: &mut App) -> crate::AsyncSetupResult<'_> {
	async {
		log::info!("setting up graphics context");

		let DomElements { window, canvas, .. } = app.world().non_send_resource::<DomElements>();
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

		let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
			backends: wgpu::Backends::BROWSER_WEBGPU,
			..default()
		});

		let handle = WebCanvasWindowHandle::new(NonNull::from(canvas).cast());
		let target = wgpu::SurfaceTargetUnsafe::RawHandle {
			raw_display_handle: RawDisplayHandle::Web(WebDisplayHandle::new()),
			raw_window_handle: RawWindowHandle::WebCanvas(handle),
		};
		let surface = unsafe { instance.create_surface_unsafe(target) }.map_err(JsError::from)?;

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::HighPerformance,
				compatible_surface: Some(&surface),
				..default()
			})
			.await
			.map_err(JsError::from)?;

		let (device, queue) = adapter
			.request_device(&wgpu::DeviceDescriptor {
				required_features: wgpu::Features::empty(),
				required_limits: wgpu::Limits::downlevel_webgl2_defaults()
					.using_resolution(adapter.limits()),
				..default()
			})
			.await
			.map_err(JsError::from)?;

		let ctx = GraphicsContext {
			instance,
			surface,
			adapter,
			device,
			queue,
		};
		app.insert_non_send_resource(setup_pipelines(&ctx).await?);
		app.insert_non_send_resource(ctx);

		app.init_schedule(RenderPre);
		app.init_schedule(Render);
		app.init_schedule(RenderPost);
		let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
		order.insert_after(Update, RenderPre);
		order.insert_after(RenderPre, Render);
		order.insert_after(Render, RenderPost);

		app.add_event::<WindowResized>();

		app.add_systems(Update, dispatch_resize);
		app.add_systems(RenderPre, frame_start);
		app.add_systems(Render, frame);

		Ok(())
	}
	.boxed_local()
}

fn matrix_bytes(mat: &Mat4) -> &[u8] {
	let slice = mat.as_ref();
	bytemuck::cast_slice(slice)
}

async fn setup_pipelines(ctx: &GraphicsContext) -> JsResult<Pipelines> {
	let uniforms = ctx.device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("uniforms"),
		size: size_of::<[f32; 4 * 4 * 2 + 1 + 3]>() as _,
		usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});

	let uniforms_layout = ctx
		.device
		.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("uniforms layout"),
			entries: &[wgpu::BindGroupLayoutEntry {
				binding: 0,
				count: None,
				visibility: ShaderStages::VERTEX_FRAGMENT,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: Some(NonZero::new(uniforms.size()).unwrap()),
				},
			}],
		});
	let uniforms_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
		label: Some("uniforms group"),
		layout: &uniforms_layout,
		entries: &[wgpu::BindGroupEntry {
			binding: 0,
			resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
				buffer: &uniforms,
				offset: 0,
				size: None,
			}),
		}],
	});

	let instances = create_instances_buffer(ctx, 4);
	// DEBUG
	let instances_data = [
		vec4(1.0, 1.0, 0.0, 0.0),
		vec4(-1.0, 1.0, 0.0, 0.0),
		vec4(1.0, 2.5, 0.0, 0.0),
		vec4(-1.0, 2.5, 0.0, 0.0),
	];
	let instances_bytes = instances_data.map(|v| v.to_array());
	let instances_bytes = bytemuck::cast_slice(&instances_bytes);
	ctx.queue.write_buffer(&instances, 0, instances_bytes);

	let shader_src = include_str!("shaders/quad.wgsl");
	let shader_module = ctx
		.device
		.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: None,
			source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
		});

	let pipeline_layout = ctx
		.device
		.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[&uniforms_layout],
			push_constant_ranges: &[],
		});
	let pipeline = ctx
		.device
		.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: None,
			layout: Some(&pipeline_layout),
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			multiview: None,
			cache: None,
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleStrip,
				..default()
			},
			vertex: wgpu::VertexState {
				module: &shader_module,
				compilation_options: wgpu::PipelineCompilationOptions::default(),
				entry_point: None,
				buffers: &[wgpu::VertexBufferLayout {
					step_mode: wgpu::VertexStepMode::Instance,
					array_stride: size_of::<[f32; 4]>() as _,
					attributes: &[wgpu::VertexAttribute {
						shader_location: 0,
						offset: 0,
						format: wgpu::VertexFormat::Float32x4,
					}],
				}],
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader_module,
				compilation_options: wgpu::PipelineCompilationOptions::default(),
				entry_point: None,
				targets: &[Some(
					ctx.surface.get_capabilities(&ctx.adapter).formats[0].into(),
				)],
			}),
		});

	Ok(Pipelines {
		uniforms,
		uniforms_group,
		instances,
		pipeline,
	})
}

fn create_instances_buffer(ctx: &GraphicsContext, length: usize) -> wgpu::Buffer {
	let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("instances"),
		size: (size_of::<[f32; 4]>() * length) as _,
		usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});
	buffer
}

fn dispatch_resize(mut resize: EventWriter<WindowResized>, _: Option<NonSend<NonSendMarker>>) {
	let Some(new_size) = PENDING_RESIZE.take() else {
		return;
	};
	log::trace!("resizing to {new_size}");
	resize.write(WindowResized(new_size));
}

fn frame_start(
	ctx: NonSend<GraphicsContext>,
	pipelines: NonSend<Pipelines>,
	time: Res<Time<Virtual>>,
	mut resizes: EventReader<WindowResized>,
	camera: Query<&Transform, (With<Camera>, Changed<Transform>)>,
) {
	ctx.queue.write_buffer(
		&pipelines.uniforms,
		size_of::<[f32; 4 * 4 * 2]>() as _,
		&time.elapsed_secs().to_ne_bytes(),
	);

	if let Some(&WindowResized(size)) = resizes.read().next() {
		let surface_config = ctx
			.surface
			.get_default_config(&ctx.adapter, size.x, size.y)
			.expect("couldn't get surface config");
		ctx.surface.configure(&ctx.device, &surface_config);

		let aspect = size.x as f32 / size.y as f32;
		// TODO: configurable FOV
		let fov = 120f32.to_radians() / aspect;
		let projection = Mat4::perspective_rh(fov, aspect, 0.01, 1000.0);
		ctx.queue
			.write_buffer(&pipelines.uniforms, 0, matrix_bytes(&projection));
	}

	if let Ok(transform) = camera.single() {
		let view_mat = transform.as_view_matrix();
		ctx.queue.write_buffer(
			&pipelines.uniforms,
			size_of::<[f32; 4 * 4]>() as _,
			matrix_bytes(&view_mat),
		);
	}
}

fn frame(ctx: NonSend<GraphicsContext>, pipelines: NonSend<Pipelines>, time: Res<Time<Virtual>>) {
	let canvas_texture = ctx
		.surface
		.get_current_texture()
		.expect("couldn't get canvas texture");
	let texture_view = canvas_texture
		.texture
		.create_view(&wgpu::TextureViewDescriptor::default());

	let mut encoder = ctx
		.device
		.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

	let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
		color_attachments: &[Some(wgpu::RenderPassColorAttachment {
			view: &texture_view,
			depth_slice: None,
			resolve_target: None,
			ops: wgpu::Operations {
				load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
				store: wgpu::StoreOp::Store,
			},
		})],
		..default()
	});
	pass.set_pipeline(&pipelines.pipeline);
	pass.set_vertex_buffer(0, pipelines.instances.slice(..));
	pass.set_bind_group(0, &pipelines.uniforms_group, &[]);
	pass.draw(0 .. 4, 0 .. 4);
	drop(pass);

	ctx.queue.submit([encoder.finish()]);
	canvas_texture.present();
}
