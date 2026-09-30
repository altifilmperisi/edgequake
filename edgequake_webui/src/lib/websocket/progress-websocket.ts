/**
 * @module progress-websocket
 * @description WebSocket client for multiplexed ingestion progress (SPEC-149).
 *
 * @implements FEAT0724 - Real-time ingestion progress
 * @implements FEAT0725 - Heartbeat keep-alive
 * @implements FEAT0726 - Subscription-based updates
 * @implements SPEC-149 - Auth-aware reconnect, desired-sub replay, CONNECTING guard
 */

import type {
  ClientCommand,
  WebSocketProgressMessage,
} from "@/types/ingestion";
import { normalizeProgressEvent } from "./progress-event-normalizer";
import { websocketAuthProtocols } from "./ws-auth";

export interface ProgressWebSocketOptions {
  /** Resolve the WS URL on every connect (LAW-149-1). */
  urlResolver?: () => string;
  /** @deprecated Prefer urlResolver — static URL freezes credentials. */
  url?: string;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  /** Cap exponential backoff delay (ms). */
  maxReconnectDelayMs?: number;
  heartbeatInterval?: number;
  onConnected?: () => void;
  onDisconnected?: (event: { code: number; reason: string }) => void;
  onReconnecting?: (attempt: number) => void;
  onMaxReconnectsReached?: () => void;
  onError?: (error: Error) => void;
  onMessage?: (message: WebSocketProgressMessage) => void;
}

type WebSocketEventType =
  | "connected"
  | "disconnected"
  | "reconnecting"
  | "max_reconnects_reached"
  | "error"
  | "progress"
  | "status_snapshot"
  | "pdf_progress"
  | "graph_storage_progress";

type WebSocketEventCallback = (...args: unknown[]) => void;

/**
 * WebSocket client for real-time ingestion progress tracking.
 */
export class ProgressWebSocket {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private heartbeatTimer?: ReturnType<typeof setInterval>;
  private reconnectTimer?: ReturnType<typeof setTimeout>;
  private messageQueue: ClientCommand[] = [];
  private listeners: Map<WebSocketEventType, Set<WebSocketEventCallback>> =
    new Map();
  /** Desired track subscriptions — replayed after every reconnect (LAW-149-7). */
  private desiredSubs = new Set<string>();
  /** Ignores callbacks from superseded sockets (LAW-149-8). */
  private generation = 0;
  private intentionalClose = false;

  public readonly options: Required<
    Omit<
      ProgressWebSocketOptions,
      | "url"
      | "onConnected"
      | "onDisconnected"
      | "onReconnecting"
      | "onMaxReconnectsReached"
      | "onError"
      | "onMessage"
    >
  > &
    Partial<
      Pick<
        ProgressWebSocketOptions,
        | "onConnected"
        | "onDisconnected"
        | "onReconnecting"
        | "onMaxReconnectsReached"
        | "onError"
        | "onMessage"
      >
    >;

  private _connected = false;
  private _reconnecting = false;

  get connected(): boolean {
    return this._connected;
  }

  get reconnecting(): boolean {
    return this._reconnecting;
  }

  /** Snapshot of desired subscription track ids (tests / diagnostics). */
  getDesiredSubscriptions(): string[] {
    return [...this.desiredSubs].sort();
  }

  constructor(options: ProgressWebSocketOptions) {
    const { urlResolver: providedResolver, url, ...rest } = options;
    const urlResolver =
      providedResolver ??
      (() => {
        if (!url) {
          throw new Error("ProgressWebSocket requires urlResolver or url");
        }
        return url;
      });

    this.options = {
      reconnectInterval: 3000,
      maxReconnectAttempts: 10,
      maxReconnectDelayMs: 30_000,
      heartbeatInterval: 30000,
      ...rest,
      urlResolver,
    };
  }

  /**
   * Connect to the WebSocket server (idempotent for OPEN and CONNECTING).
   */
  connect(): void {
    if (
      this.ws?.readyState === WebSocket.OPEN ||
      this.ws?.readyState === WebSocket.CONNECTING
    ) {
      return;
    }

    this.intentionalClose = false;
    const gen = ++this.generation;

    try {
      const url = this.options.urlResolver();
      const protocols = websocketAuthProtocols();
      this.ws = protocols
        ? new WebSocket(url, protocols)
        : new WebSocket(url);
      this.setupEventHandlers(gen);
    } catch (error) {
      this.handleError(error as Error);
    }
  }

  /**
   * Reset backoff and reconnect with a freshly resolved URL (Retry / token refresh).
   */
  reconnectFresh(): void {
    this.stopHeartbeat();
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
    this.reconnectAttempts = 0;
    this._reconnecting = false;
    this.intentionalClose = true;
    if (this.ws) {
      try {
        this.ws.close(1000, "Client reconnect");
      } catch {
        /* ignore */
      }
      this.ws = null;
    }
    this._connected = false;
    this.intentionalClose = false;
    this.connect();
  }

