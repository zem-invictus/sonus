# Mysterium: Bevy Spatial Audio Plugin Handoff Dump (Part 2)

This document contains the updated context, new modular architecture, resolved issues, and next steps for the Bevy spatial audio plugin project.

---

## 1. Project Context & Goal
We are building a Bevy 0.19.0 spatial audio plugin implementing a **Source-Engine-like sound occlusion and obstruction system** with real-time DSP low-pass filtering.
- **Occlusion**: Direct path and reflections are blocked (muffled sound + volume reduction).
- **Obstruction**: Only the direct path is blocked (muffled sound, reflections propagate freely).

---

## 2. Refactored Modular Architecture
We split the code into smaller, highly cohesive modules under `src/spatial_audio/`:

1. **`src/spatial_audio/biquad.rs`**: Contains pure DSP math (`BiquadCoefficients` and `BiquadState`) for processing floating-point samples. No game-engine dependencies.
2. **`src/spatial_audio/control.rs`**: Thread-safe communication structures.
   - **`AudioParam`**: Encompasses `Arc<AtomicU32>` to store thread-safe `f32` parameters using raw bit operations. Visually exposed as a type to avoid manual getter/setter boilerplate (e.g. Web Audio API standard).
   - **`BiquadControl`** and **`ReverbControl`**: Lightweight structs composed of `AudioParam` properties.
   - **`PlaybackControl`**: A composite control panel that groups optional filter controllers.
   - **`PlaybackRegistration`**: The payload sent via the MPSC channel when a sound starts playing.
3. **`src/spatial_audio/filter.rs`**: Holds **`BiquadFilter<I>`**, a generic decorator implementing `rodio::Source` and `Iterator<Item = f32>` which wraps *any* source `I`. Block-processing updates parameters every 64 samples to reduce CPU overhead.
4. **`src/spatial_audio/source.rs`**: The Bevy Asset (`SpatialAudioSource`) and its `Decodable` implementation.
   - We use **Option 1 (Dynamic Chain via Boxing)**: It returns `Box<dyn Source<Item = f32> + Send>` dynamically assembled at startup based on a `HashMap` configuration. This avoids the "type explosion" in Bevy ECS while keeping the internal filters chained statically, preventing double-boxing overhead.
5. **`src/spatial_audio/mod.rs`**: Exposes the modules.

All modules compile successfully with **0 errors**.

---

## 3. Immediate Next Steps (For the Laptop Agent)

### Task 1: Integrate with Bevy ECS in `src/main.rs`
Define resources, components, and the initialization code in `main.rs`:

1. **Define Resources & Components**:
```rust
use std::sync::mpsc::{Receiver, Sender};
use crate::spatial_audio::control::{PlaybackRegistration, PlaybackControl};
use crate::spatial_audio::source::SpatialAudioSource;

#[derive(Resource)]
pub struct AudioPlaybackRegistry {
    pub receiver: Receiver<PlaybackRegistration>,
}

#[derive(Resource)]
pub struct AudioPlaybackSender {
    pub sender: Sender<PlaybackRegistration>,
}

#[derive(Component)]
pub struct SpatialAudioEmitter {
    pub playback_id: u64,
    pub control: Option<PlaybackControl>,
}
```

2. **Initialize in `main()`**:
```rust
let (sender, receiver) = std::sync::mpsc::channel::<PlaybackRegistration>();

App::new()
    .add_plugins(DefaultPlugins)
    .add_audio_source::<SpatialAudioSource>()
    .insert_resource(AudioPlaybackSender { sender })
    .insert_resource(AudioPlaybackRegistry { receiver })
    .add_systems(Update, sync_audio_controls)
    // ...
```

3. **Implement `sync_audio_controls` System**:
Polls the receiver using non-blocking `.try_recv()`, finds the corresponding `SpatialAudioEmitter` entity by `playback_id`, and stores the `PlaybackControl` panel in it.

### Task 2: Implement Sound Spawning System
Write a system that takes a standard loaded `AudioSource`, checks if a `SpatialAudioEmitter` needs to be initialized, converts the raw bytes into a `SpatialAudioSource`, inserts it into `Assets<SpatialAudioSource>`, and spawns Bevy's `AudioPlayer` to kick off playback and MPSC registration.

### Task 3: Raycast Occlusion System
Write a system that traces rays from the `AudioListener` to active `SpatialAudioEmitter` entities, checks for intersections with `AudioOccluder` obstacles, calculates attenuation/cutoff, and writes the results to `emitter.control.biquad.cutoff_hz.set(...)` smoothly.
