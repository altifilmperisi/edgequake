/**
 * Browser WebSocket auth helpers (SPEC-154).
 * Browsers cannot set Authorization on `WebSocket()` — use Sec-WebSocket-Protocol.
 */

import { getTokens } from "@/lib/api/client-context";

/** Sentinel subprotocol echoed by the server. */
export const WS_AUTH_PROTOCOL = "edgequake.bearer";

/**
 * URL helper — never appends `?token=` (server rejects query credentials).
 */
export function withAuthToken(url: string): string {
  return url;
}

/** Protocols for `new WebSocket(url, protocols)` when an access token is present. */
export function websocketAuthProtocols(): string[] | undefined {
  const { accessToken } = getTokens();
  if (!accessToken) {
    return undefined;
  }
  return [WS_AUTH_PROTOCOL, accessToken];
}
