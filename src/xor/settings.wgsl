#define_import_path bevy_crt_galore::xor

struct CrtSettings {
	mask_intensity		: f32,		// RGB Mask intensity(0 to 1)
	mask_size			: f32,		// Mask size (in pixels)
	mask_border			: f32,		// Border intensity (0 to 1)
	aberration_offset	: vec2f,	// Chromatic abberration offset in texels (0 = no aberration)
	screen_curvature	: f32,		// Curvature intensity
	screen_vignette		: f32,		// Screen vignette
	pulse_intensity		: f32,		// Intensity of pulsing animation
	pulse_width			: f32,		// Pulse width in pixels (times tau)
	pulse_rate			: f32,		// Pulse animation speed
	glow_amount			: f32,		// Multiply color by this value to make it emmissive and trigger Bevy's bloom
}