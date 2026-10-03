import { beforeEach, describe, expect, it, vi } from "vitest";

const redeem = vi.fn();
vi.mock("@/lib/api/edgequake/sso", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/edgequake/sso")>("@/lib/api/edgequake/sso");
  return { ...actual, redeemSsoHandoff: (...a: unknown[]) => redeem(...a) };
});

const login = vi.fn();
vi.mock("@/stores/use-auth-store", () => ({ useAuthStore: { getState: () => ({ login }) } }));

const selectTenant = vi.fn();
const selectWorkspace = vi.fn();
vi.mock("@/stores/use-tenant-store", () => ({
  useTenantStore: { getState: () => ({ selectTenant, selectWorkspace }) },
}));

import { completeSsoLogin } from "../complete-sso-login";

const base = {
  access_token: "jwt",
  token_type: "Bearer",
  expires_in: 900,
  user: { username: "alice", role: "user" },
  tenant_id: "t-1",
};

describe("completeSsoLogin (SPEC-158)", () => {
  beforeEach(() => vi.clearAllMocks());

  it("adopts the session and pins the tenant + workspace", async () => {
    redeem.mockResolvedValue({ ...base, workspace_id: "w-1", redirect_after: "/documents" });
    const out = await completeSsoLogin("code-1");
    expect(redeem).toHaveBeenCalledWith("code-1");
    expect(login).toHaveBeenCalledWith(expect.objectContaining({ access_token: "jwt" }));
    expect(selectTenant).toHaveBeenCalledWith("t-1");
    expect(selectWorkspace).toHaveBeenCalledWith("w-1");
    expect(out.redirectTo).toBe("/documents");
  });

  it("defaults to / and refuses absolute or protocol-relative redirects", async () => {
    for (const evil of ["https://evil.example", "//evil.example", "/\\evil", null, undefined]) {
      redeem.mockResolvedValue({ ...base, redirect_after: evil });
      expect((await completeSsoLogin("c")).redirectTo).toBe("/");
    }
    expect(selectWorkspace).not.toHaveBeenCalled();
  });

  it("propagates a failed redemption without touching client state", async () => {
    redeem.mockRejectedValue(new Error("401"));
    await expect(completeSsoLogin("used")).rejects.toThrow("401");
    expect(login).not.toHaveBeenCalled();
    expect(selectTenant).not.toHaveBeenCalled();
  });
});
