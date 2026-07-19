import {
  Component,
  createElement,
  type ErrorInfo,
  type ReactNode,
} from "react";

export type ReportErrorOptions = {
  /** Extra context (e.g. React componentStack). */
  componentStack?: string | null;
};

/**
 * Report a JS / React error to the Bevy host so the in-game overlay can show it.
 * Prefers `__react_report_error`; falls back to `console.error`.
 */
export function reportErrorToHost(
  error: unknown,
  options?: ReportErrorOptions
): void {
  const message = formatErrorMessage(error);
  const stack = formatErrorStack(error, options?.componentStack);

  if (typeof __react_report_error === "function") {
    __react_report_error(message, stack ?? undefined);
    return;
  }

  console.error("[bevy-react]", message);
  if (stack) {
    console.error(stack);
  }
}

function formatErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message || error.name || "Unknown error";
  }
  if (typeof error === "string") {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function formatErrorStack(
  error: unknown,
  componentStack?: string | null
): string | undefined {
  const parts: string[] = [];
  if (error instanceof Error && error.stack) {
    parts.push(error.stack);
  } else if (error && typeof error === "object" && "stack" in error) {
    const s = (error as { stack?: unknown }).stack;
    if (typeof s === "string" && s.length > 0) {
      parts.push(s);
    }
  }
  if (componentStack && componentStack.trim().length > 0) {
    parts.push(`Component stack:${componentStack}`);
  }
  return parts.length > 0 ? parts.join("\n") : undefined;
}

type ErrorBoundaryProps = {
  children?: ReactNode;
};

type ErrorBoundaryState = {
  error: Error | null;
};

/**
 * Host-side error boundary used by createRoot / createBevyApp.
 * Failures report to Bevy (`JsRuntimeError` → in-game overlay).
 */
export class BevyErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    reportErrorToHost(error, { componentStack: info.componentStack });
  }

  render(): ReactNode {
    if (this.state.error) {
      // Host overlay shows the details; keep the React tree empty for this root.
      return null;
    }
    return this.props.children ?? null;
  }
}

/** Wrap an element tree so render failures surface in the Bevy overlay. */
export function withErrorBoundary(element: ReactNode): ReactNode {
  return createElement(BevyErrorBoundary, null, element);
}
