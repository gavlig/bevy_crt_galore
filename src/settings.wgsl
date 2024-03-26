#define_import_path bevy_crt_galore

struct CrtGaloreSettings {
	frame_count			: u32,
	resolution			: vec2f,
	aberration_amount	: f32,
	noise_amount		: f32,
	vignette_amount		: f32,
	pixelate_amount		: f32,
	mask_amount			: f32,
	distortion_amount	: f32,
	bloom_amount		: f32,
}