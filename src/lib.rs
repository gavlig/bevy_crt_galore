use bevy::prelude::*;

mod endesga;
mod xor;
mod gavlig;

use endesga :: *;
use xor :: *;
use gavlig :: *;

pub use endesga	:: { CrtEndesgaSettings, CrtEndesgaPreset };
pub use xor		:: { CrtXorSettings, CrtXorPreset };
pub use gavlig	:: { CrtGavligSettings, CrtGavligPreset };

pub struct CrtGalorePlugin;

impl Plugin for CrtGalorePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			EndesgaCrtPlugin,
			XorCrtPlugin,
			GavligCrtPlugin,
		));
	}
}

pub(crate) const MIN_AMOUNT	: f32 = 0.001;
pub(crate) const MIN_SCALE	: f32 = 0.01;
