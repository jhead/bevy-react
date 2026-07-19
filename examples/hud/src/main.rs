//! HUD example: bind Bevy ECS game state into React via [`ReactBridge`].
//!
//! HP / score tick on the Rust side and are published on the `"hud"` channel.
//! The UI reads them with `useBridgeState` and can call `add_score` via `callNative`.

use bevy::prelude::*;
use bevy_react::{
    ReactBridge, ReactBundle, ReactHmrRoot, ReactPlugin, ReactScriptSource, ViteDevSource,
    js_bevy::JsPlugin,
};
use serde::Serialize;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(JsPlugin)
        .add_plugins(ReactPlugin)
        .insert_resource(PlayerStats {
            hp: 100,
            max_hp: 100,
            score: 0,
        })
        .add_systems(Startup, (setup_ui, setup_bridge))
        .add_systems(Update, (tick_player, sync_hud))
        .run();
}

#[derive(Resource, Clone, Serialize)]
struct PlayerStats {
    hp: i32,
    max_hp: i32,
    score: u32,
}

#[derive(Serialize)]
struct HudPayload {
    hp: i32,
    max_hp: i32,
    score: u32,
    /// 0.0–1.0 fill fraction for the HP bar.
    hp_ratio: f32,
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
    bridge.register("add_score", |world, args| {
        let points = args.as_i64().unwrap_or(10) as u32;
        if let Some(mut stats) = world.get_resource_mut::<PlayerStats>() {
            stats.score = stats.score.saturating_add(points);
        }
        serde_json::Value::Null
    });

    bridge.register("heal", |world, _args| {
        if let Some(mut stats) = world.get_resource_mut::<PlayerStats>() {
            stats.hp = (stats.hp + 15).min(stats.max_hp);
        }
        serde_json::Value::Null
    });
}

fn tick_player(time: Res<Time>, mut stats: ResMut<PlayerStats>) {
    // Slow drain so the HUD visibly updates without player input.
    let drain = (time.delta_secs() * 4.0) as i32;
    if drain > 0 {
        stats.hp = (stats.hp - drain).max(0);
    }
}

fn sync_hud(bridge: Res<ReactBridge>, stats: Res<PlayerStats>) {
    if !stats.is_changed() && !stats.is_added() {
        return;
    }
    let hp_ratio = if stats.max_hp > 0 {
        stats.hp as f32 / stats.max_hp as f32
    } else {
        0.0
    };
    bridge.publish(
        "hud",
        HudPayload {
            hp: stats.hp,
            max_hp: stats.max_hp,
            score: stats.score,
            hp_ratio,
        },
    );
}
