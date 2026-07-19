/**
 * Subscribe to an ECS query store published via
 * `ReactBridge::register_query_store` / `register_query_store_each_frame`.
 *
 * Re-exports {@link useQuery} / {@link useBridgeState} from the bridge module.
 */
export { useQuery, useBridgeState } from "../bridge";
