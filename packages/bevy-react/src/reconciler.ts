import Reconciler from "react-reconciler";
import type {
  BevyInstance,
  BevyTextInstance,
  ChangeEventData,
  KeyboardEventData,
  PointerEventData,
  ScrollEventData,
  WheelEventData,
} from "./types";
import {
  encodeBatch,
  isBinaryOpsEnabled,
  type BinaryOp,
} from "./protocol";

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
 * Shared JS-side node id allocator for the binary commit path.
 * Advanced past host-allocated ids when mixing is not expected; Rust also
 * bumps its counter on `__react_commit_ops`.
 */
let binaryNextNodeId = 1;

/** @internal test helper */
export function resetBinaryNodeIdCounter(next = 1): void {
  binaryNextNodeId = next;
}

function allocBinaryNodeId(): number {
  return binaryNextNodeId++;
}

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

/** Coerce a single React text child (string | number | boolean) to content. */
function textChildContent(children: unknown): string | undefined {
  if (typeof children === "string") return children;
  if (typeof children === "number" || typeof children === "boolean") {
    return String(children);
  }
  return undefined;
}

/**
 * Flatten text-only children (including arrays like `["count: ", n]`) to a string.
 * Returns undefined when children include host elements / non-text values.
 */
function flattenTextChildren(children: unknown): string | undefined {
  const single = textChildContent(children);
  if (single !== undefined) return single;
  if (!Array.isArray(children)) return undefined;

  let out = "";
  for (const child of children) {
    if (child == null || child === false || child === true) continue;
    const piece = textChildContent(child);
    if (piece === undefined) return undefined;
    out += piece;
  }
  return out;
}

function isTextInstance(
  child: Instance | TextInstance
): child is TextInstance {
  return !("type" in child);
}

function flushBevyTextContent(hostConfig: BevyHostConfig, host: Instance): void {
  const content = (host.textSlots ?? []).map((slot) => slot.text).join("");
  hostConfig.updateNode(host.nodeId, JSON.stringify({ content }));
}

