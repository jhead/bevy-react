use bevy::prelude::*;
use bevy_react::{ReactBundle, ViteDevSource, ReactPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ReactPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Use Vite dev server to load the React app with hot reloading support
    let js_source = ViteDevSource::default()
        .with_module_name("bevy-react-demo-vite")
        .with_entry_point("src/main.tsx");

    // Spawn the React UI bundle covering the right half of the screen
    commands.spawn(ReactBundle::new(
        Node {
            width: Val::Percent(50.0),
            height: Val::Percent(100.0),
            left: Val::Percent(50.0),
            top: Val::Percent(0.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        js_source.into(),
    ));
}
