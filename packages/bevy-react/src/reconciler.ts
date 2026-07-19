import Reconciler from "react-reconciler";
import type {
  BevyInstance,
  BevyTextInstance,
  KeyboardEventData,
  PointerEventData,
  ScrollEventData,
  WheelEventData,
} from "./types";

type Type = string;
type Props = Record<string, unknown>;
type Container = { rootId: number };
type Instance = BevyInstance;
type TextInstance = BevyTextInstance;
export type PublicInstance = Instance | TextInstance;
type HostContext = Record<string, never>;
type UpdatePayload = Props;

/** Per-root map of host instances keyed by node id. */
export type BevyInstanceMap = Map<number, PublicInstance>;

/**
 * Deep-compare values for update diffing.
 * Functions are compared by presence only (serialized as boolean flags).
 */
function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a === "function" && typeof b === "function") return true;
  if (typeof a !== typeof b) return false;
  if (a === null || b === null) return a === b;
  if (typeof a !== "object") return false;

  const aObj = a as Record<string, unknown>;
  const bObj = b as Record<string, unknown>;
  const aKeys = Object.keys(aObj);
  const bKeys = Object.keys(bObj);
  if (aKeys.length !== bKeys.length) return false;
  for (const key of aKeys) {
    if (!Object.prototype.hasOwnProperty.call(bObj, key)) return false;
    if (!deepEqual(aObj[key], bObj[key])) return false;
  }
  return true;
}

/**
 * Build a props diff for host updates. Returns null when nothing serializable changed.
 * Skips `children` (handled via text/content) and treats event handlers as presence flags.
 */
function diffProps(oldProps: Props, newProps: Props): UpdatePayload | null {
  const diff: Props = {};
  let changed = false;

  for (const key of Object.keys(newProps)) {
    if (key === "children") continue;
    const next = newProps[key];
    const prev = oldProps[key];
    if (typeof next === "function") {
      // Handler presence is what Rust cares about
      if (typeof prev !== "function") {
        diff[key] = next;
        changed = true;
      }
      continue;
    }
    if (!deepEqual(prev, next)) {
      diff[key] = next;
      changed = true;
    }
  }

  for (const key of Object.keys(oldProps)) {
    if (key === "children") continue;
    if (!(key in newProps)) {
      // Prop removed — include as undefined so serialize/Rust can clear it
      diff[key] = undefined;
      changed = true;
    }
  }

  // String children map to text content
  const oldContent =
    typeof oldProps.children === "string" ? oldProps.children : undefined;
  const newContent =
    typeof newProps.children === "string" ? newProps.children : undefined;
  if (oldContent !== newContent) {
    if (newContent !== undefined) {
      diff.children = newContent;
    } else {
      diff.children = undefined;
    }
    changed = true;
  }

  return changed ? diff : null;
}

/**
 * Serialize props to JSON for the RPC call
 */
function serializeProps(props: Props, type?: string): string {
  const serializable: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(props)) {
    // For text elements, include children as the text content
    if (key === "children") {
      if (typeof value === "string") {
        serializable["content"] = value;
      } else if (value === undefined) {
        serializable["content"] = null;
      }
      continue;
    }

    // Cleared props: send null so Rust can remove components
    if (value === undefined) {
      serializable[key] = null;
      continue;
    }

    // For event handlers, just send a boolean flag
    if (typeof value === "function") {
      serializable[key] = true;
      continue;
    }

    serializable[key] = value;
  }

  return JSON.stringify(serializable);
}

/**
 * Log helper for debugging
 */
function log(...args: unknown[]): void {
    console.log("[bevy-react]", ...args);
}

/**
 * Event payload from the host (keyboard, pointer, wheel/scroll, or absent).
 */
type HostEventData =
  | KeyboardEventData
  | PointerEventData
  | WheelEventData
  | ScrollEventData
  | null
  | undefined;

/** Events that bubble from target toward ancestors until `stopPropagation`. */
const BUBBLING_EVENTS = new Set([
  "click",
  "press",
  "release",
  "mousemove",
  "drag",
  "keydown",
  "keyup",
  "wheel",
  "scroll",
]);