/**
 * Build a props diff for host updates. Returns null when nothing serializable changed.
 * Skips structural `children` (host children) but tracks text content for bevy-text.
 * Treats event handlers as presence flags.
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

  // Text-only children (string/number/boolean or arrays of those) → content.
  const oldContent = flattenTextChildren(oldProps.children);
  const newContent = flattenTextChildren(newProps.children);
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
    // For text hosts, flatten text-only children into `content`.
    // Do not set content when children include elements — HostText folding handles that.
    if (key === "children") {
      if (type === "bevy-text") {
        const content = flattenTextChildren(value);
        if (content !== undefined) {
          serializable["content"] = content;
        } else if (value === undefined || value === null) {
          serializable["content"] = null;
        }
        // else: mixed/host children — leave content unset; slots sync later
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
  | ChangeEventData
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
    case "change": {
      const onChange = props.onChange;
      if (typeof onChange === "function") onChange(synthetic);
      break;
    }
    case "input": {
      const onInput = props.onInput;
      if (typeof onInput === "function") onInput(synthetic);
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
  readonly useBinary: boolean;
  private pendingOps: BinaryOp[] = [];

  constructor(readonly props: ReconcilerProps) {
    this.useBinary = isBinaryOpsEnabled(props.binaryOps);
  }

  get instanceMap(): BevyInstanceMap {
    return this.props.instanceMap;
  }

  /** Queue or immediately send a create-node; returns the node id. */
  createNode(type: string, propsJson: string): number {
    if (this.useBinary) {
      const nodeId = allocBinaryNodeId();
      this.pendingOps.push({
        op: "CreateNode",
        nodeId,
        nodeType: type,
        propsJson,
      });
      return nodeId;
    }
    return __react_create_node(this.props.rootId, type, propsJson);
  }

  createText(content: string): number {
    if (this.useBinary) {
      const nodeId = allocBinaryNodeId();
      this.pendingOps.push({ op: "CreateText", nodeId, content });
      return nodeId;
    }
    return __react_create_text(this.props.rootId, content);
  }

  appendChildRpc(parentId: number, childId: number): void {
    if (this.useBinary) {
      this.pendingOps.push({ op: "AppendChild", parentId, childId });
      return;
    }
    __react_append_child(this.props.rootId, parentId, childId);
  }

  insertBeforeRpc(
    parentId: number,
    childId: number,
    beforeId: number
  ): void {
    if (this.useBinary) {
      this.pendingOps.push({
        op: "InsertBefore",
        parentId,
        childId,
        beforeId,
      });
      return;
    }
    __react_insert_before(this.props.rootId, parentId, childId, beforeId);
  }

  removeChildRpc(parentId: number, childId: number): void {
    if (this.useBinary) {
      this.pendingOps.push({ op: "RemoveChild", parentId, childId });
      return;
    }
    __react_remove_child(this.props.rootId, parentId, childId);
  }

  updateNode(nodeId: number, propsJson: string): void {
    if (this.useBinary) {
      this.pendingOps.push({ op: "UpdateNode", nodeId, propsJson });
      return;
    }
    __react_update_node(this.props.rootId, nodeId, propsJson);
  }

  updateText(nodeId: number, content: string): void {
    if (this.useBinary) {
      this.pendingOps.push({ op: "UpdateText", nodeId, content });
      return;
    }
    __react_update_text(this.props.rootId, nodeId, content);
  }

  destroyNode(nodeId: number): void {
    if (this.useBinary) {
      this.pendingOps.push({ op: "DestroyNode", nodeId });
      return;
    }
    __react_destroy_node(this.props.rootId, nodeId);
  }

  flushBinaryCommit(): void {
    if (!this.useBinary || this.pendingOps.length === 0) {
      return;
    }
    const ops = this.pendingOps;
    this.pendingOps = [];
    ops.push({ op: "Commit" });
    const bytes = encodeBatch(this.props.rootId, ops);
    __react_commit_ops(bytes);
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
    if (isTextInstance(instance)) {
      if (instance.textHost) {
        // Folded into bevy-text — host owns the ECS entity.
        return;
      }
      if (instance.nodeId < 0) {
        return;
      }
      this.instanceMap.delete(instance.nodeId);
      this.destroyNode(instance.nodeId);
      return;
    }
    this.instanceMap.delete(instance.nodeId);
    // Host text nodes skip detachDeletedInstance in react-reconciler;
    // host components land here. Destroy is also called from removeChild
    // so this is safe if the entity was already despawned.
    this.destroyNode(instance.nodeId);
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
    const nodeId = this.createNode(type, propsJson);

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
    // Defer ECS create until we know the parent. Text under `bevy-text` is folded
    // into the host's content (see appendInitialChild) so UpdateText cannot target
    // a sibling entity while glyphs live on the host.
    return {
      nodeId: -1,
      text,
    };
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
    this.updateNode(instance.nodeId, propsJson);
    instance.props = newProps;
  }

  commitTextUpdate = (
    textInstance: TextInstance,
    _oldText: string,
    newText: string
  ): void => {
    textInstance.text = newText;
    if (textInstance.textHost) {
      flushBevyTextContent(this, textInstance.textHost);
      return;
    }
    if (textInstance.nodeId < 0) {
      return;
    }
    this.updateText(textInstance.nodeId, newText);
  }

  resetTextContent = (instance: Instance): void => {
    if (instance.type === "bevy-text") {
      instance.textSlots = [];
      this.updateNode(instance.nodeId, JSON.stringify({ content: "" }));
    }
  }

  // -------------------
  // Tree Operations
  // -------------------

  appendInitialChild = (parent: Instance, child: Instance | TextInstance): void => {
    this.attachChild(parent, child);
  }

  appendChild = (parent: Instance, child: Instance | TextInstance): void => {
    this.attachChild(parent, child);
  }

  private attachChild = (
    parent: Instance,
    child: Instance | TextInstance
  ): void => {
    if (isTextInstance(child) && parent.type === "bevy-text") {
      child.textHost = parent;
      child.nodeId = parent.nodeId;
      parent.textSlots = parent.textSlots ?? [];
      parent.textSlots.push(child);
      flushBevyTextContent(this, parent);
      return;
    }

    if (isTextInstance(child)) {
      if (child.nodeId < 0) {
        child.nodeId = this.createText(child.text);
        this.instanceMap.set(child.nodeId, child);
      }
      this.appendChildRpc(parent.nodeId, child.nodeId);
      return;
    }

    this.appendChildRpc(parent.nodeId, child.nodeId);
    child.parentId = parent.nodeId;
    parent.children.push(child);
  }

  removeChild = (parent: Instance, child: Instance | TextInstance): void => {
    this.detachChild(parent, child);
  }

  removeChildFromContainer = (
    container: Container,
    child: Instance | TextInstance
  ): void => {
    if (isTextInstance(child)) {
      if (child.textHost) {
        const host = child.textHost;
        host.textSlots = (host.textSlots ?? []).filter((s) => s !== child);
        child.textHost = undefined;
        flushBevyTextContent(this, host);
        return;
      }
      if (child.nodeId >= 0) {
        this.removeChildRpc(container.rootId, child.nodeId);
        this.destroyNode(child.nodeId);
        this.instanceMap.delete(child.nodeId);
      }
      return;
    }

    this.removeChildRpc(container.rootId, child.nodeId);
    this.destroyNode(child.nodeId);
    this.instanceMap.delete(child.nodeId);
  }

  private detachChild = (
    parent: Instance,
    child: Instance | TextInstance
  ): void => {
    if (isTextInstance(child) && child.textHost === parent) {
      parent.textSlots = (parent.textSlots ?? []).filter((s) => s !== child);
      child.textHost = undefined;
      flushBevyTextContent(this, parent);
      return;
    }

    if (isTextInstance(child)) {
      if (child.nodeId >= 0) {
        this.removeChildRpc(parent.nodeId, child.nodeId);
        this.destroyNode(child.nodeId);
        this.instanceMap.delete(child.nodeId);
      }
      return;
    }

    this.removeChildRpc(parent.nodeId, child.nodeId);
    this.destroyNode(child.nodeId);
    this.instanceMap.delete(child.nodeId);
    delete child.parentId;
    const idx = parent.children.findIndex((c) => c.nodeId === child.nodeId);
    if (idx !== -1) {
      parent.children.splice(idx, 1);
    }
  }

  appendChildToContainer = (
    container: Container,
    child: Instance | TextInstance
  ): void => {
    if (isTextInstance(child)) {
      if (child.nodeId < 0) {
        child.nodeId = this.createText(child.text);
        this.instanceMap.set(child.nodeId, child);
      }
      this.appendChildRpc(container.rootId, child.nodeId);
      return;
    }
    this.appendChildRpc(container.rootId, child.nodeId);
    child.parentId = undefined;
  }

  insertBefore = (
    parent: Instance,
    child: Instance | TextInstance,
    beforeChild: Instance | TextInstance
  ): void => {
    if (isTextInstance(child) && parent.type === "bevy-text") {
      child.textHost = parent;
      child.nodeId = parent.nodeId;
      parent.textSlots = parent.textSlots ?? [];
      const beforeIdx = isTextInstance(beforeChild)
        ? parent.textSlots.indexOf(beforeChild)
        : parent.textSlots.length;
      const existingIdx = parent.textSlots.indexOf(child);
      if (existingIdx !== -1) {
        parent.textSlots.splice(existingIdx, 1);
      }
      if (beforeIdx >= 0) {
        parent.textSlots.splice(beforeIdx, 0, child);
      } else {
        parent.textSlots.push(child);
      }
      flushBevyTextContent(this, parent);
      return;
    }

    if (isTextInstance(child) && child.nodeId < 0) {
      child.nodeId = this.createText(child.text);
      this.instanceMap.set(child.nodeId, child);
    }
    if (isTextInstance(beforeChild) && beforeChild.nodeId < 0) {
      return;
    }

    this.insertBeforeRpc(
      parent.nodeId,
      child.nodeId,
      beforeChild.nodeId
    );

    if (!isTextInstance(child)) {
      child.parentId = parent.nodeId;
      const existingIdx = parent.children.findIndex((c) => c.nodeId === child.nodeId);
      if (existingIdx !== -1) {
        parent.children.splice(existingIdx, 1);
      }
      const beforeIdx = !isTextInstance(beforeChild)
        ? parent.children.findIndex((c) => c.nodeId === beforeChild.nodeId)
        : -1;
      if (beforeIdx !== -1) {
        parent.children.splice(beforeIdx, 0, child);
      } else {
        parent.children.push(child);
      }
    }
  }

  insertInContainerBefore = (
    container: Container,
    child: Instance | TextInstance,
    beforeChild: Instance | TextInstance
  ): void => {
    if (isTextInstance(child) && child.nodeId < 0) {
      child.nodeId = this.createText(child.text);
      this.instanceMap.set(child.nodeId, child);
    }
    this.insertBeforeRpc(
      container.rootId,
      child.nodeId,
      beforeChild.nodeId
    );
  }

  clearContainer = (_container: Container): void => {
    // Concurrent roots call clearContainer AFTER creating the new tree and
    // BEFORE appendChildToContainer. Despawning here wiped every consumer's
    // first paint (Create → Append → Clear → root append fails). Removals go
    // through removeChild / removeChildFromContainer instead.
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

  prepareForCommit = (_containerInfo: Container): Record<string, unknown> | null => {
    return null;
  }

  resetAfterCommit = (_containerInfo: Container): void => {
    this.flushBinaryCommit();
  }

  // -------------------
  // Misc
  // -------------------

  shouldSetTextContent = (type: Type, props: Props): boolean => {
    // Prefer host text content for bevy-text when children are text-only
    // (including `["label: ", n]`). Falls back to HostText folding if React
    // still creates text instances.
    if (type === "bevy-text") {
      return flattenTextChildren(props.children) !== undefined;
    }
    return flattenTextChildren(props.children) !== undefined;
  };

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
  /**
   * When true, batch host mutations into one BRRP frame per commit.
   * When omitted: `__BEVY_REACT_BINARY_OPS`, else auto-detect
   * `__react_commit_ops`. Pass `false` to force per-op enum natives.
   */
  binaryOps?: boolean;
};

/**
 * Create the reconciler instance
 */
export const createBevyReconciler = (props: ReconcilerProps) => Reconciler(new BevyHostConfig(props) as any);

export type BevyReconciler = ReturnType<typeof createBevyReconciler>;
export type { ReconcilerProps as BevyReconcilerProps };