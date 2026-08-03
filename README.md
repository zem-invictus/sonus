# bevy_sonus

[![Crates.io](https://img.shields.io/crates/v/bevy_sonus)](https://crates.io/crates/bevy_sonus)
[![Documentation](https://docs.rs/bevy_sonus/badge.svg)](https://docs.rs/bevy_sonus)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

Spatial audio plugin for [Bevy Engine](https://bevyengine.org).

<video src="https://github.com/user-attachments/assets/a381ec43-c59d-41bb-8e9f-bdd30557eab0" controls autoplay loop muted width="100%">
</video>

## Features

* **Multi-ray occlusion**: 5-ray cross pattern around emitter radius.
* **3-band crossover filtering**: Frequency-dependent low, mid, and high attenuation based on material transmission.
* **Material presets**: Transmission coefficients for Wood, Stone, Concrete, Glass, Metal, and ThinPlaster.
* **Sound diffraction**: Perceived direction bending around obstacle edges.
* **Distance attenuation**: Linear and inverse-distance models.
* **Blender & Skein workflow**: Spawn audio emitters and materials directly from GLTF scenes.
* **Lock-free DSP**: Atomic parameters shared between Bevy ECS and audio threads.

## Quickstart

Add `bevy_sonus` to `Cargo.toml`:

```toml
[dependencies]
bevy = "0.19"
bevy_sonus = "0.1"
```

### Spawning Emitter and Listener

```rust
use bevy::prelude::*;
use bevy_sonus::{AcousticMaterialPreset, AttenuationModel, SonusAudioPlugin, SonusEmitter, SonusListener};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SonusAudioPlugin::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spatial audio listener on player camera
    commands.spawn((
        SonusListener,
        Transform::from_xyz(0.0, 1.5, 5.0),
    ));

    // Spatial sound emitter with occlusion and distance attenuation
    commands.spawn((
        SonusEmitter::new("audio/siren.wav")
            .with_occlusion()
            .with_attenuation(AttenuationModel::Linear { min_dist: 2.0, max_dist: 20.0 }),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));

    // Obstacle wall with Concrete preset
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 3.0, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        AcousticMaterialPreset::Concrete,
        Transform::from_xyz(0.0, 1.5, 2.5),
    ));
}
```

## Debug Gizmos

Enable debug gizmos to view attenuation spheres, rays, and wall AABBs:

```rust
App::new()
    .add_plugins((
        DefaultPlugins,
        SonusAudioPlugin::default().with_debug(),
    ));
```

### Indicators
* **Green ray**: Clear line of sight.
* **Orange ray**: Partial obstruction.
* **Red ray**: Blocked ray.
* **Green sphere**: Minimum distance ($d_{\text{min}}$) with zero attenuation.
* **Yellow sphere**: Maximum distance ($d_{\text{max}}$) cutoff point.

## Blender Workflow

1. Attach `SonusEmitterConfig` to empty objects or meshes in Blender.
2. Attach `AcousticMaterialPreset` or `AcousticMaterial` to obstacle meshes.
3. Export scene as `.gltf` / `.glb` and load via `bevy_skein`.

## License

Licensed under either of:
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
