import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: string | { defaultValue?: string; name?: string }) => {
      const dv = typeof opts === "string" ? opts : opts?.defaultValue ?? key;
      return typeof opts === "object" && opts?.name ? dv.replace("{{name}}", opts.name) : dv;
    },
  }),
}));

let providers: unknown[] | undefined;
vi.mock("@/hooks/use-sso-providers", () => ({ useSsoProviders: () => ({ data: providers }) }));

import { SsoButtons } from "../sso-buttons";

describe("SsoButtons (SPEC-158)", () => {
  const assign = vi.fn();
  beforeEach(() => {
    assign.mockReset();
    Object.defineProperty(window, "location", { value: { assign }, writable: true });
  });
  afterEach(() => {
    cleanup();
    providers = undefined;
  });

  it("renders nothing when no provider is configured (password-only installs unchanged)", () => {
    providers = [];
    const { container } = render(<SsoButtons />);
    expect(container).toBeEmptyDOMElement();
  });

  it("starts the Keycloak flow with the org hint and a safe redirect via full-page navigation", () => {
    providers = [{ slug: "keycloak", display_name: "Acme SSO", kind: "keycloak", login_path: "x" }];
    render(<SsoButtons redirect="/documents" />);
    fireEvent.change(screen.getByLabelText("Organization (optional)"), { target: { value: " acme " } });
    fireEvent.click(screen.getByTestId("sso-provider-keycloak"));
    expect(assign).toHaveBeenCalledTimes(1);
    const url = new URL(assign.mock.calls[0][0] as string, "http://x");
    expect(url.pathname).toBe("/api/v1/auth/oidc/login");
    expect(url.searchParams.get("provider")).toBe("keycloak");
    expect(url.searchParams.get("org")).toBe("acme");
    expect(url.searchParams.get("redirect")).toBe("/documents");
  });

  it("hides the org field and never sends an org for non-Keycloak providers", () => {
    providers = [{ slug: "google", display_name: "Google", kind: "google", login_path: "x" }];
    render(<SsoButtons defaultOrg="acme" />);
    expect(screen.queryByLabelText("Organization (optional)")).toBeNull();
    fireEvent.click(screen.getByTestId("sso-provider-google"));
    expect(new URL(assign.mock.calls[0][0] as string, "http://x").searchParams.has("org")).toBe(false);
  });
});
