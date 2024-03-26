use bevy::{
    asset::load_internal_asset,
	core::FrameCount,
	core_pipeline::{
		core_3d::graph::{Core3d, Node3d},
		fullscreen_vertex_shader::fullscreen_shader_vertex_state,
	},
	ecs::query::QueryItem,
	prelude::*,
	render::{
		extract_component::{
			ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
		},
		render_graph::{
			NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
		},
		render_resource::{
			binding_types::{sampler, texture_2d, uniform_buffer},
			*,
		},
		renderer::{RenderContext, RenderDevice},
		texture::BevyDefault,
		view::ViewTarget,
		RenderApp,
	},
	window::PrimaryWindow,
};

// IMPORTANT! keep this in sync with settings.wgsl
#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct CrtGaloreSettings {
	pub frame_count			: u32,
	pub resolution			: Vec2,
	pub aberration_amount	: f32,
	pub noise_amount		: f32,
	pub vignette_amount		: f32,
	pub pixelate_amount		: f32,
	pub mask_amount			: f32,
	pub distortion_amount	: f32,
	pub bloom_amount		: f32,
}

// $ uuidgen
pub const SETTINGS_SHADER_HANDLE		: Handle<Shader> = Handle::weak_from_u128(0x9a62c467e77c4d8eb486acc975a47304u128);
pub const ENDESGA_PASS0_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0xe280e380bdb74d6b85045cba17e4ba0cu128);
pub const ENDESGA_PASS1_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0xd509e7b0ae5743db8169573727464586u128);
pub const ENDESGA_PASS2_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0xf867645387c7435db10084e000805d0du128);

pub struct CrtGalorePlugin;

impl Plugin for CrtGalorePlugin {
	fn build(&self, app: &mut App) {
		load_internal_asset!(app, SETTINGS_SHADER_HANDLE, "settings.wgsl", Shader::from_wgsl);
		load_internal_asset!(app, ENDESGA_PASS0_SHADER_HANDLE, "../assets/shaders/endesga/pass0.wgsl", Shader::from_wgsl);
		load_internal_asset!(app, ENDESGA_PASS1_SHADER_HANDLE, "../assets/shaders/endesga/pass1.wgsl", Shader::from_wgsl);
		load_internal_asset!(app, ENDESGA_PASS2_SHADER_HANDLE, "../assets/shaders/endesga/pass2.wgsl", Shader::from_wgsl);

		app.add_plugins((
			ExtractComponentPlugin::<CrtGaloreSettings>::default(),
			UniformComponentPlugin::<CrtGaloreSettings>::default(),
		));

		app.add_systems(Update, update_settings);

		let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

		render_app
			.add_render_graph_node::<ViewNodeRunner<CrtGaloreNode>>(
				Core3d,
				CrtGaloreLabel,
			)
			.add_render_graph_edges(
				Core3d,
				(
					Node3d::Tonemapping,
					CrtGaloreLabel,
					Node3d::EndMainPassPostProcessing,
				),
			);
	}

	fn finish(&self, app: &mut App) {
		let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};

		render_app.init_resource::<CrtGalorePipeline>();
	}
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CrtGaloreLabel;

#[derive(Default)]
struct CrtGaloreNode;

impl ViewNode for CrtGaloreNode {
	type ViewQuery = (
		&'static ViewTarget,
		&'static CrtGaloreSettings,
	);

	fn run(
		&self,
		_graph: &mut RenderGraphContext,
		render_context: &mut RenderContext,
		(view_target, _post_process_settings): QueryItem<Self::ViewQuery>,
		world: &World,
	) -> Result<(), NodeRunError> {
		let post_process_pipeline = world.resource::<CrtGalorePipeline>();

		let pipeline_cache = world.resource::<PipelineCache>();

		let Some(pass0_pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pass0_pipeline_id) else { return Ok(()) };
		let Some(pass1_pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pass1_pipeline_id) else { return Ok(()) };
		let Some(pass2_pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pass2_pipeline_id) else { return Ok(()) };

		let settings_uniforms = world.resource::<ComponentUniforms<CrtGaloreSettings>>();

		let Some(settings_binding) = settings_uniforms.uniforms().binding() else { return Ok(()) };
		
		let globals_buffer = world.resource::<GlobalsBuffer>();
		
		let Some(global_uniforms) = globals_buffer.buffer.binding() else { return Ok(()) };

		let mut envoke_render_pass = |pipeline: &RenderPipeline, name: &str| {
			let post_process = view_target.post_process_write();

			let bind_group = render_context.render_device().create_bind_group(
				"crt_galore_bind_group",
				&post_process_pipeline.layout,
				// It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
				&BindGroupEntries::sequential((
					// Make sure to use the source view
					post_process.source,
					// Use the sampler created for the pipeline
					&post_process_pipeline.sampler,
					// Set the settings binding
					settings_binding.clone(),
					// Bevy default global uniforms
					global_uniforms.clone(),
				)),
			);

			let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
				label: Some(name),
				color_attachments: &[Some(RenderPassColorAttachment {
					view: post_process.destination,
					resolve_target: None,
					ops: Operations::default(),
				})],
				depth_stencil_attachment: None,
				timestamp_writes: None,
				occlusion_query_set: None,
			});

