// GM Shaders: CRT by Xor shadertoy: https://www.shadertoy.com/view/DtscRf (Buffer B)
// Adapted to WGSL for Bevy by gavlig

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::globals::Globals
#import bevy_crt_galore::xor::CrtSettings

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

	let frag_coord: vec2f = in.uv * resolution.xy;
	
	//Signed uv coordinates (ranging from -1 to +1)
	var uv : vec2f = frag_coord / resolution * 2.0 - 1.0;
	//Scale inward using the square of the distance
	uv *= 1.0 + (dot(uv, uv) - 1.0) * settings.screen_curvature;
	//Convert back to pixel coordinates
	let pixel : vec2f = (uv * 0.5 + 0.5) * resolution;

	//Square distance to the edge
	let edge : vec2f = max(1.0 - uv * uv, vec2f(0.0));
	//Compute vignette from x/y edges
	let vignette : f32 = pow(edge.x * edge.y, settings.screen_vignette);

	// RGB cell and subcell coordinates
	let coord : vec2f = pixel / settings.mask_size;
	let subcoord : vec2f = coord * vec2(3,1);
	//Offset for staggering every other cell
	let cell_offset : vec2f = vec2f(0, fract(floor(coord.x) * 0.5));
    
	//Pixel coordinates rounded to the nearest cell
	let mask_coord : vec2f = floor(coord + cell_offset) * settings.mask_size;

	//Chromatic aberration
	var aberration : vec4f	= textureSample(screen_texture, texture_sampler, (mask_coord - settings.aberration_offset) / resolution);
	//Color shift the green channel
	aberration.g			= textureSample(screen_texture, texture_sampler, (mask_coord + settings.aberration_offset) / resolution).g;
   
	//Output color with chromatic aberration
	var color : vec3f = aberration.rgb;
    
	//Compute the RGB color index from 0 to 2
	let ind : f32 = modulo(floor(subcoord.x), 3.0);
	//Convert that value to an RGB color (multiplied to maintain brightness)
	var mask_color = vec3f(f32(ind == 0.0), f32(ind == 1.0), f32(ind == 2.0)) * 3.0;
    
	//Signed subcell uvs (ranging from -1 to +1)
	let cell_uv : vec2f = fract(subcoord + cell_offset) * 2.0 - 1.0;
	//X and y borders
	let border : vec2f = 1.0 - cell_uv * cell_uv * settings.mask_border;
	//Blend x and y mask borders
	mask_color *= border.x * border.y;
	//Blend with color mask
	color *= 1.0 + (mask_color - 1.0) * settings.mask_intensity;  
    
	//Apply vignette
	color *= vignette;
	//Apply pulsing glow
	color *= 1.0 + settings.pulse_intensity * cos(pixel.x / settings.pulse_width + globals.time * settings.pulse_rate);
	//Glow
	color *= settings.glow_amount;

    return vec4f(color, aberration.w);
};
