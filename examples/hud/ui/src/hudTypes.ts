/**
 * Hand-written TypeScript mirror of the Rust `PlayerStats` resource in
 * `examples/hud/src/main.rs`. Field names / JSON shape must match serde output.
 *
 * Future: replace with specta / ts-rs codegen (see docs/BRIDGE.md).
 */
export type PlayerStats = {
  hp: number;
  max_hp: number;
  score: number;
};

/** Expected JSON object keys, in serde field order. */
export const PLAYER_STATS_KEYS = ["hp", "max_hp", "score"] as const;

export const INITIAL_PLAYER_STATS: PlayerStats = {
  hp: 100,
  max_hp: 100,
  score: 0,
};

export function hpRatio(stats: PlayerStats): number {
  return stats.max_hp > 0 ? stats.hp / stats.max_hp : 0;
}
