//! Spatial audio plugin for Bevy engine.
//!
//! Provides real-time occlusion filtering, distance attenuation, and lock-free
//! parameter synchronization between the Bevy ECS main thread and the audio processing thread.

pub mod config;
pub mod dsp;
pub mod ecs;
pub mod source;

pub use ecs::{AcousticMaterial, AudioListener, SonusEmitter, SpatialAudioPlugin};