#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_crt_galore::CrtGaloreSettings

alias vec2f = vec2<f32>;
alias vec3f = vec3<f32>;
alias vec4f = vec4<f32>;

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: CrtGaloreSettings;

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
	let frag_coord : vec2f = in.uv * settings.resolution.xy;

	var uv = (in.uv * 2.) - 1.;

	let r = length(uv);

	uv /= (2. * settings.distortion_amount * r * r);

	uv = ((uv * (1. - sqrt(1. - 4. * settings.distortion_amount * r * r))) + 1.) / 2.;

	let v : f32 = min(min(uv.x, 1. - uv.x), min(uv.y, 1. - uv.y));

	let AA : f32 = 0.5 * length(vec2f(dpdx(v), dpdy(v)));

	var output = vec4f(0.);
	var weight = array<f32, 7>(0.25, 0.5, 1.0, 2.0, 1.0, 0.5, 0.25);
	for (var x = -3; x <= 3; x += 1) {
		for (var y = -3; y <= 3; y += 1) {
			output += weight[x + 3] * weight[y + 3] *
				textureSample(screen_texture, texture_sampler, uv + vec2f(f32(x), f32(y)) * (1.0 / settings.resolution.xy));
		}
	}

	return mix(
		textureSample(screen_texture, texture_sampler, uv),
		output / 7.,
		settings.bloom_amount * settings.mask_amount
	)
	// barrel-distortion mask
	* smoothstep( -AA, AA, v );

}
