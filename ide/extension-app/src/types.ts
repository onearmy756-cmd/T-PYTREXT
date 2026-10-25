export type Dispose = () => void;

export interface PageMountContext {
  container: HTMLElement;
  path: string;
  navigate: (path: string) => void;
}

export interface AgentServerRequest {
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  path: string;
  body?: unknown;
  headers?: Record<string, string>;
}

export interface CanvasExtensionHost {
  readonly apiVersion: string;
  readonly extension: Readonly<{
    name: string;
    version: string;
    resolvedRef: string | null;
  }>;
  readonly backend: Readonly<{
    id: string;
    kind: "local" | "cloud";
    orgId: string | null;
  }>;
  registerPage: (
    contributionId: string,
    mount: (context: PageMountContext) => void | Dispose | Promise<void | Dispose>,
  ) => Dispose;
  navigate: (path: string) => void;
  agentServer?: {
    request: <T = unknown>(request: AgentServerRequest) => Promise<T>;
  };
}
