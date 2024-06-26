use std::ops;

use bevy::{
    asset::load_internal_asset,
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
		globals::{GlobalsBuffer, GlobalsUniform},
		texture::BevyDefault,
		view::ViewTarget,
		RenderApp,
	},
};

use super :: *;

// $ uuidgen
pub const ENDESGA_SETTINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0x9a62c467e77c4d8eb486acc975a47304u128);
pub const ENDESGA_PASS0_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0xe280e380bdb74d6b85045cba17e4ba0cu128);
pub const ENDESGA_PASS1_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0xd509e7b0ae5743db8169573727464586u128);
pub const ENDESGA_PASS2_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0xf867645387c7435db10084e000805d0du128);

pub struct EndesgaCrtPlugin;

impl Plugin for EndesgaCrtPlugin {
	fn build(&self, app: &mut App) {
		load_internal_asset!(app, ENDESGA_SETTINGS_SHADER_HANDLE, "settings.wgsl", Shader::from_wgsl);
		load_internal_asset!(app, ENDESGA_PASS0_SHADER_HANDLE, "../../assets/shaders/endesga/pass0.wgsl", Shader::from_wgsl);
		load_internal_asset!(app, ENDESGA_PASS1_SHADER_HANDLE, "../../assets/shaders/endesga/pass1.wgsl", Shader::from_wgsl);
		load_internal_asset!(app, ENDESGA_PASS2_SHADER_HANDLE, "../../assets/shaders/endesga/pass2.wgsl", Shader::from_wgsl);

		app.add_plugins((
			ExtractComponentPlugin::<CrtEndesgaSettings>::default(),
			UniformComponentPlugin::<CrtEndesgaSettings>::default(),
		));

		let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

		render_app
			.add_render_graph_node::<ViewNodeRunner<CrtEndesgaNode>>(
				Core3d,
				CrtEndesgaLabel,
			)
			.add_render_graph_edges(
				Core3d,
				(
					Node3d::EndMainPass,
					CrtEndesgaLabel,
					Node3d::Bloom,
				),
			);
	}

	fn finish(&self, app: &mut App) {
		let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};

		render_app.init_resource::<CrtEndesgaPipeline>();
	}
}

// IMPORTANT! keep this in sync with src/endesga/settings.wgsl
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct CrtEndesgaSettings {
	pub aberration_amount	: f32,
	pub noise_amount		: f32,
	pub vignette_amount		: f32,
	pub rounded_amount		: f32,
	pub pixelate_amount		: f32,
	pub mask_amount			: f32,
	pub distortion_amount	: f32,
	pub glow_amount			: f32,
}

impl CrtEndesgaSettings {
	pub const STRONG : Self = Self {
		aberration_amount	: 0.07,
		noise_amount		: 0.7,
		vignette_amount		: 0.7,
		rounded_amount		: 0.07,
		pixelate_amount		: 0.7,
		mask_amount			: 0.7,
		distortion_amount	: 0.07,
		glow_amount			: 3.0,
	};

    pub const MILD : Self = Self {
		aberration_amount	: 0.003,
		noise_amount		: 0.05,
		vignette_amount		: 0.7,
		rounded_amount		: 0.03,
		pixelate_amount		: 0.01,
		mask_amount			: 0.01,
		distortion_amount	: 0.017,
		glow_amount			: 1.9,
	};

	pub fn new(preset: CrtEndesgaPreset) -> Self {
		match preset {
			CrtEndesgaPreset::Mild	=> CrtEndesgaSettings::MILD,
			CrtEndesgaPreset::Strong=> CrtEndesgaSettings::STRONG,
		}
	}

	pub fn with_scale(mut self, scale: f32) -> Self {
		self = self * scale;
		self
	}

	pub fn without_pixelate(mut self) -> Self {
		self.pixelate_amount = 0.0;
		self.mask_amount = 0.0;
		self
	}

	pub fn set_preset_scaled(&mut self, preset: CrtEndesgaPreset, scale: f32) {
		*self = CrtEndesgaSettings::new(preset).with_scale(scale);
	}
}

impl ops::Mul<f32> for CrtEndesgaSettings {
	type Output = CrtEndesgaSettings;

	fn mul(self, rhs: f32) -> Self::Output {
		let scale = rhs.max(MIN_SCALE);

		let glow_amount = (self.glow_amount * scale).max(1.0);

		Self::Output {
			aberration_amount	: self.aberration_amount	* scale,
			noise_amount		: self.noise_amount			* scale,
			vignette_amount		: self.vignette_amount		* scale,
			rounded_amount		: self.rounded_amount		* scale,
			pixelate_amount		: self.pixelate_amount		* scale,
			mask_amount			: self.mask_amount			* scale,
			distortion_amount	: self.distortion_amount	* scale,
			glow_amount,
		}
    }
}

