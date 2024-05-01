
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

use std::ops;

use super::*;

// $ uuidgen
pub const GAVLIG_SETTINGS_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0x70bd17f74d8647efb71db80a90f49adfu128);
pub const GAVLIG_PASS0_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0xe259305642954a56a21724ad34e39ba9u128);

pub struct GavligCrtPlugin;

impl Plugin for GavligCrtPlugin {
	fn build(&self, app: &mut App) {
		load_internal_asset!(app, GAVLIG_SETTINGS_SHADER_HANDLE, "settings.wgsl", Shader::from_wgsl);
		load_internal_asset!(app, GAVLIG_PASS0_SHADER_HANDLE, "../../assets/shaders/gavlig/pass0.wgsl", Shader::from_wgsl);

		app.add_plugins((
			ExtractComponentPlugin::<CrtGavligSettings>::default(),
			UniformComponentPlugin::<CrtGavligSettings>::default(),
		));

		let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

		render_app
			.add_render_graph_node::<ViewNodeRunner<CrtGavligNode>>(
				Core3d,
				CrtGavligLabel,
			)
			.add_render_graph_edges(
				Core3d,
				(
					Node3d::EndMainPass,
					CrtGavligLabel,
					Node3d::Bloom,
				),
			);
	}

	fn finish(&self, app: &mut App) {
		let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};

		render_app.init_resource::<CrtGavligPipeline>();
	}
}

// IMPORTANT! keep this in sync with src/gavlig/settings.wgsl
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct CrtGavligSettings {
	pub vignette_strength	: f32,
	pub vignette_alpha		: f32,
	pub glow_threshold		: f32,
	pub glow_strength		: f32,
	pub grain_strength		: f32,
}

impl CrtGavligSettings {
	pub const MILD : Self = Self {
		vignette_strength	: 0.4,
		vignette_alpha		: 1.0,
		glow_threshold		: 0.5,
		glow_strength		: 2.4,
		grain_strength		: 0.2,
	};
		
	pub fn new(preset: CrtGavligPreset) -> Self {
		match preset {
			CrtGavligPreset::Mild => CrtGavligSettings::MILD,
		}
	}

	pub fn with_scale(mut self, scale: f32) -> Self {
		self = self * scale;
		self
	}

	pub fn set_preset_scaled(&mut self, preset: CrtGavligPreset, scale: f32) {
		*self = CrtGavligSettings::new(preset).with_scale(scale);
	}
}

impl ops::Mul<f32> for CrtGavligSettings {
	type Output = CrtGavligSettings;

	fn mul(self, rhs: f32) -> Self::Output {
		let scale = rhs.max(MIN_SCALE);

		let glow_threshold = self.glow_threshold;
		let glow_strength = (self.glow_strength * scale).max(1.0);

		// let vignette_strength = CrtGavligSettings::ZERO.vignette_strength.lerp(self.vignette_strength, scale);

		// println!("vigalpha: {:.1} vigstr: {:.1} scale: {:.1}", self.vignette_alpha * scale, vignette_strength, scale);

		Self::Output {
			vignette_strength		: self.vignette_strength		* scale,
			vignette_alpha			: self.vignette_alpha			* scale,
			grain_strength			: self.grain_strength			* scale,
			glow_threshold,
			glow_strength,
		}
    }
}

impl Default for CrtGavligSettings {
	fn default() -> Self {
        CrtGavligSettings::MILD
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrtGavligPreset {
	Mild,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CrtGavligLabel;

#[derive(Default)]
struct CrtGavligNode;

impl ViewNode for CrtGavligNode {
	type ViewQuery = (
		&'static ViewTarget,
		&'static CrtGavligSettings,
	);

	fn run(
		&self,
		_graph: &mut RenderGraphContext,
		render_context: &mut RenderContext,
		(view_target, _settings): QueryItem<Self::ViewQuery>,
		world: &World,
	) -> Result<(), NodeRunError> {
		let crt_pipeline = world.resource::<CrtGavligPipeline>();

		let pipeline_cache = world.resource::<PipelineCache>();

		let Some(pass0_pipeline) = pipeline_cache.get_render_pipeline(crt_pipeline.pass0_pipeline_id) else { return Ok(()) };

		let settings_uniforms = world.resource::<ComponentUniforms<CrtGavligSettings>>();

		let Some(settings_binding) = settings_uniforms.uniforms().binding() else { return Ok(()) };

		let globals_buffer = world.resource::<GlobalsBuffer>();

		let Some(global_uniforms) = globals_buffer.buffer.binding() else { return Ok(()) };

		let mut envoke_render_pass = |pipeline: &RenderPipeline, name: &str| {
			let post_process = view_target.post_process_write();

			let bind_group = render_context.render_device().create_bind_group(
				"crt_gavlig_bind_group",
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

		envoke_render_pass(pass0_pipeline, "crt_gavlig_pass0");

		Ok(())
	}
}

#[derive(Resource)]
struct CrtGavligPipeline {
	layout				: BindGroupLayout,
	sampler				: Sampler,
	pass0_pipeline_id	: CachedRenderPipelineId,
}

impl FromWorld for CrtGavligPipeline {
	fn from_world(world: &mut World) -> Self {
		let render_device = world.resource::<RenderDevice>();

		let layout = render_device.create_bind_group_layout(
			"crt_gavlig_bind_group_layout",
			&BindGroupLayoutEntries::sequential(
				ShaderStages::FRAGMENT,
				(
					// The screen texture
					texture_2d(TextureSampleType::Float { filterable: true }),
					// The screen texture sampler
					sampler(SamplerBindingType::Filtering),
					// The settings uniform that will control the effect
					uniform_buffer::<CrtGavligSettings>(false),
					// Default bevy globals
					uniform_buffer::<GlobalsUniform>(false)
				),
			),
		);

		// We can create the sampler here since it won't change at runtime and doesn't depend on the view
		let sampler = render_device.create_sampler(&SamplerDescriptor::default());

		let shader0 = GAVLIG_PASS0_SHADER_HANDLE.clone();

		fn make_pipeline(
			world			: &mut World,
			layout			: &BindGroupLayout,
			shader			: Handle<Shader>,
			pipeline_label	: &'static str
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

		let pipeline0_id = make_pipeline(world, &layout, shader0, "crt_gavlig_pass0_pipeline");

		Self {
			layout,
			sampler,
			pass0_pipeline_id: pipeline0_id,
		}
	}
}