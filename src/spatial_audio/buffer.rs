use std::num::NonZero;

pub struct BlockBuffer {
    data: Vec<f32>,
    read_index: u32,
    block_size: u16,
    channels: NonZero<u16>,
}

impl BlockBuffer {
    pub fn new(block_size: u16, channels: NonZero<u16>) -> Self {
        Self {
            data: Vec::with_capacity((block_size * channels.get()) as usize),
            read_index: 0,
            block_size,
            channels,
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        (self.block_size * self.channels.get()) as usize
    }

    pub fn read_index(&self) -> usize {
        self.read_index as usize
    }

    #[inline]
    pub fn channels(&self) -> NonZero<u16> {
        self.channels
    }

    #[inline]
    pub fn push(&mut self, sample: f32) {
        debug_assert!(
            self.data.len() < self.data.capacity(),
            "Попытка переполнить BlockBuffer! Это вызвало бы аллокацию памяти в аудио-потоке."
        );
        self.data.push(sample);
    }

    #[inline]
    pub fn pop(&mut self) -> f32 {
        let sample = self.data[self.read_index as usize];
        self.read_index += 1;
        sample
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
        self.read_index = 0;
    }

    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.read_index as usize >= self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }
}
