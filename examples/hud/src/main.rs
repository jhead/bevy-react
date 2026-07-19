//! HUD example: bind Bevy ECS game state into React via [`ReactBridge`].
//!
//! `PlayerStats` is registered as a resource store on the `"hud"` channel.
//! The UI reads it with `useResource` / `useBridgeState` and can call
//! `add_score` / `heal` via `callNative` (Promise-returning).

use bevy::prelude::*;
use bevy_react::{
    ReactBridge, ReactBundle, ReactHmrRoot, ReactPlugin, ReactScriptSource, ViteDevSource,
    js_bevy::JsPlugin,
};
use serde::Serialize;

#[path = "../../common/auto_screenshot.rs"]
mod auto_screenshot;

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

/// ECS resource mirrored to React via `register_resource_store`.
///
/// Keep field names in sync with `examples/hud/ui/src/hudTypes.ts`
/// (hand-written parallel types; see `docs/BRIDGE.md`).
#[derive(Resource, Clone, Serialize)]
struct PlayerStats {
    hp: i32,
    max_hp: i32,
    score: u32,
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
    bridge.register_resource_store::<PlayerStats>("hud");

    bridge.register("add_score", |world, args| {
        let points = args.as_i64().unwrap_or(10) as u32;
        let score = if let Some(mut stats) = world.get_resource_mut::<PlayerStats>() {
            stats.score = stats.score.saturating_add(points);
            stats.score
        } else {
            0
        };
        serde_json::json!({ "score": score })
    });

    bridge.register("heal", |world, _args| {
        let hp = if let Some(mut stats) = world.get_resource_mut::<PlayerStats>() {
            stats.hp = (stats.hp + 15).min(stats.max_hp);
            stats.hp
        } else {
            0
        };
        serde_json::json!({ "hp": hp })
    });
}

fn tick_player(time: Res<Time>, mut stats: ResMut<PlayerStats>) {
    // Slow drain so the HUD visibly updates without player input.
    let drain = (time.delta_secs() * 4.0) as i32;
    if drain > 0 {
        stats.hp = (stats.hp - drain).max(0);
    }
}
