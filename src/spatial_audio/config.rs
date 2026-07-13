use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;

pub struct AudioParam {
    pub value: AtomicU32,
}

impl AudioParam {
    pub fn new(val: f32) -> Self {
        Self {
            value: AtomicU32::new(val.to_bits()),
        }
    }

    #[inline]
    pub fn get(&self) -> f32 {
        f32::from_bits(self.value.load(Relaxed))
    }

    #[inline]
    pub fn set(&self, val: f32) {
        self.value.store(val.to_bits(), Relaxed)
    }
}