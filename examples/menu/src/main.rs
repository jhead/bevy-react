use bevy::prelude::*;
use bevy_react::{
    ReactBundle, ReactHmrRoot, ReactPlugin, ReactScriptSource, ViteDevSource,
    js_bevy::JsPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(JsPlugin)
        .add_plugins(ReactPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let js_source = ReactScriptSource::auto_with(
        || {
            ViteDevSource::default()
                .with_module_name("bevy-react-menu-vite")
                .with_entry_point("src/main.tsx")
                .into()
        },
        || {
            ReactScriptSource::from_path(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/ui/dist/app.js"
            ))
            .expect(
                "Release builds need examples/menu/ui/dist/app.js — run `pnpm build` there",
            )
        },
    );

    commands.spawn((
        ReactBundle::new(
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            js_source,
        ),
        ReactHmrRoot,
    ));
}
