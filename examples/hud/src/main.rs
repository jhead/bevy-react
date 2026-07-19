//! HUD example: bind Bevy ECS game state into React via [`ReactBridge`].
//!
//! `PlayerStats` is registered as a resource store on the `"hud"` channel.
//! The UI reads it with `useResource` / `useBridgeState` and can call
//! `add_score` / `heal` via `callNative` (Promise-returning).
//!
//! TypeScript bindings are generated from [`bridge_types`] — see
//! `docs/BRIDGE.md` and `./scripts/generate-bridge-types.sh`.

use bevy::prelude::*;
use bevy_react::{
    ReactBridge, ReactBundle, ReactHmrRoot, ReactPlugin, ReactScriptSource, ViteDevSource,
    js_bevy::JsPlugin,
};

#[path = "../../common/auto_screenshot.rs"]
mod auto_screenshot;
mod bridge_types;

use bridge_types::{PlayerStats, apply_hud_bridge};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(JsPlugin)
        .add_plugins(ReactPlugin)
        .add_plugins(auto_screenshot::AutoScreenshotPlugin)
        .insert_resource(PlayerStats {
            hp: 100,
            max_hp: 100,
            score: 0,
        })
        .add_systems(Startup, (setup_ui, setup_bridge))
        .add_systems(Update, tick_player)
        .run();
}

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d);

    let js_source = ReactScriptSource::auto_with(
        || {
            ViteDevSource::default()
                .with_module_name("bevy-react-hud-vite")
                .with_entry_point("src/main.tsx")
                .into()
        },
        || {
            ReactScriptSource::from_path(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/ui/dist/app.js"
            ))
            .expect("Release builds need examples/hud/ui/dist/app.js — run `pnpm build` there")
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

fn setup_bridge(bridge: Res<ReactBridge>) {
    apply_hud_bridge(&bridge);
}

fn tick_player(time: Res<Time>, mut stats: ResMut<PlayerStats>) {
    // Slow drain so the HUD visibly updates without player input.
    let drain = (time.delta_secs() * 4.0) as i32;
    if drain > 0 {
        stats.hp = (stats.hp - drain).max(0);
    }
}
