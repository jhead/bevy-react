import Reconciler from "react-reconciler";
import type { BevyInstance, BevyTextInstance } from "./types";

type Type = string;
type Props = Record<string, unknown>;
type Container = { rootId: number };
type Instance = BevyInstance;
type TextInstance = BevyTextInstance;
type PublicInstance = Instance | TextInstance;
type HostContext = Record<string, never>;
type UpdatePayload = Props;

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
      }
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
 * Map of node IDs to instances, used for event dispatching
 */
const instanceMap = new Map<number, PublicInstance>();

/**
 * Event data passed from Rust for keyboard events
 */
interface KeyboardEventData {
  key: string;
}

/**
 * Global event dispatcher called from Rust
 */
export function dispatchEvent(nodeId: number, event: string, data?: KeyboardEventData) {
  const instance = instanceMap.get(nodeId);
  if (!instance) {
    log("Dispatch event: instance not found", nodeId);
    return;
  }

  log("Dispatch event", event, "nodeId:", nodeId, data);

  // Text instances don't have props
  if (!("props" in instance) || !instance.props) {
    return;
  }

  const props = instance.props;

  switch (event) {
    case "click": {
      const onClick = props.onClick;
      if (typeof onClick === "function") {
        onClick();
      }
      break;
    }
    case "focus": {
      const onFocus = props.onFocus;
      if (typeof onFocus === "function") {
        onFocus();
      }
      break;
    }
    case "blur": {
      const onBlur = props.onBlur;
      if (typeof onBlur === "function") {
        onBlur();
      }
      break;
    }
    case "keydown": {
      const onKeyDown = props.onKeyDown;
      if (typeof onKeyDown === "function" && data) {
        onKeyDown(data);
      }
      break;
    }
  }
}

/**
 * Host config for react-reconciler.
 * Using 'any' to avoid complex type gymnastics with react-reconciler's evolving API.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
class BevyHostConfig {
  constructor(readonly props: ReconcilerProps) {}

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

  getInstanceFromNode = (nodeId: number) => instanceMap.get(nodeId) || null;
  beforeActiveInstanceBlur = () => {};
  afterActiveInstanceBlur = () => {};
  prepareScopeUpdate = () => {};
  getInstanceFromScope = () => null;
  detachDeletedInstance = (instance: Instance) => {
      instanceMap.delete(instance.nodeId);
      // Ideally we would also tell Rust to destroy the entity here?
      // Or do we assume removeChild was enough?
      // For now, let's at least clean up our map.
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
    
    instanceMap.set(nodeId, instance);
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

    instanceMap.set(nodeId, instance);
    return instance;
  }

  // -------------------
  // Instance Updates
  // -------------------

  prepareUpdate = (
    _instance: Instance,
    _type: Type,
    oldProps: Props,
    newProps: Props
  ): UpdatePayload | null => {
    for (const key in newProps) {
      if (key === "children") continue;
      if (oldProps[key] !== newProps[key]) {
        return newProps;
      }
    }
    for (const key in oldProps) {
      if (key === "children") continue;
      if (!(key in newProps)) {
        return newProps;
      }
    }
    return null;
  }

  commitUpdate = (
    instance: Instance,
    type: Type,
    oldProps: Props,
    newProps: Props
  ): void => {
    const propsJson = serializeProps(newProps, type);
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
      parent.children.push(child as Instance);
    }
  }

  appendChild = (parent: Instance, child: Instance | TextInstance): void => {
    __react_append_child(this.props.rootId, parent.nodeId, child.nodeId);

    if ("children" in parent && "type" in child) {
      parent.children.push(child as Instance);
    }
  }

  appendChildToContainer = (
    container: Container,
    child: Instance | TextInstance
  ): void => {
    __react_append_child(this.props.rootId, container.rootId, child.nodeId);
  }

  removeChild = (parent: Instance, child: Instance | TextInstance): void => {
    __react_remove_child(this.props.rootId, parent.nodeId, child.nodeId);
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
  }

  insertBefore = (
    parent: Instance,
    child: Instance | TextInstance,
    _beforeChild: Instance | TextInstance
  ): void => {
    // For now, we just append - proper ordering requires more RPC support
    __react_append_child(this.props.rootId, parent.nodeId, child.nodeId);
  }

  insertInContainerBefore = (
    container: Container,
    child: Instance | TextInstance,
    _beforeChild: Instance | TextInstance
  ): void => {
    __react_append_child(this.props.rootId, container.rootId, child.nodeId);
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
}

/**
 * Create the reconciler instance
 */
export const createBevyReconciler = (props: ReconcilerProps) => Reconciler(new BevyHostConfig(props) as any);

export type BevyReconciler = ReturnType<typeof createBevyReconciler>;
