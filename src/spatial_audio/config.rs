use crate::spatial_audio::control::{BiquadControl, FilterControl, LowPassControl};

pub trait FilterConfig: 'static {
    type Control: FilterControl;
    fn build_control(self) -> Self::Control;
}

#[derive(Clone, Copy)]
pub struct LowPassConfig {
    pub(crate) cutoff: f32,
    pub(crate) volume: f32,
}

impl FilterConfig for LowPassConfig {
    type Control = LowPassControl;

    fn build_control(self) -> Self::Control {
        Self::Control::new(self.cutoff, self.volume)
    }
}