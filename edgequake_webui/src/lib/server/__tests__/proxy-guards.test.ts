import { describe, expect, it } from "vitest";
import { NextRequest } from "next/server";
import {
  applyAuthGuard,
  applySwaggerSlashRedirect,
  authRequired,
  isPublicPath,
} from "../proxy-guards";

function req(path: string, cookie?: string): NextRequest {
  const headers = new Headers();
  if (cookie) {
    headers.set("cookie", `edgequake_access_token=${cookie}`);
  }
  return new NextRequest(new URL(path, "http://localhost:3000"), { headers });
}

describe("proxy-guards (SPEC-144)", () => {
  it("authRequired is false when auth env unset", () => {
    expect(authRequired({})).toBe(false);
  });

  it("authRequired is true when AUTH_ENABLED", () => {
    expect(authRequired({ NEXT_PUBLIC_AUTH_ENABLED: "true" })).toBe(true);
  });

  it("isPublicPath treats /login and /api as public, / as protected", () => {
    expect(isPublicPath("/login")).toBe(true);
    expect(isPublicPath("/api/v1/health")).toBe(true);
    expect(isPublicPath("/pdf.worker.min.mjs")).toBe(true);
    expect(isPublicPath("/")).toBe(false);
    expect(isPublicPath("/documents")).toBe(false);
  });

  it("isPublicPath allows the SPEC-158 SSO landing page (single-use ?code= / ?error=)", () => {
    expect(isPublicPath("/auth/callback")).toBe(true);
    expect(isPublicPath("/auth/other")).toBe(false);
  });

  it("isPublicPath allows MCP OAuth discovery paths", () => {
    expect(isPublicPath("/mcp")).toBe(true);
    expect(isPublicPath("/.well-known/oauth-protected-resource")).toBe(true);
    expect(isPublicPath("/.well-known/oauth-protected-resource/mcp")).toBe(true);
    expect(isPublicPath("/.well-known/oauth-authorization-server")).toBe(true);
    expect(isPublicPath("/.well-known/openid-configuration")).toBe(true);
    expect(isPublicPath("/.well-known/mcp/server.json")).toBe(true);
    expect(isPublicPath("/oauth/authorize")).toBe(true);
    expect(isPublicPath("/oauth/token")).toBe(true);
    expect(isPublicPath("/oauth/revoke")).toBe(true);
    expect(isPublicPath("/oauth/register")).toBe(true);
  });

  it("swaggerSlash redirects exact /swagger-ui to trailing slash", () => {
    const res = applySwaggerSlashRedirect(req("/swagger-ui"));
    expect(res).not.toBeNull();
    expect(res!.status).toBe(307);
    expect(res!.headers.get("location")).toBe("http://localhost:3000/swagger-ui/");
  });

  it("swaggerSlash leaves /swagger-ui/ alone", () => {
    expect(applySwaggerSlashRedirect(req("/swagger-ui/"))).toBeNull();
  });

  it("authGuard redirects unauthenticated protected path", () => {
    const res = applyAuthGuard(req("/documents"), {
      NEXT_PUBLIC_AUTH_ENABLED: "true",
    });
    expect(res).not.toBeNull();
    expect(res!.status).toBe(307);
    const loc = res!.headers.get("location")!;
    expect(loc).toContain("/login");
    expect(loc).toContain("redirect=%2Fdocuments");
  });

  it("authGuard allows cookie-bearing request", () => {
    const res = applyAuthGuard(req("/documents", "tok"), {
      NEXT_PUBLIC_AUTH_ENABLED: "true",
    });
    expect(res).toBeNull();
  });

  it("authGuard no-ops when auth off", () => {
    expect(applyAuthGuard(req("/documents"), {})).toBeNull();
  });
});
