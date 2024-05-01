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
pub const XOR_SETTINGS_SHADER_HANDLE	: Handle<Shader> = Handle::weak_from_u128(0xd72b3e4dc59d492090d3c531b481c26du128);
pub const XOR_PASS0_SHADER_HANDLE		: Handle<Shader> = Handle::weak_from_u128(0x1f85bffde64d4bc0be5b3906d8d4be9cu128);

pub struct XorCrtPlugin;

impl Plugin for XorCrtPlugin {
	fn build(&self, app: &mut App) {
		load_internal_asset!(app, XOR_SETTINGS_SHADER_HANDLE, "settings.wgsl", Shader::from_wgsl);
		load_internal_asset!(app, XOR_PASS0_SHADER_HANDLE, "../../assets/shaders/xor/pass0.wgsl", Shader::from_wgsl);

		app.add_plugins((
			ExtractComponentPlugin::<CrtXorSettings>::default(),
			UniformComponentPlugin::<CrtXorSettings>::default(),
		));

		let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

		render_app
			.add_render_graph_node::<ViewNodeRunner<CrtXorNode>>(
				Core3d,
				CrtXorLabel,
			)
			.add_render_graph_edges(
				Core3d,
				(
					Node3d::EndMainPass,
					CrtXorLabel,
					Node3d::Bloom,
				),
			);
	}

	fn finish(&self, app: &mut App) {
		let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};

		render_app.init_resource::<CrtXorPipeline>();
	}
}

// IMPORTANT! keep this in sync with src/xor/settings.wgsl
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct CrtXorSettings {
	pub mask_intensity		: f32,		// RGB Mask intensity(0 to 1)
	pub mask_size			: f32,		// Mask size (in pixels)
	pub mask_border			: f32,		// Border intensity (0 to 1)
	pub aberration_offset	: Vec2,		// Chromatic abberration offset in texels (0 = no aberration)
	pub screen_curvature	: f32,		// Curvature intensity
	pub screen_vignette		: f32,		// Screen vignette
	pub pulse_intensity		: f32,		// Intensity of pulsing animation
	pub pulse_width			: f32,		// Pulse width in pixels (times tau)
	pub pulse_rate			: f32,		// Pulse animation speed
	pub glow_amount			: f32,		// Multiply color by this value to make it emmissive and trigger Bevy's bloom
}

impl CrtXorSettings {
	pub const STRONG : Self = Self {
		mask_intensity		: 1.0,
		mask_size			: 12.0,
		mask_border			: 0.8,
		aberration_offset	: Vec2::new(2.0, 0.0),
		screen_curvature	: 0.08,
		screen_vignette		: 0.4,
		pulse_intensity		: 0.03,
		pulse_width			: 6e1,
		pulse_rate			: 2e1,
		glow_amount			: 3.0,
	};

	pub const MILD : Self = Self {
		mask_intensity		: 0.1,
		mask_size			: 2.0,
		mask_border			: 0.2,
		aberration_offset	: Vec2::new(1.0, 0.0),
		screen_curvature	: 0.013,
		screen_vignette		: 0.1,
		pulse_intensity		: 0.03,
		pulse_width			: 60.0,
		pulse_rate			: 5.0,
		glow_amount			: 1.7,
	};
	
	pub fn new(preset: CrtXorPreset) -> Self {
		match preset {
			CrtXorPreset::Mild	=> CrtXorSettings::MILD,
			CrtXorPreset::Strong=> CrtXorSettings::STRONG,
		}
	}

	pub fn with_scale(mut self, scale: f32) -> Self {
		self = self * scale;
		self
	}

	pub fn set_preset_scaled(&mut self, preset: CrtXorPreset, scale: f32) {
		*self = CrtXorSettings::new(preset).with_scale(scale);
	}
}

impl ops::Mul<f32> for CrtXorSettings {
	type Output = CrtXorSettings;

	fn mul(self, rhs: f32) -> Self::Output {
		let scale = rhs.max(MIN_SCALE);

		let glow_amount = (self.glow_amount * scale).max(1.0);

		Self::Output {
			mask_intensity		: self.mask_intensity		* scale,
			mask_size			: self.mask_size			* scale,
			mask_border			: self.mask_border			* scale,
			aberration_offset	: self.aberration_offset	* scale,
			screen_curvature	: self.screen_curvature		* scale,
			screen_vignette		: self.screen_vignette		* scale,
			pulse_intensity		: self.pulse_intensity		* scale,
			pulse_width			: self.pulse_width			* scale,
			pulse_rate			: self.pulse_rate			* scale,
			glow_amount,
		}
    }
}

impl Default for CrtXorSettings {
	fn default() -> Self {
        CrtXorSettings::STRONG
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrtXorPreset {
	Mild,
	Strong
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CrtXorLabel;

#[derive(Default)]
struct CrtXorNode;

impl ViewNode for CrtXorNode {
	type ViewQuery = (
		&'static ViewTarget,
		&'static CrtXorSettings,
	);

	fn run(
		&self,
		_graph: &mut RenderGraphContext,
		render_context: &mut RenderContext,
		(view_target, _settings): QueryItem<Self::ViewQuery>,
		world: &World,
	) -> Result<(), NodeRunError> {
		let crt_pipeline = world.resource::<CrtXorPipeline>();

		let pipeline_cache = world.resource::<PipelineCache>();

		let Some(pass0_pipeline) = pipeline_cache.get_render_pipeline(crt_pipeline.pass0_pipeline_id) else { return Ok(()) };

		let settings_uniforms = world.resource::<ComponentUniforms<CrtXorSettings>>();

		let Some(settings_binding) = settings_uniforms.uniforms().binding() else { return Ok(()) };

		let globals_buffer = world.resource::<GlobalsBuffer>();

		let Some(global_uniforms) = globals_buffer.buffer.binding() else { return Ok(()) };

		let mut envoke_render_pass = |pipeline: &RenderPipeline, name: &str| {
			let post_process = view_target.post_process_write();

			let bind_group = render_context.render_device().create_bind_group(
				"crt_xor_bind_group",
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

		envoke_render_pass(pass0_pipeline, "crt_xor_pass0");

		Ok(())
	}
}

#[derive(Resource)]
struct CrtXorPipeline {
	layout				: BindGroupLayout,
	sampler				: Sampler,
	pass0_pipeline_id	: CachedRenderPipelineId,
}

impl FromWorld for CrtXorPipeline {
	fn from_world(world: &mut World) -> Self {
		let render_device = world.resource::<RenderDevice>();

		let layout = render_device.create_bind_group_layout(
			"crt_xor_bind_group_layout",
			&BindGroupLayoutEntries::sequential(
				ShaderStages::FRAGMENT,
				(
					// The screen texture
					texture_2d(TextureSampleType::Float { filterable: true }),
					// The screen texture sampler
					sampler(SamplerBindingType::Filtering),
					// The settings uniform that will control the effect
					uniform_buffer::<CrtXorSettings>(false),
					// Default bevy globals
					uniform_buffer::<GlobalsUniform>(false)
				),
			),
		);

		// We can create the sampler here since it won't change at runtime and doesn't depend on the view
		let sampler = render_device.create_sampler(&SamplerDescriptor::default());

		let shader0 = XOR_PASS0_SHADER_HANDLE.clone();

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

		let pipeline0_id = make_pipeline(world, &layout, shader0, "crt_xor_pass0_pipeline");

		Self {
			layout,
			sampler,
			pass0_pipeline_id: pipeline0_id,
		}
	}
}