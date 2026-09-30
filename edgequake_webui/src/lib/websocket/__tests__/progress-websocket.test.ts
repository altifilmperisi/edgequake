/**
 * SPEC-149 — ProgressWebSocket lifecycle (production class, not reimplemented helpers).
 * SPEC-154 — auth via Sec-WebSocket-Protocol, never `?token=`.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProgressWebSocket } from "../progress-websocket";
import { withAuthToken } from "../ws-auth";

class FakeWebSocket {
  static OPEN = 1;
  static CONNECTING = 0;
  static CLOSING = 2;
  static CLOSED = 3;
  static instances: FakeWebSocket[] = [];

  readyState = FakeWebSocket.CONNECTING;
  url: string;
  protocols: string | string[] | undefined;
  onopen: ((ev: Event) => void) | null = null;
  onclose: ((ev: CloseEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  sent: string[] = [];

  constructor(url: string, protocols?: string | string[]) {
    this.url = url;
    this.protocols = protocols;
    FakeWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.(
      new CloseEvent("close", { code: 1000, reason: "close", wasClean: true }),
    );
  }

  open() {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.(new Event("open"));
  }

  dirtyClose() {
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.(
      new CloseEvent("close", { code: 1006, reason: "abnormal", wasClean: false }),
    );
  }
}

vi.mock("@/lib/api/client-context", () => ({
  getTokens: vi.fn(() => ({ accessToken: null as string | null, refreshToken: null })),
}));

describe("ProgressWebSocket SPEC-149 / SPEC-154", () => {
  beforeEach(async () => {
    FakeWebSocket.instances = [];
    vi.stubGlobal("WebSocket", FakeWebSocket as unknown as typeof WebSocket);
    const { getTokens } = await import("@/lib/api/client-context");
    vi.mocked(getTokens).mockReturnValue({
      accessToken: null,
      refreshToken: null,
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("withAuthToken never appends query token", () => {
    expect(withAuthToken("ws://example.test/ws")).not.toContain("token=");
    expect(withAuthToken("ws://example.test/ws?x=1")).not.toContain("token=");
  });

  it("U-149-01 resolves URL without query token; protocol carries JWT", async () => {
    const { getTokens } = await import("@/lib/api/client-context");
    vi.mocked(getTokens).mockReturnValue({
      accessToken: null,
      refreshToken: null,
    });

    const client = new ProgressWebSocket({
      urlResolver: () => "ws://example.test/ws/pipeline/progress",
      maxReconnectAttempts: 3,
    });

    client.connect();
    expect(FakeWebSocket.instances[0].url).toBe(
      "ws://example.test/ws/pipeline/progress",
    );
    expect(FakeWebSocket.instances[0].url).not.toContain("token=");
    expect(FakeWebSocket.instances[0].protocols).toBeUndefined();
    FakeWebSocket.instances[0].open();

    vi.mocked(getTokens).mockReturnValue({
      accessToken: "tok-2",
      refreshToken: null,
    });
    client.reconnectFresh();
    const next = FakeWebSocket.instances.at(-1)!;
    expect(next.url).not.toContain("token=");
    expect(next.protocols).toEqual(["edgequake.bearer", "tok-2"]);
    client.disconnect();
  });

  it("U-149-03 replays desired subscriptions after reconnect", () => {
    const client = new ProgressWebSocket({
      urlResolver: () => "ws://example.test/ws",
    });
    client.connect();
    FakeWebSocket.instances[0].open();
    client.subscribe(["track-a"]);
    expect(FakeWebSocket.instances[0].sent.some((s) => s.includes("track-a"))).toBe(
      true,
    );

    client.reconnectFresh();
    const next = FakeWebSocket.instances.at(-1)!;
    next.open();
    expect(next.sent.some((s) => s.includes("subscribe") && s.includes("track-a"))).toBe(
      true,
    );
    client.disconnect();
  });

  it("U-149-04 guards CONNECTING and resets attempts on reconnectFresh", () => {
    vi.useFakeTimers();
    const client = new ProgressWebSocket({
      urlResolver: () => "ws://example.test/ws",
      reconnectInterval: 10,
      maxReconnectAttempts: 3,
      maxReconnectDelayMs: 100,
    });

    client.connect();
    expect(FakeWebSocket.instances).toHaveLength(1);
    client.connect(); // CONNECTING — no second socket
    expect(FakeWebSocket.instances).toHaveLength(1);

    FakeWebSocket.instances[0].dirtyClose();
    vi.advanceTimersByTime(20);
    expect(FakeWebSocket.instances.length).toBeGreaterThan(1);

    client.reconnectFresh();
    const after = FakeWebSocket.instances.at(-1)!;
    after.open();
    after.dirtyClose();
    vi.advanceTimersByTime(20);
    expect(FakeWebSocket.instances.length).toBeGreaterThan(2);
    client.disconnect();
  });
});