function invokeHandler(
  props: Record<string, unknown>,
  event: string,
  synthetic: Record<string, unknown>
): void {
  switch (event) {
    case "click": {
      const onClick = props.onClick;
      if (typeof onClick === "function") onClick(synthetic);
      break;
    }
    case "press": {
      const onPress = props.onPress;
      if (typeof onPress === "function") onPress(synthetic);
      break;
    }
    case "release": {
      const onRelease = props.onRelease;
      if (typeof onRelease === "function") onRelease(synthetic);
      break;
    }
    case "focus": {
      const onFocus = props.onFocus;
      if (typeof onFocus === "function") onFocus();
      break;
    }
    case "blur": {
      const onBlur = props.onBlur;
      if (typeof onBlur === "function") onBlur();
      break;
    }
    case "keydown": {
      const onKeyDown = props.onKeyDown;
      if (typeof onKeyDown === "function") onKeyDown(synthetic);
      break;
    }
    case "keyup": {
      const onKeyUp = props.onKeyUp;
      if (typeof onKeyUp === "function") onKeyUp(synthetic);
      break;
    }
    case "mouseenter": {
      const onHover = props.onHover;
      if (typeof onHover === "function") onHover(synthetic);
      const onMouseEnter = props.onMouseEnter;
      if (typeof onMouseEnter === "function") onMouseEnter(synthetic);
      break;
    }
    case "mouseleave": {
      const onMouseLeave = props.onMouseLeave;
      if (typeof onMouseLeave === "function") onMouseLeave(synthetic);
      break;
    }
    case "mousemove": {
      const onMouseMove = props.onMouseMove;
      if (typeof onMouseMove === "function") onMouseMove(synthetic);
      break;
    }
    case "drag": {
      const onDrag = props.onDrag;
      if (typeof onDrag === "function") onDrag(synthetic);
      break;
    }
    case "wheel": {
      const onWheel = props.onWheel;
      if (typeof onWheel === "function") onWheel(synthetic);
      break;
    }
    case "scroll": {
      const onScroll = props.onScroll;
      if (typeof onScroll === "function") onScroll(synthetic);
      break;
    }
  }
}

/**
 * Root-scoped event dispatcher called from Rust (via events.ts host bridge).
 * Bubbling events walk `parentId` until `stopPropagation()` or the root.
 */
export function dispatchEvent(
  rootId: string,
  nodeId: number,
  event: string,
  data?: HostEventData
) {
  const target = lookupInstance(rootId, nodeId);
  if (!target) {
    log("Dispatch event: instance not found", rootId, nodeId);
    return;
  }

  log("Dispatch event", event, "rootId:", rootId, "nodeId:", nodeId, data);

  if (!("props" in target) || !target.props) {
    return;
  }

  const bubbles = BUBBLING_EVENTS.has(event);
  const state = { stopped: false };
  const payload =
    data && typeof data === "object" ? { ...(data as object) } : {};

  let currentId: number | undefined = nodeId;
  while (currentId !== undefined) {
    const instance = lookupInstance(rootId, currentId);
    if (!instance || !("props" in instance) || !instance.props) {
      break;
    }

    const synthetic: Record<string, unknown> = {
      ...payload,
      type: event,
      target: nodeId,
      currentTarget: currentId,
      stopPropagation: () => {
        state.stopped = true;
      },
    };

    invokeHandler(instance.props, event, synthetic);

    if (!bubbles || state.stopped) {
      break;
    }

    currentId =
      "parentId" in instance
        ? (instance as BevyInstance).parentId
        : undefined;
  }
}

/**
 * Host config for react-reconciler.
 * Using 'any' to avoid complex type gymnastics with react-reconciler's evolving API.
 */
/**
 * Optional instance lookup injected by roots.ts to avoid a circular import.
 * Falls back to scanning nothing when unset (tests that construct a reconciler
 * directly pass `instanceMap` on the host config instead).
 */
let instanceLookup:
  | ((rootId: string, nodeId: number) => PublicInstance | undefined)
  | null = null;

export function setInstanceLookup(
  fn: (rootId: string, nodeId: number) => PublicInstance | undefined
): void {
  instanceLookup = fn;
}

