use std::num::NonZero;
use crate::spatial_audio::buffer::BlockBuffer;
use rodio::Source;

#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    LowPass,
    HighPass,
    Reverb,
}

#[derive(Debug, Clone, Copy)]
pub enum FilterParams {
    LowPass {
        cutoff: f32,
        resonance: f32,
    },
    HighPass {
        cutoff: f32,
    },
    Reverb {
        room_size: f32,
        damping: f32,
        wet: f32,
    },
}

pub trait IntoAudioFilter {
    type Filter: AudioFilter + 'static;
}

pub trait AudioFilter: Send + Sync {
    fn process(&mut self, samples: &mut [f32], channels: u16);
    fn update(&mut self, sample_rate: u32);
}

pub struct SpatialAudioChain<I: Source> {
    input: I,
    sample_rate: NonZero<u32>,
    buffer: BlockBuffer,
    filters: Vec<Box<dyn AudioFilter>>,
}

impl<I: Source> SpatialAudioChain<I> {
    pub fn new(input: I, filters: Vec<Box<dyn AudioFilter>>) -> Self {
        let channels = NonZero::new(input.channels().get()).expect("Number of audio source channels is 0!");
        let sample_rate = NonZero::new(input.sample_rate().get()).expect("Sample rate of audio source is 0!");
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
            filter.process(self.buffer.as_mut_slice(), channels.get());
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
// Реализуем rodio::Source, чтобы проксировать метаданные
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
