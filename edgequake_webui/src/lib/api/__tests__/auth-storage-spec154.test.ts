/**
 * SPEC-154 Wave 5 — access/refresh must not land in localStorage.
 * Uses a minimal in-memory Storage stub (vitest env is node).
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

function installLocalStorageStub() {
  const map = new Map<string, string>();
  const storage = {
    getItem: (k: string) => (map.has(k) ? map.get(k)! : null),
    setItem: (k: string, v: string) => {
      map.set(k, String(v));
    },
    removeItem: (k: string) => {
      map.delete(k);
    },
    clear: () => map.clear(),
    get length() {
      return map.size;
    },
    key: (i: number) => Array.from(map.keys())[i] ?? null,
  };
  vi.stubGlobal("localStorage", storage);
  vi.stubGlobal("window", { localStorage: storage, location: { protocol: "http:" } });
  vi.stubGlobal("document", {
    cookie: "",
  });
  return storage;
}

describe("SPEC-154 auth storage", () => {
  beforeEach(() => {
    installLocalStorageStub();
    vi.resetModules();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("setTokens does not persist access or refresh in localStorage", async () => {
    localStorage.setItem("accessToken", "stale");
    localStorage.setItem("refreshToken", "stale-refresh");
    const { assertNoTokenLocalStorage, setTokens } = await import("../client-context");
    setTokens("mem-access", "body-refresh-ignored");
    expect(localStorage.getItem("accessToken")).toBeNull();
    expect(localStorage.getItem("refreshToken")).toBeNull();
    expect(assertNoTokenLocalStorage()).toBe(true);
  });
});
