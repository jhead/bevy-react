//! Bridge TypeScript codegen for the HUD example.
//!
//! Kept separate from `main.rs` so `cargo test` can export without launching Bevy.

use bevy::prelude::*;
use bevy_react::{
    BridgeCommandMeta, GeneratedBridgeTs, assert_bridge_typescript_fresh,
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

/// Commands registered in `setup_bridge` — keep names in sync with `main.rs`.
pub const HUD_COMMANDS: &[BridgeCommandMeta] = &[
    BridgeCommandMeta {
        name: "add_score",
        ts_fn: "addScore",
        args_ts: "number",
        result_ts: "{ score: number }",
    },
    BridgeCommandMeta {
        name: "heal",
        ts_fn: "heal",
        args_ts: "void",
        result_ts: "{ hp: number }",
    },
];

/// Serde / JSON object keys for `PlayerStats` (field declaration order).
pub const PLAYER_STATS_KEYS: &[&str] = &["hp", "max_hp", "score"];

pub fn hud_generated_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/src/generated")
}

pub fn hud_bridge_ts_bundle() -> GeneratedBridgeTs {
    GeneratedBridgeTs::new()
        .with_type::<PlayerStats>("PlayerStats.ts")
        .with_keys("PlayerStatsKeys.ts", "PLAYER_STATS_KEYS", PLAYER_STATS_KEYS)
        .with_commands("commands.ts", HUD_COMMANDS)
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
}
