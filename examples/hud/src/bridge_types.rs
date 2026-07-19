//! Bridge TypeScript codegen for the HUD example.
//!
//! Kept separate from `main.rs` so `cargo test` can export without launching Bevy.
//! Commands are defined once via [`BridgeCommandSet`] (meta + handlers together).

use bevy::prelude::*;
use bevy_react::{
    BridgeCommandMeta, BridgeCommandSet, GeneratedBridgeTs, ReactBridge,
    assert_bridge_typescript_fresh,
};
use serde::Serialize;
use ts_rs::TS;

/// ECS resource mirrored to React via `register_resource_store("hud")`.
///
/// TypeScript is generated into `ui/src/generated/` via `ts-rs`
/// (`./scripts/generate-bridge-types.sh`).
#[derive(Resource, Clone, Serialize, TS)]
#[ts(export_to = "PlayerStats.ts")]
pub struct PlayerStats {
    pub hp: i32,
    pub max_hp: i32,
    pub score: u32,
}

/// Commands for the HUD bridge: TypeScript meta and Bevy handlers in one place.
pub fn hud_bridge_commands() -> BridgeCommandSet {
    BridgeCommandSet::new()
        .command(
            BridgeCommandMeta::new(
                "add_score",
                "addScore",
                "number",
                "{ score: number }",
            ),
            |world, args| {
                let points = args.as_i64().unwrap_or(10) as u32;
                let score = if let Some(mut stats) = world.get_resource_mut::<PlayerStats>() {
                    stats.score = stats.score.saturating_add(points);
                    stats.score
                } else {
                    0
                };
                serde_json::json!({ "score": score })
            },
        )
        .command(
            BridgeCommandMeta::new("heal", "heal", "void", "{ hp: number }"),
            |world, _args| {
                let hp = if let Some(mut stats) = world.get_resource_mut::<PlayerStats>() {
                    stats.hp = (stats.hp + 15).min(stats.max_hp);
                    stats.hp
                } else {
                    0
                };
                serde_json::json!({ "hp": hp })
            },
        )
}

/// Register resource store + HUD commands on the bridge.
pub fn apply_hud_bridge(bridge: &ReactBridge) {
    bridge.register_resource_store::<PlayerStats>("hud");
    hud_bridge_commands().apply(bridge);
}

/// Serde / JSON object keys for `PlayerStats` (field declaration order).
pub const PLAYER_STATS_KEYS: &[&str] = &["hp", "max_hp", "score"];

pub fn hud_generated_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/src/generated")
}

pub fn hud_bridge_ts_bundle() -> GeneratedBridgeTs {
    GeneratedBridgeTs::new()
        .with_type::<PlayerStats>("PlayerStats.ts")
        .with_keys("PlayerStatsKeys.ts", "PLAYER_STATS_KEYS", PLAYER_STATS_KEYS)
        .with_command_set("commands.ts", &hud_bridge_commands())
        .with_barrel(
            "index.ts",
            &["PlayerStats.ts", "PlayerStatsKeys.ts", "commands.ts"],
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn player_stats_json_shape_matches_keys() {
        let stats = PlayerStats {
            hp: 80,
            max_hp: 100,
            score: 1200,
        };
        let value = serde_json::to_value(&stats).expect("serialize");
        let obj = value.as_object().expect("object");
        let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, PLAYER_STATS_KEYS);
        assert_eq!(
            value,
            json!({ "hp": 80, "max_hp": 100, "score": 1200 })
        );
    }

    #[test]
    fn generated_bridge_typescript_is_fresh() {
        assert_bridge_typescript_fresh(&hud_generated_dir(), &hud_bridge_ts_bundle());
    }

    #[test]
    fn hud_commands_meta_matches_wrappers() {
        let commands = hud_bridge_commands();
        let meta = commands.meta();
        assert_eq!(meta.len(), 2);
        assert_eq!(meta[0].name, "add_score");
        assert_eq!(meta[0].ts_fn, "addScore");
        assert_eq!(meta[1].name, "heal");
    }
}
