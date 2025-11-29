/**
 * Native functions exposed by Rust to the JS global scope.
 * These are the RPC interface between React and Bevy.
 */
declare global {
  const console: {
    log: (...args: unknown[]) => void;
    warn: (...args: unknown[]) => void;
    error: (...args: unknown[]) => void;
  };

  function setTimeout(callback: () => void, delay?: number): number;
  function clearTimeout(handle: number): void;
  
  /**
   * Creates a UI node (NodeBundle, ButtonBundle, ImageBundle, etc.)
   * @param type - The node type: "node", "button", "image"
   * @param propsJson - JSON string of props including style
   * @returns The node ID for future reference
   */
  function __react_create_node(rootId: string, type: string, propsJson: string): number;

  /**
   * Creates a text node (TextBundle)
   * @param content - The text content to display
   * @returns The node ID for future reference
   */
  function __react_create_text(rootId: string, content: string): number;

  /**
   * Appends a child node to a parent node
   * @param parentId - The parent node ID (0 for root container)
   * @param childId - The child node ID to append
   */
  function __react_append_child(rootId: string, parentId: number, childId: number): void;

  /**
   * Removes a child node from a parent node
   * @param parentId - The parent node ID
   * @param childId - The child node ID to remove
   */
  function __react_remove_child(rootId: string, parentId: number, childId: number): void;

  /**
   * Updates a node's props
   * @param nodeId - The node ID to update
   * @param propsJson - JSON string of updated props
   */
  function __react_update_node(rootId: string, nodeId: number, propsJson: string): void;

  /**
   * Updates a text node's content
   * @param nodeId - The text node ID to update
   * @param content - The new text content
   */
  function __react_update_text(rootId: string, nodeId: number, content: string): void;

  /**
   * Destroys a node and its resources
   * @param nodeId - The node ID to destroy
   */
  function __react_destroy_node(rootId: string, nodeId: number): void;

  /**
   * Clears all nodes from the root container
   */
  function __react_clear_container(rootId: string): void;
}

export {};

