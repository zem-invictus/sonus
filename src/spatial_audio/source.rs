use super::biquad::{BiquadFilter, BiquadMode};
use super::control::{BiquadControl, PlaybackControl};
use crate::spatial_audio::buffer::BlockBuffer;
use crate::spatial_audio::config::AudioParam;
use bevy::audio::Decodable;
use bevy::prelude::{Asset, TypePath};
use rodio::source::Repeat;
use rodio::{Decoder, Source};
use std::f32::consts::FRAC_1_SQRT_2;
use std::io::Cursor;
use std::num::NonZero;
use std::sync::Arc;

#[derive(Asset, TypePath, Clone)]
pub struct SpatialAudioSource {
    pub bytes: Arc<[u8]>,
    pub playback_id: u64,
    pub control: Arc<PlaybackControl>,
}

impl SpatialAudioSource {
    pub fn new(bytes: Arc<[u8]>, playback_id: u64) -> Self {
        Self {
            bytes,
            playback_id,
            control: Arc::new(PlaybackControl::new()),
        }
    }
    pub fn with_lowpass_filter(mut self, cutoff_hz: f32) -> Self {
        if let Some(pl_control) = Arc::get_mut(&mut self.control) {
            pl_control.low_pass = Some(Arc::new(BiquadControl {
                mode: BiquadMode::LowPass,
                cutoff_hz: AudioParam::new(cutoff_hz),
                q: AudioParam::new(FRAC_1_SQRT_2),
                gain_db: None,
            }))
        }
        self
    }
}

impl Decodable for SpatialAudioSource {
    type Decoder = SpatialAudioChain<Repeat<Decoder<Cursor<Arc<[u8]>>>>>;

    fn decoder(&self) -> Self::Decoder {
        let cursor = Cursor::new(self.bytes.clone());
        let raw_decoder = Decoder::new(cursor)
            .expect("Failed to create decoder!")
            .repeat_infinite();
        let channels = raw_decoder.channels().get();
        let sample_rate = raw_decoder.sample_rate().get();

        let low_pass = self.control.low_pass.as_ref().map(|lpf_control| {
            BiquadFilter::new(lpf_control.clone(), channels, sample_rate as f32)
        });

        SpatialAudioChain::new(raw_decoder, self.control.clone(), low_pass)
    }
}

pub struct SpatialAudioChain<I: Source> {
    input: I,
    sample_rate: NonZero<u32>,
    buffer: BlockBuffer,
    control: Arc<PlaybackControl>,

    low_pass: Option<BiquadFilter>,
}

impl<I: Source> SpatialAudioChain<I> {
    pub fn new(input: I, control: Arc<PlaybackControl>, low_pass: Option<BiquadFilter>) -> Self {
        let channels =
            NonZero::new(input.channels().get()).expect("Number of audio source channels is 0!");
        let sample_rate =
            NonZero::new(input.sample_rate().get()).expect("Sample rate of audio source is 0!");
        let buffer = BlockBuffer::new(128, channels);

        Self {
            input,
            sample_rate,
            buffer,
            control,
            low_pass,
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

        let sample_rate = self.sample_rate.get();

        if let Some(lpf) = &mut self.low_pass {
            lpf.update(sample_rate);
            lpf.process(self.buffer.as_mut_slice());
        }

        let volume = self.control.volume.get();

        for sample in self.buffer.as_mut_slice() {
            *sample *= volume;
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
