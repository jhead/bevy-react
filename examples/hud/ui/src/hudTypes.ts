/**
 * HUD TypeScript surface: shared generated Rust types + thin app helpers.
 *
 * Generated symbols come from the `bridge-types` workspace package
 * (`./scripts/generate-bridge-types.sh`).
 */
export type { PlayerStats } from 'bridge-types'
export { PLAYER_STATS_KEYS, addScore, heal } from 'bridge-types'
export type { AddScoreResult, HealResult } from 'bridge-types'

import type { PlayerStats } from 'bridge-types'

export const INITIAL_PLAYER_STATS: PlayerStats = {
  hp: 100,
  max_hp: 100,
  score: 0,
}

export function hpRatio(stats: PlayerStats): number {
  return stats.max_hp > 0 ? stats.hp / stats.max_hp : 0
}
