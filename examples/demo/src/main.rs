use bevy::prelude::*;
use bevy_react::{
    ReactBundle, ReactHmrRoot, ReactPlugin, ReactScriptSource, ViteDevSource,
    js_bevy::JsPlugin,
};

#[path = "../../common/auto_screenshot.rs"]
mod auto_screenshot;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(JsPlugin)
        .add_plugins(ReactPlugin)
        .add_plugins(auto_screenshot::AutoScreenshotPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Debug: Vite HMR. Release: single ESM from `pnpm build` in examples/demo/ui
    // (see docs/BUILD.md). Prefer EmbeddedBundleSource / ReactAssetBundle in apps.
    let js_source = ReactScriptSource::auto_with(
        || {
            ViteDevSource::default()
                .with_module_name("bevy-react-demo-vite")
                .with_entry_point("src/main.tsx")
                .into()
        },
        || {
            ReactScriptSource::from_path(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/ui/dist/app.js"
            ))
            .expect(
                "Release builds need examples/demo/ui/dist/app.js — run `pnpm build` there (docs/BUILD.md)",
            )
        },
    );

    commands.spawn((
        ReactBundle::new(
            Node {
                width: Val::Percent(50.0),
                height: Val::Percent(100.0),
                left: Val::Percent(50.0),
                top: Val::Percent(0.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            js_source,
        ),
        // Enables Vite WebSocket → ReactDirtyFlag reloads in debug.
        ReactHmrRoot,
    ));
}
