#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::globals::Globals
#import bevy_crt_galore::gavlig::CrtSettings

alias vec2f = vec2<f32>;
alias vec3f = vec3<f32>;
alias vec4f = vec4<f32>;

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: CrtSettings;
@group(0) @binding(3) var<uniform> globals: Globals;

fn calc_grain(st: vec2<f32>) -> f32 {
    return fract(sin(dot(st.xy, vec2(17.0,180.)))* 2500. + globals.time);
}

const STRENGTH_NORMALIZER = 9.0;
const STRENGTH_NORMALIZER_SQ = STRENGTH_NORMALIZER * STRENGTH_NORMALIZER;

fn calc_vignette(uv: vec2<f32>) -> f32 {
	// roughly making it so that strength 1.0 is almost black screen and 0.5 is acceptable
	let vignette_strength_inv = clamp(1.0 - settings.vignette_strength, 0.0000001, 0.99);
	let vignette_strength_inv_sq = vignette_strength_inv * vignette_strength_inv;
	let vignette_coef = vignette_strength_inv_sq * STRENGTH_NORMALIZER_SQ;

	// square distance to the edge
    let edge = uv * (1. - uv) * vignette_coef;
    var norm = edge.x * edge.y;

    return saturate(norm);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
	let sample = textureSample(screen_texture, texture_sampler, in.uv);

	let color = sample.rgb;

	let vignette_alpha_inv = 1.0 - settings.vignette_alpha;

	let vignette = saturate(calc_vignette(in.uv) + vignette_alpha_inv);

	var glow = 1.0;
	if length(color) > settings.glow_threshold { glow = settings.glow_strength; }

    let grain = vec3<f32>(calc_grain(in.uv));

	return vec4<f32>(mix(color, grain, settings.grain_strength * 0.1) * vignette * glow, sample.a);
}