/**
 * ECS escape hatch — resolve Bevy entity handles from React UI nodes.
 *
 * Host components expose a public instance `{ nodeId }` via reconciler refs.
 * After Bevy processes CreateNode, `__react_entity_id(nodeId)` returns
 * `Entity::to_bits()` for use from Rust (`Entity::from_bits`) or gameplay bridges.
 */

import { useCallback, useEffect, useState, type RefCallback } from "react";

/** Public instance returned by host component refs (`getPublicInstance`). */
export type BevyHostInstance = {
  nodeId: number;
};

/**
 * Opaque Bevy entity handle usable from JS/TS.
 * `bits` is `Entity::to_bits()` — reconstruct with `Entity::from_bits(bits)` in Rust.
 */
export type EntityId = {
  /** Packed index + generation (`Entity::to_bits()`). */
  bits: number;
  /** Entity index (low 32 bits). */
  index: number;
  /** Entity generation (high 32 bits). */
  generation: number;
  /** React reconciler node id. */
  nodeId: number;
};

/** Decode Bevy's `Entity::to_bits` packing: `index | (generation << 32)`. */
export function entityFromBits(bits: number, nodeId: number): EntityId {
  // Use unsigned 32-bit split without BigInt for Boa-friendly math.
  const index = bits >>> 0;
  const generation = Math.floor(bits / 0x1_0000_0000) >>> 0;
  return { bits, index, generation, nodeId };
}

/**
 * Look up the Bevy entity for a React node id.
 * Returns `null` until the host has spawned the ECS entity (typically next frame).
 */
export function resolveEntity(nodeId: number): EntityId | null {
  if (typeof __react_entity_id !== "function") {
    return null;
  }
  const bits = __react_entity_id(nodeId);
  if (bits == null || Number.isNaN(bits)) {
    return null;
  }
  return entityFromBits(bits, nodeId);
}

/**
 * Resolve an entity from a stored node id (ref object or bare number).
 * Re-renders until the host map is populated.
 */
export function useEntity(
  nodeRef: { current: number | null } | number | null | undefined
): EntityId | null {
  const nodeId =
    typeof nodeRef === "number"
      ? nodeRef
      : nodeRef && typeof nodeRef === "object"
        ? nodeRef.current
        : null;

  const [entity, setEntity] = useState<EntityId | null>(() =>
    nodeId != null ? resolveEntity(nodeId) : null
  );

  useEffect(() => {
    if (nodeId == null) {
      setEntity(null);
      return;
    }

    const tick = () => {
      const resolved = resolveEntity(nodeId);
      setEntity((prev) => {
        if (resolved == null) return null;
        if (
          prev &&
          prev.bits === resolved.bits &&
          prev.nodeId === resolved.nodeId
        ) {
          return prev;
        }
        return resolved;
      });
    };

    tick();
    // Entity mapping lands after Bevy processes CreateNode — poll briefly.
    const handle = setInterval(tick, 16);
    return () => clearInterval(handle);
  }, [nodeId]);

  return entity;
}

/**
 * Callback ref + resolved entity for a host component.
 *
 * @example
 * ```tsx
 * const [ref, entity] = useEntityRef();
 * return <Node ref={ref} components={['Glow']} />;
 * // entity?.bits → pass to Rust via bridge / gameplay code
 * ```
 */
export function useEntityRef(): [
  RefCallback<BevyHostInstance>,
  EntityId | null,
] {
  const [nodeId, setNodeId] = useState<number | null>(null);
  const ref = useCallback<RefCallback<BevyHostInstance>>((instance) => {
    setNodeId(instance?.nodeId ?? null);
  }, []);
  const entity = useEntity(nodeId);
  return [ref, entity];
}
