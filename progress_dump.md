# Mysterium: Bevy Spatial Audio Plugin Handoff Dump

This document contains the complete context, current progress, architecture description, and next steps for the Bevy spatial audio plugin project. Share this with your AI coding assistant to resume work seamlessly.

---

## 1. Project Context & Goal
We are building a Bevy 0.19.0 spatial audio plugin implementing a **Source-Engine-like sound occlusion and obstruction system** with real-time DSP low-pass filtering.

- **Occlusion (Окклюзия)**: Both direct path and reflections are blocked. The sound is muffled significantly (drastic volume reduction + low-pass filter).
- **Obstruction (Обструкция)**: Only the direct path is blocked (e.g., sound behind a pillar). The direct sound is muffled, but reflections/reverberation are unaffected.
- **Goal**: Implement a custom audio processing pipeline in Bevy where ECS systems can query raycasts and dynamically adjust the volume/cutoff parameters of playing sound sources.

---

## 2. Architecture & Completed Work

### A. The Real-time DSP Filter (`src/spatial_audio.rs`)
Because standard `bevy_audio` (built on `rodio`) doesn't natively support dynamic real-time DSP injection, we implemented a custom audio source and decoder:
1. **`SpatialAudioSource`**: A custom Bevy `Asset` holding raw audio file bytes. It implements `bevy::audio::Decodable`.
2. **`SpatialDecoder`**: A custom decoder implementing `rodio::Source` and `Iterator<Item = f32>`. It wraps `rodio::Decoder`, extracts raw sample data, and applies a **Transposed Direct Form II Biquad Low-Pass Filter** in real time.
3. **`DecoderControl`**: A thread-safe communication panel holding `cutoff_hz` and `volume` as `Arc<AtomicU32>` (floats represented as bits to allow atomic operations). It allows the ECS main thread to modify DSP parameters on the fly without locks or blocking the audio thread.
4. **`PlaybackRegistration` & MPSC**: When Bevy starts playing a sound, the audio thread instantiates the `SpatialDecoder` and sends a `PlaybackRegistration` payload containing a unique `playback_id` and the `DecoderControl` handle back to the main thread via an MPSC channel.

### B. Important Technical Issues Resolved
- **Windows Linker Error (LNK1189)**: Compiling Bevy with the `dynamic_linking` feature on Windows MSVC hit the DLL export limit of 65,535 symbols. This was fixed by disabling dynamic linking in `Cargo.toml`.
- **Rodio 0.22 Sample Format**: Standardized on `f32` samples. `convert_samples` and `SamplesConverter` were removed in this version because `rodio::Decoder` directly yields `f32` samples.
- **Rodio 0.22 Metadata Types**: `channels()` and `sample_rate()` return `NonZero<u16>` and `NonZero<u32>` wrappers. We resolve this by calling `.get()` on them before processing.

---

## 3. Current File State

- **Cargo.toml**: [Cargo.toml](file:///i:/mysterium/Cargo.toml) (Contains Bevy 0.19.0 and Rodio 0.22.2 with disabled default features).
- **src/main.rs**: [main.rs](file:///i:/mysterium/src/main.rs) (Declares `mod spatial_audio;` and initializes the Bevy app).
- **src/spatial_audio.rs**: [spatial_audio.rs](file:///i:/mysterium/src/spatial_audio.rs) (Holds the custom biquad math, atomic control, and custom decoder traits. It compiles successfully).

---

## 4. Immediate Next Steps (For the Laptop Agent)

### Task 1: Encapsulate `DecoderControl`
Currently, the atomic fields in `DecoderControl` are public. To ensure encapsulation and clear autocompletion, remove the `pub` keyword from `cutoff_hz` and `volume` fields, leaving them private to the module:
```rust
#[derive(Clone)]
pub struct DecoderControl {
    cutoff_hz: Arc<AtomicU32>,
    volume: Arc<AtomicU32>,
}
```

### Task 2: Create the Bevy ECS Registry Resource
Define a resource in Bevy to hold the `Receiver<PlaybackRegistration>` so that ECS systems can retrieve the control handles of currently playing sounds:
```rust
#[derive(Resource)]
pub struct AudioPlaybackRegistry {
    pub receiver: std::sync::mpsc::Receiver<PlaybackRegistration>,
}
```

### Task 3: Implement the Integration ECS System
Write a Bevy system that polls the receiver using non-blocking `.try_recv()`:
1. It retrieves new registrations.
2. It matches the `playback_id` with active Bevy sound entities.
3. It stores the `DecoderControl` handle as a component on those entities (e.g., `ActiveAudioControl(DecoderControl)`).

### Task 4: Raycasting & Occlusion Calculation
Create a system that:
1. Traces rays from the `AudioListener` (camera) to each sound emitter.
2. Checks for intersections with `AudioOccluder` entities.
3. Calculates the thickness/material dampening and computes the target volume and low-pass cutoff.
4. Smoothly interpolates (lerps) the parameters to avoid audio clicks, then writes the updated parameters to `DecoderControl`.
