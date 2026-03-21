/**
 * React reconciler host config for KittyUI.
 *
 * Maps React's tree operations to mutations on a RenderableTree.
 */

import { type BoxRenderable, type ImageRenderable, type KittyProps, type TextRenderable, createRenderableForType } from "./renderables.js";
import type { RenderableTree } from "@kittyui/core";

// ---------------------------------------------------------------------------
// Type aliases for the host config generics
// ---------------------------------------------------------------------------

/** JSX element tag name (e.g. "box", "text"). */
type Type = string;

type Props = KittyProps;

/** The container is a RenderableTree with a synthetic root renderable. */
export interface Container {
  root: BoxRenderable;
  tree: RenderableTree;
}

/** A host instance is one of our Renderable subclasses. */
type Instance = BoxRenderable | ImageRenderable | TextRenderable;

/** A text instance wraps a TextRenderable holding raw string content. */
type TextInstance = TextRenderable;

type PublicInstance = Instance | TextInstance;
type HostContext = Record<string, never>;

const NO_TIMEOUT = -1;
type NoTimeout = typeof NO_TIMEOUT;
type TransitionStatus = never;

// ---------------------------------------------------------------------------
// Priority helpers (required by react-reconciler >=0.30)
// ---------------------------------------------------------------------------

const DEFAULT_EVENT_PRIORITY = 2;
let currentUpdatePriority = 0;

// ---------------------------------------------------------------------------
// Commit update helper — extracted to satisfy max-params lint rule
// ---------------------------------------------------------------------------

interface CommitUpdateArgs {
  instance: Instance;
  nextProps: Props;
}

const applyCommitUpdate = ({ instance, nextProps }: CommitUpdateArgs): void => {
  instance.applyProps(nextProps);
};

// ---------------------------------------------------------------------------
// Host config
// ---------------------------------------------------------------------------

export const hostConfig = {
  afterActiveInstanceBlur(): void {},

  appendChild(parentInstance: Instance, child: Instance | TextInstance): void {
    void parentInstance;
    void child;
  },

  appendChildToContainer(container: Container, child: Instance | TextInstance): void {
    container.tree.appendChild(container.root.nodeId, child);
  },

  appendInitialChild(parentInstance: Instance, child: Instance | TextInstance): void {
    void parentInstance;
    void child;
  },

  beforeActiveInstanceBlur(): void {},

  cancelTimeout: clearTimeout,

  clearContainer(container: Container): void {
    const children = container.tree.children(container.root.nodeId);
    for (const child of children) {
      container.tree.remove(child.nodeId);
    }
  },

  commitTextUpdate(textInstance: TextInstance, _oldText: string, newText: string): void {
    textInstance.setText(newText);
  },

  // eslint-disable-next-line max-params -- Required by react-reconciler API
  commitUpdate(instance: Instance, _type: Type, _prevProps: Props, nextProps: Props): void {
    applyCommitUpdate({ instance, nextProps });
  },

  createInstance(type: Type, props: Props, _rootContainer: Container): Instance {
    const instance = createRenderableForType(type);
    instance.applyProps(props);
    return instance;
  },

  createTextInstance(text: string, _rootContainer: Container): TextInstance {
    const instance = createRenderableForType("text") as TextRenderable;
    instance.setText(text);
    return instance;
  },

  detachDeletedInstance(): void {},

  finalizeInitialChildren(): boolean {
    return false;
  },

  getChildHostContext(parentHostContext: HostContext): HostContext {
    return parentHostContext;
  },

  getCurrentUpdatePriority(): number {
    return currentUpdatePriority;
  },

  getInstanceFromNode(): undefined {
    return undefined;
  },

  getInstanceFromScope(): undefined {
    return undefined;
  },

  getPublicInstance(instance: Instance | TextInstance): PublicInstance {
    return instance;
  },

  getRootHostContext(): HostContext {
    return {};
  },

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  HostTransitionContext: undefined as any,

  hideInstance(instance: Instance): void {
    void instance;
  },

  hideTextInstance(textInstance: TextInstance): void {
    void textInstance;
  },

  insertBefore(parentInstance: Instance, child: Instance | TextInstance, _beforeChild: Instance | TextInstance): void {
    void parentInstance;
    void child;
  },

  insertInContainerBefore(
    container: Container,
    child: Instance | TextInstance,
    beforeChild: Instance | TextInstance,
  ): void {
    container.tree.insertBefore(container.root.nodeId, child, beforeChild.nodeId);
  },

  isPrimaryRenderer: true,

  maySuspendCommit(): boolean {
    return false;
  },

  NotPendingTransition: undefined as TransitionStatus | undefined,

  noTimeout: NO_TIMEOUT as NoTimeout,

  // eslint-disable-next-line unicorn/no-null -- Required by react-reconciler API
  prepareForCommit(_containerInfo: Container): null {
    // eslint-disable-next-line unicorn/no-null
    return null;
  },

  preloadInstance(): boolean {
    return true;
  },

  preparePortalMount(): void {},

  prepareScopeUpdate(): void {},

  removeChild(parentInstance: Instance, child: Instance | TextInstance): void {
    void parentInstance;
    void child;
  },

  removeChildFromContainer(container: Container, child: Instance | TextInstance): void {
    container.tree.remove(child.nodeId);
  },

  requestPostPaintCallback(): void {},

  resetAfterCommit(_containerInfo: Container): void {},

  resetFormInstance(): void {},

  resetTextContent(instance: Instance): void {
    instance.setText(undefined);
  },

  resolveEventTimeStamp(): number {
    return Date.now();
  },

  // eslint-disable-next-line unicorn/no-null -- Required by react-reconciler API
  resolveEventType(): null {
    // eslint-disable-next-line unicorn/no-null
    return null;
  },

  resolveUpdatePriority(): number {
    return currentUpdatePriority || DEFAULT_EVENT_PRIORITY;
  },

  scheduleMicrotask: queueMicrotask,

  scheduleTimeout: setTimeout,

  setCurrentUpdatePriority(newPriority: number): void {
    currentUpdatePriority = newPriority;
  },

  shouldAttemptEagerTransition(): boolean {
    return false;
  },

  shouldSetTextContent(_type: Type, _props: Props): boolean {
    return false;
  },

  startSuspendingCommit(): void {},

  supportsHydration: false,

  supportsMicrotasks: true,

  supportsMutation: true,

  supportsPersistence: false,

  suspendInstance(): void {},

  trackSchedulerEvent(): void {},

  unhideInstance(instance: Instance): void {
    void instance;
  },

  unhideTextInstance(textInstance: TextInstance, _text: string): void {
    void textInstance;
  },

  // eslint-disable-next-line unicorn/no-null -- Required by react-reconciler API
  waitForCommitToBeReady(): null {
    // eslint-disable-next-line unicorn/no-null
    return null;
  },
};
