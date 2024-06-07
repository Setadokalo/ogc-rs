use crate::ffi;

pub struct Conf;

impl Conf {
	pub fn is_display_widescreen() -> bool {
		unsafe { ffi::CONF_GetAspectRatio() == ffi::CONF_ASPECT_16_9 as _ }
	}
}

// TODO: There's a lot of stuff in libogc's conf.c, but I don't understand most of it