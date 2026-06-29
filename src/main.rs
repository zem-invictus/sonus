mod spatial_audio;

use bevy::prelude::*;

#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}
#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}
#[derive(Component)]
struct Name(String);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_game)
        .add_systems(Update, movement_system)
        .add_systems(Update, log_positions)
        .run();
}

// Startup-система: спавнит (создает) игровые сущности
// Commands — это очередь отложенных команд для изменения структуры мира (создание, удаление сущностей/компонентов)
fn setup_game(mut commands: Commands) {
    // Спавним первую сущность (игрока) с тремя компонентами
    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 0.5 },
        Name("Player 1".to_string()),
    ));
    // Спавним вторую сущность (камень) только с позицией (она не будет двигаться, так как нет Velocity)
    commands.spawn((
        Position { x: 10.0, y: -5.0 },
        Name("Static Rock".to_string()),
    ));
}
// Update-система движения: обновляет позиции на основе скорости
// Query<&mut Position, With<Velocity>> — это запрос.
// Он просит предоставить изменяемый доступ (&mut) к компоненту Position
// для всех сущностей, у которых ТАКЖЕ есть компонент Velocity.
// Time — ресурс, предоставляющий дельту времени (время между кадрами) для независимости скорости от FPS.
fn movement_system(mut query: Query<(&mut Position, &Velocity)>, time: Res<Time>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.x * time.delta_secs();
        pos.y += vel.y * time.delta_secs();
    }
}
// Update-система логирования
// Запрашивает имя и позицию сущностей, чтобы выводить их в консоль
fn log_positions(query: Query<(&Name, &Position)>) {
    for (name, pos) in query.iter() {
        println!(
            "Сущность: {} находится на ({:.2}, {:.2})",
            name.0, pos.x, pos.y
        );
    }
}
