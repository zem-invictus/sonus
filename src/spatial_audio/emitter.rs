use std::sync::Arc;
use crate::spatial_audio::control::{SonusControl, OcclusionControl};
use crate::spatial_audio::config::AudioParam;
use bevy::asset::Handle;
use bevy::audio::AudioSource;
use bevy::prelude::Component;

#[derive(Clone, Debug)]
pub(crate) enum SonusSourceInput {
    Path(String),
    AudioHandle(Handle<AudioSource>),
}

impl From<&str> for SonusSourceInput {
    fn from(path: &str) -> Self {
        Self::Path(path.to_string())
    }
}

impl From<String> for SonusSourceInput {
    fn from(path: String) -> Self {
        Self::Path(path)
    }
}

impl From<Handle<AudioSource>> for SonusSourceInput {
    fn from(handle: Handle<AudioSource>) -> Self {
        Self::AudioHandle(handle)
    }
}

#[derive(Component)]
pub struct SonusEmitter {
    pub(crate) source: SonusSourceInput,
    pub(crate) control: Arc<SonusControl>,
    pub(crate) use_occlusion: bool,
}

impl SonusEmitter {
    pub fn new(source: impl Into<SonusSourceInput>) -> Self {
        Self {
            source: source.into(),
            control: Arc::new(SonusControl::new()),
            use_occlusion: false,
        }
    }

    pub(crate) fn update_handle_status(&mut self, source: impl Into<SonusSourceInput>) {
        self.source = source.into();
    }

    pub fn with_occlusion(mut self) -> Self {
        self.use_occlusion = true;
        self.control = Arc::new(SonusControl {
            occlusion_control: Some(Arc::new(OcclusionControl {
                lowpass_hz: AudioParam::new(20000.0),
                highpass_hz: AudioParam::new(20.0),
            })),
        });
        self
    }
}