			render_pass.set_render_pipeline(pipeline);
			render_pass.set_bind_group(0, &bind_group, &[]);
			render_pass.draw(0..3, 0..1);
		};

		envoke_render_pass(pass0_pipeline, "crt_galore_pass0");
		envoke_render_pass(pass1_pipeline, "crt_galore_pass1");
		envoke_render_pass(pass2_pipeline, "crt_galore_pass2");

		Ok(())
	}
}

#[derive(Resource)]
struct CrtGalorePipeline {
	layout				: BindGroupLayout,
	sampler				: Sampler,
	pass0_pipeline_id	: CachedRenderPipelineId,
	pass1_pipeline_id	: CachedRenderPipelineId,
	pass2_pipeline_id	: CachedRenderPipelineId,
}

impl FromWorld for CrtGalorePipeline {
	fn from_world(world: &mut World) -> Self {
		let render_device = world.resource::<RenderDevice>();

		let layout = render_device.create_bind_group_layout(
			"crt_galore_bind_group_layout",
			&BindGroupLayoutEntries::sequential(
				ShaderStages::FRAGMENT,
				(
					// The screen texture
					texture_2d(TextureSampleType::Float { filterable: true }),
					// The screen texture sampler
					sampler(SamplerBindingType::Filtering),
					// The settings uniform that will control the effect
					uniform_buffer::<CrtGaloreSettings>(false),
				),
			),
		);

		// We can create the sampler here since it won't change at runtime and doesn't depend on the view
		let sampler = render_device.create_sampler(&SamplerDescriptor::default());

		let shader0 = ENDESGA_PASS0_SHADER_HANDLE.clone();
		let shader1 = ENDESGA_PASS1_SHADER_HANDLE.clone();
		let shader2 = ENDESGA_PASS2_SHADER_HANDLE.clone();
		
		fn make_pipeline(
			world: &mut World,
			layout: &BindGroupLayout,
			shader: Handle<Shader>
		) -> CachedRenderPipelineId {
			world
				.resource_mut::<PipelineCache>()
				.queue_render_pipeline(RenderPipelineDescriptor {
					label: Some("crt_galore_pipeline".into()),
					layout: vec![layout.clone()],
					vertex: fullscreen_shader_vertex_state(),
					fragment: Some(FragmentState {
						shader,
						shader_defs: vec![],
						entry_point: "fragment".into(),
						targets: vec![Some(ColorTargetState {
							format: TextureFormat::Rgba16Float, // bevy_default(),
							blend: None,
							write_mask: ColorWrites::ALL,
						})],
					}),
					primitive: PrimitiveState::default(),
					depth_stencil: None,
					multisample: MultisampleState::default(),
					push_constant_ranges: vec![],
				})
		}

		let pipeline0_id = make_pipeline(world, &layout, shader0);
		let pipeline1_id = make_pipeline(world, &layout, shader1);
		let pipeline2_id = make_pipeline(world, &layout, shader2);

		Self {
			layout,
			sampler,
			pass0_pipeline_id: pipeline0_id,
			pass1_pipeline_id: pipeline1_id,
			pass2_pipeline_id: pipeline2_id,
		}
	}
}


fn update_settings(
	frame_count: Res<FrameCount>,
	q_window_primary: Query<&Window, With<PrimaryWindow>>,
	mut settings: Query<&mut CrtGaloreSettings>
) {
	let Ok(window) = q_window_primary.get_single() else { return };

	for mut setting in &mut settings {
		setting.frame_count = frame_count.0;

		setting.resolution = Vec2::new(window.resolution.width(), window.resolution.height());
	}
}