impl Default for CrtEndesgaSettings {
	fn default() -> Self {
        CrtEndesgaSettings::STRONG
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrtEndesgaPreset {
	Mild,
	Strong
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CrtEndesgaLabel;

#[derive(Default)]
struct CrtEndesgaNode;

impl ViewNode for CrtEndesgaNode {
	type ViewQuery = (
		&'static ViewTarget,
		&'static CrtEndesgaSettings,
	);

	fn run(
		&self,
		_graph: &mut RenderGraphContext,
		render_context: &mut RenderContext,
		(view_target, settings): QueryItem<Self::ViewQuery>,
		world: &World,
	) -> Result<(), NodeRunError> {
		let crt_pipeline = world.resource::<CrtEndesgaPipeline>();

		let pipeline_cache = world.resource::<PipelineCache>();

		let Some(pass0_pipeline) = pipeline_cache.get_render_pipeline(crt_pipeline.pass0_pipeline_id) else { return Ok(()) };
		let Some(pass1_pipeline) = pipeline_cache.get_render_pipeline(crt_pipeline.pass1_pipeline_id) else { return Ok(()) };
		let Some(pass2_pipeline) = pipeline_cache.get_render_pipeline(crt_pipeline.pass2_pipeline_id) else { return Ok(()) };

		let settings_uniforms = world.resource::<ComponentUniforms<CrtEndesgaSettings>>();

		let Some(settings_binding) = settings_uniforms.uniforms().binding() else { return Ok(()) };

		let globals_buffer = world.resource::<GlobalsBuffer>();

		let Some(global_uniforms) = globals_buffer.buffer.binding() else { return Ok(()) };

		let mut envoke_render_pass = |pipeline: &RenderPipeline, name: &str| {
			let post_process = view_target.post_process_write();

			let bind_group = render_context.render_device().create_bind_group(
				"crt_endesga_bind_group",
				&crt_pipeline.layout,
				// It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
				&BindGroupEntries::sequential((
					// Make sure to use the source view
					post_process.source,
					// Use the sampler created for the pipeline
					&crt_pipeline.sampler,
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

		envoke_render_pass(pass0_pipeline, "crt_endesga_pass0");
		if settings.pixelate_amount > MIN_AMOUNT && settings.mask_amount > MIN_AMOUNT {
			envoke_render_pass(pass1_pipeline, "crt_endesga_pass1");
		}
		envoke_render_pass(pass2_pipeline, "crt_endesga_pass2");

		Ok(())
	}
}

#[derive(Resource)]
struct CrtEndesgaPipeline {
	layout				: BindGroupLayout,
	sampler				: Sampler,
	pass0_pipeline_id	: CachedRenderPipelineId,
	pass1_pipeline_id	: CachedRenderPipelineId,
	pass2_pipeline_id	: CachedRenderPipelineId,
}

impl FromWorld for CrtEndesgaPipeline {
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
					uniform_buffer::<CrtEndesgaSettings>(false),
					// Default bevy globals
					uniform_buffer::<GlobalsUniform>(false)
				),
			),
		);

		// We can create the sampler here since it won't change at runtime and doesn't depend on the view
		let sampler = render_device.create_sampler(&SamplerDescriptor::default());

		let shader0 = ENDESGA_PASS0_SHADER_HANDLE.clone();
		let shader1 = ENDESGA_PASS1_SHADER_HANDLE.clone();
		let shader2 = ENDESGA_PASS2_SHADER_HANDLE.clone();

		fn make_pipeline(
			world			: &mut World,
			layout			: &BindGroupLayout,
			shader			: Handle<Shader>,
			pipeline_label	: &'static str,
		) -> CachedRenderPipelineId {
			world
				.resource_mut::<PipelineCache>()
				.queue_render_pipeline(RenderPipelineDescriptor {
					label: Some(pipeline_label.into()),
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

		let pipeline0_id = make_pipeline(world, &layout, shader0, "crt_endesga_pass0_pipeline");
		let pipeline1_id = make_pipeline(world, &layout, shader1, "crt_endesga_pass1_pipeline");
		let pipeline2_id = make_pipeline(world, &layout, shader2, "crt_endesga_pass2_pipeline");

		Self {
			layout,
			sampler,
			pass0_pipeline_id: pipeline0_id,
			pass1_pipeline_id: pipeline1_id,
			pass2_pipeline_id: pipeline2_id,
		}
	}
}