function lookupInstance(
  rootId: string,
  nodeId: number
): PublicInstance | undefined {
  return instanceLookup?.(rootId, nodeId);
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
class BevyHostConfig {
  constructor(readonly props: ReconcilerProps) {}

  get instanceMap(): BevyInstanceMap {
    return this.props.instanceMap;
  }

  // -------------------
  // Core Configuration
  // -------------------

  supportsMutation = true;
  supportsPersistence = false;
  supportsHydration = false;

  isPrimaryRenderer = true;
  noTimeout = -1;

  // -------------------
  // Scheduling
  // -------------------

  scheduleTimeout = setTimeout;
  cancelTimeout = clearTimeout;
  getCurrentEventPriority = () => 16;

  getInstanceFromNode = (nodeId: number) =>
    this.instanceMap.get(nodeId) || null;
  beforeActiveInstanceBlur = () => {};
  afterActiveInstanceBlur = () => {};
  prepareScopeUpdate = () => {};
  getInstanceFromScope = () => null;
  detachDeletedInstance = (instance: Instance | TextInstance) => {
    this.instanceMap.delete(instance.nodeId);
    // Host text nodes skip detachDeletedInstance in react-reconciler;
    // host components land here. Destroy is also called from removeChild
    // so this is safe if the entity was already despawned.
    __react_destroy_node(this.props.rootId, instance.nodeId);
  };

  // -------------------
  // Host Context
  // -------------------

  getRootHostContext = (): HostContext => ({});
  getChildHostContext = (parentContext: HostContext): HostContext =>
    parentContext;

  // -------------------
  // Instance Creation
  // -------------------

  createInstance = (
    type: Type,
    props: Props,
    _rootContainer: Container,
    _hostContext: HostContext
  ): Instance => {
    const propsJson = serializeProps(props, type);
    const nodeId = __react_create_node(this.props.rootId, type, propsJson);

    const instance = {
      nodeId,
      type,
      props,
      children: [],
    };
    
    this.instanceMap.set(nodeId, instance);
    return instance;
  }

  createTextInstance = (
    text: string,
    _rootContainer: Container,
    _hostContext: HostContext
  ): TextInstance => {
    const nodeId = __react_create_text(this.props.rootId, text);

    const instance = {
      nodeId,
      text,
    };

    this.instanceMap.set(nodeId, instance);
    return instance;
  }

  // -------------------
  // Instance Updates
  // -------------------

  // Kept for older reconciler typings; react-reconciler 0.32 diffs in commitUpdate.
  prepareUpdate = (
    _instance: Instance,
    _type: Type,
    oldProps: Props,
    newProps: Props
  ): UpdatePayload | null => {
    return diffProps(oldProps, newProps);
  }

  commitUpdate = (
    instance: Instance,
    type: Type,
    oldProps: Props,
    newProps: Props
  ): void => {
    // Diff in commit phase (prepareUpdate is unused in reconciler 0.32).
    // Skip RPC when only function identity / identical style objects changed.
    const payload = diffProps(oldProps, newProps);
    if (payload === null) {
      instance.props = newProps;
      return;
    }

    // Send full next props (with cleared keys as null) so Rust can remove stale components.
    // Merge: start from newProps, but ensure removed keys are present as undefined.
    const toSend: Props = { ...newProps };
    for (const key of Object.keys(payload)) {
      if (!(key in newProps)) {
        toSend[key] = undefined;
      }
    }

    const propsJson = serializeProps(toSend, type);
    __react_update_node(this.props.rootId, instance.nodeId, propsJson);
    instance.props = newProps;
  }

  commitTextUpdate = (
    textInstance: TextInstance,
    _oldText: string,
    newText: string
  ): void => {
    __react_update_text(this.props.rootId, textInstance.nodeId, newText);
    textInstance.text = newText;
  }

  // -------------------
  // Tree Operations
  // -------------------

  appendInitialChild = (parent: Instance, child: Instance | TextInstance): void => {
    __react_append_child(this.props.rootId, parent.nodeId, child.nodeId);

    if ("children" in parent && "type" in child) {
      (child as Instance).parentId = parent.nodeId;
      parent.children.push(child as Instance);
    }
  }

  appendChild = (parent: Instance, child: Instance | TextInstance): void => {
    __react_append_child(this.props.rootId, parent.nodeId, child.nodeId);

    if ("children" in parent && "type" in child) {
      (child as Instance).parentId = parent.nodeId;
      parent.children.push(child as Instance);
    }
  }

  appendChildToContainer = (
    container: Container,
    child: Instance | TextInstance
  ): void => {
    __react_append_child(this.props.rootId, container.rootId, child.nodeId);
    if ("type" in child) {
      (child as Instance).parentId = undefined;
    }
  }

  removeChild = (parent: Instance, child: Instance | TextInstance): void => {
    __react_remove_child(this.props.rootId, parent.nodeId, child.nodeId);
    // Despawn now: HostText never gets detachDeletedInstance, and destroying
    // here also covers host components (detachDeletedInstance is then a no-op).
    __react_destroy_node(this.props.rootId, child.nodeId);
    this.instanceMap.delete(child.nodeId);
    if ("type" in child) {
      delete (child as Instance).parentId;
    }
    if ("children" in parent) {
      const idx = parent.children.findIndex((c) => c.nodeId === child.nodeId);
      if (idx !== -1) {
        parent.children.splice(idx, 1);
      }
    }
  }

  removeChildFromContainer = (
    container: Container,
    child: Instance | TextInstance
  ): void => {
    __react_remove_child(this.props.rootId, container.rootId, child.nodeId);
    __react_destroy_node(this.props.rootId, child.nodeId);
    this.instanceMap.delete(child.nodeId);
  }

  insertBefore = (
    parent: Instance,
    child: Instance | TextInstance,
    beforeChild: Instance | TextInstance
  ): void => {
    __react_insert_before(this.props.rootId, parent.nodeId, child.nodeId, beforeChild.nodeId);

    if ("children" in parent && "type" in child) {
      (child as Instance).parentId = parent.nodeId;
      // Remove if already present
      const existingIdx = parent.children.findIndex((c) => c.nodeId === child.nodeId);
      if (existingIdx !== -1) {
        parent.children.splice(existingIdx, 1);
      }
      // Insert before the target
      const beforeIdx = parent.children.findIndex((c) => c.nodeId === beforeChild.nodeId);
      if (beforeIdx !== -1) {
        parent.children.splice(beforeIdx, 0, child as Instance);
      } else {
        parent.children.push(child as Instance);
      }
    }
  }

  insertInContainerBefore = (
    container: Container,
    child: Instance | TextInstance,
    beforeChild: Instance | TextInstance
  ): void => {
    __react_insert_before(this.props.rootId, container.rootId, child.nodeId, beforeChild.nodeId);
  }

  clearContainer = (container: Container): void => {
    __react_clear_container(this.props.rootId);
  }

  // -------------------
  // Finalization
  // -------------------

  finalizeInitialChildren(
    _instance: Instance,
    _type: Type,
    _props: Props,
    _rootContainer: Container,
    _hostContext: HostContext
  ): boolean {
    return false;
  }

  prepareForCommit(_containerInfo: Container): Record<string, unknown> | null {
    return null;
  }

  resetAfterCommit(_containerInfo: Container): void {}

  // -------------------
  // Misc
  // -------------------

  shouldSetTextContent(_type: Type, props: Props): boolean {
    return typeof props.children === "string";
  }

  getPublicInstance(instance: Instance | TextInstance): PublicInstance {
    return instance;
  }

  preparePortalMount(_containerInfo: Container): void {}

  // -------------------
  // React 18+ / Concurrent Mode
  // -------------------

  maySuspendCommit = () => false;
  preloadInstance = () => true;
  startSuspendingCommit = () => {};
  suspendInstance = () => {};
  waitForCommitToBeReady = () => null;
  NotPendingTransition = null;
  setCurrentUpdatePriority = () => {};
  getCurrentUpdatePriority = () => 16;
  resolveUpdatePriority = () => 16;
  resetFormInstance = () => {};
  requestPostPaintCallback = () => {};
  shouldAttemptEagerTransition = () => false;
  trackSchedulerEvent = () => {};
};

type ReconcilerProps = {
  rootId: string;
  instanceMap: BevyInstanceMap;
};

/**
 * Create the reconciler instance
 */
export const createBevyReconciler = (props: ReconcilerProps) => Reconciler(new BevyHostConfig(props) as any);

export type BevyReconciler = ReturnType<typeof createBevyReconciler>;
