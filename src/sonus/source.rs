//! Rodio source adapter and audio stream processing pipeline.

use crate::sonus::config::{OcclusionControl, SonusControl};
use crate::sonus::dsp::{BlockBuffer, OcclusionAudioChain};
use bevy::audio::Decodable;
use bevy::prelude::{Asset, TypePath};
use rodio::source::Repeat;
use rodio::{Decoder, Source};
use std::io::Cursor;
use std::num::NonZero;
use std::sync::Arc;

/// Decodable audio asset wrapping raw audio bytes and control handles.
#[derive(Asset, TypePath, Clone)]
pub struct SonusSource {
    pub bytes: Arc<[u8]>,
    pub control: Arc<SonusControl>,
}

impl SonusSource {
    /// Creates a new `SonusSource` asset.
    pub fn new(bytes: Arc<[u8]>, control: Arc<SonusControl>) -> Self {
        Self { bytes, control }
    }
}

impl Decodable for SonusSource {
    type Decoder = SpatialAudioChain<Repeat<Decoder<Cursor<Arc<[u8]>>>>>;

    fn decoder(&self) -> Self::Decoder {
        let cursor = Cursor::new(self.bytes.clone());
        let raw_decoder = Decoder::new(cursor)
            .expect("Failed to create decoder!")
            .repeat_infinite();
        let channels = raw_decoder.channels().get();
        let sample_rate = raw_decoder.sample_rate().get();

        let mut chain = SpatialAudioChain::new(raw_decoder, self.control.clone());

        if let Some(occlusion_control) = self.control.occlusion_control.clone() {
            chain.add_occlusion_chain(channels, sample_rate as f32, occlusion_control);
        }

        chain
    }
}

/// Custom Rodio `Source` executing block-based spatial audio processing on the audio thread.
pub struct SpatialAudioChain<I: Source> {
    input: I,
    sample_rate: NonZero<u32>,
    buffer: BlockBuffer,
    control: Arc<SonusControl>,
    occlusion_chain: Option<OcclusionAudioChain>,
}

impl<I: Source> SpatialAudioChain<I> {
    /// Creates a new spatial audio processing chain with a 512-sample buffer.
    pub fn new(input: I, control: Arc<SonusControl>) -> Self {
        let channels =
            NonZero::new(input.channels().get()).expect("Number of audio source channels is 0!");
        let sample_rate =
            NonZero::new(input.sample_rate().get()).expect("Sample rate of audio source is 0!");
        let buffer = BlockBuffer::new(512, channels);

        Self {
            input,
            sample_rate,
            buffer,
            control,
            occlusion_chain: None,
        }
    }

    fn add_occlusion_chain(
        &mut self,
        channels: u16,
        sample_rate: f32,
        control: Arc<OcclusionControl>,
    ) -> &mut Self {
        self.occlusion_chain = Some(OcclusionAudioChain::new(channels, sample_rate, control));
        self
    }

    fn fill_and_process_block(&mut self) -> Option<()> {
        self.buffer.clear();

        self.buffer.fill_from_iter(&mut self.input);

        if self.buffer.is_empty() {
            return None;
        }

        if let Some(occlusion_chain) = &mut self.occlusion_chain {
            occlusion_chain.update();
            occlusion_chain.process(&mut self.buffer);
        }
        Some(())
    }
}

impl<I: Source> Iterator for SpatialAudioChain<I> {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.is_exhausted() {
            self.buffer.clear();
            self.fill_and_process_block()?;
        }

        let sample = self.buffer.pop();
        Some(sample)
    }
}

impl<I: Source> Source for SpatialAudioChain<I> {
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }
    fn channels(&self) -> NonZero<u16> {
        self.buffer.channels()
    }
    fn sample_rate(&self) -> NonZero<u32> {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.input.total_duration()
    }
}
