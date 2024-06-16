use crate::ffi;

pub struct Conf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
	Bottom,
	Top
}
impl Position {
	fn is_top(self) -> bool {
		self == Position::Top
	}
	fn is_bottom(self) -> bool {
		self == Position::Bottom
	}
}

impl Conf {
	pub fn is_display_widescreen() -> bool {
		unsafe { ffi::CONF_GetAspectRatio() == ffi::CONF_ASPECT_16_9 as _ }
	}
	pub fn sensor_bar_position() -> Position {
		if unsafe { ffi::CONF_GetSensorBarPosition() } == ffi::CONF_SENSORBAR_TOP as _ {
			Position::Top
		} else {
			Position::Bottom
		}
	}
}

// TODO: There's a lot of stuff in libogc's conf.c, but I don't understand most of it