# bevy_sonus

[![Crates.io](https://img.shields.io/crates/v/bevy_sonus)](https://crates.io/crates/bevy_sonus)
[![Documentation](https://docs.rs/bevy_sonus/badge.svg)](https://docs.rs/bevy_sonus)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

A high-performance, real-time spatial audio plugin for the [Bevy Engine](https://bevyengine.org).

<video src="https://github.com/user-attachments/assets/a381ec43-c59d-41bb-8e9f-bdd30557eab0" controls autoplay loop muted width="100%">
</video>

Features:
- **Multi-ray obstruction & occlusion**: Cross-pattern 5-ray casting around emitter volume.
- **3-band crossover filtering**: Frequency-dependent low, mid, and high attenuation based on physical material properties.
- **Acoustic materials & presets**: Real-time transmission coefficients for Wood, Stone, Concrete, Glass, Metal, and ThinPlaster.
- **Diffraction & perceived direction bending**: Acoustic sound bending around obstacle edges in a single pass.
- **Distance attenuation**: Linear and Inverse-Distance attenuation models.
- **Blender & Skein integration**: Spawning audio emitters and material properties directly from GLTF scenes.
- **Lock-free DSP pipeline**: Synchronized parameter updates between Bevy ECS and audio threads using `AtomicU32` (`Ordering::Relaxed`).

## Quickstart

Add `bevy_sonus` to your `Cargo.toml`:

```toml
[dependencies]
bevy = "0.19"
bevy_sonus = "0.1"
```

### 1. Spawning Listener & Emitter Programmatically

```rust
use bevy::prelude::*;
use bevy_sonus::{AttenuationModel, AcousticMaterialPreset, SonusAudioPlugin, SonusEmitter, SonusListener};

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
    // Spawn active spatial audio listener (e.g. Attached to Player Camera)
    commands.spawn((
        SonusListener,
        Transform::from_xyz(0.0, 1.5, 5.0),
    ));

    // Spawn spatial audio emitter with occlusion & distance attenuation
    commands.spawn((
        SonusEmitter::new("audio/siren.wav")
            .with_occlusion()
            .with_attenuation(AttenuationModel::Linear { min_dist: 2.0, max_dist: 20.0 }),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));

    // Spawn an acoustic obstacle wall with mesh geometry (Aabb generated automatically by Bevy)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 3.0, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        AcousticMaterialPreset::Concrete,
        Transform::from_xyz(0.0, 1.5, 2.5),
    ));
}
```

## Debug Mode & Visual Gizmos

Enable visual debug gizmos to inspect attenuation spheres, raycasts, and wall AABBs:

```rust
App::new()
    .add_plugins((
        DefaultPlugins,
        SonusAudioPlugin::default().with_debug(),
    ));
```

### Color Indicators
- **Green Ray**: Unobstructed direct line-of-sight sound propagation.
- **Orange Ray**: Partial obstruction through acoustic materials.
- **Red Ray**: Full obstruction / complete acoustic block.
- **Green Inner Sphere**: Minimum distance ($d_{\text{min}}$) with zero attenuation ($100\%$ volume).
- **Yellow Outer Sphere**: Maximum distance ($d_{\text{max}}$) cutoff point ($0\%$ volume).

## Blender & Skein Workflow

1. In Blender, attach `SonusEmitterConfig` to empty objects or sound meshes.
2. Attach `AcousticMaterialPreset` or custom `AcousticMaterial` to obstacle meshes.
3. Export scene as `.glb` / `.gltf` and load using `bevy_skein`.

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.
