// REALISTIC SUB-PIXEL OLD CRT by ENDESGA shadertoy: https://www.shadertoy.com/view/ms2fDV (Buffer B)
// Adapted to WGSL for Bevy by gavlig

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::globals::Globals
#import bevy_crt_galore::CrtGaloreSettings

alias vec2f = vec2<f32>;
alias vec3f = vec3<f32>;
alias vec4f = vec4<f32>;

const X = vec3f( 0.0 );
const R = vec3f( 1.0, 0.0, 0.0 );
const G = vec3f( 0.0, 1.0, 0.0 );
const B = vec3f( 0.0, 0.0, 1.0 );
// var M = array<vec3f, 28>( X, X, X, X, X, X, X, X, R, R, G, G, B, B, X, R, R, G, G, B, B, X, R, R, G, G, B, B );

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: CrtGaloreSettings;
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

	var uv : vec2f = floor(in.uv * (resolution.xy / vec2f(7., 4.)));

	let hex_offset : f32 = modulo(uv.x, 2.0) * 2.;

	uv.y += floor(modulo(frag_coord.y, 4.) / 2.) * hex_offset * .5;

	// 7x4 pixelation
	var output : vec4f = vec4f(0.0);

	for(var y = 0.0; y < 4.; y += 1.0) {
		for(var x = 0.0; x < 7.; x += 1.0) {
			output += textureSample(screen_texture, texture_sampler, ((uv * vec2f(7., 4.)) + vec2f(x, y)) / resolution.xy);
		}
	}

	output = mix(textureSample(screen_texture, texture_sampler, frag_coord / resolution.xy), output / 28., settings.pixelate_amount);

	// this should be const, waiting for resolution of this: https://github.com/gfx-rs/wgpu/issues/4337
	var M = array<vec3f, 28>( X, X, X, X, X, X, X, X, R, R, G, G, B, B, X, R, R, G, G, B, B, X, R, R, G, G, B, B );

	let output_rgb = output.rgb * mix(
		vec3(1.),
		// 7x4 sub-pixel RGB mask
		M[i32(
			i32(modulo(frag_coord.y + hex_offset, 4.0)) * 7 +
			i32(modulo(frag_coord.x, 7.0))
		)],
		settings.mask_amount
	);

	return vec4f(output_rgb, output.w);
}
