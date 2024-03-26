use bevy::{
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

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct CrtGaloreSettings {
	pub intensity: f32,
	pub frame_count: u32,
	pub resolution: Vec2,
}

pub struct CrtGalorePlugin;

impl Plugin for CrtGalorePlugin {
	fn build(&self, app: &mut App) {
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

		// Get the pipeline from the cache
		let Some(pipeline0) = pipeline_cache.get_render_pipeline(post_process_pipeline.pass0_pipeline_id)
		else {
			return Ok(());
		};

		let Some(pipeline1) = pipeline_cache.get_render_pipeline(post_process_pipeline.pass1_pipeline_id)
		else {
			return Ok(());
		};

		let Some(pipeline2) = pipeline_cache.get_render_pipeline(post_process_pipeline.pass2_pipeline_id)
		else {
			return Ok(());
		};

		let settings_uniforms = world.resource::<ComponentUniforms<CrtGaloreSettings>>();
		let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
			return Ok(());
		};

		let post_process = view_target.post_process_write();

		let bind_group = render_context.render_device().create_bind_group(
			"post_process_bind_group",
			&post_process_pipeline.layout,
			// It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
			&BindGroupEntries::sequential((
				// Make sure to use the source view
				post_process.source,
				// Use the sampler created for the pipeline
				&post_process_pipeline.sampler,
				// Set the settings binding
				settings_binding.clone(),
			)),
		);

		{
			let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
				label: Some("post_process_pass"),
				color_attachments: &[Some(RenderPassColorAttachment {
					view: post_process.destination,
					resolve_target: None,
					ops: Operations::default(),
				})],
				depth_stencil_attachment: None,
				timestamp_writes: None,
				occlusion_query_set: None,
			});

			render_pass.set_render_pipeline(pipeline0);
			render_pass.set_bind_group(0, &bind_group, &[]);
			render_pass.draw(0..3, 0..1);
		}


		// second render pass

		let post_process = view_target.post_process_write();

		let bind_group = render_context.render_device().create_bind_group(
			"post_process_bind_group2",
			&post_process_pipeline.layout,
			&BindGroupEntries::sequential((
				post_process.source,
				&post_process_pipeline.sampler,
				settings_binding.clone(),
			)),
		);
		{
			let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
				label: Some("post_process_pass2"),
				color_attachments: &[Some(RenderPassColorAttachment {
					view: post_process.destination,
					resolve_target: None,
					ops: Operations::default(),
				})],
				depth_stencil_attachment: None,
				timestamp_writes: None,
				occlusion_query_set: None,
			});

			render_pass.set_render_pipeline(pipeline1);
			render_pass.set_bind_group(0, &bind_group, &[]);
			render_pass.draw(0..3, 0..1);
		}

		// third render pass

		let post_process = view_target.post_process_write();

		let bind_group = render_context.render_device().create_bind_group(
			"post_process_bind_group3",
			&post_process_pipeline.layout,
			&BindGroupEntries::sequential((
				post_process.source,
				&post_process_pipeline.sampler,
				settings_binding.clone(),
			)),
		);
		{
			let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
				label: Some("post_process_pass3"),
				color_attachments: &[Some(RenderPassColorAttachment {
					view: post_process.destination,
					resolve_target: None,
					ops: Operations::default(),
				})],
				depth_stencil_attachment: None,
				timestamp_writes: None,
				occlusion_query_set: None,
			});

			render_pass.set_render_pipeline(pipeline2);
			render_pass.set_bind_group(0, &bind_group, &[]);
			render_pass.draw(0..3, 0..1);
		}

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

		let shader0 = world
			.resource::<AssetServer>()
			.load("shaders/endesga/pass0.wgsl");

		let shader1 = world
			.resource::<AssetServer>()
			.load("shaders/endesga/pass1.wgsl");

		let shader2 = world
			.resource::<AssetServer>()
			.load("shaders/endesga/pass2.wgsl");

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
	time: Res<Time>,
	frame_count: Res<FrameCount>,
	q_window_primary: Query<&Window, With<PrimaryWindow>>,
	mut settings: Query<&mut CrtGaloreSettings>
) {
	let Ok(window) = q_window_primary.get_single() else { return };

	for mut setting in &mut settings {
		let mut intensity = time.elapsed_seconds().sin();
		// Make it loop periodically
		intensity = intensity.sin();
		// Remap it to 0..1 because the intensity can't be negative
		intensity = intensity * 0.5 + 0.5;
		// Scale it to a more reasonable level
		intensity *= 0.015;

		// Set the intensity.
		// This will then be extracted to the render world and uploaded to the gpu automatically by the [`UniformComponentPlugin`]
		setting.intensity = intensity;

		setting.frame_count = frame_count.0;

		setting.resolution = Vec2::new(window.resolution.width(), window.resolution.height());
	}
}