  private setupEventHandlers(gen: number): void {
    if (!this.ws) return;
    const socket = this.ws;

    socket.onopen = () => {
      if (gen !== this.generation) return;
      this._connected = true;
      this._reconnecting = false;
      this.reconnectAttempts = 0;
      this.startHeartbeat();
      this.flushMessageQueue();
      this.replayDesiredSubscriptions();
      this.emit("connected");
      this.options.onConnected?.();
    };

    socket.onmessage = (event) => {
      if (gen !== this.generation) return;
      try {
        const raw = JSON.parse(event.data as string) as unknown;
        const message = normalizeProgressEvent(raw);
        if (!message) {
          console.warn(
            "[ProgressWebSocket] Unrecognized message shape:",
            typeof raw === "object" && raw && "type" in raw
              ? (raw as { type?: string }).type
              : raw,
          );
          return;
        }
        this.handleMessage(message);
      } catch (error) {
        console.error("[ProgressWebSocket] Failed to parse message:", error);
      }
    };

    socket.onclose = (event) => {
      if (gen !== this.generation) return;
      this._connected = false;
      this.stopHeartbeat();
      this.emit("disconnected", { code: event.code, reason: event.reason });
      this.options.onDisconnected?.({ code: event.code, reason: event.reason });

      if (!this.intentionalClose && !event.wasClean) {
        this.attemptReconnect();
      }
    };

    socket.onerror = () => {
      if (gen !== this.generation) return;
      this.handleError(new Error("WebSocket connection error"));
    };
  }

  private replayDesiredSubscriptions(): void {
    if (this.desiredSubs.size === 0) return;
    const trackIds = [...this.desiredSubs];
    this.send({ type: "subscribe", track_ids: trackIds });
  }

  private handleMessage(message: WebSocketProgressMessage): void {
    switch (message.type) {
      case "heartbeat":
      case "Heartbeat":
        break;
      case "Connected":
        console.log("[ProgressWebSocket] Backend confirmed connection");
        break;
      case "SubscribedAck":
        break;
      case "StatusSnapshot":
        this.emit("status_snapshot", message);
        this.options.onMessage?.(message);
        break;
      case "PdfPageProgress":
        this.emit("pdf_progress", message);
        this.options.onMessage?.(message);
        break;
      case "GraphStorageProgress":
        this.emit("graph_storage_progress", message);
        this.emit("progress", message);
        this.options.onMessage?.(message);
        break;
      case "ProgressSnapshot":
        this.emit("pdf_progress", message);
        this.options.onMessage?.(message);
        break;
      case "ingestion_started":
      case "stage_started":
      case "stage_progress":
      case "stage_completed":
      case "ingestion_completed":
      case "ingestion_failed":
      case "ChunkProgress":
      case "StageTransition":
      case "ChunkFailure":
      case "DeletionStarted":
      case "DeletionPhase":
      case "DeletionCompleted":
      case "DeletionFailed":
      case "BulkDeletionStarted":
      case "BulkDeletionItemProgress":
      case "BulkDeletionCompleted":
      case "BulkDeletionFailed":
        this.emit("progress", message);
        this.options.onMessage?.(message);
        break;
      default:
        console.warn(
          "[ProgressWebSocket] Unknown message type:",
          (message as { type?: string }).type,
        );
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = setInterval(() => {
      this.send({ type: "ping", client_time: new Date().toISOString() });
    }, this.options.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = undefined;
    }
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      this.emit("max_reconnects_reached");
      this.options.onMaxReconnectsReached?.();
      return;
    }

    this._reconnecting = true;
    this.reconnectAttempts++;

    const rawDelay =
      this.options.reconnectInterval *
      Math.pow(2, this.reconnectAttempts - 1);
    const delay = Math.min(rawDelay, this.options.maxReconnectDelayMs);

    this.emit("reconnecting", this.reconnectAttempts);
    this.options.onReconnecting?.(this.reconnectAttempts);

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, delay);
  }

  private handleError(error: Error): void {
    if (process.env.NODE_ENV === "development") {
      console.warn(
        "[ProgressWebSocket] Connection unavailable - backend may not be running",
      );
    }
    this.emit("error", error);
    this.options.onError?.(error);
  }

  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift();
      if (message) {
        this.send(message);
      }
    }
  }

  send(command: ClientCommand): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(command));
    } else {
      this.messageQueue.push(command);
    }
  }

  /**
   * Subscribe to ingestion progress updates for specific track IDs.
   * Updates the durable desired set and connects if needed.
   */
  subscribe(trackIds: string[]): void {
    for (const id of trackIds) {
      if (id) this.desiredSubs.add(id);
    }
    if (
      this.ws?.readyState !== WebSocket.OPEN &&
      this.ws?.readyState !== WebSocket.CONNECTING
    ) {
      this.connect();
    }
    this.send({ type: "subscribe", track_ids: trackIds });
  }

  unsubscribe(trackIds: string[]): void {
    for (const id of trackIds) {
      this.desiredSubs.delete(id);
    }
    this.send({ type: "unsubscribe", track_ids: trackIds });
  }

  clearDesiredSubscriptions(): void {
    this.desiredSubs.clear();
  }

  cancel(trackId: string): void {
    this.send({ type: "cancel", track_id: trackId });
  }

  disconnect(): void {
    this.intentionalClose = true;
    this.stopHeartbeat();
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
    this.generation++;
    if (this.ws) {
      this.ws.close(1000, "Client disconnect");
      this.ws = null;
    }
    this._connected = false;
    this._reconnecting = false;
    this.reconnectAttempts = 0;
    this.messageQueue = [];
  }

  on(event: WebSocketEventType, callback: WebSocketEventCallback): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(callback);
    return () => {
      this.listeners.get(event)?.delete(callback);
    };
  }

  off(event: WebSocketEventType, callback: WebSocketEventCallback): void {
    this.listeners.get(event)?.delete(callback);
  }

  private emit(event: WebSocketEventType, ...args: unknown[]): void {
    this.listeners.get(event)?.forEach((callback) => {
      try {
        callback(...args);
      } catch (error) {
        console.error("[ProgressWebSocket] Error in event handler:", error);
      }
    });
  }
}
