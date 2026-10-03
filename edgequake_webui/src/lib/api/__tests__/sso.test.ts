import { describe, expect, it } from "vitest";
import { buildSsoLoginUrl, safeRedirectPath } from "../edgequake/sso";

describe("SPEC-158 sso api helpers", () => {
  it("safeRedirectPath keeps same-origin paths only", () => {
    expect(safeRedirectPath("/documents?x=1")).toBe("/documents?x=1");
    for (const bad of ["//evil.com", "https://evil.com", "/a://b", "/a\\b", "", null, undefined, "relative"]) {
      expect(safeRedirectPath(bad as string | null | undefined), String(bad)).toBeNull();
    }
  });

  it("buildSsoLoginUrl targets the API login endpoint with trimmed org and safe redirect", () => {
    const url = buildSsoLoginUrl({ provider: "keycloak", org: "  acme ", redirect: "/graph" });
    expect(url).toMatch(/\/api\/v1\/auth\/oidc\/login\?/);
    const q = new URL(url, "http://x").searchParams;
    expect(q.get("provider")).toBe("keycloak");
    expect(q.get("org")).toBe("acme");
    expect(q.get("redirect")).toBe("/graph");
  });

  it("omits empty org, root redirect and unsafe redirect", () => {
    const q = new URL(buildSsoLoginUrl({ provider: "p", org: " ", redirect: "/" }), "http://x").searchParams;
    expect(q.has("org")).toBe(false);
    expect(q.has("redirect")).toBe(false);
    const evil = new URL(buildSsoLoginUrl({ provider: "p", redirect: "//evil.com" }), "http://x").searchParams;
    expect(evil.has("redirect")).toBe(false);
  });

  it("allows a provider-less URL (server default provider)", () => {
    const url = buildSsoLoginUrl({ org: "globex" });
    expect(new URL(url, "http://x").searchParams.get("org")).toBe("globex");
    expect(new URL(url, "http://x").searchParams.has("provider")).toBe(false);
  });
});
