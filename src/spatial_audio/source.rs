use std::any::TypeId;
use super::biquad::{BiquadCoefficients, BiquadFilter, BiquadState};
use super::control::{BiquadControl, FilterControl, PlaybackControl};
use crate::spatial_audio::buffer::BlockBuffer;
use crate::spatial_audio::config::FilterConfig;
use crate::spatial_audio::filter::{AudioFilter, FilterType};
use bevy::audio::Decodable;
use bevy::prelude::{Asset, TypePath};
use rodio::{Decoder, Source};
use std::collections::HashMap;
use std::io::Cursor;
use std::num::NonZero;
use std::sync::{Arc, mpsc};

#[derive(Asset, TypePath, Clone)]
pub struct SpatialAudioSource {
    pub bytes: Arc<[u8]>,
    pub playback_id: u64,
    pub control: PlaybackControl,
}

impl SpatialAudioSource {
    pub fn add_filter<C: FilterConfig>(mut self, config: C) -> Self {
        let control = Arc::new(config.build_control());
        self.control.filters.insert(TypeId::of::<C::Control>(), control);
        self
    }
}

impl Decodable for SpatialAudioSource {
    type Decoder = SpatialAudioChain<Decoder<Cursor<Arc<[u8]>>>>;

    fn decoder(&self) -> Self::Decoder {
        let cursor = Cursor::new(self.bytes.clone());
        let raw_decoder = Decoder::new(cursor).expect("Failed to create decoder!");
        let channels = raw_decoder.channels().get();
        let sample_rate = raw_decoder.sample_rate().get();

        let mut filters: Vec<Box<dyn AudioFilter>> = Vec::new();

        for control in self.control.filters.values() {
            filters.push(control.clone().build_filter(channels, sample_rate));
        }

        SpatialAudioChain::new(raw_decoder, filters)
    }
}

pub struct SpatialAudioChain<I: Source> {
    input: I,
    sample_rate: NonZero<u32>,
    buffer: BlockBuffer,
    filters: Vec<Box<dyn AudioFilter>>,
}

impl<I: Source> SpatialAudioChain<I> {
    pub fn new(input: I, filters: Vec<Box<dyn AudioFilter>>) -> Self {
        let channels =
            NonZero::new(input.channels().get()).expect("Number of audio source channels is 0!");
        let sample_rate =
            NonZero::new(input.sample_rate().get()).expect("Sample rate of audio source is 0!");
        let buffer = BlockBuffer::new(128, channels);

        Self {
            input,
            sample_rate,
            buffer,
            filters,
        }
    }
    fn fill_and_process_block(&mut self) -> Option<()> {
        self.buffer.clear();

        let total_samples = self.buffer.capacity();

        for _ in 0..total_samples {
            if let Some(sample) = self.input.next() {
                self.buffer.push(sample);
            } else {
                break;
            }
        }

        if self.buffer.is_empty() {
            return None;
        }

        for filter in &mut self.filters {
            filter.update(self.sample_rate.get());
        }

        let channels = self.buffer.channels();

        for filter in &mut self.filters {
            filter.process(self.buffer.as_mut_slice());
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
