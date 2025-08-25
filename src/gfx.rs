use std::{cell::Cell, f64::consts::PI};

use bevy_app::MainScheduleOrder;
use bevy_math::DVec2;
use futures_util::FutureExt;
use js_sys::{Array as JsArray, Object as JsObject};
use wasm_bindgen::{
	JsCast,
	JsValue,
	prelude::{Closure, wasm_bindgen},
};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
	Gpu,
	GpuAdapter,
	GpuBindGroup,
	GpuBindGroupDescriptor,
	GpuBindGroupEntry,
	GpuBindGroupLayoutDescriptor,
	GpuBindGroupLayoutEntry,
	GpuBuffer,
	GpuBufferBinding,
	GpuBufferBindingLayout,
	GpuBufferBindingType,
	GpuBufferDescriptor,
	GpuCanvasAlphaMode,
	GpuCanvasConfiguration,
	GpuCanvasContext,
	GpuColorTargetState,
	GpuDevice,
	GpuDeviceDescriptor,
	GpuFragmentState,
	GpuLoadOp,
	GpuPipelineLayoutDescriptor,
	GpuPowerPreference,
	GpuQueue,
	GpuRenderPassColorAttachment,
	GpuRenderPassDescriptor,
	GpuRenderPipeline,
	GpuRenderPipelineDescriptor,
	GpuRequestAdapterOptions,
	GpuShaderModule,
	GpuShaderModuleDescriptor,
	GpuStoreOp,
	GpuVertexAttribute,
	GpuVertexBufferLayout,
	GpuVertexFormat,
	GpuVertexState,
	GpuVertexStepMode,
	gpu_buffer_usage as buffer_usage,
	gpu_shader_stage as shader_stage,
	js_sys,
};

use crate::{DomElements, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RenderPre;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct Render;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RenderPost;

pub struct GraphicsContext {
	pub gpu: Gpu,
	pub adapter: GpuAdapter,
	pub device: GpuDevice,
	pub queue: GpuQueue,
	pub context: GpuCanvasContext,
}

pub struct Pipelines {
	pub uniforms: GpuBuffer,
	pub uniforms_group: GpuBindGroup,

	pub instances: GpuBuffer,

	pub pipeline: GpuRenderPipeline,
}

#[derive(Clone, Copy, Debug, Event)]
pub struct WindowResized(pub UVec2);

thread_local! {
	static PENDING_RESIZE: Cell<Option<UVec2>> = panic!("trying to use PENDING_RESIZE from background thread");
}

#[wasm_bindgen(inline_js = "export async function sleep(msecs) { const p = \
                            Promise.withResolvers(); setTimeout(p.resolve, msecs); await \
                            p.promise; }")]
