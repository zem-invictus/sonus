use crate::spatial_audio::control::{BiquadControl, FilterControl, LowPassControl};

pub trait FilterConfig: 'static {
    type Control: FilterControl;
    fn build_control(self) -> Self::Control;
}

#[derive(Clone, Copy)]
pub struct LowPassConfig {
    cutoff: f32,
    resonance: f32,
}

impl FilterConfig for LowPassConfig {
    type Control = LowPassControl;

    fn build_control(self) -> Self::Control {
        LowPassControl::new(self.cutoff, self.resonance)
    }
}