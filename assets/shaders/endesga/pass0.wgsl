#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

const aberration_amount = 0.07;
const noise_amount = 0.3;
const vignette_amount = 0.7;

alias vec2f = vec2<f32>;
alias vec3f = vec3<f32>;
alias vec4f = vec4<f32>;

struct BevyGaloreSettings {
	frame_count: u32,
	intensity: f32,
	resolution: vec2f
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: BevyGaloreSettings;

fn hash(p: vec3f) -> f32 {
	var p_var = p;
	p_var = fract(p_var * 0.1031);
	p_var = p_var + (dot(p_var, p_var.yzx + 19.19));
	return fract((p_var.x + p_var.y) * p_var.z);
}

fn noise(x: vec3f) -> f32 {
	let p: vec3f = floor(x);
	let f: vec3f = fract(x);
	let m: vec3f = f * f * (3. - 2. * f);
	let i: vec3f = p + vec3f(1., 0., 0.);
	let hash: vec4f = vec4f(hash(p), hash(i), hash(p + vec3f(0., 1., 0.)), hash(i + vec3f(0., 1., 0.)));
	return mix(mix(hash.x, hash.y, m.x), mix(hash.z, hash.w, m.x), m.y);
}

fn grain(x: vec3f) -> f32 {
	return 0.5 + (4. * noise(x) - noise(x + 1.) + noise(x - 1.)) / 4.;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
	let aber_dis: vec2f = (in.uv - vec2f(0.5)) * aberration_amount * length(in.uv - 0.5);
	let aberration: vec3f = vec3f(
		textureSample(screen_texture, texture_sampler, in.uv).r,
		textureSample(screen_texture, texture_sampler, in.uv - aber_dis).g,
		textureSample(screen_texture, texture_sampler, in.uv - 2. * aber_dis).b
	);

	let frag_coord: vec2f = in.uv * settings.resolution.xy;

 	let radius: f32 = 0.07 * ((settings.resolution.x + settings.resolution.y) * 0.5) * 0.5;

	let screen_ratio_y = settings.resolution.y / settings.resolution.x;

	let vignette_step = smoothstep(
		0.25,
		1.0,
		length((in.uv - vec2f(0.5)) * vec2f(1.0, screen_ratio_y * 2.0))
	);

	let vignette = mix(
		1.0,
		1.0 - clamp(vignette_step, 0.0, 1.0),
		vignette_amount
	);

	let half_res = settings.resolution / 2.0;

	let rounded_corners = step(
		length(
			max(
				vec2f(0.0),
				abs(frag_coord - half_res) - half_res + radius
			)
		) - radius,
		0.0
	);

	let frame: f32 = floor(f32(settings.frame_count));
	let rgb_grain = vec3f(
		grain(vec3f(frag_coord, frame)),
		grain(vec3f(frag_coord, frame + 9.0)),
		grain(vec3f(frag_coord, frame - 9.0))
	);

	let aberration_wgrain = mix(aberration, mix(aberration * rgb_grain, aberration + (rgb_grain - 1.0), 0.5), noise_amount);

	return vec4f(aberration_wgrain * vignette * rounded_corners, 1.0);
}
