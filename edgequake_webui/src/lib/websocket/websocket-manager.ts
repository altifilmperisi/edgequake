/**
 * @module websocket-manager
 * @description WebSocket Manager Singleton (SPEC-149).
 *
 * Provides a single shared WebSocket connection for the application.
 * URL + auth token are resolved on every connect attempt (LAW-149-1).
 *
 * IMPORTANT: Prefer reconnectFresh() over destroying the singleton. Provider
 * listeners bind once to the instance; replacing it orphans those listeners
 * and leaves the UI stuck Offline.
 *
 * @implements FEAT0722 - Singleton WebSocket connection
 * @implements FEAT0723 - Auto-reconnect on disconnect
 * @implements SPEC-149 - Credential-aware reset / reconnect
 */

import { getRuntimeServerBaseUrl } from "@/lib/runtime-config";
import { ProgressWebSocket } from "./progress-websocket";
import { withAuthToken } from "./ws-auth";

export { withAuthToken, websocketAuthProtocols, WS_AUTH_PROTOCOL } from "./ws-auth";

let instance: ProgressWebSocket | null = null;

/**
 * Resolve the WebSocket URL from the current runtime config + tokens.
 * Called on every connect (never cached on the singleton).
 */
export function resolveWebSocketUrl(): string {
  const baseUrl = getRuntimeServerBaseUrl();

  if (baseUrl) {
    const wsUrl = baseUrl.replace(/^https:/, "wss:").replace(/^http:/, "ws:");
    return withAuthToken(`${wsUrl}/ws/pipeline/progress`);
  }

  if (typeof window !== "undefined") {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return withAuthToken(
      `${protocol}//${window.location.host}/ws/pipeline/progress`,
    );
  }

  return "/ws/pipeline/progress";
}

/**
 * Get the shared WebSocket client instance.
 * Creates a new instance if one doesn't exist.
 */
export function getWebSocketClient(): ProgressWebSocket {
  if (!instance) {
    instance = new ProgressWebSocket({
      urlResolver: resolveWebSocketUrl,
      reconnectInterval: 3000,
      maxReconnectAttempts: 10,
      maxReconnectDelayMs: 30_000,
      heartbeatInterval: 30000,
    });
  }
  return instance;
}

/**
 * Disconnect the socket and clear desired subscriptions, but keep the
 * singleton so existing event listeners remain valid (LAW-149-8).
 */
export function disconnectWebSocket(): void {
  if (instance) {
    instance.disconnect();
    instance.clearDesiredSubscriptions();
  }
}

/**
 * Drop the singleton entirely (provider unmount). Next getWebSocketClient()
 * creates a fresh instance that must be re-bound by the provider.
 */
export function destroyWebSocketClient(): void {
  disconnectWebSocket();
  instance = null;
}

/**
 * Reconnect on the existing singleton with a freshly resolved URL.
 * Preserves listeners and desired subscriptions (replayed on open).
 */
export function resetWebSocketClient(): ProgressWebSocket {
  const client = getWebSocketClient();
  client.reconnectFresh();
  return client;
}

/**
 * Manual recovery helper used by banner + toast Retry actions.
 */
export function reconnectRealtime(): ProgressWebSocket {
  return resetWebSocketClient();
}

/**
 * Check if the WebSocket client is connected.
 */
export function isWebSocketConnected(): boolean {
  return instance?.connected ?? false;
}

/**
 * Check if the WebSocket client is reconnecting.
 */
export function isWebSocketReconnecting(): boolean {
  return instance?.reconnecting ?? false;
}