extern "C" {
	async fn sleep(msecs: f64);
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

		// TODO: error out when WebGPU unsupported
		let gpu = window.navigator().gpu();

		let mut opt = GpuRequestAdapterOptions::new();
		opt.set_power_preference(GpuPowerPreference::HighPerformance);
		let adapter: GpuAdapter = JsFuture::from(gpu.request_adapter_with_options(&opt))
			.await?
			.dyn_into()?;

		let device: GpuDevice = JsFuture::from(adapter.request_device()).await?.dyn_into()?;
		let queue = device.queue();

		let context: GpuCanvasContext = canvas
			.get_context("webgpu")?
			.ok_or("couldn't get canvas context")?
			.dyn_into()?;
		let config = GpuCanvasConfiguration::new(&device, gpu.get_preferred_canvas_format());
		config.set_alpha_mode(GpuCanvasAlphaMode::Opaque);
		context.configure(&config)?;

		let ctx = GraphicsContext {
			gpu,
			adapter,
			device,
			queue,
			context,
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

async fn setup_pipelines(ctx: &GraphicsContext) -> JsResult<Pipelines> {
	let uniforms = ctx.device.create_buffer(&GpuBufferDescriptor::new(
		size_of::<f32>() as _,
		buffer_usage::UNIFORM | buffer_usage::COPY_DST,
	))?;

	let binding = GpuBufferBindingLayout::new();
	binding.set_type(GpuBufferBindingType::Uniform);
	let mut entry = GpuBindGroupLayoutEntry::new(0, shader_stage::VERTEX | shader_stage::FRAGMENT);
	entry.set_buffer(&binding);
	let uniforms_layout =
		ctx.device
			.create_bind_group_layout(&GpuBindGroupLayoutDescriptor::new(&JsArray::from_iter([
				entry,
			])))?;
	let uniforms_group = ctx.device.create_bind_group(&GpuBindGroupDescriptor::new(
		&JsArray::from_iter([&GpuBindGroupEntry::new(
			0,
			&GpuBufferBinding::new(&uniforms),
		)]),
		&uniforms_layout,
	));

	let instances = create_instances_buffer(&ctx, 4); // FIXME: initially 0
	// DEBUG
	let instances_bytes = [
		vec4(-0.5, 0.5, 0.0, 0.0),
		vec4(0.5, 0.5, 0.0, 0.0),
		vec4(-0.5, -0.5, 0.0, 0.0),
		vec4(0.5, -0.5, 0.0, 0.0),
	];
	let instances_bytes = instances_bytes.map(|v| v.to_array().map(f32::to_ne_bytes));
	let instances_bytes = instances_bytes.as_flattened().as_flattened(); // : ^ )
	ctx.queue
		.write_buffer_with_u32_and_u8_slice(&instances, 0, instances_bytes)
		.expect("couldn't write instances buffer");

	let shader_src = include_str!("shaders/quad.wgsl");
	let shader_module = ctx
		.device
		.create_shader_module(&GpuShaderModuleDescriptor::new(shader_src));

	let pipeline_layout = ctx
		.device
		.create_pipeline_layout(&GpuPipelineLayoutDescriptor::new(&JsArray::from_iter([
			&uniforms_layout,
		])));

	let mut vertex_state = GpuVertexState::new(&shader_module);
	vertex_state.set_entry_point("vertex_main");
	let mut buffer_layout = GpuVertexBufferLayout::new(
		size_of::<[f32; 4]>() as _,
		&JsArray::from_iter([&GpuVertexAttribute::new(GpuVertexFormat::Float32x4, 0.0, 0)]),
	);
	buffer_layout.set_step_mode(GpuVertexStepMode::Instance);
	let buffers = JsArray::from_iter([&buffer_layout]);
	vertex_state.set_buffers(&buffers);
	let mut fragment_state = GpuFragmentState::new(
		&shader_module,
		&JsArray::from_iter([&GpuColorTargetState::new(
			ctx.gpu.get_preferred_canvas_format(),
		)]),
	);
	fragment_state.set_entry_point("fragment_main");
	let mut pipeline_desc = GpuRenderPipelineDescriptor::new(&pipeline_layout, &vertex_state);
	pipeline_desc.set_fragment(&fragment_state);
	let pipeline = ctx.device.create_render_pipeline(&pipeline_desc)?;

	Ok(Pipelines {
		uniforms,
		uniforms_group,
		instances,
		pipeline,
	})
}

fn create_instances_buffer(ctx: &GraphicsContext, length: usize) -> GpuBuffer {
	let buffer = ctx
		.device
		.create_buffer(&GpuBufferDescriptor::new(
			(size_of::<[f32; 4]>() * length) as _,
			buffer_usage::VERTEX | buffer_usage::COPY_DST,
		))
		.expect("couldn't allocate instances buffer");
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
) {
	ctx.queue
		.write_buffer_with_u32_and_u8_slice(
			&pipelines.uniforms,
			0,
			&time.elapsed_secs().to_ne_bytes(),
		)
		.expect("couldn't write uniforms buffer");
}

fn frame(ctx: NonSend<GraphicsContext>, pipelines: NonSend<Pipelines>, time: Res<Time<Virtual>>) {
	let canvas_texture = ctx
		.context
		.get_current_texture()
		.expect("couldn't get canvas texture");
	let texture_view = canvas_texture
		.create_view()
		.expect("couldn't get canvas texture view");

	let encoder = ctx.device.create_command_encoder();

	let mut attachment =
		GpuRenderPassColorAttachment::new(GpuLoadOp::Clear, GpuStoreOp::Store, &texture_view);
	attachment.set_clear_value(
		&[0.0, 1.0, 1.0, 1.0]
			.into_iter()
			.map(JsValue::from)
			.collect::<JsArray>(),
	);
	let attachments = JsArray::from_iter([&attachment]);
	let pass_encoder = encoder
		.begin_render_pass(&GpuRenderPassDescriptor::new(&attachments))
		.expect("couldn't begin render pass");
	pass_encoder.set_pipeline(&pipelines.pipeline);
	pass_encoder.set_vertex_buffer(0, Some(&pipelines.instances));
	pass_encoder.set_bind_group(0, Some(&pipelines.uniforms_group));
	pass_encoder.draw_with_instance_count(3, 4); // FIXME: instance count
	pass_encoder.end();

	let command_buffer = encoder.finish();
	ctx.device
		.queue()
		.submit(&JsArray::from_iter([&command_buffer]));
}
