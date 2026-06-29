use bevy::audio::Decodable;
use bevy::prelude::{Asset, TypePath};
use hound::{WavReader, WavSpec, WavWriter};
use rodio::{ChannelCount, Decoder, SampleRate, Source};
use std::io::Cursor;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, mpsc};
use std::time::Duration;

type InnerDecoder = Decoder<Cursor<Arc<[u8]>>>;

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoefficients {
    pub fn low_pass(cutoff_hz: f32, sample_rate: f32, q: f32) -> Self {
        let cutoff = cutoff_hz.clamp(20.0, sample_rate * 0.49);
        let omega = 2.0 * std::f32::consts::PI * cutoff / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);
        let a0 = 1.0 + alpha;

        Self {
            b0: ((1.0 - cos_w) / 2.0) / a0,
            b1: (1.0 - cos_w) / a0,
            b2: ((1.0 - cos_w) / 2.0) / a0,
            a1: (-2.0 * cos_w) / a0,
            a2: (1.0 - alpha) / a0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    s1: f32,
    s2: f32,
}

impl BiquadState {
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32, coeffs: &BiquadCoefficients) -> f32 {
        let output = coeffs.b0 * input + self.s1;
        self.s1 = coeffs.b1 * input - coeffs.a1 * output + self.s2;
        self.s2 = coeffs.b2 * input - coeffs.a2 * output;
        output
    }
}

#[derive(Clone)]
pub struct DecoderControl {
    cutoff_hz: Arc<AtomicU32>,
    volume: Arc<AtomicU32>,
}

impl DecoderControl {
    pub fn new(cutoff_hz: f32, volume: f32) -> Self {
        Self {
            cutoff_hz: Arc::new(AtomicU32::new(cutoff_hz.to_bits())),
            volume: Arc::new(AtomicU32::new(volume.to_bits())),
        }
    }
    pub fn set_cutoff(&self, cutoff_hz: f32) {
        self.cutoff_hz.store(cutoff_hz.to_bits(), Relaxed);
    }
    pub fn get_cutoff(&self) -> f32 {
        f32::from_bits(self.cutoff_hz.load(Relaxed))
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume.store(volume.to_bits(), Relaxed);
    }
    pub fn get_volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Relaxed))
    }
}

pub struct PlaybackRegistration {
    pub playback_id: u64,
    pub control: DecoderControl,
}

#[derive(Asset, TypePath, Clone)]
pub struct SpatialAudioSource {
    pub bytes: Arc<[u8]>,
    pub playback_id: u64,
    pub registration_sender: mpsc::Sender<PlaybackRegistration>,
}

pub struct SpatialDecoder {
    inner: InnerDecoder,
    control: DecoderControl,
    channels: u16,
    channel_states: Vec<BiquadState>,
    sample_rate: u32,
    current_cutoff_hz: f32,
    coeffs: BiquadCoefficients,
    sample_counter: usize,
}

impl Decodable for SpatialAudioSource {
    type Decoder = SpatialDecoder;

    fn decoder(&self) -> Self::Decoder {
        let cursor = Cursor::new(self.bytes.clone());
        let inner = Decoder::new(cursor).expect("Failed to create decoder!");

        let control = DecoderControl::new(20000.0, 1.0);

        let channels = inner.channels().get();
        let channel_states = vec![BiquadState::default(); channels as usize];

        let sample_rate = inner.sample_rate().get();

        let _ = self.registration_sender.send(PlaybackRegistration {
            playback_id: self.playback_id,
            control: control.clone(),
        });

        let initial_cutoff = 20000.0;
        let coeffs = BiquadCoefficients::low_pass(
            initial_cutoff,
            sample_rate as f32,
            std::f32::consts::FRAC_1_SQRT_2,
        );

        SpatialDecoder {
            inner,
            control,
            channels,
            channel_states,
            sample_rate,
            current_cutoff_hz: initial_cutoff,
            coeffs,
            sample_counter: 0,
        }
    }
}

impl Source for SpatialDecoder {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(self.channels).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(self.sample_rate).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

impl Iterator for SpatialDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let raw_sample = self.inner.next()?;

        if self.sample_counter % 64 == 0 {
            let target_cutoff = self.control.get_cutoff();
            if (target_cutoff - self.current_cutoff_hz).abs() > 1.0 {
                self.current_cutoff_hz = target_cutoff;
                self.coeffs = BiquadCoefficients::low_pass(
                    target_cutoff,
                    self.sample_rate as f32,
                    std::f32::consts::FRAC_1_SQRT_2,
                );
            }
        }
        self.sample_counter += 1;

        let volume = self.control.get_volume();
        let float_sample = raw_sample * volume;

        let channel = (self.sample_counter - 1) % (self.channels as usize);

        let filtered_sample = self.channel_states[channel].process_sample(float_sample, &self.coeffs);

        Some(filtered_sample)
    }
}
