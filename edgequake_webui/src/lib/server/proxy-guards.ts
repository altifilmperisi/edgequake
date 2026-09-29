/**
 * SPEC-144 / SPEC-083 X-27 — pure proxy helpers (no Next file-convention export).
 *
 * Kept separate from `src/proxy.ts` so unit tests can exercise auth + swagger
 * redirects without loading the Next proxy entrypoint.
 */
import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

export const AUTH_COOKIE = "edgequake_access_token";

export const PUBLIC_PREFIXES = [
  "/login",
  "/api",
  "/_next",
  "/favicon",
  "/e2e-fixtures",
  "/oauth",
] as const;

/** Exact public paths (MCP OAuth discovery must not hit the login wall). */
export const PUBLIC_EXACT_PATHS = ["/mcp"] as const;

/** Prefixes for RFC 9728 / RFC 8414 / MCP registry well-known documents. */
export const PUBLIC_WELL_KNOWN_PREFIXES = [
  "/.well-known/oauth-protected-resource",
  "/.well-known/oauth-authorization-server",
  "/.well-known/openid-configuration",
  "/.well-known/mcp",
] as const;

type EnvLike = Record<string, string | undefined>;

export function authRequired(env: EnvLike = process.env): boolean {
  const authEnabled = env.NEXT_PUBLIC_AUTH_ENABLED === "true";
  const disableDemo = env.NEXT_PUBLIC_DISABLE_DEMO_LOGIN === "true";
  return authEnabled || disableDemo;
}

export function isPublicPath(pathname: string): boolean {
  if (pathname === "/") return false;
  if (pathname === "/pdf.worker.min.mjs") return true;
  if ((PUBLIC_EXACT_PATHS as readonly string[]).includes(pathname)) {
    return true;
  }
  if (
    PUBLIC_WELL_KNOWN_PREFIXES.some(
      (p) => pathname === p || pathname.startsWith(`${p}/`),
    )
  ) {
    return true;
  }
  return PUBLIC_PREFIXES.some(
    (p) => pathname === p || pathname.startsWith(`${p}/`),
  );
}

/** Returns a redirect Response, or null to continue. */
export function applySwaggerSlashRedirect(
  request: NextRequest,
): NextResponse | null {
  if (request.nextUrl.pathname === "/swagger-ui") {
    // Plain URL — NextURL strips trailing slashes when skipTrailingSlashRedirect is set.
    return NextResponse.redirect(new URL("/swagger-ui/", request.url), 307);
  }
  return null;
}

/**
 * Coarse HTML navigation guard. Not a cryptographic authorization boundary —
 * API handlers still enforce JWT/API-key auth.
 *
 * Returns a redirect Response, or null to continue.
 */
export function applyAuthGuard(
  request: NextRequest,
  env: EnvLike = process.env,
): NextResponse | null {
  if (!authRequired(env)) {
    return null;
  }

  const { pathname } = request.nextUrl;
  if (isPublicPath(pathname)) {
    return null;
  }

  const token = request.cookies.get(AUTH_COOKIE)?.value;
  if (!token) {
    const login = new URL("/login", request.url);
    // Preserve query for OAuth authorize return (`/oauth/authorize?...`).
    const redirectTarget = `${pathname}${request.nextUrl.search}`;
    login.searchParams.set("redirect", redirectTarget);
    return NextResponse.redirect(login);
  }

  return null;
}
