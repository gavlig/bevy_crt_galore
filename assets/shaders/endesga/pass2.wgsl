// REALISTIC SUB-PIXEL OLD CRT by ENDESGA shadertoy: https://www.shadertoy.com/view/ms2fDV (Image)
// Adapted to WGSL for Bevy by gavlig

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::globals::Globals
#import bevy_crt_galore::endesga::CrtSettings

alias vec2f = vec2<f32>;
alias vec3f = vec3<f32>;
alias vec4f = vec4<f32>;

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: CrtSettings;
@group(0) @binding(3) var<uniform> globals: Globals;

fn modulo(a: f32, b: f32) -> f32 {
	var m = a % b;
	if (m < 0.0) {
		if (b < 0.0) {
			m -= b;
		} else {
			m += b;
		}
	}
	return m;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let resolution = vec2f(textureDimensions(screen_texture));

	let frag_coord : vec2f = in.uv * resolution.xy;

	var uv = (in.uv * 2.) - 1.;

	let r = length(uv);

	uv /= (2. * settings.distortion_amount * r * r);

	uv = ((uv * (1. - sqrt(1. - 4. * settings.distortion_amount * r * r))) + 1.) / 2.;

	let v : f32 = min(min(uv.x, 1. - uv.x), min(uv.y, 1. - uv.y));

	let AA : f32 = 0.5 * length(vec2f(dpdx(v), dpdy(v)));

	return textureSample(screen_texture, texture_sampler, uv) * settings.glow_amount
	// barrel-distortion mask
	* smoothstep( -AA, AA, v );

}