/**
 * HUD TypeScript surface: generated Rust types + thin app helpers.
 *
 * Regenerated files live in `./generated/` (`./scripts/generate-bridge-types.sh`).
 */
export type { PlayerStats } from './generated/PlayerStats'
export { PLAYER_STATS_KEYS } from './generated/PlayerStatsKeys'
export { addScore, heal, type AddScoreResult, type HealResult } from './generated/commands'

import type { PlayerStats } from './generated/PlayerStats'

export const INITIAL_PLAYER_STATS: PlayerStats = {
  hp: 100,
  max_hp: 100,
  score: 0,
}

export function hpRatio(stats: PlayerStats): number {
  return stats.max_hp > 0 ? stats.hp / stats.max_hp : 0
}
