//! Rodio source adapter and audio stream processing pipeline.

use crate::config::{AttenuationControl, OcclusionControl, PanningControl, SonusControl};
use crate::dsp::{AttenuationChain, BlockBuffer, OcclusionChain, PanningChain};
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

        let mut chain = SpatialAudioChain::new(raw_decoder);

        if let Some(occlusion_control) = self.control.occlusion_control.clone() {
            chain.add_occlusion_chain(channels, sample_rate as f32, occlusion_control);
        }

        if let Some(attenuation_control) = self.control.attenuation_control.clone() {
            chain.add_attenuation_chain(attenuation_control);
        }

        if let Some(panning_control) = self.control.panning_control.clone() {
            chain.add_panning_chain(panning_control);
        }

        chain
    }
}

/// Custom Rodio `Source` executing block-based spatial audio processing on the audio thread.
pub struct SpatialAudioChain<I: Source> {
    input: I,
    sample_rate: NonZero<u32>,
    input_buffer: BlockBuffer,
    output_buffer: Option<BlockBuffer>,
    occlusion_chain: Option<OcclusionChain>,
    attenuation_chain: Option<AttenuationChain>,
    panning_chain: Option<PanningChain>,
}

impl<I: Source> SpatialAudioChain<I> {
    /// Creates a new spatial audio processing chain with a 512-sample buffer.
    pub fn new(input: I) -> Self {
        let channels =
            NonZero::new(input.channels().get()).expect("Number of audio source channels is 0!");
        let sample_rate =
            NonZero::new(input.sample_rate().get()).expect("Sample rate of audio source is 0!");
        let input_buffer = BlockBuffer::new(512, channels);

        Self {
            input,
            sample_rate,
            input_buffer,
            output_buffer: None,
            occlusion_chain: None,
            attenuation_chain: None,
            panning_chain: None,
        }
    }

    fn add_occlusion_chain(
        &mut self,
        channels: u16,
        sample_rate: f32,
        control: Arc<OcclusionControl>,
    ) -> &mut Self {
        self.occlusion_chain = Some(OcclusionChain::new(channels, sample_rate, control));
        self
    }

    fn add_attenuation_chain(&mut self, control: Arc<AttenuationControl>) -> &mut Self {
        self.attenuation_chain = Some(AttenuationChain::new(control));
        self
    }

    fn add_panning_chain(&mut self, control: Arc<PanningControl>) -> &mut Self {
        if self.input_buffer.channels().get() == 1 {
            let stereo_channels = NonZero::new(2).unwrap();
            self.output_buffer = Some(BlockBuffer::new(512, stereo_channels));
        }
        self.panning_chain = Some(PanningChain::new(control));
        self
    }

    fn fill_and_process_block(&mut self) -> Option<()> {
        self.input_buffer.clear();
        self.input_buffer.fill_from_iter(&mut self.input);

        if self.input_buffer.is_empty() {
            return None;
        }

        if let Some(occlusion_chain) = &mut self.occlusion_chain {
            occlusion_chain.update();
            occlusion_chain.process(&mut self.input_buffer);
        }

        if let Some(attenuation_chain) = &mut self.attenuation_chain {
            attenuation_chain.update();
            attenuation_chain.process(&mut self.input_buffer);
        }

        if let Some(panning_chain) = &mut self.panning_chain {
            panning_chain.update();
            if self.input_buffer.channels().get() == 1 {
                if let Some(output_buffer) = &mut self.output_buffer {
                    panning_chain.process_mono_to_stereo(&self.input_buffer, output_buffer);
                }
            } else {
                panning_chain.process_stereo(&mut self.input_buffer);
            }
        }

        Some(())
    }
}

impl<I: Source> Iterator for SpatialAudioChain<I> {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let active_buffer = if let Some(out_buf) = &mut self.output_buffer {
            out_buf
        } else {
            &mut self.input_buffer
        };

        if active_buffer.is_exhausted() {
            self.fill_and_process_block()?;
        }

        let active_buffer = if let Some(out_buf) = &mut self.output_buffer {
            out_buf
        } else {
            &mut self.input_buffer
        };

        Some(active_buffer.pop())
    }
}

impl<I: Source> Source for SpatialAudioChain<I> {
    fn current_span_len(&self) -> Option<usize> {
        if self.output_buffer.is_some() {
            self.input.current_span_len().map(|len| len * 2)
        } else {
            self.input.current_span_len()
        }
    }
    fn channels(&self) -> NonZero<u16> {
        if self.output_buffer.is_some() {
            NonZero::new(2).unwrap()
        } else {
            self.input_buffer.channels()
        }
    }
    fn sample_rate(&self) -> NonZero<u32> {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.input.total_duration()
    }
}
