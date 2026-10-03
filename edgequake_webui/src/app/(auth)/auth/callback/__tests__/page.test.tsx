import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const replace = vi.fn();
let query = "";
vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace }),
  useSearchParams: () => new URLSearchParams(query),
}));

const complete = vi.fn();
vi.mock("@/lib/auth/complete-sso-login", () => ({ completeSsoLogin: (...a: unknown[]) => complete(...a) }));

import SsoCallbackPage from "../page";

describe("SSO callback page (SPEC-158)", () => {
  afterEach(cleanup);
  beforeEach(() => {
    replace.mockReset();
    complete.mockReset();
  });

  it("redeems the code exactly once and navigates to the validated landing path", async () => {
    query = "code=abc";
    complete.mockResolvedValue({ redirectTo: "/documents" });
    render(<SsoCallbackPage />);
    await waitFor(() => expect(replace).toHaveBeenCalledWith("/documents"));
    expect(complete).toHaveBeenCalledTimes(1);
    expect(complete).toHaveBeenCalledWith("abc");
  });

  it("renders a translated error for a denial code and offers the way back", async () => {
    query = "error=tenant_suspended";
    render(<SsoCallbackPage />);
    expect(await screen.findByRole("alert")).toHaveTextContent("auth.sso.errors.tenantSuspended");
    expect(complete).not.toHaveBeenCalled();
    expect(screen.getByText("auth.sso.backToLogin")).toBeInTheDocument();
  });

  it("offers an organization picker for org_ambiguous", async () => {
    query = "error=org_ambiguous%3Aglobex%2Cacme";
    render(<SsoCallbackPage />);
    expect(await screen.findByTestId("sso-org-picker")).toBeInTheDocument();
    expect(screen.getByText("globex")).toBeInTheDocument();
    expect(screen.getByText("acme")).toBeInTheDocument();
  });

  it("treats a failed redemption (expired / reused code) as invalid_handoff", async () => {
    query = "code=used";
    complete.mockRejectedValue(new Error("401"));
    render(<SsoCallbackPage />);
    expect(await screen.findByRole("alert")).toHaveTextContent("auth.sso.errors.invalidHandoff");
    expect(replace).not.toHaveBeenCalled();
  });

  it("without code or error shows invalid_handoff instead of hanging", async () => {
    query = "";
    render(<SsoCallbackPage />);
    expect(await screen.findByRole("alert")).toHaveTextContent("auth.sso.errors.invalidHandoff");
  });
});
