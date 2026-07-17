use bevy::asset::Handle;
use bevy::audio::AudioSource;
use bevy::prelude::Component;

#[derive(Component)]
pub struct SonusEmitter {
    pub source: Handle<AudioSource>,

    pub(crate) use_occlusion: bool,
}

impl SonusEmitter {
    pub fn new(source: Handle<AudioSource>) -> Self {
        Self {
            source,
            use_occlusion: false,
        }
    }

    pub fn with_occlusion(mut self) -> Self {
        self.use_occlusion = true;
        self
    }
}
