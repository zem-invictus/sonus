use super::biquad::{BiquadCoefficients, BiquadState};
use super::control::{BiquadControl, PlaybackControl, PlaybackRegistration};
use crate::spatial_audio::filter::{BiquadFilter, FilterType};
use bevy::audio::Decodable;
use bevy::prelude::{Asset, TypePath};
use rodio::{Decoder, Source};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, mpsc};

#[derive(Asset, TypePath, Clone)]
pub struct SpatialAudioSource {
    pub bytes: Arc<[u8]>,
    pub playback_id: u64,
    pub control: PlaybackControl,
}

impl AsRef<[u8]> for SpatialAudioSource {
    fn as_ref(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl Decodable for SpatialAudioSource {
    type Decoder = Decoder<Cursor<SpatialAudioSource>>;

    fn decoder(&self) -> Self::Decoder {
        let cursor = Cursor::new(self.bytes.clone());

        let raw_decoder = Decoder::new(cursor).expect("Failed to create decoder!");

        let channels = raw_decoder.channels().get();

        let sample_rate = raw_decoder.sample_rate().get();

        let mut source: BoxedAudioSource = Box::new(raw_decoder);

        let use_low_pass = self.config.get("low_pass").copied().unwrap_or(false);

        let mut control_panel = PlaybackControl {
            biquad: None,
            reverb: None,
        };

        if use_low_pass {
            let control = BiquadControl::new(20000.0, 1.0);
            control_panel.biquad = Some(control.clone());
            let initial_cutoff = 20000.0;
            let coeffs = BiquadCoefficients::low_pass(
                initial_cutoff,
                sample_rate as f32,
                std::f32::consts::FRAC_1_SQRT_2,
            );
            let channel_states = vec![BiquadState::default(); channels as usize];

            source = Box::new(BiquadFilter {
                inner: source,
                control,
                channels,
                channel_states,
                sample_rate,
                current_cutoff_hz: initial_cutoff,
                coeffs,
                sample_counter: 0,
            });
        }

        source
    }
}
