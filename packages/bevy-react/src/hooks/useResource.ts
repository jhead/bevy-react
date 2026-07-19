/**
 * Subscribe to an ECS-backed resource store published via
 * `ReactBridge::register_resource_store`.
 *
 * Re-exports {@link useResource} / {@link useBridgeState} from the bridge module.
 */
export { useResource, useBridgeState } from "../bridge";
